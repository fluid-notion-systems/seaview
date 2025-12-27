# Bevy Asset Loading Research: Idiomatic Mesh Loading

**Date**: 2024
**Bevy Version**: 0.17.x (with compatibility notes for 0.14+)
**Focus**: Loading well-behaved, indexed mesh files (glTF/GLB) using Bevy's native asset system
**Constraints**: One .glb file per mesh, loading from arbitrary directories (not just `assets/`)

## Executive Summary

Bevy has a mature, built-in asset loading system that handles:
- **Async loading** off the render thread
- **Parallel loading** via background tasks
- **Dependency tracking** between assets
- **Hot reloading** in development
- **Reference counting** for automatic memory management

For glTF/GLB files, **Bevy's built-in `GltfLoader` is production-ready** and should be preferred over custom implementations. It properly handles indexed meshes, materials, textures, and more.

## Key Insight: Don't Reinvent the Wheel

The current Seaview implementation uses a custom parallel loader that:
1. Loads glTF files with the `gltf` crate
2. Extracts vertex data manually
3. Ignores indices (causing the exploded mesh bug)
4. Recreates meshes from scratch

**This approach has problems:**
- Ignores indices, causing mesh corruption
- Reinvents async/parallel loading
- Harder to maintain

**However**, Seaview has specific requirements:
- **One .glb file per mesh** - no need for sub-mesh labels like `#Mesh0/Primitive0`
- **Arbitrary directory loading** - files are not in the standard `assets/` directory
- **Sequence playback** - loading 100s of meshes from user-specified paths

Bevy's asset system can handle all of this, but requires custom configuration.

## Bevy's Asset Loading Architecture

### The Three Core Components

1. **`AssetServer`** (Resource)
   - Coordinates all asset loading
   - Manages loading state
   - Handles hot reloading
   - Returns `Handle<T>` for assets

2. **`Assets<T>`** (Resource)
   - Stores loaded assets in memory
   - Reference-counted storage
   - Direct access to asset data

3. **`Handle<T>`** (Component/Value)
   - Lightweight reference to an asset
   - Can be cloned cheaply
   - Strong handles keep assets alive
   - Weak handles don't prevent cleanup

### How It Works

```rust
// Loading is async and non-blocking
fn load_mesh(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // This returns immediately with a handle
    // Loading happens in the background
    let mesh_handle: Handle<Mesh> = asset_server.load("model.glb#Mesh0/Primitive0");
    
    // Spawn entity with handle - rendering will start when loaded
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
    ));
}
```

## Using Bevy's Built-in glTF Loader

### Seaview's Use Case: One GLB Per Mesh

For Seaview, each .glb file contains a single mesh. This simplifies loading:

```rust
use bevy::prelude::*;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Each .glb file has one mesh at index 0
    // We load the first (and only) primitive of the first (and only) mesh
    let mesh_handle: Handle<Mesh> = asset_server.load("model.glb#Mesh0/Primitive0");
    
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));
}
```

**Note**: The `#Mesh0/Primitive0` label is still required even for single-mesh files, as it tells Bevy to load the mesh data rather than the scene. In the future, if we need to support multi-mesh .glb files, we can add support for other indices (`#Mesh1/Primitive0`, etc.).

### Asset Labels Reference

