# Seaview Research Documentation

This directory contains research and planning documents for the Seaview mesh sequence viewer project.

## Documents

### [Reimplementation Plan](./reimplimentation.md)
A comprehensive phased approach to reimplementing mesh-ripper functionality using modern Rust and Bevy (0.14+). This is the main project roadmap outlining:

- 9 development phases from foundation to testing
- Technical architecture decisions
- Risk mitigation strategies
- Success metrics and timelines
- **Status**: Active development guide

### [Camera Plugins Research](./cameras.md)
Research and evaluation of available Bevy camera controller plugins for implementing FPS-style controls with recording/playback capabilities. Covers:

- Analysis of 6+ camera plugins
- Version compatibility and feature comparison
- Recommendations for Seaview implementation
- Custom implementation strategy
- **Status**: Research complete, implementation pending

### [Mesh File Loading Research](./mesh-loading.md)
Research on implementing multi-format mesh file loading in Bevy, with immediate focus on STL support. Covers:

- Native Bevy asset system architecture
- Third-party format support (STL, PLY, OBJ)
- Implementation strategies and code examples
- Performance considerations
- **Status**: Research complete, ready for implementation

## Project Status

Currently in **Phase 1: Foundation and Core Infrastructure**

### Completed:
- [x] Project workspace setup
- [x] Bevy 0.14 integration
- [x] Basic 3D scene rendering
- [x] Research documentation

### Next Steps:
- [ ] Simple mesh loading (Phase 1.2)
- [ ] Camera system implementation (Phase 4)
- [ ] File system utilities (Phase 1.3)

## Contributing

When adding new research documents:
1. Create the document in this directory
2. Add a link and description to this index
3. Follow the existing format for consistency
4. Include status information (research/planning/implementation/complete)

## Related Files

- [`../README.md`](../README.md) - Project overview and build instructions
- [`../Cargo.toml`](../Cargo.toml) - Workspace configuration
- [`../vendor/mesh-ripper/`](../vendor/mesh-ripper/) - Original implementation for reference
