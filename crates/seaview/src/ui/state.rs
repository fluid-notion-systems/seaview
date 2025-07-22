//! UI state management for Seaview
//!
//! This module contains the global UI state that persists across frames
//! and is shared between different UI systems.

use bevy::prelude::*;
use uuid::Uuid;

/// Global UI state resource
#[derive(Resource, Default)]
pub struct UiState {
    /// Currently active session ID
    pub active_session: Option<Uuid>,

    /// Whether the session panel is visible
    pub show_session_panel: bool,

    /// Whether the network panel is visible
    pub show_network_panel: bool,

    /// Whether the playback controls are visible
    pub show_playback_controls: bool,

    /// Current playback state
    pub playback: PlaybackState,

    /// UI panel sizes for persistence
    pub panel_sizes: PanelSizes,

    /// Temporary UI state (dialogs, etc)
    pub temp_state: TempUiState,
}

/// Playback control state
#[derive(Default, Debug, Clone)]
pub struct PlaybackState {
    /// Whether playback is active
    pub is_playing: bool,

    /// Current frame index
    pub current_frame: usize,

    /// Total number of frames
    pub total_frames: usize,

    /// Playback speed multiplier
    pub speed: f32,

    /// Whether to loop at the end
    pub loop_enabled: bool,
}

/// Sizes of UI panels for layout persistence
#[derive(Debug, Clone)]
pub struct PanelSizes {
    /// Width of the left session panel
    pub session_panel_width: f32,

    /// Height of the bottom playback panel
    pub playback_panel_height: f32,

    /// Height of the top menu bar
    pub menu_bar_height: f32,
}

impl Default for PanelSizes {
    fn default() -> Self {
        Self {
            session_panel_width: 300.0,
            playback_panel_height: 100.0,
            menu_bar_height: 25.0,
        }
    }
}

/// Temporary UI state for dialogs and transient interactions
#[derive(Default, Debug, Clone)]
pub struct TempUiState {
    /// Whether to show the new session dialog
    pub show_new_session_dialog: bool,

    /// Whether to show the delete confirmation dialog
    pub show_delete_confirmation: Option<Uuid>,

    /// Current error message to display
    pub error_message: Option<String>,

    /// Current info message to display
    pub info_message: Option<String>,
}

impl UiState {
    /// Create a new UI state with default values
    /// Create a new UI state with sensible defaults
    pub fn new() -> Self {
        Self {
            show_session_panel: true,
            show_network_panel: true,
            show_playback_controls: true,
            playback: PlaybackState {
                speed: 1.0,
                total_frames: 100, // Mock data for testing
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Set the active session
    pub fn set_active_session(&mut self, session_id: Option<Uuid>) {
        self.active_session = session_id;
        // Reset playback when switching sessions
        self.playback.current_frame = 0;
        self.playback.is_playing = false;
    }

    /// Toggle playback state
    pub fn toggle_playback(&mut self) {
        self.playback.is_playing = !self.playback.is_playing;
    }

    /// Advance to the next frame
    pub fn next_frame(&mut self) {
        if self.playback.current_frame < self.playback.total_frames.saturating_sub(1) {
            self.playback.current_frame += 1;
        } else if self.playback.loop_enabled {
            self.playback.current_frame = 0;
        } else {
            self.playback.is_playing = false;
        }
    }

    /// Go to the previous frame
    pub fn previous_frame(&mut self) {
        if self.playback.current_frame > 0 {
            self.playback.current_frame -= 1;
        } else if self.playback.loop_enabled && self.playback.total_frames > 0 {
            self.playback.current_frame = self.playback.total_frames - 1;
        }
    }

    /// Jump to a specific frame
    pub fn seek_to_frame(&mut self, frame: usize) {
        self.playback.current_frame = frame.min(self.playback.total_frames.saturating_sub(1));
    }

    /// Show an error message
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.temp_state.error_message = Some(message.into());
        error!(
            "UI Error: {}",
            self.temp_state.error_message.as_ref().unwrap()
        );
    }

    /// Show an info message
    pub fn show_info(&mut self, message: impl Into<String>) {
        self.temp_state.info_message = Some(message.into());
        info!(
            "UI Info: {}",
            self.temp_state.info_message.as_ref().unwrap()
        );
    }

    /// Clear all temporary messages
    pub fn clear_messages(&mut self) {
        self.temp_state.error_message = None;
        self.temp_state.info_message = None;
    }
}

/// Event sent when the user requests to create a new session
#[derive(Event)]
pub struct CreateSessionEvent {
    pub name: String,
    pub source_type: SessionSourceType,
}

/// Event sent when the user requests to delete a session
#[derive(Event)]
pub struct DeleteSessionEvent {
    pub session_id: Uuid,
}

/// Event sent when the user requests to switch sessions
#[derive(Event)]
pub struct SwitchSessionEvent {
    pub session_id: Uuid,
}

/// Types of session sources
#[derive(Debug, Clone, PartialEq)]
pub enum SessionSourceType {
    Network { port: u16 },
    File { path: std::path::PathBuf },
    DataLake { connection_string: String },
}

/// Plugin for UI state management
pub struct UiStatePlugin;

impl Plugin for UiStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiState::new())
            .add_event::<CreateSessionEvent>()
            .add_event::<DeleteSessionEvent>()
            .add_event::<SwitchSessionEvent>();
    }
}