For single-mesh .glb files (Seaview's current format):
```rust
"model.glb#Mesh0/Primitive0"  // The mesh data
"model.glb#Material0"         // The material (if present)
```

For future multi-mesh support:
```rust
"model.glb#Mesh0/Primitive0"  // First mesh
"model.glb#Mesh1/Primitive0"  // Second mesh
"model.glb#Scene0"            // Entire scene with all meshes
```

### Advanced Configuration

```rust
use bevy::gltf::GltfLoaderSettings;
use bevy::render::render_asset::RenderAssetUsages;

fn load_with_settings(
    asset_server: Res<AssetServer>,
) {
    let mesh_handle = asset_server.load_with_settings(
        "model.glb#Mesh0/Primitive0",
        |settings: &mut GltfLoaderSettings| {
            // Control what gets loaded
            settings.load_meshes = RenderAssetUsages::all();
            settings.load_materials = RenderAssetUsages::RENDER_WORLD;
            settings.load_cameras = false;
            settings.load_lights = false;
            
            // Include source glTF data for inspection
            settings.include_source = true;
        },
    );
}

## Loading from Arbitrary Directories

**Key Issue**: By default, Bevy's `AssetServer` only loads from the `assets/` directory. Seaview needs to load from user-specified paths anywhere on the filesystem.

### Solution: Register Custom Asset Source (Recommended)

Bevy supports registering named asset sources that can point to any directory. This is the idiomatic approach that:
- ✅ Uses Bevy's built-in parallel async loading
- ✅ Supports file watching for hot reload
- ✅ Emits `AssetEvent`s for UI tracking
- ✅ Works with any absolute path

```rust
use bevy::prelude::*;
use bevy::asset::io::{AssetSource, AssetSourceId, AssetSourceBuilder};
use std::path::PathBuf;

/// Plugin to register a sequence directory as an asset source
pub struct SequenceSourcePlugin {
    pub source_name: String,
    pub directory: PathBuf,
}

impl Plugin for SequenceSourcePlugin {
    fn build(&self, app: &mut App) {
        let dir = self.directory.clone();
        let source_name = self.source_name.clone();
        
        // Register the asset source BEFORE DefaultPlugins/AssetPlugin
        // Use platform_default which sets up FileAssetReader with file watching
        app.register_asset_source(
            source_name,
            AssetSourceBuilder::platform_default(
                dir.to_string_lossy().to_string(),
                None, // No processed path needed
            ),
        );
    }
}

// Usage - must be added BEFORE DefaultPlugins:
fn main() {
    App::new()
        .add_plugins(SequenceSourcePlugin {
            source_name: "sim".to_string(),
            directory: PathBuf::from("/data/simulations/run_001"),
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, load_sequence)
        .run();
}

fn load_sequence(asset_server: Res<AssetServer>) {
    // Load using the custom source with "source://" prefix
    let mesh: Handle<Mesh> = asset_server.load("sim://frame_0000.glb#Mesh0/Primitive0");
}
```

### Runtime Source Registration

For registering sources after app startup (e.g., when user selects a directory):

```rust
use bevy::asset::io::file::FileAssetReader;

/// Resource to track dynamically registered sources
#[derive(Resource, Default)]
pub struct DynamicAssetSources {
    pub sources: HashMap<String, PathBuf>,
}

/// System to handle directory selection (e.g., from file dialog)
fn register_new_source(
    mut commands: Commands,
    mut sources: ResMut<DynamicAssetSources>,
    // Note: Can't modify AssetSourceBuilders after app start
    // Instead, we track and use direct loading for dynamic paths
) {
    // For truly dynamic paths, use the direct loading approach below
}
```

**Important**: Asset sources must be registered before `AssetPlugin` is built (i.e., before `DefaultPlugins`). For runtime-selected directories, use the direct loading approach.

### Direct Loading for Runtime Paths

When the directory isn't known at startup:

```rust
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;

/// Load a glTF mesh synchronously (call from async task)
pub fn load_glb_mesh(path: &PathBuf) -> Result<Mesh, String> {
    let (document, buffers, _images) = gltf::import(path)
        .map_err(|e| format!("Failed to load glTF {:?}: {}", path, e))?;
    
    // Seaview: one mesh per file
    let gltf_mesh = document.meshes().next()
        .ok_or_else(|| format!("No mesh in {:?}", path))?;
    let primitive = gltf_mesh.primitives().next()
        .ok_or_else(|| format!("No primitive in {:?}", path))?;
    
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    
    // Positions (required)
    let positions: Vec<[f32; 3]> = reader.read_positions()
        .ok_or_else(|| format!("No positions in {:?}", path))?
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    
    // Normals (optional)
    if let Some(normals) = reader.read_normals() {
        let normals: Vec<[f32; 3]> = normals.collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    
    // UVs (optional)
    if let Some(uvs) = reader.read_tex_coords(0) {
        let uvs: Vec<[f32; 2]> = uvs.into_f32().collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    
    // Indices (critical for proper mesh rendering!)
    if let Some(indices) = reader.read_indices() {
        let indices: Vec<u32> = indices.into_u32().collect();
        mesh.insert_indices(Indices::U32(indices));
    }
    
    Ok(mesh)
}
```

## Parallel, Ordered Loading with Event Watching

For sequence loading, we want:
1. **Parallel** - Load multiple frames concurrently
2. **Ordered** - Prioritize earlier frames (so playback can start sooner)
3. **Events** - Track which frames are loaded for UI feedback

### Ordered Parallel Loading Strategy

```rust
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Represents a frame being loaded
#[derive(Debug, Clone)]
pub struct FrameLoadTask {
    pub frame_index: usize,
    pub path: PathBuf,
}

/// Event emitted when a frame finishes loading
#[derive(Event, Debug, Clone)]
pub struct FrameLoadedEvent {
    pub frame_index: usize,
    pub path: PathBuf,
    pub success: bool,
    pub handle: Option<Handle<Mesh>>,
}

/// Resource managing sequence loading
#[derive(Resource)]
pub struct SequenceLoader {
    /// Base directory for the sequence
    pub base_path: PathBuf,
    
    /// Total frames to load
    pub total_frames: usize,
    
    /// Loaded mesh handles, indexed by frame number
    pub loaded_frames: BTreeMap<usize, Handle<Mesh>>,
    
    /// Currently loading tasks
    loading_tasks: Vec<(usize, Task<Result<Mesh, String>>)>,
    
    /// Frames queued but not yet started
    pending_frames: Vec<usize>,
    
    /// Maximum concurrent loads (tune based on system)
    pub max_concurrent: usize,
    
    /// Frames that failed to load
    pub failed_frames: Vec<usize>,
}

impl SequenceLoader {
    pub fn new(base_path: PathBuf, total_frames: usize) -> Self {
        // Queue all frames, with lower indices first (ordered)
        let pending_frames: Vec<usize> = (0..total_frames).collect();
        
        Self {
            base_path,
            total_frames,
            loaded_frames: BTreeMap::new(),
            loading_tasks: Vec::new(),
            pending_frames,
            max_concurrent: 4, // Reasonable default
            failed_frames: Vec::new(),
        }
    }
    
    /// Get the path for a frame
    pub fn frame_path(&self, index: usize) -> PathBuf {
        self.base_path.join(format!("frame_{:04}.glb", index))
    }
    
    /// Check if all frames are loaded
    pub fn is_complete(&self) -> bool {
        self.loaded_frames.len() + self.failed_frames.len() >= self.total_frames
    }
    
    /// Get loading progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        (self.loaded_frames.len() + self.failed_frames.len()) as f32 
            / self.total_frames as f32
    }
    
    /// Get the first N loaded frames (for starting playback early)
    pub fn ready_frame_count(&self) -> usize {
        // Count consecutive loaded frames from the start
        let mut count = 0;
        while self.loaded_frames.contains_key(&count) {
            count += 1;
        }
        count
    }
}

/// System to drive parallel loading
pub fn update_sequence_loader(
    mut loader: ResMut<SequenceLoader>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut events: EventWriter<FrameLoadedEvent>,
) {
    let pool = AsyncComputeTaskPool::get();
    
    // Start new tasks if we have capacity
    while loader.loading_tasks.len() < loader.max_concurrent {
        if let Some(frame_index) = loader.pending_frames.first().copied() {
            loader.pending_frames.remove(0);
            
            let path = loader.frame_path(frame_index);
            let task = pool.spawn(async move {
                load_glb_mesh(&path)
            });
            
            loader.loading_tasks.push((frame_index, task));
        } else {
            break; // No more pending frames
        }
    }
    
    // Poll existing tasks
    let mut completed = Vec::new();
    
    for (idx, (frame_index, task)) in loader.loading_tasks.iter_mut().enumerate() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            completed.push(idx);
            
            match result {
                Ok(mesh) => {
                    let handle = meshes.add(mesh);
                    let frame_idx = *frame_index;
                    let path = loader.frame_path(frame_idx);
                    
                    loader.loaded_frames.insert(frame_idx, handle.clone());
                    
                    events.send(FrameLoadedEvent {
                        frame_index: frame_idx,
                        path,
                        success: true,
                        handle: Some(handle),
                    });
                    
                    info!("Loaded frame {} ({}/{})", 
                        frame_idx, 
                        loader.loaded_frames.len(), 
                        loader.total_frames
                    );
                }
                Err(e) => {
                    let frame_idx = *frame_index;
                    let path = loader.frame_path(frame_idx);
                    
                    loader.failed_frames.push(frame_idx);
                    
                    events.send(FrameLoadedEvent {
                        frame_index: frame_idx,
                        path: path.clone(),
                        success: false,
                        handle: None,
                    });
                    
                    error!("Failed to load frame {}: {}", frame_idx, e);
                }
            }
        }
    }
    
    // Remove completed tasks (in reverse order to preserve indices)
    for idx in completed.into_iter().rev() {
        loader.loading_tasks.remove(idx);
    }
}

/// System to handle loading events in UI
pub fn handle_frame_loaded_events(
    mut events: EventReader<FrameLoadedEvent>,
    loader: Res<SequenceLoader>,
    // Add your UI state here
) {
    for event in events.read() {
        if event.success {
            // Update UI: mark frame as loaded
            // e.g., update progress bar, enable play button if enough frames loaded
            info!(
                "UI: Frame {} loaded. Progress: {:.1}%, Ready: {} frames",
                event.frame_index,
                loader.progress() * 100.0,
                loader.ready_frame_count()
            );
        } else {
            // Update UI: show error indicator for failed frame
            warn!("UI: Frame {} failed to load", event.frame_index);
        }
    }
}
```

### Using with Bevy's AssetServer (Pre-registered Sources)

If the source is registered at startup, you can use `AssetServer` directly with events:

```rust
use bevy::prelude::*;
use bevy::asset::{AssetEvent, LoadState, RecursiveDependencyLoadState};

/// Resource for tracking sequence loading via AssetServer
#[derive(Resource)]
pub struct AssetServerSequence {
    pub source_name: String,
    pub handles: Vec<Handle<Mesh>>,
    pub loaded_count: usize,
}

impl AssetServerSequence {
    /// Start loading a sequence
    pub fn load(
        asset_server: &AssetServer,
        source_name: &str,
        frame_count: usize,
    ) -> Self {
        // Load in order - Bevy will parallelize internally
        let handles: Vec<Handle<Mesh>> = (0..frame_count)
            .map(|i| {
                let path = format!("{}://frame_{:04}.glb#Mesh0/Primitive0", source_name, i);
                asset_server.load(path)
            })
            .collect();
        
        Self {
            source_name: source_name.to_string(),
            handles,
            loaded_count: 0,
        }
    }
}

/// System to watch AssetEvents for loading progress
pub fn watch_asset_events(
    mut events: EventReader<AssetEvent<Mesh>>,
    mut sequence: ResMut<AssetServerSequence>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id } => {
                // Check if this is one of our sequence meshes
                for (index, handle) in sequence.handles.iter().enumerate() {
                    if handle.id() == *id {
                        info!("Frame {} loaded via AssetServer", index);
                        sequence.loaded_count += 1;
                        break;
                    }
                }
            }
            AssetEvent::LoadedWithDependencies { id } => {
                // Mesh and all its dependencies are ready
                info!("Asset {:?} fully loaded with dependencies", id);
            }
            _ => {}
        }
    }
}

