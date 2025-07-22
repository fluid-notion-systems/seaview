//! Playback controls system for Seaview
//!
//! This module implements the bottom panel with timeline and playback controls.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::ui::state::UiState;

/// System that renders the playback controls panel
pub fn playback_controls_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    time: Res<Time>,
) {
    if !ui_state.show_playback_controls {
        return;
    }

    let ctx = contexts.ctx_mut().unwrap();

    let panel_height = ui_state.panel_sizes.playback_panel_height;

    egui::TopBottomPanel::bottom("playback_panel")
        .resizable(false)
        .exact_height(panel_height)
        .show(ctx, |ui| {
            // Add 5px margin at the top
            ui.add_space(5.0);

            ui.vertical(|ui| {
                // Playback controls row
                ui.horizontal(|ui| {
                    ui.add_space(10.0);

                    // Previous frame button
                    if ui.button("⏮").on_hover_text("Previous frame").clicked() {
                        ui_state.previous_frame();
                    }

                    // Play/Pause button
                    let play_pause_text = if ui_state.playback.is_playing {
                        "⏸"
                    } else {
                        "▶"
                    };
                    if ui
                        .button(play_pause_text)
                        .on_hover_text(if ui_state.playback.is_playing {
                            "Pause"
                        } else {
                            "Play"
                        })
                        .clicked()
                    {
                        ui_state.toggle_playback();
                    }

                    // Next frame button
                    if ui.button("⏭").on_hover_text("Next frame").clicked() {
                        ui_state.next_frame();
                    }

                    ui.separator();

                    // Frame counter
                    ui.label(format!(
                        "Frame: {} / {}",
                        ui_state.playback.current_frame, ui_state.playback.total_frames
                    ));

                    ui.separator();

                    // Speed control
                    ui.label("Speed:");
                    ui.add(
                        egui::Slider::new(&mut ui_state.playback.speed, 0.1..=5.0)
                            .text("x")
                            .fixed_decimals(1)
                            .custom_formatter(|n, _| format!("{:.1}x", n))
                            .custom_parser(|s| s.trim_end_matches('x').parse::<f64>().ok()),
                    );

                    ui.separator();

                    // Loop toggle
                    ui.checkbox(&mut ui_state.playback.loop_enabled, "Loop");

                    // Add some spacing before the right side
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // FPS display
                        let fps = 1.0 / time.delta_secs();
                        ui.label(format!("FPS: {:.0}", fps));
                        ui.separator();

                        // Time display
                        if ui_state.playback.total_frames > 0 {
                            let current_time = ui_state.playback.current_frame as f32 / 30.0; // Assuming 30 fps
                            let total_time = ui_state.playback.total_frames as f32 / 30.0;
                            ui.label(format!("Time: {:.1}s / {:.1}s", current_time, total_time));
                        }
                    });
                });

                ui.add_space(5.0);

                // Timeline scrubber
                ui.horizontal(|ui| {
                    ui.add_space(10.0);

                    let total_frames = ui_state.playback.total_frames.saturating_sub(1);
                    let timeline_response = ui.add(
                        egui::Slider::new(&mut ui_state.playback.current_frame, 0..=total_frames)
                            .show_value(false),
                    );

                    if timeline_response.changed() {
                        let current_frame = ui_state.playback.current_frame;
                        ui_state.seek_to_frame(current_frame);
                    }

                    ui.add_space(10.0);
                });

                // Progress bar visualization
                ui.horizontal(|ui| {
                    ui.add_space(10.0);

                    let progress = if ui_state.playback.total_frames > 0 {
                        ui_state.playback.current_frame as f32
                            / ui_state.playback.total_frames as f32
                    } else {
                        0.0
                    };

                    let desired_size = egui::Vec2::new(ui.available_width() - 20.0, 10.0);
                    let (rect, _response) =
                        ui.allocate_exact_size(desired_size, egui::Sense::hover());

                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter();

                        // Background
                        painter.rect_filled(rect, 2.0, egui::Color32::from_gray(50));

                        // Progress
                        if progress > 0.0 {
                            let progress_rect = egui::Rect::from_min_size(
                                rect.min,
                                egui::Vec2::new(rect.width() * progress, rect.height()),
                            );
                            painter.rect_filled(
                                progress_rect,
                                2.0,
                                egui::Color32::from_rgb(100, 150, 255),
                            );
                        }

                        // Frame markers (every 10% of the timeline)
                        for i in 1..10 {
                            let marker_x = rect.min.x + (rect.width() * (i as f32 / 10.0));
                            painter.line_segment(
                                [
                                    egui::pos2(marker_x, rect.min.y),
                                    egui::pos2(marker_x, rect.max.y),
                                ],
                                egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                            );
                        }
                    }
                });
            });
        });
}

/// System that handles automatic playback advancement
pub fn playback_update_system(
    mut ui_state: ResMut<UiState>,
    time: Res<Time>,
    mut last_update: Local<f32>,
) {
    if !ui_state.playback.is_playing || ui_state.playback.total_frames == 0 {
        return;
    }

    // Calculate frame duration based on speed
    let frame_duration = 1.0 / (30.0 * ui_state.playback.speed); // 30 FPS base rate

    *last_update += time.delta_secs();

    if *last_update >= frame_duration {
        *last_update -= frame_duration;
        ui_state.next_frame();
    }
}
