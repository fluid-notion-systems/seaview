# Mesh Ripper Reimplementation Game Plan

## Overview

This document outlines a phased approach to reimplementing mesh-ripper functionality using modern Rust and Bevy (0.14+). The goal is to create a modular, maintainable, and performant mesh sequence viewer while addressing the limitations of the original implementation.

## Baby Shark Integration Plan

**Goal**: Standardize mesh representation across the codebase using baby_shark's geometry processing capabilities

### Key Components:
1. **Unified Mesh Representation**
   - Use `baby_shark::mesh::Mesh` as the standard indexed mesh format
   - Implement converters a vertex list into `baby_shark::mesh::Mesh`, utilizing CornerTable inter
   - Leverage baby_shark's mesh processing algorithms

2. **Mesh Conversion Pipeline**
   - Polygon soup → Mesh conversion via `merge_points`
   - Automatic normal calculation and repair
   - Mesh validation and cleanup

3. **Integration Points**
   - STL loader → baby_shark `CornerTable`
   - PLY/OBJ loaders → baby_shark `CornerTable`
   - Mesh generation (marching cubes, etc.) → baby_shark conversion
   - BRP mesh transmission → Send indexed format

### Implementation Steps:
1. Add baby_shark dependency to Cargo.toml
2. Create mesh conversion trait for all loaders
3. Update STL loader to output `CornerTable`
4. Modify mesh_sender_test to use baby_shark conversion
5. Create utilities for `CornerTable` ↔ Bevy `Mesh` conversion

## Phase 1: Foundation with Multi-Format Mesh Support
**Duration: 4-5 weeks**
**Goal: Establish project structure, basic rendering, and support multiple mesh formats**

### Milestones:
1. **Project Setup**
   - [x] Initialize new Rust project with Bevy 0.16
   - [x] Set up cargo workspace structure
   - [x] Configure development environment (rustfmt, clippy, CI)
   - [x] Create basic plugin architecture

2. **Basic Rendering Pipeline**
   - [x] Implement minimal Bevy app with 3D scene
   - [x] Set up basic camera system (static first)
   - [x] Implement basic lighting setup
   - [ ] Create ground plane and reference objects



3. **Mesh Format Abstraction**
   - [x] Design unified mesh loading interface
   - [x] Create mesh format detection system
   - [x] Implement error handling for loading
   - [ ] Create plugin system for format loaders
   - [ ] **Baby Shark Integration**
     - [ ] Add baby_shark dependency
     - [ ] Create `CornerTable` ↔ Bevy `Mesh` converters
     - [ ] Implement mesh conversion trait

4. **Format Implementations**
   - [x] STL loader (already implemented)
   - [ ] PLY loader (using bevy_ply or custom)
   - [ ] OBJ loader with normals/UV support
   - [ ] GLTF/GLB support (bonus)

5. **Mesh Processing Pipeline**
   - [x] Vertex/index buffer management
   - [x] Normal calculation for formats lacking them
   - [x] Bounding box calculation (via baby_shark)
   - [ ] Basic mesh statistics
   - [x] Mesh validation and repair (via baby_shark)
   - [ ] **Baby Shark Processing**
     - [ ] Polygon soup to indexed conversion
     - [ ] Mesh simplification integration
     - [ ] Boolean operations support

### Deliverables:
- [x] Basic app that can load and display STL files from any filesystem path
- [x] Direct STL file loading (bypassing Bevy asset system)
- [x] CLI argument parsing for file/directory input
- [ ] Modular plugin structure with format loader plugins
- [x] Unified mesh representation
- [x] Basic mesh processing utilities

## Phase 2: Sequence Management
**Duration: 3-4 weeks**
**Goal: Handle mesh sequences efficiently**

### Milestones:
1. **File System Foundation**
   - [x] Design file discovery trait/interface
   - [x] Implement directory scanning
   - [x] Build path management utilities
2. **Sequence Discovery**
   - [x] Pattern-based sequence detection
   - [x] Frame number extraction
   - [x] Sequence validation

3. **Memory Management**
   - [x] Implement mesh pool with configurable size
   - [x] LRU cache for loaded meshes
   - [x] Preloading/prefetching system
   - [x] Memory usage monitoring

4. **Playback System**
   - [x] Timeline representation
   - [x] Play/pause/stop controls
   - [x] Frame rate control
   - [x] Frame stepping (forward/backward)

