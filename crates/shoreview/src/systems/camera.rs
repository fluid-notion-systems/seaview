//! FPS-style camera controller with mouse look and keyboard movement
//!
//! Controls:
//! - WASD: Move forward/back/left/right
//! - Q/E: Move down/up
//! - Mouse: Look around (when cursor is grabbed)
//! - Left Click: Grab cursor for mouse look
//! - Escape: Release cursor
//! - Shift: 10x movement speed boost
//! - Alt: 0.1x movement speed (precision mode)

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

#[derive(Component)]
pub struct FpsCamera {
    pub sensitivity: f32,
    pub speed: f32,
    pub escape_mode: bool,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            sensitivity: 0.3,
            speed: 10.0,
            escape_mode: false,
        }
    }
}

pub fn camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &FpsCamera), With<Camera>>,
) {
    let Ok((mut transform, fps_camera)) = query.single_mut() else {
        return;
    };

    // Don't process camera controls if in escape mode
    if fps_camera.escape_mode {
        // Clear mouse events to prevent them from accumulating
        mouse_events.clear();
        return;
    }

    // Handle mouse look
    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.read() {
        mouse_delta += mouse_event.delta;
    }

    if mouse_delta != Vec2::ZERO {
        // Apply mouse sensitivity
        mouse_delta *= fps_camera.sensitivity * 0.01;

        // Calculate new rotation
        let yaw = -mouse_delta.x;
        let pitch = -mouse_delta.y;

        // Apply rotations
        transform.rotate_y(yaw);
        transform.rotate_local_x(pitch);

        // Prevent camera from flipping upside down
        let forward = *transform.forward();
        let right = *transform.right();
        let up = right.cross(forward);

        // Ensure we don't over-rotate on the pitch axis
        if up.y < 0.1 && pitch < 0.0 {
            // Don't rotate further down
        } else if up.y > 0.9 && pitch > 0.0 {
            // Don't rotate further up
        } else {
            // Rotation is fine
        }
    }

    // Handle movement
    let mut movement = Vec3::ZERO;
    let base_speed = fps_camera.speed;

    // Apply speed modifiers
    // Shift: 10x speed for fast movement
    // Alt: 0.1x speed for precision movement
    let speed_modifier = if input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
    {
        10.0 // Shift for 10x speed
    } else if input.pressed(KeyCode::AltLeft) || input.pressed(KeyCode::AltRight) {
        0.1 // Alt for slow movement (all directions)
    } else {
        1.0
    };

    let speed = base_speed * speed_modifier;

    if input.pressed(KeyCode::KeyW) {
        movement += *transform.forward();
    }
    if input.pressed(KeyCode::KeyS) {
        movement -= *transform.forward();
    }
    if input.pressed(KeyCode::KeyA) {
        movement -= *transform.right();
    }
    if input.pressed(KeyCode::KeyD) {
        movement += *transform.right();
    }

    // Handle vertical movement
    if input.pressed(KeyCode::KeyE) {
        movement += Vec3::Y;
    }
    if input.pressed(KeyCode::KeyQ) {
        movement -= Vec3::Y;
    }

    // Apply movement
    if movement != Vec3::ZERO {
        movement = movement.normalize();
        transform.translation += movement * speed * time.delta_secs();
    }
}

pub fn cursor_grab_system(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut FpsCamera, With<Camera>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let Ok(mut fps_camera) = camera_query.single_mut() else {
        return;
    };

    if input.just_pressed(MouseButton::Left) {
        // Grab cursor when left mouse button is pressed
        window.cursor_options.visible = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        fps_camera.escape_mode = false;
    }

    if key_input.just_pressed(KeyCode::Escape) {
        // Release cursor when Escape is pressed
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
        fps_camera.escape_mode = true;
    }
}
