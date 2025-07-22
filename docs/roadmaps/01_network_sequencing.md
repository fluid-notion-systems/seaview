# Network Sequencing Roadmap

## Overview

This roadmap outlines the implementation of network-based sequencing capabilities for Seaview, including session management, real-time mesh streaming, and a comprehensive UI built with bevy_egui.

## Goals

1. **Complete UI Cleanup**: ✅ Remove all existing UI code and fix any bevy_egui interaction issues
2. **Enhanced Logging/Debugging**: ✅ Standardize on tracing with maximum debug output
3. **Session Management**: 🚧 Organize received network data into sessions with persistent configuration
4. **Real-time Network Streaming**: 🚧 Receive and display mesh sequences from network sources
5. **UI Enhancement**: ✅ Build new UI from scratch with bevy_egui
6. **Data Persistence**: ⏳ Store session data locally with future data lake integration
7. **Multi-source Support**: ⏳ Handle multiple simultaneous network sources

**Current Focus**: Session Management backend implementation with network integration testing

## Architecture Components

### 1. Session Management System

#### Session Structure
```
.config/fluid-notion/seaview/sessions/{session_id}.toml
sessions/{session_id}/
├── meshes/           # Received mesh files
├── metadata/         # Frame metadata
└── thumbnails/       # Preview images (future)
```

#### Session Configuration Schema
```toml
[session]
id = "uuid-v4"
name = "Fluid Simulation Run #42"
created_at = "2024-01-15T10:30:00Z"
last_accessed = "2024-01-15T11:45:00Z"
source_type = "network" # or "file", "data_lake"

[network]
source_address = "192.168.1.100:8080"
protocol_version = "1.0"
total_frames_expected = 1000
frames_received = 847

[playback]
current_frame = 0
playback_speed = 1.0
loop_enabled = false
auto_cleanup = true

[metadata]
simulation_uuid = "sim-12345"
timestep = 0.001
spatial_bounds = { min = [-10.0, -10.0, -10.0], max = [10.0, 10.0, 10.0] }
```

### 2. UI Migration to bevy_egui

#### Dependencies to Add
```toml
[dependencies]
bevy_egui = "0.30"
egui_extras = "0.30"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
uuid = { version = "1.0", features = ["v4"] }
directories = "5.0"
```

#### UI Layout Structure
```
┌─────────────────────────────────────────────────────┐
│ Menu Bar: File | Session | View | Network | Help    │
├─────────────────────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────────────────────────┐ │
│ │ Session     │ │ 3D Viewport                     │ │
│ │ Manager     │ │                                 │ │
│ │             │ │                                 │ │
│ │ [Sessions]  │ │                                 │ │
│ │ ├─Sim#42    │ │                                 │ │
│ │ ├─Sim#43    │ │                                 │ │
│ │ └─Sim#44    │ │                                 │ │
│ │             │ │                                 │ │
│ │ [Network]   │ │                                 │ │
│ │ Status:●On  │ │                                 │ │
│ │ Port: 8080  │ │                                 │ │
│ │             │ │                                 │ │
│ └─────────────┘ └─────────────────────────────────┘ │
├─────────────────────────────────────────────────────┤
│ Playback Controls: [◀◀] [⏸] [▶] [▶▶] Frame: 42/1000│
│ Timeline: ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
└─────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 0: UI Cleanup & Debugging (Week 0 - Critical Foundation)

#### 0.1 Complete UI Removal
- [x] Remove existing `UIPlugin` completely
- [x] Remove all bevy_ui related code from main.rs
- [x] Remove ui/mod.rs and all UI-related systems
- [x] Clean up any leftover UI components/resources
- [x] Verify app runs without any UI (3D viewport only)

#### 0.2 Use Bevy's Built-in Logging & Maximum Debugging
- [x] ✅ Bevy's logging already correctly configured via DefaultPlugins
- [x] ✅ RUST_LOG environment variable works (e.g., RUST_LOG=seaview=trace)
- [x] ✅ Module-specific filtering working perfectly
- [x] ✅ All systems logging properly with trace/debug/info levels
- [x] ✅ Add bevy_egui dependency (uses Bevy's logging automatically)
- [x] ✅ Remove old log/env_logger dependencies (cleanup - kept for binary tools only)

```toml
# Add to Cargo.toml - minimal additions
[dependencies]
bevy_egui = "0.35.1"  # Latest version for Bevy 0.16

