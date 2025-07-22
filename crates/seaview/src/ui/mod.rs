//! User interface module for Seaview
//!
//! This module contains all UI-related functionality using bevy_egui.
//! The UI is organized into separate submodules for different features.

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

pub mod systems;
pub mod widgets;

pub use systems::*;

/// Main UI plugin that sets up all UI functionality
pub struct SeaviewUiPlugin;

impl Plugin for SeaviewUiPlugin {
    fn build(&self, app: &mut App) {
        // Add bevy_egui plugin if not already added
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }

        // Register UI systems
        app.add_plugins(systems::UiSystemsPlugin);

        // Log that the UI plugin is loaded
        info!("Seaview UI Plugin loaded - Phase 0.2 egui integration active");
    }
}
