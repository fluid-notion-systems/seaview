//! UI module for the Seaview application

use crate::sequence::{LoadingState, SequenceEvent, SequenceManager};
use bevy::prelude::*;

/// Plugin for UI functionality
pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui).add_systems(
            Update,
            (
                update_status_text,
                update_loading_ui,
                handle_sequence_events,
            ),
        );
    }
}

/// Marker component for the status text
#[derive(Component)]
struct StatusText;

/// Marker component for frame info text
#[derive(Component)]
struct FrameInfoText;

/// Marker component for loading bar container
#[derive(Component)]
struct LoadingBarContainer;

/// Marker component for loading bar background
#[derive(Component)]
struct LoadingBarBackground;

/// Marker component for loading bar progress
#[derive(Component)]
struct LoadingBarProgress;

/// Marker component for loading text
#[derive(Component)]
struct LoadingText;

/// Setup the UI layout
fn setup_ui(mut commands: Commands) {
    // Create a text bundle for status display
    commands.spawn((
        Text::new("Press Space to play/pause\n← → to step frames"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(JustifyText::Left),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        StatusText,
    ));

    // Create frame info text
    commands.spawn((
        Text::new("No sequence loaded"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        TextLayout::new_with_justify(JustifyText::Left),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(60.0),
            ..default()
        },
        FrameInfoText,
    ));

    // Create loading bar container
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(50.0),
                height: Val::Px(30.0),
                left: Val::Percent(25.0),
                top: Val::Percent(45.0),
                ..default()
            },
            Visibility::Hidden,
            LoadingBarContainer,
        ))
        .with_children(|parent| {
            // Background of loading bar
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                    LoadingBarBackground,
                ))
                .with_children(|parent| {
                    // Progress bar
                    parent.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.8, 0.2)),
                        LoadingBarProgress,
                    ));
                });

            // Loading text
            parent.spawn((
                Text::new("Loading..."),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                LoadingText,
            ));
        });
}

/// Update status text based on sequence state
fn update_status_text(
    sequence_manager: Res<SequenceManager>,
    mut status_query: Query<&mut Text, (With<StatusText>, Without<FrameInfoText>)>,
    mut frame_query: Query<&mut Text, (With<FrameInfoText>, Without<StatusText>)>,
) {
    // Update status text
    for mut text in status_query.iter_mut() {
        let status = if sequence_manager.current_sequence.is_some() {
            if sequence_manager.is_playing {
                format!(
                    "▶ Playing @ {:.1} fps\nSpace: pause, ←→: step, Home/End: jump",
                    sequence_manager.playback_fps
                )
            } else {
                "⏸ Paused\nSpace: play, ←→: step, Home/End: jump".to_string()
            }
        } else {
            "No sequence loaded\nLoad a directory with STL files".to_string()
        };

        text.0 = status;
    }

    // Update frame info text
    for mut text in frame_query.iter_mut() {
        if let Some(sequence) = sequence_manager.current_sequence() {
            let frame_info = format!(
                "Frame {}/{} - {}",
                sequence_manager.current_frame + 1,
                sequence.frame_count(),
                sequence
                    .frame_info(sequence_manager.current_frame)
                    .map(|info| info.filename.clone())
                    .unwrap_or_else(|| "Unknown".to_string())
            );
            text.0 = frame_info;
        } else {
            text.0 = "No sequence loaded".to_string();
        }
    }
}

/// System to update loading UI
fn update_loading_ui(
    loading_state: Res<LoadingState>,
    mut container_query: Query<&mut Visibility, With<LoadingBarContainer>>,
    mut progress_query: Query<&mut Node, (With<LoadingBarProgress>, Without<LoadingText>)>,
    mut text_query: Query<&mut Text, With<LoadingText>>,
) {
    // Update visibility
    for mut visibility in container_query.iter_mut() {
        *visibility = if loading_state.is_preloading {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update progress bar width
    for mut node in progress_query.iter_mut() {
        node.width = Val::Percent(loading_state.progress() * 100.0);
    }

    // Update loading text
    for mut text in text_query.iter_mut() {
        text.0 = loading_state.progress_text();
    }
}

/// Handle sequence events
fn handle_sequence_events(mut events: EventReader<SequenceEvent>) {
    for event in events.read() {
        match event {
            SequenceEvent::SequenceLoaded(name) => {
                info!("UI: Sequence loaded - {}", name);
            }
            SequenceEvent::FrameChanged(frame) => {
                trace!("UI: Frame changed to {}", frame);
            }
            SequenceEvent::PlaybackStarted => {
                info!("UI: Playback started");
            }
            SequenceEvent::PlaybackStopped => {
                info!("UI: Playback stopped");
            }
            SequenceEvent::Error(msg) => {
                error!("UI: Error - {}", msg);
            }
            _ => {}
        }
    }
}
