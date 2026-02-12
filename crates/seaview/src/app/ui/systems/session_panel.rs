//! Session panel system for Seaview
//!
//! This module implements the left side panel that displays and manages sessions,
//! network status, and lighting rig configuration using an accordion-style UI.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use uuid::Uuid;

use crate::app::ui::state::{DeleteSessionEvent, SwitchSessionEvent, UiState};
use crate::lib::lighting::{NightLightingConfig, PlacementAlgorithm};
use crate::lib::session::SessionManager;

/// System that renders the session management panel
pub fn session_panel_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut switch_events: EventWriter<SwitchSessionEvent>,
    _delete_events: EventWriter<DeleteSessionEvent>,
    session_manager: Res<SessionManager>,
    mut lighting_config: ResMut<NightLightingConfig>,
) {
    if !ui_state.show_session_panel {
        debug!("Session panel is hidden");
        return;
    }

    debug!("Rendering session panel");

    let ctx = contexts.ctx_mut().unwrap();

    // Get real sessions from SessionManager
    let sessions = session_manager.get_all_sessions();

    let panel_width = ui_state.panel_sizes.session_panel_width;

    egui::SidePanel::left("session_panel")
        .resizable(true)
        .default_width(panel_width)
        .width_range(200.0..=500.0)
        .show(ctx, |ui| {
            debug!("Session panel UI callback running");
            // Update panel width if resized
            ui_state.panel_sizes.session_panel_width = ui.available_width();

            // Main scroll area for all accordion sections
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Sessions Section (Collapsible)
                    egui::CollapsingHeader::new("Sessions")
                        .default_open(ui_state.collapsible.sessions_open)
                        .show(ui, |ui| {
                            ui_state.collapsible.sessions_open = true;

                            if sessions.is_empty() {
                                ui.label("No sessions yet");
                                ui.label("Create a new session from the Session menu");
                            } else {
                                for session in sessions {
                                    let is_active = ui_state
                                        .active_session
                                        .map(|id| id == session.id)
                                        .unwrap_or(false);

                                    let delete_requested = ui
                                        .push_id(session.id, |ui| {
                                            render_session_item(
                                                ui,
                                                session,
                                                is_active,
                                                &mut switch_events,
                                            )
                                        })
                                        .inner;

                                    if let Some(session_id) = delete_requested {
                                        ui_state.temp_state.show_delete_confirmation =
                                            Some(session_id);
                                    }

                                    ui.add_space(5.0);
                                }
                            }
                        });

                    ui.add_space(5.0);

                    // Network Status Section (Collapsible)
                    if ui_state.show_network_panel {
                        egui::CollapsingHeader::new("Network Status")
                            .default_open(ui_state.collapsible.network_status_open)
                            .show(ui, |ui| {
                                ui_state.collapsible.network_status_open = true;

                                // Get sessions again to avoid borrow conflict
                                let sessions_for_network = session_manager.get_all_sessions();
                                let network_sessions: Vec<_> = sessions_for_network
                                    .iter()
                                    .filter_map(|s| match &s.source {
                                        crate::lib::session::types::SessionSource::Network {
                                            port,
                                            ..
                                        } => Some((*port, s.frame_count())),
                                        _ => None,
                                    })
                                    .collect();

                                if network_sessions.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label("Status:");
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 255, 0),
                                            "● No Active",
                                        );
                                    });
                                } else {
                                    for (port, frame_count) in network_sessions {
                                        ui.horizontal(|ui| {
                                            ui.label("Port:");
                                            ui.label(format!("{}", port));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Frames:");
                                            ui.label(format!("{}", frame_count));
                                        });
                                    }
                                }

                                ui.add_space(10.0);

                                ui.horizontal(|ui| {
                                    if ui.button("Start").clicked() {
                                        info!("Starting network receiver");
                                    }
                                    if ui.button("Stop").clicked() {
                                        info!("Stopping network receiver");
                                    }
                                });
                            });

                        ui.add_space(5.0);
                    }

                    // Lighting Rig Section (Collapsible)
                    egui::CollapsingHeader::new("Lighting Rig")
                        .default_open(ui_state.collapsible.lighting_rig_open)
                        .show(ui, |ui| {
                            ui_state.collapsible.lighting_rig_open = true;

                            render_lighting_rig_controls(ui, &mut lighting_config);
                        });
                });
        });
}

