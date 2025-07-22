use bevy::prelude::*;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use super::gltf_loader::load_gltf_as_mesh;
use crate::lib::sequence::loader::FileLoadStats;
use baby_shark::mesh::Mesh as BabySharkMesh;

/// Type alias for mesh data result to reduce complexity
type MeshDataResult = Result<(Vec<f32>, Vec<f32>, Vec<f32>, FileLoadStats), String>;

pub struct AsyncStlLoaderPlugin;

impl Plugin for AsyncStlLoaderPlugin {
    fn build(&self, app: &mut App) {
        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        info!(
            "Initializing async STL loader with {} worker threads",
            num_workers
        );

        app.insert_resource(AsyncStlLoader::new(num_workers))
            .add_systems(
                Update,
                (process_completed_loads, update_loading_progress).chain(),
            );
    }
}

/// Handle for tracking async load operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadHandle(u64);

/// Request to load an STL file
#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub path: PathBuf,
    pub handle: LoadHandle,
    pub priority: LoadPriority,
    pub use_fallback: bool,
}

/// Priority for load operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Result of an async load operation
pub struct LoadResult {
    pub handle: LoadHandle,
    pub path: PathBuf,
    pub result: MeshDataResult,
}

/// Status of a load operation
#[derive(Debug, Clone)]
pub enum LoadStatus {
    Queued,
    Loading,
    Completed,
    #[allow(dead_code)]
    Failed(String),
    #[allow(dead_code)]
    Cancelled,
}

/// Internal message for worker threads
enum WorkerMessage {
    Load(LoadRequest),
    Shutdown,
}

/// The main async STL loader resource
#[derive(Resource)]
pub struct AsyncStlLoader {
    // Channels for communication with worker threads
    request_sender: Sender<WorkerMessage>,
    result_receiver: Receiver<LoadResult>,

    // Tracking state
    loading_state: Arc<Mutex<LoadingState>>,
    next_handle: Arc<Mutex<u64>>,

    // Worker thread handles
    workers: Vec<thread::JoinHandle<()>>,
}

struct LoadingState {
    active_loads: HashMap<LoadHandle, LoadStatus>,
    handle_to_path: HashMap<LoadHandle, PathBuf>,
    path_to_handle: HashMap<PathBuf, LoadHandle>,
    completed_loads: VecDeque<(LoadHandle, LoadResult)>,
    queue_size: usize,
}

impl AsyncStlLoader {
    pub fn new(num_workers: usize) -> Self {
        let (request_tx, request_rx) = unbounded();
        let (result_tx, result_rx) = bounded(100);

        let loading_state = Arc::new(Mutex::new(LoadingState {
            active_loads: HashMap::new(),
            handle_to_path: HashMap::new(),
            path_to_handle: HashMap::new(),
            completed_loads: VecDeque::new(),
            queue_size: 0,
        }));

        let next_handle = Arc::new(Mutex::new(0));

        // Spawn worker threads
        let mut workers = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let rx = request_rx.clone();
            let tx = result_tx.clone();
            let state = loading_state.clone();

            let handle = thread::spawn(move || {
                worker_thread(worker_id, rx, tx, state);
            });

            workers.push(handle);
        }