### Deliverables:
- [x] Efficient mesh sequence loading with direct filesystem access
- [x] Smooth playback of sequences with visual mesh updates
- [x] Memory-conscious design with LRU caching
- [x] Support for directory-based sequence discovery
- [x] Real-time sequence preloading with progress tracking

## Phase 3: Advanced Camera System
**Duration: 3-4 weeks**
**Goal: Implement FPS-style camera with recording/playback**

### Milestones:
1. **Interactive Camera**
   - [x] FPS-style mouse look (already implemented)
   - [x] WASD movement (already implemented)
   - [x] Speed modifiers (Alt for slower movement)
   - [x] Cursor grab/release with Escape key
   - [ ] Smooth movement interpolation

2. **Camera Recording**
   - [ ] Keyframe data structure
   - [ ] Recording system with timeline
   - [ ] Interpolation between keyframes
   - [ ] Multiple timeline support

3. **Camera Playback**
   - [ ] Timeline playback system
   - [ ] Sync with mesh sequence
   - [ ] Timeline visualization
   - [ ] Export/import camera paths

### Deliverables:
- Fully interactive camera system
- Camera recording and playback
- Camera path visualization
<!--
## Phase 4: Particle and Point Cloud Rendering
**Duration: 2-3 weeks**
**Goal: Efficient rendering of particle data**

### Milestones:
1. **Point Cloud Rendering**
   - [ ] Detect meshes without indices
   - [ ] Implement point sprite rendering
   - [ ] GPU instancing for particles

2. **Particle Features**
   - [ ] Variable particle size
   - [ ] Color mapping (by velocity, pressure, etc.)
   - [ ] Directional arrows for velocity
   - [ ] Particle sampling for performance

3. **Render Optimization**
   - [ ] Frustum culling
   - [ ] LOD for particles
   - [ ] Render caching system -->

### Deliverables:
- Efficient particle rendering
- Visual options for scientific data
- Performance optimization

## Phase 5: User Interface and Configuration
**Duration: 2-3 weeks**
**Goal: Create intuitive UI and persistent configuration**

### Milestones:
1. **UI Framework**
   - [ ] Integrate egui or bevy_ui
   - [ ] Design UI layout system
   - [ ] Create reusable UI components

2. **Control Panels**
   - [ ] Playback controls
   - [ ] Render settings
   - [ ] Camera controls
   - [ ] Performance metrics

3. **Configuration System**
   - [ ] Settings serialization (RON/TOML)
   - [ ] Per-dataset configuration
   - [ ] Hotkey customization
   - [ ] Theme support

### Deliverables:
- Full-featured UI
- Persistent configuration
- User-friendly controls

## Phase 6: Level of Detail and Performance
**Duration: 2-3 weeks**
**Goal: Optimize for large datasets**

### Milestones:
1. **LOD System**
   - [ ] Intelligent file sampling
   - [ ] Progressive loading
   - [ ] Dynamic LOD switching
   - [ ] Quality settings

2. **Async Loading**
   - [ ] Background mesh loading
   - [ ] Loading progress indication
   - [ ] Cancellable operations
   - [ ] Priority queue for loading

3. **Performance Monitoring**
   - [ ] FPS counter
   - [ ] Memory usage tracking
   - [ ] GPU timing
   - [ ] Performance profiling integration

### Deliverables:
- Scalable LOD system
- Non-blocking asset loading
- Performance analytics

## Phase 7: Advanced Features and Polish
**Duration: 3-4 weeks**
**Goal: Add professional features and polish**

### Milestones:
1. **Advanced Rendering**
   - [ ] Multiple mesh display
   - [ ] Transparency support
   - [ ] Advanced materials
   - [ ] Post-processing effects

2. **Data Analysis Tools**
   - [ ] Mesh statistics overlay
   - [ ] Measurement tools
   - [ ] Clipping planes
   - [ ] Cross-sections

3. **Export and Integration**
   - [ ] Screenshot/video export
   - [ ] Camera path export
   - [ ] Integration with simulation tools
   - [ ] Plugin system for extensions

### Deliverables:
- Professional visualization features
- Analysis tools
- Export capabilities

## Phase 8: Testing and Documentation
**Duration: 2 weeks**
**Goal: Ensure reliability and usability**

### Milestones:
1. **Testing**
   - [ ] Unit tests for core systems
   - [ ] Integration tests
   - [ ] Performance benchmarks
   - [ ] Example datasets

