# Seaview Architecture

## Overview

Seaview is a Bevy-based 3D mesh viewer for time-series simulation data. It loads sequences of mesh files (STL, glTF/GLB) from arbitrary filesystem paths and plays them back as animations.

## Crate Layout

```
crates/seaview/
├── src/
│   ├── main.rs              # App setup, CLI handling, asset source registration
│   ├── lib.rs               # Public API surface
│   ├── app/
│   │   ├── cli/             # CLI argument parsing (clap)
│   │   ├── systems/         # Bevy systems: camera, diagnostics
│   │   └── ui/              # egui-based UI: state, widgets, panels
│   └── lib/
│       ├── asset_loaders/   # Custom Bevy AssetLoaders (STL)
│       ├── coordinates.rs   # Source coordinate system transforms (Y-up, Z-up, etc.)
│       ├── lighting/        # Lighting config and placement
│       ├── mesh_info.rs     # Mesh bounds / dimensions tracking
│       ├── sequence/        # Core: discovery, loading, playback
│       ├── session/         # Session management
│       └── settings.rs      # Per-directory seaview.toml config
```

## Key Subsystems

### File Loading

All mesh loading goes through **Bevy's asset pipeline** for async I/O.

- **`seq://` asset source** — registered at startup pointing to the CLI-provided path (directory or file parent). This allows loading from anywhere on the filesystem, not just `assets/`.
- **STL** — custom `StlLoader` (`AssetLoader` impl) parses binary/ASCII STL via `stl_io`, produces indexed `Mesh` with positions + normals.
- **glTF/GLB** — Bevy's built-in `bevy_gltf`. Loaded via `GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }` to extract the first mesh.

Format detection is by file extension. The loader module (`sequence/loader.rs`) routes to the right strategy via `MeshFileFormat`.

### Sequence Pipeline

```
CLI path
  → handle_input_path()
    ├── Directory → DiscoverSequenceRequest (component)
    │                 → discovery system scans dir, matches numbered file patterns
    │                 → emits LoadSequenceRequest
    └── Single file → LoadSequenceRequest directly

LoadSequenceRequest
  → handle_load_requests() loads all frames via asset server
  → track_asset_loading() monitors AssetEvent<Mesh>, spawns entity on first frame
  → update_mesh_display() swaps mesh handle when frame index changes
```

### Playback

`SequenceManager` (resource) holds current sequence, frame index, play state. `SequencePlaybackPlugin` handles keyboard input (arrow keys, space, home/end, number keys for percentage jumps). Frame changes propagate via `SequenceEvent::FrameChanged` messages.

### Coordinate Systems

`SourceOrientation` maps from source coordinates (Y-up, Z-up, FluidX3D) to Bevy's Y-up right-handed system. Applied as a `Transform` on the mesh entity at spawn time.

### Settings

`seaview.toml` in the sequence directory persists camera position/rotation, playback speed, loop state, mesh bounds, and source coordinate preference. Loaded at startup, saved on `Ctrl+S`.

## Plugin Registration Order

```rust
app.add_plugins(DefaultPlugins)        // Bevy core + bevy_gltf
   .add_plugins(AssetLoadersPlugin)    // Registers StlLoader for .stl
   .add_plugins(SequencePlugin)        // Discovery + loader + playback
   .add_plugins(SessionPlugin)
   .add_plugins(SeaviewUiPlugin)       // egui panels
   .add_plugins(NightLightingPlugin)
   .add_plugins(MeshInfoPlugin)
```

## Data Flow

```
Filesystem ──[seq:// AssetSource]──→ AssetServer
  ├── .stl  → StlLoader  → Handle<Mesh>
  └── .glb  → GltfLoader → GltfAssetLabel → Handle<Mesh>

Handle<Mesh> stored in SequenceAssets.frame_handles[]
  → SequenceMeshDisplay entity swaps Mesh3d handle per frame
```