        Self {
            request_sender: request_tx,
            result_receiver: result_rx,
            loading_state,
            next_handle,
            workers,
        }
    }

    /// Queue an STL file for loading
    pub fn queue_load(
        &self,
        path: PathBuf,
        priority: LoadPriority,
        use_fallback: bool,
    ) -> Result<LoadHandle, String> {
        // Check if already loading
        {
            let state = self.loading_state.lock().unwrap();
            if let Some(&existing_handle) = state.path_to_handle.get(&path) {
                return Ok(existing_handle);
            }
        }

        // Generate new handle
        let handle = {
            let mut next = self.next_handle.lock().unwrap();
            let handle = LoadHandle(*next);
            *next += 1;
            handle
        };

        // Create request
        let request = LoadRequest {
            path: path.clone(),
            handle,
            priority,
            use_fallback,
        };

        // Update state
        {
            let mut state = self.loading_state.lock().unwrap();
            state.active_loads.insert(handle, LoadStatus::Queued);
            state.handle_to_path.insert(handle, path.clone());
            state.path_to_handle.insert(path, handle);
            state.queue_size += 1;
        }

        // Send to workers
        self.request_sender
            .send(WorkerMessage::Load(request))
            .map_err(|_| "Failed to queue load request")?;

        Ok(handle)
    }

    /// Get the status of a load operation
    pub fn get_status(&self, handle: LoadHandle) -> Option<LoadStatus> {
        let state = self.loading_state.lock().unwrap();
        state.active_loads.get(&handle).cloned()
    }

    /// Cancel a pending load operation
    #[allow(dead_code)]
    pub fn cancel(&self, handle: LoadHandle) -> bool {
        let mut state = self.loading_state.lock().unwrap();

        if let Some(status) = state.active_loads.get_mut(&handle) {
            match status {
                LoadStatus::Queued => {
                    *status = LoadStatus::Cancelled;
                    state.queue_size = state.queue_size.saturating_sub(1);
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Poll for completed loads
    pub fn poll_completed(&self) -> Vec<(LoadHandle, LoadResult)> {
        // First, receive any new results from workers
        while let Ok(result) = self.result_receiver.try_recv() {
            let handle = result.handle;

            let mut state = self.loading_state.lock().unwrap();

            // Update status
            match &result.result {
                Ok(_) => {
                    state.active_loads.insert(handle, LoadStatus::Completed);
                }
                Err(e) => {
                    state
                        .active_loads
                        .insert(handle, LoadStatus::Failed(e.clone()));
                }
            }

            state.completed_loads.push_back((handle, result));
            state.queue_size = state.queue_size.saturating_sub(1);
        }

        // Return all completed loads
        let mut state = self.loading_state.lock().unwrap();
        state.completed_loads.drain(..).collect()
    }

    /// Get loading statistics
    pub fn stats(&self) -> LoaderStats {
        let state = self.loading_state.lock().unwrap();

        let mut stats = LoaderStats {
            queued: 0,
            loading: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            total_active: state.active_loads.len(),
        };

        for status in state.active_loads.values() {
            match status {
                LoadStatus::Queued => stats.queued += 1,
                LoadStatus::Loading => stats.loading += 1,
                LoadStatus::Completed => stats.completed += 1,
                LoadStatus::Failed(_) => stats.failed += 1,
                LoadStatus::Cancelled => stats.cancelled += 1,
            }
        }

        stats
    }

    /// Shutdown the loader (called on drop)
    pub fn shutdown(&mut self) {
        // Send shutdown signal to all workers
        for _ in &self.workers {
            let _ = self.request_sender.send(WorkerMessage::Shutdown);
        }

        // Wait for workers to finish
        while let Some(worker) = self.workers.pop() {
            let _ = worker.join();
        }
    }
}

impl Drop for AsyncStlLoader {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Debug, Clone)]
pub struct LoaderStats {
    pub queued: usize,
    pub loading: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total_active: usize,
}

/// Worker thread function
fn worker_thread(
    id: usize,
    receiver: Receiver<WorkerMessage>,
    sender: Sender<LoadResult>,
    state: Arc<Mutex<LoadingState>>,
) {
    debug!("Worker thread {} started", id);

    // Priority queue for requests
    let mut queue: Vec<LoadRequest> = Vec::new();

    loop {
        // Try to get requests, with timeout to check queue
        match receiver.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(WorkerMessage::Load(request)) => {
                queue.push(request);
            }
            Ok(WorkerMessage::Shutdown) => {
                debug!("Worker thread {} shutting down", id);
                break;
            }
            Err(_) => {
                // Timeout - process queue if not empty
            }
        }

        // Drain additional requests without blocking
        while let Ok(msg) = receiver.try_recv() {
            match msg {
                WorkerMessage::Load(request) => queue.push(request),
                WorkerMessage::Shutdown => {
                    debug!("Worker thread {} shutting down", id);
                    return;
                }
            }
        }

        // Sort by priority (highest first)
        queue.sort_by_key(|r| std::cmp::Reverse(r.priority));

        // Process highest priority request
        if let Some(request) = queue.pop() {
            // Check if cancelled
            let should_process = {
                let state = state.lock().unwrap();
                matches!(
                    state.active_loads.get(&request.handle),
                    Some(LoadStatus::Queued)
                )
            };

            if !should_process {
                continue;
            }

            // Update status to loading
            {
                let mut state = state.lock().unwrap();
                state
                    .active_loads
                    .insert(request.handle, LoadStatus::Loading);
            }

            debug!("Worker {} loading: {:?}", id, request.path);

            // Perform the actual loading
            let result = load_stl_parallel(&request.path, request.use_fallback);

            // Send result
            let load_result = LoadResult {
                handle: request.handle,
                path: request.path,
                result,
            };

            if sender.send(load_result).is_err() {
                error!("Worker {} failed to send result", id);
                break;
            }
        }
    }

    debug!("Worker thread {} exited", id);
}

/// Parallel mesh loading implementation (supports STL and glTF/GLB)
fn load_stl_parallel(path: &Path, use_fallback: bool) -> MeshDataResult {
    use std::fs::File;
    use std::io::BufReader;

    // Check file extension to determine format
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "gltf" | "glb" => {
            // Load as glTF/GLB
            let (mesh, _material) = load_gltf_as_mesh(path)?;

            // Extract mesh data
            let positions: Vec<f32> = match mesh.attribute(bevy::prelude::Mesh::ATTRIBUTE_POSITION)
            {
                Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos)) => {
                    pos.iter().flatten().copied().collect()
                }
                _ => return Err("Failed to extract positions from glTF mesh".to_string()),
            };

            let normals: Vec<f32> = match mesh.attribute(bevy::prelude::Mesh::ATTRIBUTE_NORMAL) {
                Some(bevy::render::mesh::VertexAttributeValues::Float32x3(norm)) => {
                    norm.iter().flatten().copied().collect()
                }
                _ => [0.0, 1.0, 0.0].repeat(positions.len() / 3), // Default normals
            };

            let uvs = match mesh.attribute(bevy::prelude::Mesh::ATTRIBUTE_UV_0) {
                Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uv)) => {
                    uv.iter().flatten().copied().collect()
                }
                _ => [0.0, 0.0].repeat(positions.len() / 3), // Default UVs
            };

            let stats = FileLoadStats {
                faces_processed: positions.len() / 9, // 9 floats per triangle (3 vertices * 3 coords)
                faces_skipped: 0,
            };

            Ok((positions, normals, uvs, stats))
        }
        "stl" => {
            // Original STL loading code
            let file =
                File::open(path).map_err(|e| format!("Failed to open STL file {path:?}: {e}"))?;
            let mut reader = BufReader::new(file);

            // Read STL file
            let stl = stl_io::read_stl(&mut reader)
                .map_err(|e| format!("Failed to parse STL file: {e}"))?;

            // Validate
            if stl.faces.is_empty() {
                return Err("STL file contains no faces".into());
            }

            // Convert IndexedMesh to raw vertex data
            let mut positions = Vec::with_capacity(stl.faces.len() * 9);
            let mut normals = Vec::with_capacity(stl.faces.len() * 9);
            let mut uvs = Vec::with_capacity(stl.faces.len() * 6);

            for face in &stl.faces {
                // Add vertices for this face
                for &vertex_idx in &face.vertices {
                    let vertex = &stl.vertices[vertex_idx];
                    positions.push(vertex[0]);
                    positions.push(vertex[1]);
                    positions.push(vertex[2]);

                    // Add normal (same for all vertices of a face)
                    normals.push(face.normal[0]);
                    normals.push(face.normal[1]);
                    normals.push(face.normal[2]);

                    // Simple UV mapping
                    uvs.push(0.0);
                    uvs.push(0.0);
                }
            }

            let stats = FileLoadStats {
                faces_processed: stl.faces.len(),
                faces_skipped: 0,
            };

            Ok((positions, normals, uvs, stats))
        }
        _ => {
            if use_fallback {
                // Try to load as STL anyway
                let file =
                    File::open(path).map_err(|e| format!("Failed to open file {path:?}: {e}"))?;
                let mut reader = BufReader::new(file);

                // Read STL file
                let stl = stl_io::read_stl(&mut reader)
                    .map_err(|e| format!("Failed to parse file as STL: {e}"))?;

                // Validate
                if stl.faces.is_empty() {
                    return Err("File contains no faces".into());
                }

                // Convert IndexedMesh to raw vertex data
                let mut positions = Vec::with_capacity(stl.faces.len() * 9);
                let mut normals = Vec::with_capacity(stl.faces.len() * 9);
                let mut uvs = Vec::with_capacity(stl.faces.len() * 6);

                for face in &stl.faces {
                    // Add vertices for this face
                    for &vertex_idx in &face.vertices {
                        let vertex = &stl.vertices[vertex_idx];
                        positions.push(vertex[0]);
                        positions.push(vertex[1]);
                        positions.push(vertex[2]);

                        // Add normal (same for all vertices of a face)
                        normals.push(face.normal[0]);
                        normals.push(face.normal[1]);
                        normals.push(face.normal[2]);

                        // Simple UV mapping
                        uvs.push(0.0);
                        uvs.push(0.0);
                    }
                }

                let stats = FileLoadStats {
                    faces_processed: stl.faces.len(),
                    faces_skipped: 0,
                };

                Ok((positions, normals, uvs, stats))
            } else {
                Err(format!("Unsupported file format: {extension}"))
            }
        }
    }
}