/// Check load state for individual assets
pub fn check_load_states(
    asset_server: Res<AssetServer>,
    sequence: Res<AssetServerSequence>,
) {
    for (index, handle) in sequence.handles.iter().enumerate() {
        match asset_server.load_state(handle.id()) {
            LoadState::NotLoaded => { /* Not started */ }
            LoadState::Loading => { /* In progress */ }
            LoadState::Loaded => { /* Ready to use */ }
            LoadState::Failed(err) => {
                error!("Frame {} failed: {:?}", index, err);
            }
        }
    }
}
```

### Priority Loading (Load Visible Frames First)

For streaming/scrubbing, prioritize frames near current playback position:

```rust
impl SequenceLoader {
    /// Reorder pending frames to prioritize around a target frame
    pub fn prioritize_around(&mut self, target_frame: usize, window: usize) {
        // Sort pending frames by distance from target
        self.pending_frames.sort_by_key(|&frame| {
            let dist = (frame as i64 - target_frame as i64).abs() as usize;
            dist
        });
    }
    
    /// Prioritize loading forward from current position (for playback)
    pub fn prioritize_forward(&mut self, current_frame: usize) {
        self.pending_frames.sort_by_key(|&frame| {
            if frame >= current_frame {
                frame - current_frame  // Forward frames first
            } else {
                self.total_frames + frame  // Then wrap-around
            }
        });
    }
}
```

## Summary: Recommended Architecture

### For Known Directories at Startup

```rust
App::new()
    // 1. Register source BEFORE DefaultPlugins
    .add_plugins(SequenceSourcePlugin {
        source_name: "sim".to_string(),
        directory: PathBuf::from("/data/simulation"),
    })
    .add_plugins(DefaultPlugins)
    
    // 2. Use AssetServer with source:// prefix
    // let mesh = asset_server.load("sim://frame_0000.glb#Mesh0/Primitive0");
    
    // 3. Watch AssetEvent<Mesh> for loading progress
    .add_systems(Update, watch_asset_events)
