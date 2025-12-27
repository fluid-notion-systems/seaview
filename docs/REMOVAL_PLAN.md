# Loading System Removal Plan

## Overview

Remove all custom mesh loading infrastructure (STL/glTF loaders, parallel loading, custom caching) in preparation for using Bevy's native asset loading system.

## Phase 1: Identify Dependencies

### Files to Delete Completely

1. **`src/lib/parallel_loader/`** (entire directory)
   - Custom parallel loading infrastructure
   - Worker thread pool management
   - Load queue management

2. **`src/lib/gltf_loader/`** (entire directory)
   - Custom glTF loader that ignores indices
   - Manual mesh extraction

3. **`src/app/systems/stl_loader/`** (entire directory)
   - STL loader plugin
   - Initial file loading systems

### Files to Modify

1. **`src/lib.rs`**
   - Remove: `pub mod parallel_loader`
   - Remove: `pub mod gltf_loader`

2. **`src/main.rs`**
   - Remove: `use seaview::lib::parallel_loader::AsyncStlLoaderPlugin`
   - Remove: `use seaview::app::systems::stl_loader::{StlFilePath, StlLoaderPlugin}`
   - Remove: `use seaview::lib::gltf_loader::GltfLoaderPlugin`
   - Remove: `.add_plugins(AsyncStlLoaderPlugin)`
   - Remove: `.add_plugins(StlLoaderPlugin)`
   - Remove: `.add_plugins(GltfLoaderPlugin)`
   - Remove: `StlFilePath` resource initialization

3. **`src/app/systems/mod.rs`**
   - Remove: `pub mod stl_loader`

4. **`src/lib/sequence/mod.rs`**
   - Remove: `pub mod async_cache` (heavily coupled to parallel_loader)
   - Remove: `pub mod loader` (heavily coupled to parallel_loader)
   - Keep: `pub mod discovery` (just finds files)
   - Keep: `pub mod playback` (just plays frames)

5. **`src/lib/sequence/playback.rs`**
   - Update to use Bevy's AssetServer directly
   - Remove dependencies on AsyncMeshCache

### Cargo.toml Dependencies to Review

Consider removing if no longer needed:
- `stl_io` (if not used elsewhere)
- `gltf` (Bevy includes this internally)
- `baby_shark` (review if still needed for other mesh operations)
- `threadpool` (if only used by parallel_loader)

## Phase 2: Execution Order

### Step 1: Delete Plugin Registrations (main.rs)

```rust
// REMOVE these lines from main.rs:
.add_plugins(seaview::lib::parallel_loader::AsyncStlLoaderPlugin)
.add_plugins(StlLoaderPlugin)
.add_plugins(seaview::lib::gltf_loader::GltfLoaderPlugin)

// REMOVE:
.insert_resource(StlFilePath(Some(input_path.clone())))

// REMOVE imports:
use seaview::app::systems::stl_loader::{StlFilePath, StlLoaderPlugin};
```

### Step 2: Delete Directories

```bash
rm -rf src/lib/parallel_loader/
rm -rf src/lib/gltf_loader/
rm -rf src/app/systems/stl_loader/
rm -rf src/lib/sequence/async_cache.rs
rm -rf src/lib/sequence/loader.rs
```

### Step 3: Update Module Declarations

In `src/lib.rs`:
```rust
pub mod lib {
    pub mod coordinates;
    // REMOVE: pub mod gltf_loader;
    pub mod network;
    // REMOVE: pub mod parallel_loader;
    pub mod sequence;
    pub mod session;
}
```

In `src/app/systems/mod.rs`:
```rust
pub mod camera;
pub mod diagnostics;
pub mod network;
// REMOVE: pub mod stl_loader;
```

In `src/lib/sequence/mod.rs`:
```rust
// REMOVE: pub mod async_cache;
pub mod discovery;
// REMOVE: pub mod loader;
pub mod playback;
```

### Step 4: Update SequencePlugin

In `src/lib/sequence/mod.rs`:
```rust
impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            discovery::SequenceDiscoveryPlugin,
            // REMOVE: loader::SequenceLoaderPlugin,
            playback::SequencePlaybackPlugin,
        ))
        .init_resource::<SequenceManager>()
        .add_event::<SequenceEvent>();
    }
}
```

### Step 5: Update playback.rs

Remove all references to:
- `AsyncMeshCache`
- `AsyncStlLoader`
- `LoadPriority`
- Custom loading systems

Replace with Bevy's AssetServer for loading.

## Phase 3: Verification

### Compilation Check

After each step, verify:
```bash
cd seaview
cargo check --package seaview
```

### Expected Errors

After removal, expect errors in:
- `playback.rs` - needs to be rewritten to use AssetServer
- Any systems that reference removed types

## Phase 4: What Remains

After cleanup, the sequence module should only contain:

1. **`discovery.rs`** - File pattern matching and sequence detection
2. **`playback.rs`** - Frame sequencing and playback control (needs rewrite)
3. **`mod.rs`** - Core types (Sequence, SequenceManager, FrameInfo)

## Phase 5: Next Steps (After Removal)

1. Configure `AssetPlugin::file_path` for sequence directory
2. Implement new loader using:
   - `AssetServer::load()` with `GltfAssetLabel`
   - `AssetEvent<Mesh>` for progress tracking
3. Update playback.rs to swap mesh handles
4. Update UI to use AssetEvent for progress

## Notes

- Keep `discovery.rs` - it's just file system scanning
- Keep `playback.rs` structure but rewrite internals
- Keep `SequenceManager` - high-level state management
- Session management (`src/lib/session/`) should not be affected
- Network streaming (`src/lib/network/`) should not be affected

## Safety Checks

Before deletion:
- [ ] Grep for `AsyncStlLoader` usage
- [ ] Grep for `AsyncMeshCache` usage
- [ ] Grep for `LoadCompleteEvent` usage
- [ ] Grep for `parallel_loader::` imports
- [ ] Grep for `gltf_loader::` imports
- [ ] Check if baby_shark is used elsewhere
- [ ] Check if stl_io is used elsewhere