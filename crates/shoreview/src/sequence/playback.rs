//! Sequence playback control module

use super::{SequenceEvent, SequenceManager};
use bevy::prelude::*;

/// Plugin for sequence playback controls
pub struct SequencePlaybackPlugin;

impl Plugin for SequencePlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyHoldTimer>()
            .add_systems(Update, (handle_playback_input, update_playback_ui));
    }
}

/// Resource for tracking key hold timing
#[derive(Resource)]
struct KeyHoldTimer {
    /// Time since arrow key was first pressed
    arrow_hold_time: f32,
    /// Last frame change time
    last_frame_change: f32,
    /// Initial delay before repeat starts
    initial_delay: f32,
    /// Repeat interval
    repeat_interval: f32,
}

impl Default for KeyHoldTimer {
    fn default() -> Self {
        Self {
            arrow_hold_time: 0.0,
            last_frame_change: 0.0,
            initial_delay: 0.5,    // 500ms before repeat starts
            repeat_interval: 0.05, // 50ms between frames when holding (20 fps)
        }
    }
}

/// System that handles keyboard input for playback control
fn handle_playback_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sequence_manager: ResMut<SequenceManager>,
    mut events: EventWriter<SequenceEvent>,
    time: Res<Time>,
    mut key_timer: ResMut<KeyHoldTimer>,
) {
    // Debug: Log all pressed keys
    for key in keyboard.get_just_pressed() {
        info!("Key pressed: {:?}", key);
    }

    // Debug: Log current keyboard state every second
    static mut LAST_DEBUG: f64 = 0.0;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    unsafe {
        if now - LAST_DEBUG > 1.0 {
            LAST_DEBUG = now;
            let pressed_keys: Vec<_> = keyboard.get_pressed().collect();
            info!("Debug: Currently pressed keys: {:?}", pressed_keys);
            info!(
                "Debug: Sequence loaded: {}",
                sequence_manager.current_sequence.is_some()
            );
            if let Some(seq) = &sequence_manager.current_sequence {
                info!(
                    "Debug: Current frame: {}/{}, Playing: {}",
                    sequence_manager.current_frame,
                    seq.frame_count(),
                    sequence_manager.is_playing
                );
            }
        }
    }

    // Only process input if we have a sequence loaded
    if sequence_manager.current_sequence.is_none() {
        info!("No sequence loaded, ignoring input");
        return;
    }

    // Space bar - toggle play/pause
    if keyboard.just_pressed(KeyCode::Space) {
        info!("Space pressed - toggling playback");
        sequence_manager.toggle_playback();
        if sequence_manager.is_playing {
            info!("Starting playback");
            events.write(SequenceEvent::PlaybackStarted);
        } else {
            info!("Stopping playback");
            events.write(SequenceEvent::PlaybackStopped);
        }
    }

    // Handle arrow key holding for continuous frame advancement
    let arrow_right_held = keyboard.pressed(KeyCode::ArrowRight);
    let arrow_left_held = keyboard.pressed(KeyCode::ArrowLeft);

    if arrow_right_held || arrow_left_held {
        // Stop playback when manually stepping
        if sequence_manager.is_playing {
            sequence_manager.is_playing = false;
            events.write(SequenceEvent::PlaybackStopped);
        }

        // If just pressed, reset timer and immediately advance
        if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::ArrowLeft) {
            key_timer.arrow_hold_time = 0.0;
            key_timer.last_frame_change = 0.0;

            // Immediate frame change
            if arrow_right_held {
                info!("Right arrow pressed - advancing frame");
                if sequence_manager.next_frame() {
                    info!("Advanced to frame {}", sequence_manager.current_frame);
                    events.write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
                } else {
                    info!("Cannot advance further - at end of sequence");
                }
            } else {
                info!("Left arrow pressed - going to previous frame");
                if sequence_manager.previous_frame() {
                    info!("Went back to frame {}", sequence_manager.current_frame);
                    events.write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
                } else {
                    info!("Cannot go back further - at beginning of sequence");
                }
            }
        } else {
            // Key is being held - accumulate time
            key_timer.arrow_hold_time += time.delta_secs();

            // Check if we should repeat
            if key_timer.arrow_hold_time >= key_timer.initial_delay {
                // We're in repeat mode
                let time_since_last = key_timer.arrow_hold_time - key_timer.last_frame_change;

                if time_since_last >= key_timer.repeat_interval {
                    key_timer.last_frame_change = key_timer.arrow_hold_time;

                    if arrow_right_held {
                        if sequence_manager.next_frame() {
                            events
                                .write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
                        }
                    } else {
                        if sequence_manager.previous_frame() {
                            events
                                .write(SequenceEvent::FrameChanged(sequence_manager.current_frame));
                        }
                    }
                }
            }
        }
    } else {
        // No arrow keys held - reset timer
        key_timer.arrow_hold_time = 0.0;
        key_timer.last_frame_change = 0.0;
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
