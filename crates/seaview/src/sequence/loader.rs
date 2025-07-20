//! Sequence loader module for efficient mesh loading with caching

use super::{SequenceEvent, SequenceManager};
use bevy::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Plugin for sequence loading functionality
pub struct SequenceLoaderPlugin;

impl Plugin for SequenceLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeshCache>()
            .init_resource::<LoaderConfig>()
            .init_resource::<LoadingState>()
            .add_systems(
                Update,
                (
                    preload_sequence_meshes,
                    handle_frame_changes,
                    update_cache_stats,
                )
                    .chain(),
            );
    }
}

/// Configuration for the sequence loader
#[derive(Resource)]
pub struct LoaderConfig {
    /// Maximum number of meshes to keep in cache
    pub cache_size: usize,
    /// Number of frames to prefetch ahead
    #[allow(dead_code)]
    pub prefetch_ahead: usize,
    /// Number of frames to keep behind
    #[allow(dead_code)]
    pub keep_behind: usize,
    /// Whether to enable async loading
    #[allow(dead_code)]
    pub async_loading: bool,
    /// Whether to use fallback mesh for failed loads
    pub use_fallback_mesh: bool,
    /// Whether to automatically fix inverted normals
    #[allow(dead_code)]
    pub fix_inverted_normals: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            cache_size: 100, // Increased for large sequences
            prefetch_ahead: 10,
            keep_behind: 5,
            async_loading: true,
            use_fallback_mesh: true,
            fix_inverted_normals: true,
        }
    }
}

/// State for tracking sequence preloading
#[derive(Resource, Default)]
pub struct LoadingState {
    /// Whether preloading is active
    pub is_preloading: bool,
    /// Total frames to preload
    pub total_frames: usize,
    /// Number of frames loaded
    pub frames_loaded: usize,
    /// Frames currently being loaded
    pub loading_queue: Vec<PathBuf>,
    /// Start time of preloading
    pub start_time: Option<std::time::Instant>,
    /// Handles to loading assets
    loading_handles: HashMap<PathBuf, Handle<Mesh>>,
}

impl LoadingState {
    pub fn start_preloading(&mut self, total_frames: usize) {
        self.is_preloading = true;
        self.total_frames = total_frames;
        self.frames_loaded = 0;
        self.loading_queue.clear();
        self.loading_handles.clear();
        self.start_time = Some(std::time::Instant::now());
        info!("Starting preload of {} frames", total_frames);
    }

    pub fn finish_preloading(&mut self) {
        self.is_preloading = false;
        if let Some(start) = self.start_time {
            let duration = start.elapsed();
            info!(
                "Preloading complete: {} frames in {:.2}s ({:.1} fps)",
                self.frames_loaded,
                duration.as_secs_f64(),
                self.frames_loaded as f64 / duration.as_secs_f64()
            );
        }
    }

    pub fn progress(&self) -> f32 {
        if self.total_frames > 0 {
            self.frames_loaded as f32 / self.total_frames as f32
        } else {
            0.0
        }
    }

    pub fn progress_text(&self) -> String {
        if self.is_preloading {
            format!(
                "Loading: {}/{} ({:.0}%)",
                self.frames_loaded,
                self.total_frames,
                self.progress() * 100.0
            )
        } else if self.frames_loaded > 0 {
            format!("Loaded: {} frames", self.frames_loaded)
        } else {
            "Ready".to_string()
        }
    }
}

/// Cache for loaded meshes
#[derive(Resource, Default)]
pub struct MeshCache {
    /// Map from file path to cached mesh handle
    cache: HashMap<PathBuf, Handle<Mesh>>,
    /// Access order for LRU eviction
    access_order: Vec<PathBuf>,
    /// Current mesh entity
    pub current_mesh_entity: Option<Entity>,
    /// Material handle for all meshes
    material_handle: Option<Handle<StandardMaterial>>,
    /// Track the last displayed frame to avoid redundant updates
    last_displayed_frame: Option<usize>,
    /// Statistics for tracking loading issues
    stats: LoadingStats,
}

/// Statistics for tracking mesh loading
#[derive(Default, Debug)]
pub struct LoadingStats {
    /// Total files attempted to load
    pub total_attempts: usize,
    /// Successfully loaded files
    pub successful_loads: usize,
    /// Files that failed to load
    pub failed_loads: usize,
    /// Files that used fallback mesh
    pub fallback_used: usize,
    /// Total faces processed
    pub total_faces_processed: usize,
    /// Total faces skipped due to errors
    pub total_faces_skipped: usize,
}

