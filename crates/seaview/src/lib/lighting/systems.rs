//! Systems for managing night lighting
//!
//! This module contains Bevy systems that handle spawning, updating, and
//! despawning night lights based on the NightLightingConfig resource.

use bevy::prelude::*;

use super::{NightLight, NightLightMarker, NightLightingConfig};

/// System that updates night lights based on configuration changes
///
/// This system:
/// - Spawns new lights when config changes
/// - Updates existing light positions, angles, and properties
/// - Despawns lights when they're no longer needed
pub fn update_night_lights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<NightLightingConfig>,
    existing_lights: Query<(Entity, &NightLight)>,
    existing_markers: Query<(Entity, &NightLightMarker)>,
) {
    // If lighting is disabled, despawn all lights and markers
    if !config.enabled {
        for (entity, _) in existing_lights.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_markers.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Only update if config has changed
    if !config.is_changed() {
        return;
    }

    // Get current number of lights
    let current_count = existing_lights.iter().count();
    let target_count = config.num_lights;

    // If count changed, despawn all and respawn
    // (In the future, we could be smarter about only adding/removing what's needed)
    if current_count != target_count {
        for (entity, _) in existing_lights.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_markers.iter() {
            commands.entity(entity).despawn();
        }

        spawn_lights(&mut commands, &mut meshes, &mut materials, &config);
    } else {
        // Update existing lights' properties
        for (entity, light) in existing_lights.iter() {
            update_light_transform(&mut commands, entity, light.index, &config);
        }

        // Update marker visibility and size
        update_markers(&mut commands, &mut meshes, &existing_markers, &config);
    }
}

/// Spawn all lights according to current configuration
fn spawn_lights(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    config: &NightLightingConfig,
) {
    // Calculate scene bounds (TODO: get from actual scene data)
    let bounds_min = Vec2::new(-100.0, -100.0);
    let bounds_max = Vec2::new(100.0, 100.0);

    // Calculate light positions using the configured algorithm
    let positions = config
        .placement_algorithm
        .calculate_positions(config.num_lights, bounds_min, bounds_max);

    // Spawn each light
    for (index, pos) in positions.iter().enumerate() {
        let transform = calculate_light_transform(*pos, config.height, config.cone_angle);

        commands.spawn((
            SpotLight {
                intensity: config.intensity,
                color: config.color,
                range: config.range,
                outer_angle: config.cone_angle.to_radians(),
                inner_angle: (config.cone_angle * 0.8).to_radians(),
                shadows_enabled: true,
                ..default()
            },
            transform,
            NightLight { index },
        ));

        // Spawn marker sphere if enabled
        if config.show_markers {
            spawn_marker(commands, meshes, materials, *pos, config.height, index, config);
        }
    }
}

/// Update transform for an existing light
fn update_light_transform(
    commands: &mut Commands,
    entity: Entity,
    index: usize,
    config: &NightLightingConfig,
) {
    // Calculate scene bounds (TODO: get from actual scene data)
    let bounds_min = Vec2::new(-100.0, -100.0);
    let bounds_max = Vec2::new(100.0, 100.0);

    // Calculate light positions
    let positions = config
        .placement_algorithm
        .calculate_positions(config.num_lights, bounds_min, bounds_max);

    if let Some(pos) = positions.get(index) {
        let transform = calculate_light_transform(*pos, config.height, config.cone_angle);

        commands.entity(entity).insert((
            transform,
            SpotLight {
                intensity: config.intensity,
                color: config.color,
                range: config.range,
                outer_angle: config.cone_angle.to_radians(),
                inner_angle: (config.cone_angle * 0.8).to_radians(),
                shadows_enabled: true,
                ..default()
            },
        ));
    }
}

/// Calculate transform for a light at the given XZ position
fn calculate_light_transform(pos: Vec2, height: f32, _cone_angle: f32) -> Transform {
    let translation = Vec3::new(pos.x, height, pos.y);

    // Point straight down (negative Y axis)
    let rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    Transform {
        translation,
        rotation,
        scale: Vec3::ONE,
    }
}

/// Spawn a marker sphere at the light position
fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec2,
    height: f32,
    index: usize,
    config: &NightLightingConfig,
) {
    let sphere_mesh = meshes.add(Sphere::new(config.marker_size));
    let material = materials.add(StandardMaterial {
        base_color: config.color,
        emissive: (config.color.to_linear() * 2.0).into(),
        ..default()
    });

    commands.spawn((
        Mesh3d(sphere_mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(pos.x, height, pos.y),
        NightLightMarker { light_index: index },
    ));
}

/// Update marker visibility and size
fn update_markers(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    existing_markers: &Query<(Entity, &NightLightMarker)>,
    config: &NightLightingConfig,
) {
    if !config.show_markers {
        // Hide markers by despawning them
        for (entity, _) in existing_markers.iter() {
            commands.entity(entity).despawn();
        }
    } else {
        // Update marker meshes if size changed
        let new_sphere = meshes.add(Sphere::new(config.marker_size));
        for (entity, _) in existing_markers.iter() {
            commands.entity(entity).insert(Mesh3d(new_sphere.clone()));
        }
    }
}
