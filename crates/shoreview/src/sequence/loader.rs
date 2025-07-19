//! Sequence loader module for efficient mesh loading with caching

use super::{SequenceEvent, SequenceManager};
use crate::systems::stl_loader::StlFilePath;
use bevy::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Plugin for sequence loading functionality
pub struct SequenceLoaderPlugin;

impl Plugin for SequenceLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeshCache>()
            .init_resource::<LoaderConfig>()
            .add_systems(
                Update,
                (handle_frame_changes, prefetch_frames, cleanup_cache).chain(),
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
            cache_size: 50,
            prefetch_ahead: 5,
            keep_behind: 2,
            async_loading: true,
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
) {
    // Handle playback
    if sequence_manager.is_playing {
        let frame_duration = 1.0 / sequence_manager.playback_fps;
        let elapsed = time.elapsed_secs_f64();

        // Simple frame timing - could be improved with accumulator
        static mut LAST_FRAME_TIME: f64 = 0.0;
        unsafe {
            if elapsed - LAST_FRAME_TIME >= frame_duration as f64 {
                LAST_FRAME_TIME = elapsed;
                if !sequence_manager.next_frame() {
                    // Loop back to start
                    sequence_manager.jump_to_frame(0);
                }
                events.write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
            }
        }
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

/// System that prefetches upcoming frames
fn prefetch_frames(
    sequence_manager: Res<SequenceManager>,
    mut mesh_cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<LoaderConfig>,
) {
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