impl MeshCache {
    /// Get or load a mesh from cache
    pub fn get_or_load(
        &mut self,
        path: &PathBuf,
        meshes: &mut Assets<Mesh>,
        use_fallback: bool,
    ) -> Option<Handle<Mesh>> {
        // Update access order
        if let Some(pos) = self.access_order.iter().position(|p| p == path) {
            self.access_order.remove(pos);
        }
        self.access_order.push(path.clone());

        // Check if already cached
        if let Some(handle) = self.cache.get(path) {
            return Some(handle.clone());
        }

        // Track loading attempt
        self.stats.total_attempts += 1;

        // Load the mesh
        match load_stl_file_optimized(path) {
            Ok((mesh, stats)) => {
                let handle = meshes.add(mesh);
                self.cache.insert(path.clone(), handle.clone());
                self.stats.successful_loads += 1;
                self.stats.total_faces_processed += stats.faces_processed;
                self.stats.total_faces_skipped += stats.faces_skipped;

                if stats.faces_skipped > 0 {
                    let skip_percentage = (stats.faces_skipped as f32
                        / (stats.faces_processed + stats.faces_skipped) as f32)
                        * 100.0;
                    info!(
                        "Loaded {:?} with {:.1}% faces skipped",
                        path.file_name().unwrap_or_default(),
                        skip_percentage
                    );
                }

                Some(handle)
            }
            Err(e) => {
                self.stats.failed_loads += 1;
                warn!("Failed to load STL file {:?}: {}", path, e);

                if use_fallback {
                    warn!("Using fallback mesh for {:?}", path);
                    let fallback_mesh = create_fallback_mesh();
                    let handle = meshes.add(fallback_mesh);
                    self.cache.insert(path.clone(), handle.clone());
                    self.stats.fallback_used += 1;
                    Some(handle)
                } else {
                    None
                }
            }
        }
    }

    /// Check if a mesh is loaded and ready
    #[allow(dead_code)]
    pub fn is_loaded(&self, path: &PathBuf, meshes: &Assets<Mesh>) -> bool {
        self.cache
            .get(path)
            .map(|handle| meshes.get(handle).is_some())
            .unwrap_or(false)
    }

