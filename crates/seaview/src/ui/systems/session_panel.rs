//! Session panel system for Seaview
//!
//! This module implements the left side panel that displays and manages sessions.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use uuid::Uuid;

use crate::ui::state::{DeleteSessionEvent, SwitchSessionEvent, UiState};

/// Mock session data for now (will be replaced with actual SessionManager)
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: Uuid,
    pub name: String,
    pub frame_count: usize,
    pub source: String,
    pub created: String,
}

/// System that renders the session management panel
pub fn session_panel_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut switch_events: EventWriter<SwitchSessionEvent>,
    _delete_events: EventWriter<DeleteSessionEvent>,
) {
    if !ui_state.show_session_panel {
        debug!("Session panel is hidden");
        return;
    }

    debug!("Rendering session panel");

    let ctx = contexts.ctx_mut().unwrap();

    // Create some mock sessions for testing
    let mock_sessions = vec![
        SessionInfo {
            id: Uuid::new_v4(),
            name: "Simulation Run #42".to_string(),
            frame_count: 847,
            source: "Network (192.168.1.100)".to_string(),
            created: "2024-01-15 10:30".to_string(),
        },
        SessionInfo {
            id: Uuid::new_v4(),
            name: "Simulation Run #43".to_string(),
            frame_count: 1200,
            source: "File (sim_43.tar.gz)".to_string(),
            created: "2024-01-15 14:22".to_string(),
        },
        SessionInfo {
            id: Uuid::new_v4(),
            name: "Test Session".to_string(),
            frame_count: 150,
            source: "Network (localhost:9877)".to_string(),
            created: "2024-01-16 09:15".to_string(),
        },
    ];

    let panel_width = ui_state.panel_sizes.session_panel_width;

    egui::SidePanel::left("session_panel")
        .resizable(true)
        .default_width(panel_width)
        .width_range(200.0..=500.0)
        .show(ctx, |ui| {
            debug!("Session panel UI callback running");
            // Update panel width if resized
            ui_state.panel_sizes.session_panel_width = ui.available_width();

            ui.heading("Sessions");
            ui.separator();

            // Session list
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for session in &mock_sessions {
                        let is_active = ui_state
                            .active_session
                            .map(|id| id == session.id)
                            .unwrap_or(false);

                        let delete_requested = ui
                            .push_id(session.id, |ui| {
                                render_session_item(ui, session, is_active, &mut switch_events)
                            })
                            .inner;

                        if let Some(session_id) = delete_requested {
                            ui_state.temp_state.show_delete_confirmation = Some(session_id);
                        }

                        ui.add_space(5.0);
                    }
                });

            ui.separator();

            // Network status section
            if ui_state.show_network_panel {
                ui.add_space(10.0);
                ui.heading("Network Status");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(egui::Color32::from_rgb(0, 255, 0), "● Connected");
                });

                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.label("9877");
                });

                ui.horizontal(|ui| {
                    ui.label("Received:");
                    ui.label("0 frames");
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Start").clicked() {
                        info!("Starting network receiver");
                    }
                    if ui.button("Stop").clicked() {
                        info!("Stopping network receiver");
                    }
                });
            }
        });
}

/// Render a single session item in the list
fn render_session_item(
    ui: &mut egui::Ui,
    session: &SessionInfo,
    is_active: bool,
    switch_events: &mut EventWriter<SwitchSessionEvent>,
) -> Option<Uuid> {
    let mut delete_requested = None;
    let frame_color = if is_active {
        egui::Color32::from_rgb(100, 150, 255)
    } else {
        egui::Color32::from_gray(80)
    };

    egui::Frame::new()
        .fill(if is_active {
            egui::Color32::from_gray(50)
        } else {
            egui::Color32::from_gray(30)
        })
        .stroke(egui::Stroke::new(1.0, frame_color))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Session name
                ui.horizontal(|ui| {
                    if is_active {
                        ui.label(
                            egui::RichText::new(&session.name)
                                .strong()
                                .color(egui::Color32::from_rgb(150, 200, 255)),
                        );
                    } else {
                        ui.label(&session.name);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Delete button
                        if ui
                            .small_button("🗑")
                            .on_hover_text("Delete session")
                            .clicked()
                        {
                            delete_requested = Some(session.id);
                        }
                    });
                });

                // Session info
                ui.label(
                    egui::RichText::new(format!("{} frames", session.frame_count))
                        .small()
                        .color(egui::Color32::from_gray(180)),
                );
                ui.label(
                    egui::RichText::new(&session.source)
                        .small()
                        .color(egui::Color32::from_gray(180)),
                );
                ui.label(
                    egui::RichText::new(&session.created)
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );

                // Activate button
                if !is_active {
                    ui.add_space(5.0);
                    if ui.button("Activate").clicked() {
                        switch_events.write(SwitchSessionEvent {
                            session_id: session.id,
                        });
                    }
                }
            });
        });

    delete_requested
}

/// System that shows the delete confirmation dialog
pub fn delete_confirmation_dialog_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut delete_events: EventWriter<DeleteSessionEvent>,
) {
    if let Some(session_id) = ui_state.temp_state.show_delete_confirmation {
        let ctx = contexts.ctx_mut().unwrap();

        let mut show_dialog = true;

        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Are you sure you want to delete this session?");
                ui.label("This action cannot be undone.");

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        delete_events.write(DeleteSessionEvent { session_id });
                        show_dialog = false;
                    }

                    if ui.button("Cancel").clicked() {
                        show_dialog = false;
                    }
                });
            });

        if !show_dialog {
            ui_state.temp_state.show_delete_confirmation = None;
        }
    }
}
