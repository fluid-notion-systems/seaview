//! UI systems module
//!
//! This module contains all the UI systems and their registration logic.

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

mod event_handlers;
mod menu_bar;
mod playback_controls;
mod session_panel;

pub use event_handlers::*;
pub use menu_bar::*;
pub use playback_controls::*;
pub use session_panel::*;

/// Plugin that registers all UI systems
pub struct UiSystemsPlugin;

impl Plugin for UiSystemsPlugin {
    fn build(&self, app: &mut App) {
        // Register all UI systems to run in the EguiPrimaryContextPass schedule
        app.add_systems(
            EguiPrimaryContextPass,
            (
                menu_bar_system,
                session_panel_system,
                new_session_dialog_system,
                delete_confirmation_dialog_system,
                message_display_system,
                playback_controls_system,
            ),
        );

        // Register update systems that run outside of egui pass
        app.add_systems(
            Update,
            (
                handle_switch_session_events,
                handle_delete_session_events,
                handle_create_session_events,
                playback_update_system,
            ),
        );
    }
}