    /// Get or create material handle
    pub fn get_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = &self.material_handle {
            handle.clone()
        } else {
            // Log material creation for debugging
            info!("Creating material with double-sided rendering enabled");
            let handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.9, 0.9, 0.9),
                metallic: 0.0,
                perceptual_roughness: 0.5,
                reflectance: 0.3,
                double_sided: true, // Enable double-sided rendering to debug normal issues
                cull_mode: None,    // Disable culling temporarily to see all faces
                unlit: false,       // Ensure the material is lit
                ..default()
            });
            self.material_handle = Some(handle.clone());
            handle
        }
    }

    /// Evict least recently used meshes to stay within cache size
    pub fn evict_lru(&mut self, max_size: usize) {
        while self.cache.len() > max_size && !self.access_order.is_empty() {
            if let Some(path) = self.access_order.first().cloned() {
                self.cache.remove(&path);
                self.access_order.remove(0);
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> &LoadingStats {
        &self.stats
    }
}

/// System to preload all sequence meshes at startup
fn preload_sequence_meshes(
    sequence_manager: Res<SequenceManager>,
    mut loading_state: ResMut<LoadingState>,
    mut mesh_cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    config: Res<LoaderConfig>,
) {
    // Check if we need to start preloading
    if !loading_state.is_preloading {
        if let Some(sequence) = &sequence_manager.current_sequence {
            if loading_state.total_frames == 0 {
                // Initialize preloading
                loading_state.start_preloading(sequence.frame_count());

                // Queue all frames for loading
                for i in 0..sequence.frame_count() {
                    if let Some(path) = sequence.frame_path(i) {
                        loading_state.loading_queue.push(path.to_path_buf());
                    }
                }
            }
        }
        return;
    }

    // Start loading assets
    let paths_to_load: Vec<_> = loading_state
        .loading_queue
        .iter()
        .filter(|path| !loading_state.loading_handles.contains_key(*path))
        .cloned()
        .collect();

    for path in paths_to_load {
        if let Some(handle) = mesh_cache.get_or_load(&path, &mut meshes, config.use_fallback_mesh) {
            loading_state.loading_handles.insert(path, handle);
        }
    }

    // Check for completed loads
    let mut completed_paths = Vec::new();
    for (path, handle) in &loading_state.loading_handles {
        if meshes.get(handle).is_some() {
            completed_paths.push(path.clone());
        }
    }

    // Remove completed from loading handles and update progress
    for path in completed_paths {
        loading_state.loading_handles.remove(&path);
        loading_state.frames_loaded += 1;

        // Log progress every 10%
        let progress = loading_state.progress();
        if (progress * 10.0) as u32 > ((progress - 0.1) * 10.0) as u32 {
            info!("{}", loading_state.progress_text());
        }
    }

    // Check if preloading is complete
    if loading_state.frames_loaded >= loading_state.total_frames {
        loading_state.finish_preloading();

        // Ensure cache size is respected
        mesh_cache.evict_lru(config.cache_size);
    }
}

/// System that handles frame changes and loads new meshes
#[allow(clippy::too_many_arguments)]
fn handle_frame_changes(
    mut commands: Commands,
    mut sequence_manager: ResMut<SequenceManager>,
    mut mesh_cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<SequenceEvent>,
    time: Res<Time>,
    loading_state: Res<LoadingState>,
) {
    // Don't update frames while preloading
    if loading_state.is_preloading {
        return;
    }

    // Handle playback
    if sequence_manager.is_playing {
        let delta = time.delta_secs();
        let frame_duration = 1.0 / sequence_manager.playback_fps;

        // Accumulate time
        sequence_manager.frame_timer += delta;

        // Check if it's time to advance to the next frame
        if sequence_manager.frame_timer >= frame_duration {
            // Reset timer, keeping any excess time
            sequence_manager.frame_timer -= frame_duration;

            if !sequence_manager.next_frame() {
                // Loop back to start
                sequence_manager.jump_to_frame(0);
            }
            events.write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
        }
    } else {
        // Reset timer when not playing
        sequence_manager.frame_timer = 0.0;
    }

    // Check if we need to update the mesh (only when frame changes)
    let current_frame = sequence_manager.current_frame;
    if mesh_cache.last_displayed_frame == Some(current_frame) {
        // Frame hasn't changed, no need to update mesh
        return;
    }

    // Load current frame mesh
    if let Some(path) = sequence_manager.current_frame_path() {
        info!(
            "Frame changed to {}, updating mesh from: {:?}",
            current_frame, path
        );

        // Always try to get or load the mesh for the current frame
        if let Some(mesh_handle) = mesh_cache.get_or_load(path, &mut meshes, false) {
            // Remove old mesh entity
            if let Some(entity) = mesh_cache.current_mesh_entity {
                info!("Despawning old mesh entity: {:?}", entity);
                commands.entity(entity).despawn();
            }

            // Get material
            let material_handle = mesh_cache.get_material(&mut materials);

            // Spawn new mesh entity
            let transform = if let Some(sequence) = &sequence_manager.current_sequence {
                sequence.source_orientation.to_transform()
            } else {
                Transform::IDENTITY
            };

            let entity = commands
                .spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material_handle),
                    transform,
                    Name::new(format!("Frame {}", sequence_manager.current_frame)),
                ))
                .id();

            info!(
                "Spawned new mesh entity: {:?} for frame {}",
                entity, sequence_manager.current_frame
            );
            mesh_cache.current_mesh_entity = Some(entity);
            mesh_cache.last_displayed_frame = Some(current_frame);
        } else {
            warn!("Failed to get mesh handle from cache for path: {:?}", path);
        }
    } else {
        warn!(
            "No path for current frame {}",
            sequence_manager.current_frame
        );
    }
}