```

### For Runtime-Selected Directories

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_event::<FrameLoadedEvent>()
    
    // Use custom loader with AsyncComputeTaskPool
    .insert_resource(SequenceLoader::new(path, count))
    .add_systems(Update, (
        update_sequence_loader,
        handle_frame_loaded_events,
    ))
```

Both approaches provide:
- ✅ **Parallel loading** (via task pool or AssetServer internals)
- ✅ **Ordered loading** (lower frames first, or prioritized)
- ✅ **Event watching** (FrameLoadedEvent or AssetEvent<Mesh>)
- ✅ **UI integration** (progress tracking, ready frame count)
- ✅ **Proper index handling** (fixes exploded mesh bug)

## Checking Load State

```rust
use bevy::asset::{LoadState, RecursiveDependencyLoadState};

fn check_loading(
    asset_server: Res<AssetServer>,
    mesh_handle: &Handle<Mesh>,
) {
    // Check if this specific asset is loaded
    match asset_server.load_state(mesh_handle.id()) {
        LoadState::NotLoaded => println!("Not started"),
        LoadState::Loading => println!("Loading..."),
        LoadState::Loaded => println!("Loaded!"),
        LoadState::Failed(err) => println!("Failed: {:?}", err),
    }
    
    // Check if asset AND all dependencies are loaded
    match asset_server.recursive_dependency_load_state(mesh_handle.id()) {
        RecursiveDependencyLoadState::NotLoaded => println!("Not started"),
        RecursiveDependencyLoadState::Loading => println!("Loading..."),
        RecursiveDependencyLoadState::Loaded => println!("Ready!"),
        RecursiveDependencyLoadState::Failed => println!("Failed"),
    }
}
```