# Keep old logging only for binary tools
log = "0.4"         # Still needed for binary tools
env_logger = "0.10" # Still needed for binary tools
```

#### 0.3 Minimal bevy_egui Sanity Test
- [x] ✅ Add bevy_egui with minimal setup
- [x] ✅ Create single button that prints to console when clicked
- [x] ✅ Add extensive tracing around button creation/interaction
- [x] ✅ Test actual button clicking functionality
- [x] ✅ Verify mouse events are reaching egui properly
- [x] ✅ Fix cursor grab interference with egui (camera now respects egui input)
- [x] ✅ Created modular UI structure with proper plugin architecture

```rust
// Minimal test system
fn debug_ui_system(mut egui_ctx: EguiContexts) {
    tracing::trace!("debug_ui_system called");

    egui::Window::new("Debug Test")
        .show(egui_ctx.ctx_mut(), |ui| {
            tracing::trace!("Window content rendering");

            if ui.button("TEST BUTTON").clicked() {
                tracing::error!("BUTTON CLICKED - SUCCESS!");
                println!("BUTTON CLICKED - SUCCESS!");
            }

            ui.label(format!("Mouse pos: {:?}", ui.input(|i| i.pointer.hover_pos())));
            ui.label(format!("Frame: {}", ui.ctx().frame_nr()));
        });
}
```

#### 0.4 Input Event Debugging
- [x] ✅ Add comprehensive input event logging (using Bevy's built-in tracing)
- [x] ✅ Trace mouse position, clicks, hover states (visible in test panel)
- [x] ✅ Log egui context state and focus (button clicks logged)
- [x] ✅ Verify bevy window events reach egui (working correctly)
- [x] ✅ Test with different input methods (mouse clicks work, cursor grab fixed)

### Phase 0 Complete! ✅
All UI cleanup and bevy_egui integration tasks completed successfully:
- Removed all old UI code
- Bevy's built-in logging configured and working
- bevy_egui integrated with proper modular structure
- Input handling fixed (camera respects egui focus)
- Test panel with working button interaction

### Phase 0.5: UI State Persistence (See roadmaps/04_config.md)
- [ ] Implement basic configuration system for UI state (deferred - see Phase 4)
- [ ] Save/load panel visibility and sizes
- [ ] Remember last active session
- [ ] Platform-specific config directories

### Phase 1: Foundation (Week 1-2) ✅ COMPLETE

#### 1.1 Build New UI Foundation (Post-Sanity Test)
- [x] ✅ Create comprehensive EguiUIPlugin architecture (modular structure in place)
- [x] ✅ Implement basic window layout with panels
- [x] ✅ Add proper error handling for UI operations
- [x] ✅ Create UI state management system

**Completed Features**:
- Full menu bar with File, Session, View, Network, Help menus
- Session panel with mock sessions (left sidebar)
- Playback controls with timeline (bottom panel)
- UI state management with events
- Proper egui/camera interaction (no cursor grab by default)
- Message display system for errors/info
- Delete confirmation dialogs
- New session dialog

**Current Priority**: Session Management Infrastructure (Phase 1.2)

#### 1.2 Session Management Infrastructure (CURRENT PHASE)
- [ ] Create `SessionManager` resource
- [ ] Implement session configuration loading/saving (deferred to Phase 4)
- [ ] Create session directory structure utilities
- [ ] Add session CRUD operations
- [ ] Test with `mesh_sender_test` binary for network ingestion

```rust
// Core session types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub session: SessionInfo,
    pub network: NetworkConfig,
    pub playback: PlaybackConfig,
    pub metadata: SessionMetadata,
}