/// Create fallback mesh data
#[allow(dead_code)]
fn create_fallback_mesh_data() -> (Vec<f32>, Vec<f32>, Vec<f32>, FileLoadStats) {
    // Simple cube
    let positions = vec![
        // Front
        -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0,
        1.0,
        // Back (similarly for other faces...)
    ];

    let normals = vec![
        // Front faces
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
    ];

    let uvs = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];

    let stats = FileLoadStats {
        faces_processed: 2,
        faces_skipped: 0,
    };

    (positions, normals, uvs, stats)
}

// Events for loader progress
#[derive(Event)]
pub struct LoadCompleteEvent {
    #[allow(dead_code)]
    pub handle: LoadHandle,
    pub path: PathBuf,
    pub success: bool,
}

/// System to process completed loads and create mesh assets
fn process_completed_loads(
    loader: Res<AsyncStlLoader>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut events: EventWriter<LoadCompleteEvent>,
    mut mesh_cache: ResMut<crate::lib::sequence::async_cache::AsyncMeshCache>,
) {
    for (handle, result) in loader.poll_completed() {
        match result.result {
            Ok((positions, normals, uvs, stats)) => {
                // Reshape flat arrays into proper format
                let positions: Vec<[f32; 3]> = positions
                    .chunks_exact(3)
                    .map(|c| [c[0], c[1], c[2]])
                    .collect();

                let _normals: Vec<[f32; 3]> = normals
                    .chunks_exact(3)
                    .map(|c| [c[0], c[1], c[2]])
                    .collect();

                let _uvs: Vec<[f32; 2]> = uvs.chunks_exact(2).map(|c| [c[0], c[1]]).collect();

                // Use baby_shark for mesh optimization with vertex deduplication
                let baby_shark_mesh =
                    BabySharkMesh::from_iter(positions.iter().flat_map(|&[x, y, z]| [x, y, z]));
                // baby_shark handles normal and UV computation automatically
                let mesh: Mesh = baby_shark_mesh.into();

                let mesh_handle = meshes.add(mesh);

                // Update cache
                if let Some(path) = loader
                    .loading_state
                    .lock()
                    .unwrap()
                    .handle_to_path
                    .get(&handle)
                {
                    mesh_cache.insert(path.clone(), mesh_handle, stats);
                }

                events.write(LoadCompleteEvent {
                    handle,
                    path: result.path,
                    success: true,
                });
            }
            Err(error) => {
                error!("Failed to load {:?}: {}", result.path, error);

                if let Some(path) = loader
                    .loading_state
                    .lock()
                    .unwrap()
                    .handle_to_path
                    .get(&handle)
                {
                    mesh_cache.mark_failed(path.clone());
                }

                events.write(LoadCompleteEvent {
                    handle,
                    path: result.path,
                    success: false,
                });
            }
        }
    }
}

/// System to update loading progress UI
fn update_loading_progress(
    loader: Res<AsyncStlLoader>,
    mut loading_state: ResMut<crate::lib::sequence::loader::LoadingState>,
) {
    let stats = loader.stats();

    if stats.total_active > 0 {
        loading_state.is_preloading = true;
        loading_state.total_frames = stats.total_active;
        loading_state.frames_loaded = stats.completed;

        if stats.queued == 0 && stats.loading == 0 {
            // All loads complete
            loading_state.finish_preloading();
        }
    }
}

// Re-export for convenience
