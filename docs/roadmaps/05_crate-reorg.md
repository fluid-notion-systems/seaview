# Crate Reorganization Roadmap

## Overview

Based on Bevy best practices, we should reorganize the crate to have a clear separation between shared library code (`lib/`) and binary-specific application code (`app/`). This will fix the current issues with circular dependencies and unclear module boundaries.

## Current Issues

1. **Circular Dependencies**: Binary-specific modules (like `systems/camera.rs`) can't import from the library part
2. **Duplicate Definitions**: `NetworkMeshReceived` exists in multiple places
3. **Unclear Boundaries**: Some types need to be shared between library and binary
4. **Complex Imports**: Using `crate::` in binary modules that need library types is confusing

## Recommended Structure

Separate shared library code from binary-specific code:

```
seaview/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Library entry point
│   ├── main.rs         # Minimal binary entry point
│   │
│   ├── lib/            # Shared library modules (accessible from both lib and bin)
│   │   ├── camera.rs       # Shared camera components
│   │   ├── coordinates.rs  # Coordinate systems
│   │   ├── network/        # Network protocol and shared types
│   │   ├── session/        # Session management
│   │   ├── sequence/       # Sequence handling
│   │   └── ui/            # UI components and shared types
│   │
│   └── app/            # Binary-only modules (not exposed in lib.rs)
│       ├── cli.rs      # CLI argument parsing
│       └── systems/    # Binary-specific systems
│           ├── camera.rs       # Camera controller system
│           ├── diagnostics.rs  # Diagnostics systems
│           ├── gltf_loader.rs  # GLTF loading system
│           ├── network.rs      # Network receiving system
│           ├── parallel_loader.rs
│           └── stl_loader.rs
```

## Key Principles

1. **Minimal main.rs**: Following Bevy best practices, main.rs should be minimal:
   ```rust
   use bevy::prelude::*;
   use seaview::SeaviewPlugin;

   mod app;

   fn main() {
       let args = app::cli::Args::parse();

       App::new()
           .add_plugins(DefaultPlugins)
           .add_plugins(SeaviewPlugin)
           .add_plugins(app::AppPlugin { args })
           .run();
   }
   ```

2. **Library exposes shared types**: `lib.rs` exposes only what needs to be shared
3. **Binary modules use library via `seaview::`**: Clear import paths
4. **Systems stay in binary**: Game-specific systems remain in the `app/` directory

## Implementation Steps

### Step 1: Create directory structure
1. Create `src/lib/` directory for shared code
2. Create `src/app/` directory for binary-specific code
3. Move existing modules to appropriate directories

### Step 2: Move shared modules to lib/
1. Move `camera.rs` to `lib/camera.rs`
2. Move `coordinates.rs` to `lib/coordinates.rs`
3. Move `network/` to `lib/network/`
4. Move `session/` to `lib/session/`
5. Move `sequence/` to `lib/sequence/`
6. Move `ui/` to `lib/ui/`

### Step 3: Move binary-specific code to app/
1. Move `cli/` to `app/cli.rs`
2. Move `systems/` to `app/systems/`
3. Create `app/mod.rs` to organize binary modules

### Step 4: Update lib.rs
```rust
// Expose shared library modules
pub mod camera;
pub mod coordinates;
pub mod network;
pub mod session;
pub mod sequence;
pub mod ui;

// Internal module organization
mod lib {
    pub use super::camera;
    pub use super::coordinates;
    pub use super::network;
    pub use super::session;
    pub use super::sequence;
    pub use super::ui;
}

// Convenient prelude for common imports
pub mod prelude {
    pub use crate::camera::{FpsCamera, CameraPlugin};
    pub use crate::network::{NetworkConfig, NetworkMeshReceived};
    pub use crate::session::{Session, SessionPlugin};
    pub use crate::sequence::SequencePlugin;
    pub use crate::ui::SeaviewUiPlugin;
}

use bevy::prelude::*;

pub struct SeaviewPlugin;

impl Plugin for SeaviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SessionPlugin,
            SeaviewUiPlugin,
            SequencePlugin,
            CameraPlugin,
        ));
    }
}
```

### Step 5: Create app/mod.rs
```rust
pub mod cli;
pub mod systems;

use bevy::prelude::*;
use seaview::prelude::*;

pub struct AppPlugin {
    pub args: cli::Args,
}

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Insert CLI args as a resource
        app.insert_resource(self.args.clone());

        // Add binary-specific systems
        app.add_plugins((
            systems::camera::CameraControllerPlugin,
            systems::network::NetworkReceiverPlugin,
            systems::stl_loader::StlLoaderPlugin,
            systems::gltf_loader::GltfLoaderPlugin,
            systems::diagnostics::DiagnosticsPlugin,
        ));
    }
}
```

### Step 6: Fix imports
1. Update all imports in `app/` modules to use `seaview::` for library types
2. Update imports within `lib/` modules to use relative paths or `crate::`
3. Remove any circular dependencies

## Module Organization Details

### lib/ modules (shared code)
- **camera.rs**: Camera components and basic setup
- **coordinates.rs**: Coordinate system types and conversions
- **network/**: Network protocol definitions, shared types
- **session/**: Session management, persistence
- **sequence/**: Sequence data structures, loading traits
- **ui/**: UI components, state management

### app/ modules (binary-specific)
- **cli.rs**: Command-line argument parsing
- **systems/camera.rs**: Camera movement and control systems
- **systems/network.rs**: Network message receiving and processing
- **systems/stl_loader.rs**: STL file loading implementation
- **systems/gltf_loader.rs**: GLTF file loading implementation
- **systems/diagnostics.rs**: Performance diagnostics

## Benefits

1. **Clear boundaries**: Library vs application code is obvious
2. **Simple imports**: Binary uses `seaview::` for library types
3. **Follows Rust conventions**: Clear separation of concerns
4. **No circular dependencies**: Clear hierarchy with `lib/` and `app/`
5. **Easy to test**: Library can be tested independently
6. **Reusable**: Library code can be used by other binaries or crates

## Migration Checklist
Please proceed with the following steps:
Do a cargo check after each one, where code changes, and fix errors

- [ ] Create `src/lib/` and `src/app/` directories
- [ ] Move shared modules to `lib/`
- [ ] Move binary-specific code to `app/`
- [ ] Update lib.rs to expose lib modules
- [ ] Create app/mod.rs with AppPlugin
- [ ] Update main.rs to use new structure
- [ ] Fix all import paths
- [ ] Remove duplicate type definitions
- [ ] Test compilation
- [ ] Update documentation

## Example File Moves

```bash
# Shared library modules
mv src/camera.rs src/lib/camera.rs
mv src/coordinates.rs src/lib/coordinates.rs
mv src/network/ src/lib/network/
mv src/session/ src/lib/session/
mv src/sequence/ src/lib/sequence/
mv src/ui/ src/lib/ui/

# Binary-specific modules
mv src/cli/ src/app/cli.rs
mv src/systems/ src/app/systems/
```

## Notes

- The `lib/` directory contains all shared, reusable code
- The `app/` directory contains binary-specific implementations
- This structure makes it easy to create additional binaries that reuse the library code
- The separation helps with testing and modularity