#[derive(Resource)]
pub struct SessionManager {
    sessions: HashMap<Uuid, SessionConfig>,
    active_session: Option<Uuid>,
    sessions_dir: PathBuf,
    config_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Self;
    pub fn create_session(&mut self, name: String, source: SessionSource) -> Result<Uuid>;
    pub fn load_session(&mut self, id: Uuid) -> Result<()>;
    pub fn save_session(&self, id: Uuid) -> Result<()>;
    pub fn delete_session(&mut self, id: Uuid) -> Result<()>;
    pub fn list_sessions(&self) -> Vec<&SessionConfig>;
}
```

### Phase 2: Network Integration (Week 3-4) - NEXT PRIORITY

#### 2.1 Enhanced Network Receiver
- [ ] Extend existing `MeshReceiver` for session integration
- [ ] Add session-aware mesh storage
- [ ] Implement automatic session creation from network streams
- [ ] Add connection status monitoring
- [ ] Test with `mesh_sender_test` for initial integration
- [ ] Test with FluidX3D for real fluid simulation data

#### 2.2 Session-Network Bridge
- [ ] Create `NetworkSessionReceiver` component
- [ ] Implement mesh-to-file persistence
- [ ] Add frame metadata extraction and storage
- [ ] Handle connection lifecycle events

```rust
#[derive(Component)]
pub struct NetworkSessionReceiver {
    session_id: Uuid,
    receiver: MeshReceiver,
    frames_received: usize,
    last_frame_time: Instant,
}

impl NetworkSessionReceiver {
    pub fn start_session(port: u16, session_name: String) -> Result<Self>;
    pub fn stop_session(&mut self) -> Result<()>;
    pub fn get_session_stats(&self) -> NetworkSessionStats;
}
```

### Phase 3: UI Enhancement (Week 5-6) - MOSTLY COMPLETE

#### 3.1 Session Management UI
- [x] ✅ Session dropdown/list widget (mock data ready)
- [x] ✅ Session creation dialog
- [x] ✅ Session properties panel (showing in list)
- [x] ✅ Session deletion confirmation

#### 3.2 Network Control UI
- [x] ✅ Network connection panel (in session panel)
- [x] ✅ Start/stop network receiver buttons
- [x] ✅ Connection status indicators (mock)
- [ ] Real-time statistics display (needs real data)

#### 3.3 Enhanced Playback Controls
- [x] ✅ Timeline scrubber with frame markers
- [x] ✅ Playback speed controls
- [x] ✅ Loop mode toggle
- [x] ✅ Frame step controls
- [x] ✅ Auto-advance playback system

```rust
pub struct EguiUIPlugin;

impl Plugin for EguiUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_systems(Update, (
                session_manager_ui,
                network_control_ui,
                playback_controls_ui,
                viewport_overlay_ui,
                menu_bar_ui,
            ));
    }
}

fn session_manager_ui(
    mut egui_ctx: EguiContexts,
    mut session_manager: ResMut<SessionManager>,
    mut commands: Commands,
) {
    egui::SidePanel::left("session_panel")
        .resizable(true)
        .default_width(300.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            // Session list, controls, etc.
        });
}
```

### Phase 4: Data Persistence (Week 7-8)

#### 4.1 Robust File Management
- [ ] Implement mesh file compression
- [ ] Add file integrity checking
- [ ] Create cleanup and archival utilities
- [ ] Handle disk space management

#### 4.2 Session Import/Export
- [ ] Export sessions to portable format
- [ ] Import sessions from archives
- [ ] Session sharing capabilities
- [ ] Backup and restore functionality

### Phase 5: Advanced Features (Week 9-10)

#### 5.1 Multi-source Support
- [ ] Support multiple simultaneous network connections
- [ ] Session merging and comparison
- [ ] Source priority management
- [ ] Conflict resolution strategies

#### 5.2 Performance Optimization
- [ ] Lazy loading for large sessions
- [ ] Background mesh processing
- [ ] Memory usage optimization
- [ ] Network buffer management

## Technical Considerations

### File Organization
```
crates/seaview/src/
├── session/
│   ├── mod.rs          # SessionManager and core types
│   ├── config.rs       # Configuration serialization
│   ├── storage.rs      # File system operations
│   └── network.rs      # Network-session integration
├── ui/
│   ├── mod.rs          # EguiUIPlugin (rebuilt from scratch)
│   ├── debug.rs        # Debug UI and sanity tests
│   ├── session_ui.rs   # Session management UI
│   ├── network_ui.rs   # Network controls UI
│   ├── playback_ui.rs  # Enhanced playback UI
│   └── viewport_ui.rs  # 3D viewport overlays
└── network/
    ├── mod.rs          # Existing network code
    ├── receiver.rs     # Enhanced MeshReceiver
    └── session_bridge.rs # Session integration
