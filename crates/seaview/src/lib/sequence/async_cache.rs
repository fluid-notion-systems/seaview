use bevy::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::systems::parallel_loader::{AsyncStlLoader, LoadHandle, LoadPriority, LoadStatus};
use crate::lib::sequence::loader::{FileLoadStats, LoadingStats};

/// Async-aware mesh cache that integrates with the parallel loader
#[derive(Resource)]
pub struct AsyncMeshCache {
    /// Cached mesh handles by path
    pub cache: HashMap<PathBuf, Handle<Mesh>>,

    /// Handles currently being loaded
    loading_handles: HashMap<PathBuf, LoadHandle>,

    /// Access order for LRU eviction
    access_order: Vec<PathBuf>,

    /// Current mesh entity (for single file mode)
    pub current_mesh_entity: Option<Entity>,

    /// Material handle
    material_handle: Option<Handle<StandardMaterial>>,

    /// Last displayed frame
    pub last_displayed_frame: Option<usize>,

    /// Loading statistics
    stats: LoadingStats,

    /// Maximum cache size
    max_cache_size: usize,
}

impl Default for AsyncMeshCache {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            loading_handles: HashMap::new(),
            access_order: Vec::new(),
            current_mesh_entity: None,
            material_handle: None,
            last_displayed_frame: None,
            stats: LoadingStats::default(),
            max_cache_size: 100,
        }
    }
}

impl AsyncMeshCache {
    /// Create a new cache with specified size
    #[allow(dead_code)]
    pub fn new(max_size: usize) -> Self {
        Self {
            max_cache_size: max_size,
            ..Default::default()
        }
    }

    /// Get or queue loading of a mesh
    pub fn get_or_queue(
        &mut self,
        path: &PathBuf,
        loader: &AsyncStlLoader,
        priority: LoadPriority,
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

        // Check if already loading
        if let Some(&load_handle) = self.loading_handles.get(path) {
            // Check status
            if let Some(status) = loader.get_status(load_handle) {
                match status {
                    LoadStatus::Completed => {
                        // Should be in cache next frame
                        return None;
                    }
                    LoadStatus::Failed(_) | LoadStatus::Cancelled => {
                        // Remove failed handle and retry
                        self.loading_handles.remove(path);
                    }
                    _ => return None, // Still loading
                }
            }
        }

        // Queue for loading
        match loader.queue_load(path.clone(), priority, use_fallback) {
            Ok(handle) => {
                self.loading_handles.insert(path.clone(), handle);
                self.stats.total_attempts += 1;
                None
            }
            Err(e) => {
                warn!("Failed to queue load for {:?}: {}", path, e);
                None
            }
        }
    }

    /// Check if a path is loaded in cache
    pub fn is_loaded(&self, path: &PathBuf) -> bool {
        self.cache.contains_key(path)
    }

    /// Check if a path is currently loading
    pub fn is_loading(&self, path: &PathBuf) -> bool {
        self.loading_handles.contains_key(path)
    }

    /// Get the material handle, creating it if necessary
    pub fn get_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = &self.material_handle {
            handle.clone()
        } else {
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.8, 0.8),
                metallic: 0.1,
                perceptual_roughness: 0.8,
                reflectance: 0.5,
                double_sided: false,
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                ..default()
            });
            self.material_handle = Some(material.clone());
            material
        }
    }

    /// Evict least recently used entries to maintain cache size
    pub fn evict_lru(&mut self, target_size: usize) {
        while self.cache.len() > target_size && !self.access_order.is_empty() {
            if let Some(path) = self.access_order.first().cloned() {
                self.cache.remove(&path);
                self.access_order.remove(0);
            }
        }
    }

    /// Insert a loaded mesh into the cache
    pub fn insert(&mut self, path: PathBuf, handle: Handle<Mesh>, stats: FileLoadStats) {
        self.cache.insert(path.clone(), handle);
        self.loading_handles.remove(&path);

        self.stats.successful_loads += 1;
        self.stats.total_faces_processed += stats.faces_processed;
        self.stats.total_faces_skipped += stats.faces_skipped;

        // Evict if necessary
        if self.cache.len() > self.max_cache_size {
            self.evict_lru(self.max_cache_size);
        }
    }

    /// Mark a load as failed
    pub fn mark_failed(&mut self, path: PathBuf) {
        self.loading_handles.remove(&path);
        self.stats.failed_loads += 1;
    }

    /// Get cache statistics
    pub fn stats(&self) -> &LoadingStats {
        &self.stats
    }

    /// Get the number of items in cache
    pub fn cached_count(&self) -> usize {
        self.cache.len()
    }

    /// Get the number of items currently loading
    pub fn loading_count(&self) -> usize {
        self.loading_handles.len()
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cache.clear();
        self.loading_handles.clear();
        self.access_order.clear();
        self.current_mesh_entity = None;
        self.last_displayed_frame = None;
    }

    /// Get loading progress (0.0 to 1.0)
    pub fn loading_progress(&self) -> f32 {
        let total = self.stats.total_attempts as f32;
        if total == 0.0 {
            return 1.0;
        }

        let completed = (self.stats.successful_loads + self.stats.failed_loads) as f32;
        completed / total
    }
}

/// System to update cache from completed loads
pub fn update_cache_from_loads(
    _loader: Res<AsyncStlLoader>,
    mut cache: ResMut<AsyncMeshCache>,
    _meshes: ResMut<Assets<Mesh>>,
    mut events: EventReader<crate::app::systems::parallel_loader::LoadCompleteEvent>,
) {
    for event in events.read() {
        if event.success {
            // The mesh should already be in the Assets<Mesh> from process_completed_loads
            // We just need to update our tracking
            if let Some(_handle) = cache.cache.get(&event.path) {
                debug!("Mesh loaded successfully: {:?}", event.path);
            }
        } else {
            cache.mark_failed(event.path.clone());
        }
    }
}

/// System to handle cache statistics logging
pub fn log_cache_stats(cache: Res<AsyncMeshCache>, time: Res<Time>) {
    static mut LAST_LOG: f32 = 0.0;

    let current_time = time.elapsed_secs();

    // Log every 5 seconds
    unsafe {
        if current_time - LAST_LOG > 5.0 {
            LAST_LOG = current_time;

            let stats = cache.stats();
            if stats.total_attempts > 0 {
                info!(
                    "Mesh Cache Stats - Cached: {}, Loading: {}, Success: {}, Failed: {}, Faces: {} (skipped: {})",
                    cache.cached_count(),
                    cache.loading_count(),
                    stats.successful_loads,
                    stats.failed_loads,
                    stats.total_faces_processed,
                    stats.total_faces_skipped
                );
            }
        }
    }
}
