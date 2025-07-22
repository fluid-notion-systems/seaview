use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

/// Plugin for handling all UI functionality in Seaview
pub struct SeaviewUiPlugin;

impl Plugin for SeaviewUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, ui_system);
    }
}

/// Main UI system that runs every frame
fn ui_system(mut contexts: EguiContexts) -> Result<(), bevy::ecs::query::QuerySingleError> {
    let ctx = contexts.ctx_mut()?;

    // Create a window with a test button
    egui::Window::new("Seaview Controls")
        .default_pos([10.0, 10.0])
        .show(ctx, |ui| {
            ui.heading("Phase 0.2 Test");

            if ui.button("Test Button").clicked() {
                info!("Test button clicked!");
            }

            ui.separator();

            ui.label("This is a simple egui integration test.");
            ui.label("If you can see this and click the button above,");
            ui.label("then egui is working correctly!");
        });

    Ok(())
}
