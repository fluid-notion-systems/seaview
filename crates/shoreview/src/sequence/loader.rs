//! Sequence loader module for efficient mesh loading with caching

use super::{SequenceEvent, SequenceManager};
use crate::systems::stl_loader::StlFilePath;
use bevy::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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
                    prefetch_frames,
                    cleanup_cache,
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
    pub prefetch_ahead: usize,
    /// Number of frames to keep behind
    pub keep_behind: usize,
    /// Whether to enable async loading
    pub async_loading: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            cache_size: 100, // Increased for large sequences
            prefetch_ahead: 10,
            keep_behind: 5,
            async_loading: true,
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
}

impl LoadingState {
    pub fn start_preloading(&mut self, total_frames: usize) {
        self.is_preloading = true;
        self.total_frames = total_frames;
        self.frames_loaded = 0;
        self.loading_queue.clear();
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
    /// Map from file path to cached mesh data
    cache: HashMap<PathBuf, CachedMesh>,
    /// Access order for LRU eviction
    access_order: Vec<PathBuf>,
    /// Current mesh entity
    current_mesh_entity: Option<Entity>,
}

/// Cached mesh data
struct CachedMesh {
    /// Handle to the mesh asset
    mesh_handle: Handle<Mesh>,
    /// Handle to the material
    material_handle: Handle<StandardMaterial>,
    /// Frame number in sequence
    frame_number: usize,
    /// Last access time
    last_accessed: std::time::Instant,
}

impl MeshCache {
    /// Get or load a mesh from cache
    pub fn get_or_load(
        &mut self,
        path: &PathBuf,
        frame_number: usize,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Option<(Handle<Mesh>, Handle<StandardMaterial>)> {
        // Update access order
        if let Some(pos) = self.access_order.iter().position(|p| p == path) {
            self.access_order.remove(pos);
        }
        self.access_order.push(path.clone());

        // Check if already in cache
        if let Some(cached) = self.cache.get_mut(path) {
            cached.last_accessed = std::time::Instant::now();
            return Some((cached.mesh_handle.clone(), cached.material_handle.clone()));
        }

        // Load new mesh
        if let Ok(mesh_data) = load_stl_file(path) {
            let mesh_handle = meshes.add(mesh_data);
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.8, 0.8),
                metallic: 0.1,
                perceptual_roughness: 0.8,
                reflectance: 0.5,
                ..default()
            });

            let cached = CachedMesh {
                mesh_handle: mesh_handle.clone(),
                material_handle: material_handle.clone(),
                frame_number,
                last_accessed: std::time::Instant::now(),
            };

            self.cache.insert(path.clone(), cached);
            Some((mesh_handle, material_handle))
        } else {
            None
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

    /// Clear all cached meshes
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_meshes: self.cache.len(),
            total_memory: self.estimate_memory_usage(),
        }
    }

    fn estimate_memory_usage(&self) -> usize {
        // Rough estimate: assume each mesh uses ~1MB
        self.cache.len() * 1_000_000
    }
}

/// Cache statistics
pub struct CacheStats {
    pub total_meshes: usize,
    pub total_memory: usize,
}

/// System to preload all sequence meshes at startup
fn preload_sequence_meshes(
    mut sequence_manager: ResMut<SequenceManager>,
    mut mesh_cache: ResMut<MeshCache>,
    mut loading_state: ResMut<LoadingState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<LoaderConfig>,
    time: Res<Time>,
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

    // Load frames in batches to avoid blocking
    // Use fewer frames per update for large files to maintain responsiveness
    let frames_per_update = if loading_state.frames_loaded < 5 {
        1 // Load first few frames slowly to gauge performance
    } else {
        2 // Then adjust based on file size
    };
    let mut loaded_this_frame = 0;

    while loaded_this_frame < frames_per_update && !loading_state.loading_queue.is_empty() {
        if let Some(path) = loading_state.loading_queue.pop() {
            let frame_index = loading_state.total_frames - loading_state.loading_queue.len() - 1;

            if mesh_cache
                .get_or_load(&path, frame_index, &mut meshes, &mut materials)
                .is_some()
            {
                loading_state.frames_loaded += 1;
                loaded_this_frame += 1;

                // Log progress every 10%
                let progress = loading_state.progress();
                if (progress * 10.0) as u32 > ((progress - 0.1) * 10.0) as u32 {
                    info!("{}", loading_state.progress_text());
                }
            }
        }
    }

    // Check if preloading is complete
    if loading_state.loading_queue.is_empty() {
        loading_state.finish_preloading();

        // Ensure cache size is respected
        mesh_cache.evict_lru(config.cache_size);
    }
}

