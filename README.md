# Seaview

A modern reimplementation of mesh-ripper - a high-performance mesh sequence viewer for fluid simulations and time-series 3D data.

## System Requirements

### Linux
You need to install development libraries for Bevy:

```bash
# Ubuntu/Debian
sudo apt-get install libudev-dev libasound2-dev

# Fedora
sudo dnf install libudev-devel alsa-lib-devel

# Arch
sudo pacman -S udev alsa-lib
```

### Windows
No additional dependencies required.

### macOS
No additional dependencies required.

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Project Structure

```
seaview/
├── Cargo.toml           # Workspace configuration
├── crates/
│   └── seaview/       # Main application crate
│       ├── Cargo.toml
│       └── src/
│           └── main.rs  # Application entry point
├── research/            # Documentation and planning
│   └── reimplementation.md
└── vendor/              # Third-party code for reference
    └── mesh-ripper/
```

## Development Plan

See [research/reimplementation.md](research/reimplementation.md) for the detailed development roadmap.

## Phase 1 Status

Currently implementing Phase 1: Foundation and Core Infrastructure
- [x] Initialize Rust project with Bevy 0.14
- [x] Set up cargo workspace structure
- [x] Configure development environment
- [x] Create basic plugin architecture (minimal)
- [x] Implement minimal Bevy app with 3D scene
- [x] Set up FPS camera system with WASD controls
- [x] Mouse look controls with cursor grab/release
- [ ] Create simple mesh loading for single files
- [x] Implement basic lighting setup

### Camera Controls
- **WASD**: Move camera forward/backward/left/right
- **Q**: Move up
- **E**: Move down
- **Alt + Q/E**: Move up/down at half speed (for precise positioning)
- **Mouse**: Look around (click to grab cursor, Esc to release)
- **Escape Mode**: When Esc is pressed, all camera controls are disabled until you click again

## License

This project is dual-licensed under MIT OR Apache-2.0.
