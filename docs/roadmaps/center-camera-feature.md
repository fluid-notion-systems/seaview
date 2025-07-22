# Center Camera Feature - Implementation Notes

## Overview
This document summarizes the attempt to add a "center on mesh" button to the playback controls, including the issues encountered and lessons learned about the crate structure.

## Feature Description
- Add a 🎯 button to the playback controls that centers the camera on the current mesh
- Calculate centroid using 5% random sampling of vertices for performance
- Position camera at a good viewing distance (2.5x the mesh bounds radius)
- Automatically disable escape mode to allow immediate camera control

## Implementation Challenges

### 1. Crate Structure Issues
The main challenge was the mixed library/binary structure of the seaview crate:
- Both `lib.rs` and `main.rs` exist in `src/`
- Binary-specific modules (like `systems/camera.rs`) can't easily import from the library part
- This led to circular dependency issues when trying to share the `FpsCamera` component

### 2. Component Sharing
- `FpsCamera` was defined in the binary's `systems/camera.rs`
- UI systems in the library needed access to `FpsCamera` to modify it
- Moving `FpsCamera` to the library created import issues in the binary modules

### 3. Event System Complications
- Adding the `CenterOnMeshEvent` required coordination between library and binary
- Event registration needed to happen in the right order
- System parameter limits in Bevy required splitting the centering logic

## Attempted Solutions

### 1. Move Binary to src/bin/
- Created `src/bin/seaview/` structure
- Moved `main.rs` and binary-specific modules
- This follows some Rust conventions but doesn't align with Bevy best practices

### 2. Split Center System
- Separated event processing from camera update
- Used intermediate `CenterCameraRequest` component
- This worked around Bevy's system parameter limits

## Lessons Learned

### 1. Bevy Best Practices
After reviewing the Bevy best practices guide:
- Keep simple `lib.rs` and `main.rs` in `src/`
- Use minimal `main.rs` that delegates to plugins
- Use plugin pattern for organization
- Don't overcomplicate the structure

### 2. Better Approach
Instead of reorganizing the crate structure, a better approach would be:
1. Keep the current structure
2. Use a plugin-based architecture
3. Define shared components in the library with clear ownership
4. Use events for communication between systems
5. Consider using resources for camera control state

## Future Implementation Strategy

### Option 1: Camera Control Resource
```rust
// In library
#[derive(Resource)]
pub struct CameraControlRequest {
    pub center_on_mesh: bool,
    pub target_position: Option<Vec3>,
}

// UI writes to resource
// Camera system reads and clears
```

### Option 2: Command Pattern
```rust
// In library
pub enum CameraCommand {
    CenterOnMesh { position: Vec3, distance: f32 },
    ResetView,
}

// Use a channel or queue for commands
```

### Option 3: Simplified Event System
- Keep events simple with fewer parameters
- Use resources for complex state
- Minimize cross-boundary dependencies

## Conclusion
The feature itself is straightforward, but the crate organization made it complex. The lesson is to:
1. Follow established Bevy patterns
2. Keep the structure simple
3. Use plugins and events for decoupling
4. Don't fight the framework

The center camera feature should be reimplemented once the crate structure is clarified and a clear pattern for binary/library interaction is established.