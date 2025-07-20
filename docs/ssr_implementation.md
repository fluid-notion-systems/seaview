# Screen Space Reflections (SSR) Implementation in Seaview

This document describes the implementation of Screen Space Reflections (SSR) in the Seaview mesh viewer application.

## Overview

Screen Space Reflections have been added to Seaview to provide realistic reflections on mesh surfaces, enhancing the visual quality of loaded STL models. SSR creates reflections by reusing screen-space data, making it an efficient technique for real-time applications.

## Implementation Details

### 1. Deferred Rendering

SSR requires deferred rendering to function properly. This has been enabled in the main application setup:

```rust
App::new()
    .insert_resource(DefaultOpaqueRendererMethod::deferred())
    // ... other plugins
```

### 2. Camera Configuration

The camera has been configured with the following settings to support SSR:

- **HDR**: Enabled for better lighting calculations
- **MSAA**: Disabled (incompatible with deferred rendering)
- **SSR Component**: Added with default settings
- **Tonemapping**: Set to TonyMcMapface for better visual quality
- **Bloom**: Added for enhanced visual effects

```rust
commands.spawn((
    Camera3d::default(),
    Transform::from_xyz(100.0, 100.0, 100.0).looking_at(Vec3::new(37.0, 37.0, 27.5), Vec3::Y),
    FpsCamera::default(),
    Camera {
        hdr: true,
        ..default()
    },
    Msaa::Off,
    ScreenSpaceReflections::default(),
    Tonemapping::TonyMcMapface,
    Bloom { /* ... */ },
));
```

### 3. Material Properties

Materials have been optimized for SSR with the following properties:

#### STL Loader Materials
- **Base Color**: Light gray (0.9, 0.9, 0.9)
- **Metallic**: 0.3 (moderate metallic appearance)
- **Perceptual Roughness**: 0.2 (smooth surface for better reflections)
- **Reflectance**: 0.8 (high reflectance for visible SSR effects)

#### Sequence Loader Materials
Same properties as STL loader for consistency across different loading methods.

### 4. Lighting Adjustments

The ambient light brightness has been increased to 800.0 to compensate for the lack of environment maps and ensure proper visibility of models with the new material properties.

## Benefits

1. **Enhanced Realism**: SSR provides realistic reflections on mesh surfaces
2. **Performance**: Screen-space technique is efficient for real-time rendering
3. **No Additional Assets**: Works without requiring environment maps or reflection probes
4. **Automatic**: Applied to all loaded models without user intervention

## Limitations

1. **Screen Space Only**: Reflections are limited to what's visible on screen
2. **No MSAA**: Antialiasing is limited to post-processing techniques
3. **Performance Impact**: May affect performance on lower-end hardware

## Future Enhancements

1. **Environment Maps**: Optional environment map loading for enhanced reflections
2. **SSR Quality Settings**: User-configurable SSR quality levels
3. **Performance Profiles**: Different rendering profiles for various hardware capabilities
4. **Material Presets**: Different material presets for various types of models

## Usage

SSR is enabled by default when running Seaview. No additional configuration is required. The effect will be most visible on:
- Smooth surfaces
- Models with varying surface angles
- Scenes with multiple light sources

To see the best results, ensure your STL models have properly calculated normals and are not overly tessellated.