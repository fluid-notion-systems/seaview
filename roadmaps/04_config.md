# Configuration Management Roadmap

## Overview

This roadmap outlines the implementation of a comprehensive configuration system for Seaview, including UI state persistence, user preferences, and application settings.

## Goals

1. **UI State Persistence**: Save and restore UI layout, panel sizes, and visibility states
2. **User Preferences**: Store user-specific settings like default directories, network ports, and playback preferences
3. **Session Management**: Persist session configurations and metadata
4. **Cross-platform Support**: Use appropriate config directories for each platform
5. **Hot Reload**: Support live configuration updates without restart
6. **Migration Support**: Handle configuration version upgrades gracefully

## Architecture

### Configuration Structure

```
~/.config/seaview/                    # Linux/macOS
%APPDATA%/seaview/                    # Windows
├── config.toml                       # Main application config
├── ui_state.toml                     # UI layout and state
├── sessions/                         # Session configurations
│   ├── {session_id}.toml
│   └── index.toml                    # Session index/metadata
└── cache/                           # Temporary data
    └── recent_files.toml
```

### Configuration Schema

#### Main Configuration (config.toml)
```toml
[app]
version = "0.1.0"
last_opened = "2024-01-15T10:30:00Z"

[defaults]
mesh_directory = "/home/user/meshes"
network_port = 9877
auto_load_last_session = true

[performance]
max_mesh_size_mb = 1000
cache_size_mb = 500
parallel_loading_threads = 4

[rendering]
default_material = "standard"
ambient_light_brightness = 500.0
enable_shadows = true
```

#### UI State (ui_state.toml)
```toml
[panels]
show_session_panel = true
show_network_panel = true
show_playback_controls = true

[panel_sizes]
session_panel_width = 300.0
playback_panel_height = 100.0
menu_bar_height = 25.0

[window]
width = 1920
height = 1080
maximized = false
position = [100, 100]

[last_session]
active_session_id = "550e8400-e29b-41d4-a716-446655440000"
```

## Implementation Phases

### Phase 1: Basic Configuration System (Week 1)

#### 1.1 Core Configuration Infrastructure
- [ ] Create `ConfigManager` resource
- [ ] Implement TOML serialization/deserialization
- [ ] Add platform-specific path resolution
- [ ] Create default configurations

```rust
#[derive(Resource, Serialize, Deserialize)]
pub struct ConfigManager {
    pub app_config: AppConfig,
    pub ui_state: UiStateConfig,
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn load() -> Result<Self>;
    pub fn save(&self) -> Result<()>;
    pub fn get_config_dir() -> PathBuf;
    pub fn migrate_if_needed(&mut self) -> Result<()>;
}
```

#### 1.2 UI State Persistence
- [ ] Save UI state on shutdown
- [ ] Load UI state on startup
- [ ] Handle missing/corrupted configs gracefully
- [ ] Add UI state reset functionality

### Phase 2: Session Configuration (Week 2)

#### 2.1 Session Config Management
- [ ] Create session configuration format
- [ ] Implement session index management
- [ ] Add session metadata storage
- [ ] Support session templates

#### 2.2 Auto-save and Recovery
- [ ] Implement periodic auto-save
- [ ] Add crash recovery
- [ ] Create backup system
- [ ] Handle concurrent access

### Phase 3: User Preferences (Week 3)

#### 3.1 Preference System
- [ ] Create preference categories
- [ ] Add preference UI panels
- [ ] Implement preference validation
- [ ] Support preference profiles

#### 3.2 Import/Export
- [ ] Export configuration bundles
- [ ] Import configuration from file
- [ ] Support configuration sharing
- [ ] Add configuration templates

### Phase 4: Advanced Features (Week 4)

#### 4.1 Hot Reload
- [ ] File watcher for config changes
- [ ] Live configuration updates
- [ ] Validation before applying
- [ ] Rollback on errors

#### 4.2 Cloud Sync (Future)
- [ ] Configuration sync API
- [ ] Conflict resolution
- [ ] Selective sync
- [ ] Offline support

## Technical Implementation

### Dependencies
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
directories = "5.0"
notify = "6.0"  # For hot reload
```

### File Organization
```
crates/seaview/src/
├── config/
│   ├── mod.rs           # ConfigManager and core types
│   ├── app_config.rs    # Application configuration
│   ├── ui_config.rs     # UI state configuration
│   ├── session_config.rs # Session configuration
│   ├── migration.rs     # Config migration logic
│   └── watcher.rs       # Hot reload implementation
```

### Error Handling
- Use custom error types for configuration errors
- Provide helpful error messages
- Always fallback to defaults
- Log configuration issues

### Testing Strategy
- Unit tests for serialization/deserialization
- Integration tests for file operations
- Migration tests with old config formats
- Platform-specific path tests

## Integration Points

### With UI System
- Load UI state before creating UI
- Save UI state on panel resize/visibility change
- Debounce saves to prevent excessive I/O
- Provide UI for config management

### With Session Manager
- Store session configs in dedicated directory
- Index sessions for quick loading
- Support session templates
- Handle orphaned session configs

### With Network System
- Save network preferences
- Remember recent connections
- Store connection profiles
- Quick connect from history

## Success Metrics

1. **Reliability**: Zero data loss from config corruption
2. **Performance**: < 50ms to load/save configs
3. **Usability**: Intuitive preference management
4. **Portability**: Works on all target platforms
5. **Maintainability**: Easy to add new config options

## Future Enhancements

### Advanced Features
- Configuration encryption for sensitive data
- Multi-user support with profile switching
- Configuration inheritance/cascading
- RESTful API for remote configuration

### Integration Features
- Integration with OS-specific preference systems
- Command-line config overrides
- Environment variable support
- Configuration documentation generation

### Developer Features
- Configuration schema validation
- Type-safe configuration access
- Configuration change notifications
- Debug configuration dumping

## Migration Strategy

### Version 1 → Version 2
- Add version field to all configs
- Write migration functions for each version
- Keep backups of original configs
- Provide rollback mechanism

### Legacy Support
- Import from old INI/JSON formats
- Convert from competitor formats
- Preserve user customizations
- Document breaking changes