```

### Configuration Management
- See `roadmaps/04_config.md` for detailed configuration implementation
- Use `directories` crate for cross-platform config paths
- Implement configuration versioning for future migrations
- Add validation for session configurations
- Support both TOML and JSON formats
- UI state persistence (panel sizes, visibility, etc.)

### Debugging Strategy
- Maximum verbosity tracing for UI interactions
- Separate trace targets for different systems (ui, input, egui)
- Environment variable controls for debug levels
- Real-time debug overlay showing system state
- Comprehensive event logging for troubleshooting

### Error Handling
- Comprehensive error types for session operations
- Graceful degradation for network failures
- User-friendly error messages in UI
- Automatic recovery where possible

## Testing Strategy

### Unit Tests
- [ ] Session configuration serialization/deserialization
- [ ] File system operations
- [ ] Network-session integration
- [ ] UI component behavior

### Integration Tests
- [ ] End-to-end session lifecycle
- [ ] Network streaming with session storage
- [ ] UI interaction workflows
- [ ] Performance benchmarks

### Manual Testing Scenarios
- [ ] Create session from network stream
- [ ] Switch between multiple sessions
- [ ] Handle network disconnections gracefully
- [ ] Verify file system cleanup

## Future Considerations

### Configuration System
- Full implementation detailed in `roadmaps/04_config.md`
- Hot reload support for configuration changes
- Import/export of configuration bundles
- Cloud sync capabilities

### Data Lake Integration
- Design session format to be compatible with future data lake
- Consider cloud storage backends
- Plan for metadata indexing and search
- Prepare for distributed session management

### Advanced Analytics
- Session comparison tools
- Performance metrics dashboard
- Simulation analysis features
- Export to analysis formats

### Collaboration Features
- Session sharing between users
- Real-time collaborative viewing
- Comments and annotations
- Version control for sessions

## Success Metrics

1. **Usability**: Users can create and manage sessions through intuitive UI
2. **Performance**: Handle 60fps network streams without frame drops
3. **Reliability**: Graceful handling of network interruptions
4. **Storage**: Efficient disk usage with configurable cleanup
5. **Scalability**: Support for 100+ sessions with fast switching

## Dependencies Impact

### New Dependencies
- Use Bevy's built-in logging (no additional logging crates needed)
- `bevy_egui`: Modern immediate-mode GUI
- `egui_extras`: Additional egui widgets
- `uuid`: Session ID generation
- `directories`: Cross-platform config paths
- `serde`: Configuration serialization

### Existing Code Changes
- **REMOVE** all existing UI code completely
- Configure Bevy's LogPlugin for maximum debug output
- Use Bevy's built-in tracing macros (info!, debug!, trace!) throughout
- Add comprehensive debug logging to input systems
- Extend network receiver for session integration
- Update sequence manager for session compatibility
- Enhance file loading for session-based storage

### Critical Success Criteria for Phase 0
1. **UI Cleanup**: App runs cleanly with zero UI code
2. **Button Test**: Single bevy_egui button responds to clicks reliably
3. **Debug Visibility**: Full tracing of all UI/input events
4. **Platform Testing**: Works on target platforms/window managers
5. **Event Flow**: Clear understanding of input event pipeline