## Batch Loading with State Management

For loading multiple assets (like a sequence), use a loading state:

```rust
use bevy::prelude::*;
use bevy::utils::HashSet;

#[derive(Resource)]
struct LoadingState {
    handles: Vec<Handle<Mesh>>,
    frame_count: usize,
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Loading,
    Ready,
}

fn start_loading_sequence(
    mut commands: Commands,
    base_path: &Path,
    frame_count: usize,
) {
    let mut tasks = Vec::new();
    let thread_pool = AsyncComputeTaskPool::get();
    
    // Queue all loads at once - tasks run in parallel
    for i in 0..frame_count {
        let path = base_path.join(format!("frame_{:04}.glb", i));
        let task = thread_pool.spawn(async move {
            load_mesh_sync(path)
        });
        tasks.push(task);
    }
    
    commands.insert_resource(LoadingState {
        tasks,
        loaded_meshes: Vec::new(),
        frame_count,
    });
}

fn check_loading_complete(
    mut meshes: ResMut<Assets<Mesh>>,
    mut loading: ResMut<LoadingState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Poll tasks and collect completed meshes
    for task in &mut loading.tasks {
        if let Some(Ok(mesh)) = future::block_on(future::poll_once(task)) {
            let handle = meshes.add(mesh);
            loading.loaded_meshes.push(handle);
        }
    }
    
    // Check if all loaded
    if loading.loaded_meshes.len() == loading.frame_count {
        next_state.set(GameState::Ready);
    }
}
```

## Direct Asset Access

Sometimes you need to modify or inspect asset data:

```rust
fn modify_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_handle: &Handle<Mesh>,
) {
    if let Some(mesh) = meshes.get_mut(mesh_handle) {
        // Direct access to mesh data
        if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            println!("Vertex count: {}", positions.len());
        }
        
        // Modify mesh (affects ALL entities using this handle)
        mesh.compute_normals();
    }
}
```

6. **Handle absolute paths in sequence discovery**
```rust
// Before: Assumed assets directory
let path = format!("sequence/frame_{:04}.glb", i);
   
// After: Use absolute paths
let path = base_directory.join(format!("frame_{:04}.glb", i));
```

## Loading Strategies for Sequences

### Strategy 1: Load All Upfront (Simple)

