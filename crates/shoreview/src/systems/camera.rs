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
    let (mut transform, fps_camera) = query.single_mut();

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
    let speed = fps_camera.speed;

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

    // Handle vertical movement separately to preserve modifier effect
    let mut vertical_movement = 0.0;
    let vertical_modifier = if input.pressed(KeyCode::AltLeft) {
        0.1
    } else {
        1.0
    };

    if input.pressed(KeyCode::KeyE) {
        vertical_movement += vertical_modifier;
    }
    if input.pressed(KeyCode::KeyQ) {
        vertical_movement -= vertical_modifier;
    }

    // Apply horizontal movement
    if movement != Vec3::ZERO {
        movement = movement.normalize();
        transform.translation += movement * speed * time.delta_seconds();
    }

    // Apply vertical movement separately
    if vertical_movement != 0.0 {
        transform.translation.y += vertical_movement * speed * time.delta_seconds();
    }
}

pub fn cursor_grab_system(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut FpsCamera, With<Camera>>,
) {
    let mut window = windows.single_mut();
    let mut fps_camera = camera_query.single_mut();

    if input.just_pressed(MouseButton::Left) {
        // Grab cursor when left mouse button is pressed
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        fps_camera.escape_mode = false;
    }

    if key_input.just_pressed(KeyCode::Escape) {
        // Release cursor when Escape is pressed
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        fps_camera.escape_mode = true;
    }
}