/// System to update cache statistics
fn update_cache_stats(
    mesh_cache: Res<MeshCache>,
    _loading_state: Res<LoadingState>,
    _config: Res<LoaderConfig>,
) {
    static mut LAST_LOG: f64 = 0.0;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    unsafe {
        if now - LAST_LOG > 5.0 {
            LAST_LOG = now;
            let stats = mesh_cache.stats();
            if stats.total_attempts > 0 {
                let success_rate =
                    (stats.successful_loads as f32 / stats.total_attempts as f32) * 100.0;
                info!(
                    "Cache stats: {} meshes cached, {:.1}% success rate ({}/{} loaded, {} fallbacks)",
                    mesh_cache.cache.len(),
                    success_rate,
                    stats.successful_loads,
                    stats.total_attempts,
                    stats.fallback_used
                );

                if stats.total_faces_skipped > 0 {
                    debug!(
                        "Face processing: {} processed, {} skipped ({:.1}% skip rate)",
                        stats.total_faces_processed,
                        stats.total_faces_skipped,
                        (stats.total_faces_skipped as f32
                            / (stats.total_faces_processed + stats.total_faces_skipped) as f32)
                            * 100.0
                    );
                }
            }
        }
    }
}

/// Event for loading progress updates
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LoadingProgressEvent {
    pub current: usize,
    pub total: usize,
    pub percentage: f32,
}

/// Structure to hold loading progress information
#[allow(dead_code)]
pub struct LoadingProgress {
    pub current: usize,
    pub total: usize,
    pub path: PathBuf,
}

/// Statistics from loading a single STL file
pub struct FileLoadStats {
    pub faces_processed: usize,
    pub faces_skipped: usize,
}