```rust
#[derive(Resource)]
struct SequenceData {
    frames: Vec<Handle<Mesh>>,
    current_frame: usize,
    base_path: PathBuf,
}

fn load_sequence(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    base_path: PathBuf,
    frame_count: usize,
) {
    // Load all frames from arbitrary directory
    let mut frames = Vec::new();
    
    for i in 0..frame_count {
        let path = base_path.join(format!("frame_{:04}.glb", i));
        
        // Load synchronously for simplicity (or use async approach above)
        match load_mesh_sync(path) {
            Ok(mesh) => frames.push(meshes.add(mesh)),
            Err(e) => error!("Failed to load frame {}: {}", i, e),
        }
    }
    
    commands.insert_resource(SequenceData {
        frames,
        current_frame: 0,
        base_path,
    });
}

fn update_sequence(
    mut seq: ResMut<SequenceData>,
    time: Res<Time>,
    mut query: Query<&mut Mesh3d>,
) {
    // Update current frame based on time/input
    seq.current_frame = ((time.elapsed_secs() * 30.0) as usize) % seq.frames.len();
    
    // Update entity's mesh handle
    for mut mesh in &mut query {
        mesh.0 = seq.frames[seq.current_frame].clone();
    }
}
```

### Strategy 2: Sliding Window Cache (Memory Efficient)

```rust
#[derive(Resource)]
struct StreamingSequence {
    base_path: PathBuf,
    total_frames: usize,
    current_frame: usize,
    cache_radius: usize,
    cached_frames: HashMap<usize, Handle<Mesh>>,
    loading_tasks: HashMap<usize, Task<Result<Mesh, String>>>,
}

impl StreamingSequence {
    fn update_cache(&mut self, meshes: &mut ResMut<Assets<Mesh>>) {
        let start = self.current_frame.saturating_sub(self.cache_radius);
        let end = (self.current_frame + self.cache_radius).min(self.total_frames);
        
        // Start loading frames in window
        for i in start..=end {
            if !self.cached_frames.contains_key(&i) && !self.loading_tasks.contains_key(&i) {
                let path = self.base_path.join(format!("frame_{:04}.glb", i));
                let thread_pool = AsyncComputeTaskPool::get();
                let task = thread_pool.spawn(async move {
                    load_mesh_sync(path)
                });
                self.loading_tasks.insert(i, task);
            }
        }
        
        // Poll loading tasks
        let mut completed = Vec::new();
        for (frame, task) in &mut self.loading_tasks {
            if let Some(Ok(mesh)) = future::block_on(future::poll_once(task)) {
                let handle = meshes.add(mesh);
                self.cached_frames.insert(*frame, handle);
                completed.push(*frame);
            }
        }
        for frame in completed {
            self.loading_tasks.remove(&frame);
        }
        
        // Remove frames outside window (handles are dropped, assets unload)
        self.cached_frames.retain(|&frame, _| {
            frame >= start && frame <= end
        });
        self.loading_tasks.retain(|&frame, _| {
            frame >= start && frame <= end
        });
    }
}
```

### Strategy 3: Predictive Loading (Best Performance)

```rust
#[derive(Resource)]
struct PredictiveSequence {
    frames: Vec<Option<Handle<Mesh>>>,
    loading: HashSet<usize>,
    current_frame: usize,
    lookahead: usize,
}

impl PredictiveSequence {
    fn ensure_loaded(
        &mut self,
        frame: usize,
        base_path: &Path,
        meshes: &mut ResMut<Assets<Mesh>>,
    ) {
        if frame >= self.frames.len() {
            return;
        }
        
        if self.frames[frame].is_none() && !self.loading.contains(&frame) {
            let path = base_path.join(format!("frame_{:04}.glb", frame));
            
            // Could make this async, but for simplicity load sync
            if let Ok(mesh) = load_mesh_sync(path) {
                self.frames[frame] = Some(meshes.add(mesh));
            }
            self.loading.insert(frame);
        }
    }
    
    fn update(&mut self, base_path: &Path, meshes: &mut ResMut<Assets<Mesh>>) {
        // Load current frame and lookahead
        for i in 0..=self.lookahead {
            let frame = (self.current_frame + i) % self.frames.len();
            self.ensure_loaded(frame, base_path, meshes);
        }
    }
    
    fn get_current(&self) -> Option<&Handle<Mesh>> {
        self.frames[self.current_frame].as_ref()
    }
}
```

## Hot Reloading

Hot reloading is automatic in development:

```rust
// In Cargo.toml
[dependencies]
bevy = { version = "0.17", features = ["file_watcher"] }

// In code - no changes needed!
// When you edit a .glb file, all entities using it update automatically
```

## Error Handling

