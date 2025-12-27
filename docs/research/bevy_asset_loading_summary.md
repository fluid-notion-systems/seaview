# Bevy Asset Loading for Seaview - Implementation Guide

**Goal**: Load glTF/GLB meshes from arbitrary directories with parallel, ordered loading and event watching.

## The Problem

Current implementation ignores mesh indices, causing exploded/corrupted meshes.

## Solution: Use AssetPlugin.file_path

The simplest approach is to configure `AssetPlugin` with a custom `file_path`:

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "/data/simulations/run_001".to_string(),
            ..default()
        }))
        .add_systems(Startup, load_sequence)
        .add_systems(Update, watch_loading)
        .run();
}
```

Then use `GltfAssetLabel` for clean asset loading:

```rust
use bevy::gltf::GltfAssetLabel;

fn load_sequence(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Load mesh using GltfAssetLabel (cleaner than string labels)
    let mesh: Handle<Mesh> = asset_server.load(
        GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }
            .from_asset("frame_0000.glb")
    );
    
    // Or load multiple frames
    let handles: Vec<Handle<Mesh>> = (0..100)
        .map(|i| {
            asset_server.load(
                GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }
                    .from_asset(format!("frame_{:04}.glb", i))
            )
        })
        .collect();
    
    commands.insert_resource(SequenceHandles { handles });
}
```

---

## Key Components

### GltfAssetLabel Variants

```rust
use bevy::gltf::GltfAssetLabel;

// For Seaview (one mesh per file):
GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }  // The mesh geometry

// Other useful labels:
GltfAssetLabel::Scene(0)      // Entire scene with transforms
GltfAssetLabel::Material { index: 0, is_scale_inverted: false }
GltfAssetLabel::Texture(0)
```

### Event Watching

```rust
use bevy::asset::AssetEvent;

fn watch_loading(
    mut events: EventReader<AssetEvent<Mesh>>,
    sequence: Res<SequenceHandles>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id } => {
                // Find which frame loaded
                for (index, handle) in sequence.handles.iter().enumerate() {
                    if handle.id() == *id {
                        info!("Frame {} loaded", index);
                    }
                }
            }
            AssetEvent::LoadedWithDependencies { id } => {
                info!("Asset {:?} fully ready", id);
            }
            _ => {}
        }
    }
}
```

### Check Load State

```rust
use bevy::asset::LoadState;

fn check_progress(
    asset_server: Res<AssetServer>,
    sequence: Res<SequenceHandles>,
) {
    let loaded = sequence.handles.iter()
        .filter(|h| matches!(asset_server.load_state(h.id()), LoadState::Loaded))
        .count();
    
    info!("Progress: {}/{}", loaded, sequence.handles.len());
}
```

---

## Alternative: Register Additional Asset Source

For loading from multiple directories:

```rust
use bevy::asset::io::AssetSourceBuilder;

fn main() {
    App::new()
        // Register BEFORE DefaultPlugins
        .register_asset_source(
            "sim",
            AssetSourceBuilder::platform_default(
                "/data/simulations/run_001",
                None,
            ),
        )
        .add_plugins(DefaultPlugins)
        .run();
}

// Then load with source:// prefix
fn load(asset_server: Res<AssetServer>) {
    let mesh: Handle<Mesh> = asset_server.load(
        GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }
            .from_asset("sim://frame_0000.glb")
    );
}
```

---

## Complete Example: Sequence Loader

```rust
use bevy::prelude::*;
use bevy::gltf::GltfAssetLabel;
use bevy::asset::{AssetEvent, LoadState};

#[derive(Resource)]
pub struct SequenceHandles {
    pub handles: Vec<Handle<Mesh>>,
    pub loaded_count: usize,
}

#[derive(Event)]
pub struct FrameLoadedEvent {
    pub frame_index: usize,
}

pub fn load_sequence(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let frame_count = 100;
    
    // Bevy loads these in parallel automatically
    let handles: Vec<Handle<Mesh>> = (0..frame_count)
        .map(|i| {
            asset_server.load(
                GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }
                    .from_asset(format!("frame_{:04}.glb", i))
            )
        })
        .collect();
    
    commands.insert_resource(SequenceHandles {
        handles,
        loaded_count: 0,
    });
}

pub fn track_loading(
    mut events: EventReader<AssetEvent<Mesh>>,
    mut sequence: ResMut<SequenceHandles>,
    mut frame_events: EventWriter<FrameLoadedEvent>,
) {
    for event in events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            for (index, handle) in sequence.handles.iter().enumerate() {
                if handle.id() == *id {
                    sequence.loaded_count += 1;
                    frame_events.send(FrameLoadedEvent { frame_index: index });
                    info!(
                        "Frame {} loaded ({}/{})",
                        index,
                        sequence.loaded_count,
                        sequence.handles.len()
                    );
                }
            }
        }
    }
}

// App setup
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "/path/to/sequence".to_string(),
            ..default()
        }))
        .add_event::<FrameLoadedEvent>()
        .add_systems(Startup, load_sequence)
        .add_systems(Update, track_loading)
        .run();
}
```

---

## What This Provides

| Feature | How |
|---------|-----|
| **Parallel loading** | Bevy's AssetServer handles this automatically |
| **Ordered by index** | Frames load in order they're requested |
| **Event watching** | `AssetEvent<Mesh>` for each loaded mesh |
| **Arbitrary paths** | `AssetPlugin::file_path` or `register_asset_source` |
| **Proper indices** | Bevy's GltfLoader handles indices correctly |
| **Hot reload** | Enable `file_watcher` feature |

---

## Migration Checklist

- [ ] Remove custom `gltf_loader/mod.rs` 
- [ ] Remove glTF handling from `parallel_loader/mod.rs`
- [ ] Configure `AssetPlugin::file_path` for sequence directory
- [ ] Use `GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }` for loading
- [ ] Add `AssetEvent<Mesh>` handler for UI progress
- [ ] Test with Duck.glb to verify mesh renders correctly

---

## Key Points

1. **Use `AssetPlugin::file_path`** - Simplest way to load from arbitrary directory
2. **Use `GltfAssetLabel`** - Type-safe asset labels instead of string parsing
3. **Watch `AssetEvent<Mesh>`** - Track loading progress for UI
4. **Bevy handles parallelism** - No need for custom task pools
5. **Indices handled correctly** - Bevy's GltfLoader preserves mesh topology