/// Load an STL file from disk and convert it to a Bevy mesh
/// Handles various malformed STL files gracefully
/// Returns the mesh and statistics about the loading process
pub fn load_stl_file_optimized(
    path: &Path,
) -> Result<(Mesh, FileLoadStats), Box<dyn std::error::Error>> {
    // Try to open the file
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("Failed to open STL file {:?}: {}", path, e).into());
        }
    };
    let mut reader = BufReader::new(file);

    // Read STL file
    let stl = match stl_io::read_stl(&mut reader) {
        Ok(stl) => stl,
        Err(e) => {
            // Try to provide more helpful error messages
            let error_msg = format!("Failed to parse STL file: {}", e);
            return Err(error_msg.into());
        }
    };

    // Validate STL data
    if stl.faces.is_empty() {
        return Err("STL file contains no faces".into());
    }

    if stl.vertices.is_empty() {
        return Err("STL file contains no vertices".into());
    }

    // Check for reasonable bounds
    let vertex_count = stl.vertices.len();
    let face_count = stl.faces.len();

    if vertex_count > 10_000_000 {
        warn!(
            "STL file {:?} has {} vertices, which is unusually large",
            path, vertex_count
        );
    }

    if face_count > 10_000_000 {
        warn!(
            "STL file {:?} has {} faces, which is unusually large",
            path, face_count
        );
    }

    // Convert STL triangles to Bevy mesh
    let mut positions = Vec::with_capacity(stl.faces.len() * 3);
    let mut normals = Vec::with_capacity(stl.faces.len() * 3);
    let mut uvs = Vec::with_capacity(stl.faces.len() * 3);

    let mut valid_faces = 0;
    let mut skipped_faces = 0;
    let mut inverted_normals = 0;

    for (face_idx, face) in stl.faces.iter().enumerate() {
        // Validate vertex indices
        let mut valid_face = true;
        for &vertex_idx in &face.vertices {
            if vertex_idx >= stl.vertices.len() {
                warn!(
                    "Face {} in {:?} has invalid vertex index {} (max: {})",
                    face_idx,
                    path,
                    vertex_idx,
                    stl.vertices.len() - 1
                );
                valid_face = false;
                break;
            }
        }

        if !valid_face {
            skipped_faces += 1;
            continue;
        }

        // Get the three vertices of the triangle
        let v0 = &stl.vertices[face.vertices[0]];
        let v1 = &stl.vertices[face.vertices[1]];
        let v2 = &stl.vertices[face.vertices[2]];

        // Convert to arrays and validate for NaN/Inf
        let p0 = [v0[0], v0[1], v0[2]];
        let p1 = [v1[0], v1[1], v1[2]];
        let p2 = [v2[0], v2[1], v2[2]];

        // Check for invalid coordinates
        let coords_valid = [&p0, &p1, &p2]
            .iter()
            .all(|p| p.iter().all(|&coord| coord.is_finite()));

        if !coords_valid {
            warn!(
                "Face {} in {:?} contains non-finite coordinates (NaN or Inf)",
                face_idx, path
            );
            skipped_faces += 1;
            continue;
        }

        // Check for degenerate triangles (zero area)
        let edge1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let edge2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

        // Cross product for normal
        let mut normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        // Calculate length for normalization and degeneracy check
        let len_squared = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];

        // Skip degenerate triangles (collinear points)
        if len_squared < 1e-10 {
            debug!(
                "Face {} in {:?} is degenerate (collinear vertices)",
                face_idx, path
            );
            skipped_faces += 1;
            continue;
        }

        // Normalize
        let len = len_squared.sqrt();
        normal = [normal[0] / len, normal[1] / len, normal[2] / len];

        // Check if the face normal from the STL file agrees with our calculated normal
        // STL face normal is not optional in stl_io crate
        let stl_normal = face.normal;
        let dot_product =
            normal[0] * stl_normal[0] + normal[1] * stl_normal[1] + normal[2] * stl_normal[2];

        // If the dot product is negative, the normals point in opposite directions
        if dot_product < -0.5 {
            // The winding order is likely inverted
            inverted_normals += 1;
            // Flip the normal
            normal = [-normal[0], -normal[1], -normal[2]];

            // Also swap vertex order to fix winding
            positions.push(p0);
            positions.push(p2); // Swapped
            positions.push(p1); // Swapped

            normals.push(normal);
            normals.push(normal);
            normals.push(normal);

            uvs.push([0.0, 0.0]);
            uvs.push([0.5, 1.0]); // Swapped
            uvs.push([1.0, 0.0]); // Swapped

            valid_faces += 1;
            continue;
        }

        // Add vertices with calculated normal
        positions.push(p0);
        positions.push(p1);
        positions.push(p2);

        normals.push(normal);
        normals.push(normal);
        normals.push(normal);

        // Simple UV mapping
        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 0.0]);
        uvs.push([0.5, 1.0]);

        valid_faces += 1;
    }

    // Log if we detected and fixed inverted normals
    if inverted_normals > 0 {
        let invert_percentage = (inverted_normals as f32 / valid_faces as f32) * 100.0;
        warn!(
            "Fixed {} inverted normals ({:.1}% of valid faces) in {:?}",
            inverted_normals, invert_percentage, path
        );
    }

    // Always log face statistics for debugging
    info!(
        "Loaded mesh from {:?}: {} valid faces, {} skipped, {} inverted normals fixed",
        path.file_name().unwrap_or_default(),
        valid_faces,
        skipped_faces,
        inverted_normals
    );

    // Check if we have any valid faces
    if valid_faces == 0 {
        return Err(format!(
            "No valid faces found in STL file (skipped {} invalid faces)",
            skipped_faces
        )
        .into());
    }

    // Log statistics if we skipped any faces
    if skipped_faces > 0 {
        let skip_percentage = (skipped_faces as f32 / stl.faces.len() as f32) * 100.0;
        if skip_percentage > 50.0 {
            error!(
                "STL file {:?} is severely corrupted: {:.1}% of faces are invalid ({} valid, {} skipped)",
                path, skip_percentage, valid_faces, skipped_faces
            );
        } else {
            warn!(
                "Loaded {:?}: {} valid faces, {} skipped ({:.1}% invalid)",
                path, valid_faces, skipped_faces, skip_percentage
            );
        }
    } else {
        debug!("Successfully loaded {:?}: {} faces", path, valid_faces);
    }

    // Create mesh
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let stats = FileLoadStats {
        faces_processed: valid_faces,
        faces_skipped: skipped_faces,
    };

    Ok((mesh, stats))
}

/// Create a fallback mesh when STL loading fails
pub fn create_fallback_mesh() -> Mesh {
    // Create a simple cube as fallback
    let size = 1.0;
    let vertices = vec![
        // Front face
        ([-size, -size, size], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([size, -size, size], [0.0, 0.0, 1.0], [1.0, 0.0]),
        ([size, size, size], [0.0, 0.0, 1.0], [1.0, 1.0]),
        ([-size, size, size], [0.0, 0.0, 1.0], [0.0, 1.0]),
        // Back face
        ([size, -size, -size], [0.0, 0.0, -1.0], [0.0, 0.0]),
        ([-size, -size, -size], [0.0, 0.0, -1.0], [1.0, 0.0]),
        ([-size, size, -size], [0.0, 0.0, -1.0], [1.0, 1.0]),
        ([size, size, -size], [0.0, 0.0, -1.0], [0.0, 1.0]),
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
    ];

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for (position, normal, uv) in &vertices {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}