/// System that handles frame changes and loads new meshes
fn handle_frame_changes(
    mut commands: Commands,
    mut sequence_manager: ResMut<SequenceManager>,
    mut mesh_cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<SequenceEvent>,
    time: Res<Time>,
    config: Res<LoaderConfig>,
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

        info!(
            "Debug playback: delta={:.3}, fps={}, frame_duration={:.3}, timer={:.3}",
            delta, sequence_manager.playback_fps, frame_duration, sequence_manager.frame_timer
        );

        // Check if it's time to advance to the next frame
        if sequence_manager.frame_timer >= frame_duration {
            info!("Debug: Time to advance frame!");

            // Reset timer, keeping any excess time
            sequence_manager.frame_timer -= frame_duration;

            if !sequence_manager.next_frame() {
                info!("Debug: End of sequence, looping to start");
                // Loop back to start
                sequence_manager.jump_to_frame(0);
            } else {
                info!(
                    "Debug: Advanced to frame {}",
                    sequence_manager.current_frame
                );
            }
            events.send(SequenceEvent::FrameChanged(sequence_manager.current_frame));
        }
    } else {
        // Reset timer when not playing
        sequence_manager.frame_timer = 0.0;
    }

    // Load current frame mesh
    if let Some(path) = sequence_manager.current_frame_path() {
        if let Some((mesh_handle, material_handle)) = mesh_cache.get_or_load(
            path,
            sequence_manager.current_frame,
            &mut meshes,
            &mut materials,
        ) {
            // Remove old mesh entity
            if let Some(entity) = mesh_cache.current_mesh_entity {
                commands.entity(entity).despawn_recursive();
            }

            // Spawn new mesh entity
            let entity = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    Name::new(format!("Frame {}", sequence_manager.current_frame)),
                ))
                .id();

            mesh_cache.current_mesh_entity = Some(entity);
        }
    }

    // Evict old meshes if cache is too large
    mesh_cache.evict_lru(config.cache_size);
}

/// System to prefetch upcoming frames
fn prefetch_frames(
    sequence_manager: Res<SequenceManager>,
    mut mesh_cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<LoaderConfig>,
    loading_state: Res<LoadingState>,
) {
    // Skip prefetching during initial preload
    if loading_state.is_preloading {
        return;
    }
    if let Some(sequence) = sequence_manager.current_sequence() {
        let current = sequence_manager.current_frame;

        // Prefetch ahead
        for i in 1..=config.prefetch_ahead {
            let frame_idx = (current + i).min(sequence.frame_count() - 1);
            if let Some(path) = sequence.frame_path(frame_idx) {
                mesh_cache.get_or_load(path, frame_idx, &mut meshes, &mut materials);
            }
        }

        // Keep some frames behind for scrubbing
        for i in 1..=config.keep_behind {
            if current >= i {
                let frame_idx = current - i;
                if let Some(path) = sequence.frame_path(frame_idx) {
                    mesh_cache.get_or_load(path, frame_idx, &mut meshes, &mut materials);
                }
            }
        }
    }
}

/// System that periodically cleans up the cache
fn cleanup_cache(mut mesh_cache: ResMut<MeshCache>, config: Res<LoaderConfig>, time: Res<Time>) {
    // Run cleanup every 5 seconds
    static mut LAST_CLEANUP: f64 = 0.0;
    let elapsed = time.elapsed_secs_f64();

    unsafe {
        if elapsed - LAST_CLEANUP > 5.0 {
            LAST_CLEANUP = elapsed;
            mesh_cache.evict_lru(config.cache_size);

            let stats = mesh_cache.stats();
            debug!(
                "Cache stats: {} meshes, ~{:.1} MB",
                stats.total_meshes,
                stats.total_memory as f64 / 1_000_000.0
            );
        }
    }
}

/// Load an STL file and return a Bevy mesh
fn load_stl_file(path: &PathBuf) -> Result<Mesh, String> {
    use std::fs::File;
    use stl_io::read_stl;

    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let stl = read_stl(&mut file).map_err(|e| format!("Failed to parse STL: {}", e))?;

    // Convert STL to Bevy mesh (reusing logic from stl_loader)
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    // Extract vertices
    let vertices: Vec<[f32; 3]> = stl.vertices.iter().map(|v| [v[0], v[1], v[2]]).collect();

    // Calculate vertex normals
    let mut vertex_normals: Vec<Vec3> = vec![Vec3::ZERO; vertices.len()];
    let mut vertex_face_count: Vec<u32> = vec![0; vertices.len()];

    for face in &stl.faces {
        let normal = Vec3::new(face.normal[0], face.normal[1], face.normal[2]);
        for &vertex_idx in &face.vertices {
            vertex_normals[vertex_idx] += normal;
            vertex_face_count[vertex_idx] += 1;
        }
    }

    let normals: Vec<[f32; 3]> = vertex_normals
        .iter()
        .zip(vertex_face_count.iter())
        .map(|(normal, &count)| {
            if count > 0 {
                let averaged = normal.normalize();
                [averaged.x, averaged.y, averaged.z]
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect();

    // Simple UV mapping
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .map(|v| [v[0] * 0.5 + 0.5, v[2] * 0.5 + 0.5])
        .collect();

    // Create indices
    let indices: Vec<u32> = stl
        .faces
        .iter()
        .flat_map(|face| face.vertices.iter().map(|&v| v as u32))
        .collect();

    // Set mesh attributes
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    // Generate tangents
    mesh.generate_tangents().ok();

    Ok(mesh)
}

/// Loading progress information
#[derive(Component)]
pub struct LoadingProgress {
    pub current: usize,
    pub total: usize,
    pub path: PathBuf,
}