```rust
use bevy::asset::AssetLoadFailedEvent;

fn handle_load_errors(
    mut events: EventReader<AssetLoadFailedEvent<Mesh>>,
) {
    for event in events.read() {
        error!("Failed to load mesh: {:?}", event.path);
        error!("Error: {:?}", event.error);
        
        // Handle gracefully
        // - Show placeholder mesh
        // - Retry loading
        // - Alert user
    }
}

// Add to app
app.add_systems(Update, handle_load_errors);
```

## Performance Considerations

### What Bevy Does Automatically

1. **Parallel Loading**: Multiple assets load concurrently
2. **Background Tasks**: Loading never blocks the main thread
3. **Deduplication**: Same path = same asset (only loaded once)
4. **Caching**: Assets stay in memory while handles exist
5. **Smart Cleanup**: Assets unload when all handles are dropped

### Memory Management

```rust
// Strong handle - keeps asset alive
let strong: Handle<Mesh> = asset_server.load("model.glb#Mesh0");

// Weak handle - doesn't prevent cleanup
let weak: Handle<Mesh> = strong.clone_weak();

// Asset stays loaded until all strong handles are dropped
drop(strong); // Now the asset can be unloaded (weak handles remain valid until it is)
```

### Preloading vs Lazy Loading

```rust
// Preload: Load before needed
fn preload_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let handles: Vec<Handle<Mesh>> = (0..100)
        .map(|i| asset_server.load(format!("seq/frame_{:04}.glb#Mesh0/Primitive0", i)))
        .collect();
    
    // Store handles to keep assets loaded
    commands.insert_resource(PreloadedAssets { handles });
}

// Lazy load: Load when needed
fn spawn_on_demand(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        let handle = asset_server.load("model.glb#Mesh0/Primitive0");
        commands.spawn((Mesh3d(handle), /* ... */));
    }
}
```

## Migrating from Custom Loader

### Current Seaview Architecture (Custom)

```rust
// ❌ Manual loading
let (document, buffers, _images) = gltf::import(path)?;
let mesh = extract_mesh_manually(document, buffers)?;

// ❌ Manual async handling
loader.queue_load(path, priority, use_fallback)?;

// ❌ Manual caching
mesh_cache.insert(path, mesh_handle)?;
```

### Recommended Bevy-Native Architecture

```rust
// ✅ Use Bevy's loader
let mesh_handle: Handle<Mesh> = asset_server.load("model.glb#Mesh0/Primitive0");

// ✅ Automatic async handling (no explicit queue needed)

// ✅ Automatic caching (same path = same handle)
```

### Migration Steps

1. **Remove custom glTF loading code**
   - Delete `gltf_loader/mod.rs` custom implementation
   - Remove `gltf` crate dependency (Bevy includes it)

2. **Replace manual loading with asset server**
   ```rust
   // Before
   let (mesh, material) = load_gltf_as_mesh(path)?;
   
   // After (for arbitrary paths)
   let mesh = load_mesh_sync(path)?;
   let mesh_handle = meshes.add(mesh);
   ```

3. **Use handles instead of direct mesh data**
   ```rust
   // Before: Spawn with inline mesh
   commands.spawn((
       Mesh3d(meshes.add(mesh)),
       // ...
   ));
   
   // After: Spawn with handle (loads async)
   commands.spawn((
       Mesh3d(mesh_handle),
       // ...
   ));
   ```

4. **Replace custom cache with Bevy's built-in deduplication**
   ```rust
   // Before: Manual cache
   if let Some(handle) = cache.get(path) {
       return handle;
   }
   
   // After: Automatic (just call load again)
   let handle = asset_server.load(path); // Returns same handle if already loaded
   ```

5. **Use asset events instead of custom events**
   ```rust
   // Before: Custom LoadCompleteEvent
   fn handle_custom_events(mut events: EventReader<LoadCompleteEvent>) { }
   
   // After: Bevy's AssetEvent
   fn handle_asset_events(mut events: EventReader<AssetEvent<Mesh>>) {
       for event in events.read() {
           match event {
               AssetEvent::Added { id } => println!("Loaded: {:?}", id),
               AssetEvent::Modified { id } => println!("Modified: {:?}", id),
               AssetEvent::Removed { id } => println!("Unloaded: {:?}", id),
               AssetEvent::LoadedWithDependencies { id } => println!("Ready: {:?}", id),
           }
       }
   }
   ```

## Recommended Architecture for Seaview

