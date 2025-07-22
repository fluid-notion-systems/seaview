//! UI systems module
//!
//! This module contains all the UI systems and their registration logic.

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

mod test_panel;

pub use test_panel::*;

/// Plugin that registers all UI systems
pub struct UiSystemsPlugin;

impl Plugin for UiSystemsPlugin {
    fn build(&self, app: &mut App) {
        // Register all UI systems to run in the EguiPrimaryContextPass schedule
        app.add_systems(EguiPrimaryContextPass, test_panel_system);
    }
}
