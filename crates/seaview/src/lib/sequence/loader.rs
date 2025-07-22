//! Sequence loader module for efficient mesh loading with caching

use super::{SequenceEvent, SequenceManager};
use crate::lib::parallel_loader::{AsyncStlLoader, LoadCompleteEvent, LoadPriority};
use crate::lib::sequence::async_cache::{log_cache_stats, update_cache_from_loads, AsyncMeshCache};
use baby_shark::mesh::Mesh as BabySharkMesh;
use bevy::prelude::*;
use nalgebra::Vector3;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Plugin for sequence loading functionality
pub struct SequenceLoaderPlugin;

impl Plugin for SequenceLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AsyncMeshCache>()
            .init_resource::<LoaderConfig>()
            .init_resource::<LoadingState>()
            .add_event::<LoadCompleteEvent>()
            .add_systems(
                Update,
                (
                    preload_sequence_meshes,
                    update_cache_from_loads,
                    handle_frame_changes,
                    log_cache_stats,
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

    #[allow(dead_code)]
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

// MeshCache has been replaced by AsyncMeshCache in async_cache.rs

/// Statistics for tracking mesh loading
/// Statistics for mesh loading
#[derive(Default, Debug)]
pub struct LoadingStats {
    pub total_attempts: usize,
    pub successful_loads: usize,
    pub failed_loads: usize,
    #[allow(dead_code)]
    pub fallback_used: usize,
    pub total_faces_processed: usize,
    pub total_faces_skipped: usize,
}

/// System to preload all sequence meshes at startup
fn preload_sequence_meshes(
    sequence_manager: Res<SequenceManager>,
    mut loading_state: ResMut<LoadingState>,
    mut mesh_cache: ResMut<AsyncMeshCache>,
    async_loader: Res<AsyncStlLoader>,
    config: Res<LoaderConfig>,
) {
    // Check if we need to start preloading
    if !loading_state.is_preloading {
        if let Some(sequence) = &sequence_manager.current_sequence {
            if loading_state.total_frames == 0 {
                // Initialize preloading
                loading_state.start_preloading(sequence.frame_count());

                // Queue all frames for loading with appropriate priorities
                let current_frame = sequence_manager.current_frame;
                for i in 0..sequence.frame_count() {
                    if let Some(path) = sequence.frame_path(i) {
                        // Determine priority based on distance from current frame
                        let priority = if i == current_frame {
                            LoadPriority::Critical
                        } else if i.abs_diff(current_frame) <= 2 {
                            LoadPriority::High
                        } else if i.abs_diff(current_frame) <= 5 {
                            LoadPriority::Normal
                        } else {
                            LoadPriority::Low
                        };

                        // Queue for async loading
                        if !mesh_cache.is_loaded(&path.to_path_buf())
                            && !mesh_cache.is_loading(&path.to_path_buf())
                        {
                            mesh_cache.get_or_queue(
                                &path.to_path_buf(),
                                &async_loader,
                                priority,
                                config.use_fallback_mesh,
                            );
                        }
                    }
                }
            }
        }
        return;
    }

    // Check loading progress
    let progress = mesh_cache.loading_progress();
    if progress >= 1.0 {
        loading_state.finish_preloading();
        // Ensure cache size is respected
        mesh_cache.evict_lru(config.cache_size);
    } else {
        // Log progress every 10%
        if (progress * 10.0) as u32 > ((loading_state.progress() * 10.0) as u32) {
            info!("Loading progress: {:.1}%", progress * 100.0);
        }
    }
}

/// System that handles frame changes and loads new meshes
#[allow(clippy::too_many_arguments)]
fn handle_frame_changes(
    mut commands: Commands,
    mut sequence_manager: ResMut<SequenceManager>,
    mut mesh_cache: ResMut<AsyncMeshCache>,
    async_loader: Res<AsyncStlLoader>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<SequenceEvent>,
    time: Res<Time>,
    loading_state: Res<LoadingState>,
    config: Res<LoaderConfig>,
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

        // Check if mesh is already cached
        if let Some(mesh_handle) = mesh_cache.cache.get(path).cloned() {
            // Despawn the old mesh entity if it exists
            if let Some(old_entity) = mesh_cache.current_mesh_entity {
                commands.entity(old_entity).despawn();
                debug!("Despawned old mesh entity: {:?}", old_entity);
            }

            // Mesh is already loaded, spawn it
            let material_handle = mesh_cache.get_material(&mut materials);

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
            // Mesh not cached yet, queue it for loading with high priority
            mesh_cache.get_or_queue(
                path,
                &async_loader,
                LoadPriority::Critical,
                config.use_fallback_mesh,
            );
            info!("Queued mesh for loading: {:?}", path);
        }
    } else {
        warn!(
            "No path for current frame {}",
            sequence_manager.current_frame
        );
    }
}

/// System to update cache statistics
// update_cache_stats has been replaced by log_cache_stats from async_cache module
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
#[allow(dead_code)]
pub fn load_stl_file_optimized(
    path: &Path,
) -> Result<(Mesh, FileLoadStats), Box<dyn std::error::Error>> {
    // Try to open the file
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("Failed to open STL file {path:?}: {e}").into());
        }
    };
    let mut reader = BufReader::new(file);

    // Read STL file
    let stl = match stl_io::read_stl(&mut reader) {
        Ok(stl) => stl,
        Err(e) => {
            // Try to provide more helpful error messages
            let error_msg = format!("Failed to parse STL file: {e}");
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

    // Convert STL triangles to baby_shark mesh
    let mut vertices = Vec::with_capacity(stl.faces.len() * 3);
    let mut indices = Vec::with_capacity(stl.faces.len() * 3);
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

            // Add vertices with swapped winding order
            let base_idx = vertices.len();
            vertices.push(Vector3::new(p0[0], p0[1], p0[2]));
            vertices.push(Vector3::new(p2[0], p2[1], p2[2])); // Swapped
            vertices.push(Vector3::new(p1[0], p1[1], p1[2])); // Swapped

            indices.push(base_idx);
            indices.push(base_idx + 1);
            indices.push(base_idx + 2);

            uvs.push([0.0, 0.0]);
            uvs.push([0.5, 1.0]); // Swapped
            uvs.push([1.0, 0.0]); // Swapped

            valid_faces += 1;
            continue;
        }

        // Add vertices with normal winding order
        let base_idx = vertices.len();
        vertices.push(Vector3::new(p0[0], p0[1], p0[2]));
        vertices.push(Vector3::new(p1[0], p1[1], p1[2]));
        vertices.push(Vector3::new(p2[0], p2[1], p2[2]));

        indices.push(base_idx);
        indices.push(base_idx + 1);
        indices.push(base_idx + 2);

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
            "No valid faces found in STL file (skipped {skipped_faces} invalid faces)"
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

    // Create baby_shark mesh and convert to Bevy mesh
    let baby_shark_mesh = BabySharkMesh::new(vertices, indices);
    let mesh: Mesh = baby_shark_mesh.into();

    // Note: baby_shark now handles UV computation automatically

    let stats = FileLoadStats {
        faces_processed: valid_faces,
        faces_skipped: skipped_faces,
    };

    Ok((mesh, stats))
}

/// Create a fallback mesh when STL loading fails
#[allow(dead_code)]
pub fn create_fallback_mesh() -> Mesh {
    // Create a simple cube as fallback using baby_shark
    let size = 1.0;

    // Flat array of triangle vertices for a cube
    let triangle_vertices = vec![
        // Front face (2 triangles)
        -size, -size, size, size, -size, size, size, size, size, size, size, size, -size, size,
        size, -size, -size, size, // Back face (2 triangles)
        size, -size, -size, -size, -size, -size, -size, size, -size, -size, size, -size, size,
        size, -size, size, -size, -size,
    ];

    // Use baby_shark for mesh creation with automatic vertex deduplication
    let baby_shark_mesh = BabySharkMesh::from_iter(triangle_vertices.into_iter());
    // baby_shark handles normal and UV computation in the conversion
    baby_shark_mesh.into()
}