```rust
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::path::{Path, PathBuf};

/// Loads a single mesh from an arbitrary path
fn load_mesh_sync(path: PathBuf) -> Result<Mesh, String> {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;
    
    let (document, buffers, _images) = gltf::import(&path)
        .map_err(|e| format!("Failed to load glTF from {:?}: {}", path, e))?;
    
    // Seaview constraint: one mesh per file
    let gltf_mesh = document.meshes().next()
        .ok_or("No mesh found in glTF file")?;
    let primitive = gltf_mesh.primitives().next()
        .ok_or("No primitive found in mesh")?;
    
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    
    // Positions (required)
    if let Some(positions) = reader.read_positions() {
        let positions: Vec<[f32; 3]> = positions.collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    } else {
        return Err("No position data in mesh".to_string());
    }
    
    // Normals (optional)
    if let Some(normals) = reader.read_normals() {
        let normals: Vec<[f32; 3]> = normals.collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    
    // UVs (optional)
    if let Some(uvs) = reader.read_tex_coords(0) {
        let uvs: Vec<[f32; 2]> = uvs.into_f32().collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    
    // Indices (critical - this is what fixes the exploded mesh bug!)
    if let Some(indices) = reader.read_indices() {
        let indices: Vec<u32> = indices.into_u32().collect();
        mesh.insert_indices(Indices::U32(indices));
    }
    
    Ok(mesh)
}

/// Simple sequence player for arbitrary directory
#[derive(Resource)]
pub struct MeshSequence {
    pub base_path: PathBuf,
    pub frames: Vec<Handle<Mesh>>,
    pub current_frame: usize,
    pub fps: f32,
    pub total_frames: usize,
}

impl MeshSequence {
    pub fn load_from_directory(
        base_path: PathBuf,
        pattern: &str,
        count: usize,
        meshes: &mut ResMut<Assets<Mesh>>,
    ) -> Result<Self, String> {
        let mut frames = Vec::new();
        
        for i in 0..count {
            let filename = pattern.replace("{}", &i.to_string());
            let path = base_path.join(filename);
            
            match load_mesh_sync(path) {
                Ok(mesh) => frames.push(meshes.add(mesh)),
                Err(e) => {
                    error!("Failed to load frame {}: {}", i, e);
                    return Err(e);
                }
            }
        }
        
        Ok(Self {
            base_path,
            frames,

            current_frame: 0,
            fps: 30.0,
            total_frames: count,
        })
    }
}

#[derive(Component)]
pub struct SequencePlayer {
    pub playing: bool,
    pub looping: bool,
}

fn update_sequence_players(
    time: Res<Time>,
    mut query: Query<(&SequencePlayer, &mut Mesh3d)>,
    mut sequence: ResMut<MeshSequence>,
) {
    for (player, mut mesh) in &mut query {
        if !player.playing {
            continue;
        }
        
        // Update to current frame
        mesh.0 = sequence.frames[sequence.current_frame].clone();
    }
}
```

## Conclusion

**Key Takeaways for Seaview:**

1. **Load from arbitrary paths** - Use direct `gltf::import()` with proper index handling
2. **One .glb per mesh** - Simple model, still use `#Mesh0/Primitive0` internally
3. **Always handle indices** - This fixes the exploded mesh bug
4. **Use Bevy's task pool** - For async, parallel loading off render thread
5. **Store in `Assets<Mesh>`** - Leverage Bevy's reference counting and storage
6. **Keep mesh loading logic simple** - Don't reinvent the wheel

**What to Remove from Current Implementation:**

- Manual mesh data extraction that ignores indices (in `parallel_loader`)
- Custom `AsyncStlLoader` for glTF (keep for STL if needed)
- Redundant mesh processing pipeline
- Converting indexed meshes to triangle soup

**What to Keep:**

- Sequence discovery (file pattern matching)
- Playback controls and UI
- Frame interpolation logic
- Camera and viewport systems
- Network streaming features

**Implementation Strategy:**

1. Create `load_mesh_sync()` function that properly handles indices
2. Wrap in async tasks using `AsyncComputeTaskPool` for parallel loading
3. Store loaded meshes in `Assets<Mesh>` for reference counting
4. Keep sequence management at high level (frame selection, playback speed, etc.)

**The Result:**
- ✅ Fixes the exploded mesh bug (proper index handling)
- ✅ Loads from arbitrary directories (not limited to `assets/`)
- ✅ Async, parallel loading (using Bevy's task pool)
- ✅ Simple, maintainable code (less custom infrastructure)
- ✅ Proper memory management (Bevy's reference counting)
- ✅ Future-proof (can add multi-mesh support later with labels)