# Bevy Camera Plugins Research

## Overview

This document researches available camera controller plugins for Bevy 0.14+, evaluating their suitability for the Seaview mesh sequence viewer project. The goal is to identify the best camera solution for implementing FPS-style controls with recording/playback capabilities.

## Available Camera Plugins

### 1. smooth-bevy-cameras
**Repository**: https://github.com/bonsairobo/smooth-bevy-cameras  
**Version Compatibility**: Supports Bevy 0.14  
**Stars**: 269+ ⭐  
**License**: MIT  

**Features**:
- Exponential smoothing for buttery smooth camera movement
- Multiple camera controllers (FPS, orbit, unreal)
- Configurable smoothing parameters
- Well-maintained and popular

**Pros**:
- Professional smoothing implementation
- Multiple controller types
- Good documentation
- Active maintenance

**Cons**:
- May be overkill for simple use cases
- Smoothing might interfere with precise camera recording

### 2. bevy_flycam
**Repository**: https://github.com/sburris0/bevy_flycam  
**Version Compatibility**: Up to Bevy 0.10 (outdated)  
**Stars**: 155+ ⭐  
**License**: ISC  

**Features**:
- Simple first-person fly camera
- WASD movement + mouse look
- Shift/Space for vertical movement
- Minimal dependencies

**Pros**:
- Very simple to integrate
- Lightweight
- Good for prototyping

**Cons**:
- Not updated for recent Bevy versions
- Limited features
- No advanced controls

### 3. bevy_fps_controller
**Repository**: https://github.com/qhdwight/bevy_fps_controller  
**Version Compatibility**: Bevy 0.15 (latest)  
**Stars**: Not visible in search results  
**License**: MIT/Apache  

**Features**:
- Source engine-inspired movement
- Air strafing and bunny hopping
- Crouching and sprinting
- Noclip mode
- Slope support
- Requires bevy_rapier3d physics

**Pros**:
- Advanced FPS movement mechanics
- Physics-based
- Configurable settings
- Recently updated

**Cons**:
- Requires physics engine
- May be too game-specific
- Complex for simple visualization

### 4. bevy_panorbit_camera
**Repository**: https://github.com/Plonq/bevy_panorbit_camera  
**Version Compatibility**: Bevy 0.15 (latest)  
**Documentation**: https://docs.rs/bevy_panorbit_camera  
**License**: MIT/Apache-2.0  

**Features**:
- Orbit/pan camera controller
- Smooth rotation and panning
- Zoom support
- Touch input support
- Configurable limits and constraints

**Pros**:
- Excellent for 3D model viewing
- Well-documented API
- Touch support
- Active maintenance

**Cons**:
- Orbit-style may not suit FPS needs
- Limited to orbit camera pattern

### 5. bevy_fpc (First Person Controller)
**Repository**: https://codeberg.org/Eternahl/bevy_fpc  
**Version Compatibility**: Bevy 0.13  
**License**: MIT/Apache  

**Features**:
- Rapier-based character controller
- Modular design
- Sprint support (optional)
- Configurable inputs

**Pros**:
- Modular architecture
- Good for game development
- Physics integration

**Cons**:
- Requires Rapier physics
- Not updated for latest Bevy

### 6. bevy_fly_camera
**Documentation**: https://docs.rs/bevy_fly_camera  
**Version Compatibility**: Bevy 0.10 (outdated)  
**License**: MIT  

**Features**:
- Simple 2D/3D flying camera
- Minecraft-style controls
- Basic movement system

**Pros**:
- Very simple
- Easy to understand

**Cons**:
- Severely outdated
- Limited features

## Specialized Tools

### bevy_transform_gizmo
**Repository**: https://github.com/ForesightMiningSoftwareCorporation/bevy_transform_gizmo  
**Purpose**: Transform manipulation gizmo  

**Features**:
- 3D transform gizmo
- Translation and rotation handles
- Always renders on top
- Constant screen size
- Requires bevy_mod_picking

**Use Case**: Good for editor-like functionality, not camera control

### bevy_editor_pls
**Repository**: https://github.com/jakobhellermann/bevy_editor_pls  
**Stars**: 675+ ⭐  

**Features**:
- Complete editor-like interface
- Includes fly camera
- Inspector panels
- Performance diagnostics
- State switching

**Use Case**: Full editor solution, may be too heavy for production use

## Comparison Table

| Plugin | Bevy Version | Movement Style | Physics Required | Recording Support | Maintenance |
|--------|--------------|----------------|------------------|-------------------|-------------|
| smooth-bevy-cameras | 0.14 | FPS/Orbit/Unreal | No | No | Active |
| bevy_fps_controller | 0.15 | FPS (Source-style) | Yes (Rapier) | No | Active |
| bevy_panorbit_camera | 0.15 | Orbit | No | No | Active |
| bevy_flycam | 0.10 | Fly | No | No | Inactive |
| bevy_fpc | 0.13 | FPS | Yes (Rapier) | No | Moderate |
| bevy_fly_camera | 0.10 | Fly | No | No | Inactive |

## Recommendations for Seaview

### Primary Recommendation: Custom Implementation with smooth-bevy-cameras as Base

**Rationale**:
1. None of the existing plugins provide camera recording/playback functionality
2. smooth-bevy-cameras provides excellent smoothing algorithms we can build upon
3. We need specific features for mesh sequence visualization
4. Recording system requires tight integration with camera state

### Implementation Strategy:

1. **Start with smooth-bevy-cameras**
   - Use the FPS controller as a base
   - Extract smoothing algorithms
   - Remove unnecessary features

2. **Add Recording Layer**
   ```rust
   // Conceptual structure
   pub struct RecordableCamera {
       controller: FpsCameraController,
       recorder: CameraRecorder,
       playback: CameraPlayback,
   }
   ```

3. **Key Features to Implement**
   - Keyframe recording with timestamps
   - Interpolation between keyframes
   - Timeline visualization
   - Export/import of camera paths
   - Sync with mesh sequence playback

### Alternative: Fork bevy_fps_controller

If physics-based movement is desired:
- Fork bevy_fps_controller
- Remove game-specific features
- Add recording system
- Simplify to visualization needs

## Implementation Considerations

### 1. State Management
```rust
enum CameraMode {
    Interactive,
    Recording,
    Playback,
}
```

### 2. Keyframe Structure
```rust
struct CameraKeyframe {
    time: f32,
    position: Vec3,
    rotation: Quat,
    fov: f32,
}
```

### 3. Interpolation Methods
- Linear interpolation for position
- Spherical linear interpolation (slerp) for rotation
- Smooth step for FOV changes

### 4. UI Integration
- Timeline scrubber
- Record/play/pause buttons
- Keyframe markers
- Speed controls

## Code Example: Basic Integration

```rust
use bevy::prelude::*;
use smooth_bevy_cameras::{LookTransform, FpsCameraController, FpsCameraPlugin};

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle::default(),
        FpsCameraController::default(),
        LookTransform::default(),
        // Our custom recording component
        CameraRecorder::default(),
    ));
}

fn add_camera_plugins(app: &mut App) {
    app.add_plugins(FpsCameraPlugin::default())
       .add_plugins(CameraRecordingPlugin); // Our custom plugin
}
```

## Conclusion

For the Seaview project, building a custom camera system on top of smooth-bevy-cameras' algorithms provides the best balance of:
- Modern Bevy compatibility
- Smooth controls
- Flexibility for recording features
- Maintainable codebase

The recording/playback system will need to be custom-built regardless of the base camera plugin chosen, as none of the existing solutions provide this functionality out of the box.