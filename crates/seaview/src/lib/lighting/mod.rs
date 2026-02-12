//! Lighting system for night simulation
//!
//! This module provides configurable lighting for visualizing fluid simulations
//! under night-time conditions. It supports multiple light placement algorithms,
//! adjustable height, and cone angles.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod placement;
pub mod systems;

pub use placement::PlacementAlgorithm;

/// Resource that configures the night lighting system
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct NightLightingConfig {
    /// Number of lights to spawn
    pub num_lights: usize,

    /// Height of lights above sea level (in meters)
    pub height: f32,

    /// Cone angle for spotlights (in degrees)
    pub cone_angle: f32,

    /// Placement algorithm for distributing lights
    pub placement_algorithm: PlacementAlgorithm,

    /// Whether the lighting system is enabled
    pub enabled: bool,

    /// Light intensity
    pub intensity: f32,

    /// Light color
    pub color: Color,

    /// Light range (distance)
    pub range: f32,
}

impl Default for NightLightingConfig {
    fn default() -> Self {
        Self {
            num_lights: 9,
            height: 50.0,
            cone_angle: 60.0,
            placement_algorithm: PlacementAlgorithm::UniformGrid,
            enabled: true,
            intensity: 1000.0,
            color: Color::srgb(1.0, 0.95, 0.9), // Warm white
            range: 200.0,
        }
    }
}

/// Marker component for night lighting spotlights
#[derive(Component)]
pub struct NightLight {
    /// Index of this light in the grid
    pub index: usize,
}

/// Plugin that manages the night lighting system
pub struct NightLightingPlugin;

impl Plugin for NightLightingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NightLightingConfig::default())
            .add_systems(Update, systems::update_night_lights);
    }
}
