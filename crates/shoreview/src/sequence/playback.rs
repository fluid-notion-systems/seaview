//! Sequence playback control module

use super::{SequenceEvent, SequenceManager};
use bevy::prelude::*;

/// Plugin for sequence playback controls
pub struct SequencePlaybackPlugin;

impl Plugin for SequencePlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_playback_input, update_playback_ui));
    }
}

/// System that handles keyboard input for playback control
fn handle_playback_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sequence_manager: ResMut<SequenceManager>,
    mut events: EventWriter<SequenceEvent>,
) {
    // Only process input if we have a sequence loaded
    if sequence_manager.current_sequence.is_none() {
        return;
    }

    // Space bar - toggle play/pause
    if keyboard.just_pressed(KeyCode::Space) {
        sequence_manager.toggle_playback();
        if sequence_manager.is_playing {
            events.write(SequenceEvent::PlaybackStarted);
        } else {
            events.write(SequenceEvent::PlaybackStopped);
        }
    }

    // Right arrow - next frame
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        // Stop playback when manually stepping
        if sequence_manager.is_playing {
            sequence_manager.is_playing = false;
            events.write(SequenceEvent::PlaybackStopped);
        }

        if sequence_manager.next_frame() {
            events.write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
        }
    }

    // Left arrow - previous frame
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        // Stop playback when manually stepping
        if sequence_manager.is_playing {
            sequence_manager.is_playing = false;
            events.write(SequenceEvent::PlaybackStopped);
        }

        if sequence_manager.previous_frame() {
            events.write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
        }
    }

    // Home - jump to first frame
    if keyboard.just_pressed(KeyCode::Home) {
        if sequence_manager.is_playing {
            sequence_manager.is_playing = false;
            events.write(SequenceEvent::PlaybackStopped);
        }

        if sequence_manager.jump_to_frame(0) {
            events.write(SequenceEvent::FrameChanged(0));
        }
    }

    // End - jump to last frame
    if keyboard.just_pressed(KeyCode::End) {
        if sequence_manager.is_playing {
            sequence_manager.is_playing = false;
            events.write(SequenceEvent::PlaybackStopped);
        }

        if let Some(sequence) = sequence_manager.current_sequence() {
            let last_frame = sequence.frame_count().saturating_sub(1);
            if sequence_manager.jump_to_frame(last_frame) {
                events.write(SequenceEvent::FrameChanged(last_frame));
            }
        }
    }

    // Plus/Minus - adjust playback speed
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        let new_fps = (sequence_manager.playback_fps * 1.5).min(120.0);
        sequence_manager.set_playback_fps(new_fps);
        info!("Playback speed: {:.1} fps", new_fps);
    }

    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        let new_fps = (sequence_manager.playback_fps / 1.5).max(1.0);
        sequence_manager.set_playback_fps(new_fps);
        info!("Playback speed: {:.1} fps", new_fps);
    }

    // Number keys - jump to percentage of sequence
    for (key, percentage) in [
        (KeyCode::Digit1, 0.0),
        (KeyCode::Digit2, 0.1),
        (KeyCode::Digit3, 0.2),
        (KeyCode::Digit4, 0.3),
        (KeyCode::Digit5, 0.4),
        (KeyCode::Digit6, 0.5),
        (KeyCode::Digit7, 0.6),
        (KeyCode::Digit8, 0.7),
        (KeyCode::Digit9, 0.8),
        (KeyCode::Digit0, 0.9),
    ] {
        if keyboard.just_pressed(key) {
            if let Some(sequence) = sequence_manager.current_sequence() {
                let frame = (sequence.frame_count() as f32 * percentage) as usize;
                if sequence_manager.jump_to_frame(frame) {
                    events.write(SequenceEvent::FrameChanged(frame));
                }
            }
        }
    }
}

/// Component for UI text displaying playback status
#[derive(Component)]
pub struct PlaybackStatusText;

/// Component for UI text displaying frame info
#[derive(Component)]
pub struct FrameInfoText;

/// System that updates UI elements for playback
fn update_playback_ui(
    sequence_manager: Res<SequenceManager>,
    mut status_query: Query<&mut Text, (With<PlaybackStatusText>, Without<FrameInfoText>)>,
    mut frame_query: Query<&mut Text, (With<FrameInfoText>, Without<PlaybackStatusText>)>,
) {
    // Update playback status text
    for mut text in status_query.iter_mut() {
        let status = if sequence_manager.is_playing {
            format!("▶ Playing @ {:.1} fps", sequence_manager.playback_fps)
        } else {
            "⏸ Paused".to_string()
        };

        // Create new text with status
        *text = Text::new(status);
    }

    // Update frame info text
    for mut text in frame_query.iter_mut() {
        if let Some(sequence) = sequence_manager.current_sequence() {
            if let Some(frame_info) = sequence.frame_info(sequence_manager.current_frame) {
                let info = format!(
                    "Frame {}/{} - {}",
                    sequence_manager.current_frame + 1,
                    sequence.frame_count(),
                    frame_info.filename
                );

                *text = Text::new(info);
            }
        } else {
            *text = Text::new("No sequence loaded");
        }
    }
}

/// Controls for sequence playback
#[derive(Debug, Clone, Copy)]
pub enum PlaybackControl {
    Play,
    Pause,
    Stop,
    NextFrame,
    PreviousFrame,
    FirstFrame,
    LastFrame,
    SetSpeed(f32),
    JumpToFrame(usize),
    JumpToPercentage(f32),
}

/// Component that sends playback control commands
#[derive(Component)]
pub struct PlaybackControlSender {
    pub control: PlaybackControl,
}

/// Playback state information
#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_frame: usize,
    pub total_frames: usize,
    pub playback_speed: f32,
    pub current_time: f32,
    pub total_time: f32,
}

impl PlaybackState {
    pub fn from_manager(manager: &SequenceManager) -> Option<Self> {
        let sequence = manager.current_sequence()?;
        let total_frames = sequence.frame_count();
        let total_time = total_frames as f32 / manager.playback_fps;
        let current_time = manager.current_frame as f32 / manager.playback_fps;

        Some(Self {
            is_playing: manager.is_playing,
            current_frame: manager.current_frame,
            total_frames,
            playback_speed: manager.playback_fps,
            current_time,
            total_time,
        })
    }

    pub fn progress(&self) -> f32 {
        if self.total_frames > 0 {
            self.current_frame as f32 / self.total_frames as f32
        } else {
            0.0
        }
    }
}

/// Resource for tracking playback statistics
#[derive(Resource, Default)]
pub struct PlaybackStats {
    pub frames_played: usize,
    pub total_play_time: f32,
    pub average_frame_time: f32,
    pub dropped_frames: usize,
}

impl PlaybackStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, frame_time: f32) {
        self.frames_played += 1;
        self.total_play_time += frame_time;
        self.average_frame_time = self.total_play_time / self.frames_played as f32;
    }
}