2. **Documentation**
   - [ ] API documentation
   - [ ] User guide
   - [ ] Developer documentation
   - [ ] Video tutorials

### Deliverables:
- Comprehensive test suite
- Complete documentation
- Example projects

## Technical Considerations

### Architecture Principles:
1. **Modularity**: Each system should be independent and testable
2. **Performance**: Design for large datasets from the start
3. **Extensibility**: Plugin architecture for custom features
4. **Error Handling**: Graceful degradation and clear error messages
5. **Modern Rust**: Use latest idioms and best practices

### Key Design Decisions:
1. **ECS Architecture**: Leverage Bevy's ECS for all systems
2. **Async First**: Use async for all I/O operations
3. **GPU Compute**: Utilize GPU for particle processing
4. **Zero-Copy**: Minimize data copying for performance
5. **Progressive Enhancement**: Basic features work everywhere

### Technology Stack:
- **Bevy 0.16**: Latest stable version (updated from 0.14)
- **wgpu**: Modern graphics API
- **egui**: Immediate mode UI
- **rfd**: Native file dialogs
- **notify**: File system watching
- **serde**: Configuration serialization
- **stl_io**: STL file format support
- **bevy_brp_extras**: Remote protocol support
- **baby_shark**: Geometry processing and mesh format conversion

## Risk Mitigation

### Identified Risks:
1. **Bevy API Changes**: Maintain compatibility layer
2. **Performance Regression**: Continuous benchmarking
3. **Memory Usage**: Implement strict limits
4. **File Format Compatibility**: Extensive testing
5. **Platform Differences**: CI for all platforms

### Mitigation Strategies:
- Incremental development with working versions
- Performance regression tests
- Memory profiling from early phases
- Extensive example dataset testing
- Cross-platform CI from Phase 1

## Success Metrics

### Performance Targets:
- 60 FPS with 1M particles
- < 100ms mesh switching time
- < 1GB memory for 1000 frame sequence
- < 5s startup time

### Feature Completeness:
- All original mesh-ripper features
- Modern UI/UX improvements
- Better error handling
- Extended file format support

### Code Quality:
- 80%+ test coverage
- Zero clippy warnings
- Documented public APIs
- Example code for all features

## Current Progress

### Completed:
- [x] Basic Bevy 0.16 application setup
- [x] STL file loading support (direct filesystem access, no asset system dependency)
- [x] FPS-style camera controls (WASD movement, mouse look, cursor grab/release)
- [x] Basic lighting and materials
- [x] CLI argument parsing for single files and directories
- [x] Bevy Remote Protocol integration
- [x] File system utilities and sequence discovery
- [x] Pattern-based sequence detection with regex support
- [x] Mesh sequence loading with LRU caching
- [x] Playback system with play/pause/step controls
- [x] Memory-efficient mesh preloading
- [x] Visual mesh updates during sequence playback
- [x] Debug logging and progress tracking
- [x] Mesh caching system implementation
- [x] Performance improvements and optimization
- [x] Coordinate system transformation support
- [x] Robust STL loading with error handling
- [x] Normal validation and correction
- [x] Material system updates
- [x] Async mesh loading implementation
- [x] Screen space reflections (SSR) experimentation
- [x] BRP mesh sender test implementation
- [x] Network-based mesh transmission via BRP

### In Progress:
- [ ] Baby shark integration for mesh processing
- [ ] Polygon soup to indexed mesh conversion in mesh_sender_test
- [ ] Fixing inverted normals and mesh rendering issues
- [ ] Additional mesh format support (PLY, OBJ)
- [ ] Camera recording and playback system
- [ ] UI framework integration

## Next Steps

1. **Implement baby_shark integration** (Immediate priority)
   - Add dependency and create conversion utilities
   - Update mesh_sender_test to use indexed format
   - Test with existing STL sequences
2. **Standardize mesh pipeline** around baby_shark
   - Update STL loader to output `CornerTable`
   - Create common mesh processing utilities
   - Implement mesh statistics and validation
3. **Complete additional format support** (PLY, OBJ)
   - All formats should output to baby_shark `CornerTable`
   - Ensure consistent processing pipeline
4. **Begin Phase 3** - advanced camera features (recording/playback)
5. **Add UI framework** for better user controls
6. **Establish performance benchmarks** for optimization

This reimplementation will result in a modern, maintainable, and performant mesh visualization tool that exceeds the capabilities of the original mesh-ripper while addressing its limitations.
