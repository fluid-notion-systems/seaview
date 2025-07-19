//! Simple UI module for displaying playback status

use crate::sequence::{SequenceEvent, SequenceManager};
use bevy::prelude::*;

/// Plugin for UI functionality
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Update, (update_status_text, handle_sequence_events));
    }
}

/// Marker component for the status text
#[derive(Component)]
struct StatusText;

/// Marker component for the frame info text
#[derive(Component)]
struct FrameText;

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
        FrameText,
    ));
}

/// Update status text based on sequence state
fn update_status_text(
    sequence_manager: Res<SequenceManager>,
    mut status_query: Query<&mut Text, (With<StatusText>, Without<FrameText>)>,
    mut frame_query: Query<&mut Text, (With<FrameText>, Without<StatusText>)>,
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

    // Update frame text
    for mut text in frame_query.iter_mut() {
        if let Some(sequence) = &sequence_manager.current_sequence {
            let frame_text = format!(
                "Sequence: {}\nFrame {}/{} - {}",
                sequence.name,
                sequence_manager.current_frame + 1,
                sequence.frame_count(),
                sequence
                    .frames
                    .get(sequence_manager.current_frame)
                    .map(|f| f.filename.clone())
                    .unwrap_or_else(|| "Unknown".to_string())
            );
            text.0 = frame_text;
        } else {
            text.0 = "No sequence loaded".to_string();
        }
    }
}

/// Handle sequence events for UI feedback
fn handle_sequence_events(mut events: EventReader<SequenceEvent>) {
    for event in events.read() {
        match event {
            SequenceEvent::SequenceLoaded(name) => {
                info!("UI: Sequence loaded - {}", name);
            }
            SequenceEvent::FrameChanged(frame) => {
                debug!("UI: Frame changed to {}", frame);
            }
            SequenceEvent::PlaybackStarted => {
                debug!("UI: Playback started");
            }
            SequenceEvent::PlaybackStopped => {
                debug!("UI: Playback stopped");
            }
            SequenceEvent::Error(msg) => {
                error!("UI: Error - {}", msg);
            }
            _ => {}
        }
    }
}