/// Render a single session item in the list
fn render_session_item(
    ui: &mut egui::Ui,
    session: &crate::lib::session::Session,
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
                    egui::RichText::new(format!("{} frames", session.frame_count()))
                        .small()
                        .color(egui::Color32::from_gray(180)),
                );
                ui.label(
                    egui::RichText::new(session.source.display_string())
                        .small()
                        .color(egui::Color32::from_gray(180)),
                );
                ui.label(
                    egui::RichText::new(session.created_at.format("%Y-%m-%d %H:%M").to_string())
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

/// Render lighting rig controls
fn render_lighting_rig_controls(ui: &mut egui::Ui, config: &mut NightLightingConfig) {
    // Enable/Disable toggle
    ui.horizontal(|ui| {
        ui.label("Enabled:");
        ui.checkbox(&mut config.enabled, "");
    });

    ui.add_space(5.0);

    // Only show controls if enabled
    if !config.enabled {
        ui.label(egui::RichText::new("Enable to configure").color(egui::Color32::GRAY));
        return;
    }

    ui.separator();

    // Number of lights slider
    ui.horizontal(|ui| {
        ui.label("Lights:");
        ui.add(egui::Slider::new(&mut config.num_lights, 1..=100).suffix(" lights"));
    });

    ui.add_space(5.0);

    // Placement algorithm chooser
    ui.horizontal(|ui| {
        ui.label("Layout:");
    });
    egui::ComboBox::from_id_salt("placement_algorithm")
        .selected_text(config.placement_algorithm.name())
        .show_ui(ui, |ui| {
            for algorithm in PlacementAlgorithm::all() {
                ui.selectable_value(
                    &mut config.placement_algorithm,
                    *algorithm,
                    algorithm.name(),
                );
            }
        });

    ui.add_space(5.0);

    // Height slider (logarithmic scale)
    ui.horizontal(|ui| {
        ui.label("Height:");
    });
    let mut log_height = config.height.log10();
    if ui
        .add(
            egui::Slider::new(&mut log_height, 0.7..=2.7) // 5m to 500m
                .custom_formatter(|value, _| format!("{:.1} m", 10_f64.powf(value)))
                .suffix(""),
        )
        .changed()
    {
        config.height = 10_f32.powf(log_height);
    }

    ui.add_space(5.0);

    // Cone angle slider
    ui.horizontal(|ui| {
        ui.label("Cone Angle:");
    });
    ui.add(egui::Slider::new(&mut config.cone_angle, 5.0..=180.0).suffix("°"));

    ui.add_space(5.0);
    ui.separator();

    // Light properties section
    ui.label(egui::RichText::new("Light Properties").strong());

    ui.add_space(5.0);

    // Intensity slider (logarithmic)
    ui.horizontal(|ui| {
        ui.label("Intensity:");
    });
    let mut log_intensity = config.intensity.log10();
    if ui
        .add(
            egui::Slider::new(&mut log_intensity, 1.0..=5.0) // 10 to 100,000
                .custom_formatter(|value, _| format!("{:.0}", 10_f64.powf(value)))
                .suffix(""),
        )
        .changed()
    {
        config.intensity = 10_f32.powf(log_intensity);
    }

    ui.add_space(5.0);

    // Range slider
    ui.horizontal(|ui| {
        ui.label("Range:");
    });
    ui.add(egui::Slider::new(&mut config.range, 10.0..=1000.0).suffix(" m"));

    ui.add_space(5.0);

    // Color picker
    ui.horizontal(|ui| {
        ui.label("Color:");
    });

    // Convert Bevy Color to egui color
    let mut color_array = [
        config.color.to_srgba().red,
        config.color.to_srgba().green,
        config.color.to_srgba().blue,
    ];

    if ui.color_edit_button_rgb(&mut color_array).changed() {
        config.color = Color::srgb(color_array[0], color_array[1], color_array[2]);
    }
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
