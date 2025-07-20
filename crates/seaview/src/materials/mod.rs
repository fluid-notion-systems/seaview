use bevy::prelude::*;

pub struct WaterMaterials;

impl WaterMaterials {
    /// Crystal clear water with sharp reflections - like a calm lake
    pub fn crystal_clear(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.7),
            metallic: 0.1,
            perceptual_roughness: 0.05,
            reflectance: 0.9,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Mirror-like water surface - extremely reflective
    pub fn mirror_surface(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.8),
            metallic: 0.3,
            perceptual_roughness: 0.0,
            reflectance: 1.0,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Calm ocean water with slightly softened reflections
    pub fn calm_ocean(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.75),
            metallic: 0.2,
            perceptual_roughness: 0.2,
            reflectance: 0.8,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Standard water with moderate reflections
    pub fn standard_water(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.65),
            metallic: 0.3,
            perceptual_roughness: 0.35,
            reflectance: 0.7,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Slightly rough water surface - like a breezy day
    pub fn breezy_water(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.6),
            metallic: 0.25,
            perceptual_roughness: 0.5,
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Choppy water with diffused reflections
    pub fn choppy_water(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.55),
            metallic: 0.2,
            perceptual_roughness: 0.65,
            reflectance: 0.5,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Rough sea water with very soft reflections
    pub fn rough_sea(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.5),
            metallic: 0.15,
            perceptual_roughness: 0.8,
            reflectance: 0.4,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Murky water with minimal reflections
    pub fn murky_water(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.4),
            metallic: 0.05,
            perceptual_roughness: 0.9,
            reflectance: 0.3,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Deep ocean water - darker with subtle reflections
    pub fn deep_ocean(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.98),
            metallic: 0.4,
            perceptual_roughness: 0.4,
            reflectance: 0.85,
            alpha_mode: AlphaMode::Multiply,
            ..default()
        }
    }

    /// Tropical water - bright with moderate reflections
    pub fn tropical_water(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color.with_alpha(0.7),
            metallic: 0.1,
            perceptual_roughness: 0.3,
            reflectance: 0.75,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }
    }

    /// Default oceanic blue color for water
    pub fn default_ocean_blue() -> Color {
        Color::srgb(0.0, 0.3, 0.5)
    }

    /// Deep ocean blue color
    pub fn deep_blue() -> Color {
        Color::srgb(0.0, 0.15, 0.35)
    }

    /// Tropical turquoise color
    pub fn tropical_turquoise() -> Color {
        Color::srgb(0.0, 0.5, 0.5)
    }

    /// Caribbean blue color
    pub fn caribbean_blue() -> Color {
        Color::srgb(0.0, 0.6, 0.8)
    }

    /// Murky green-blue color
    pub fn murky_green() -> Color {
        Color::srgb(0.1, 0.3, 0.3)
    }
}

// Helper function to create water material with double-sided rendering
impl WaterMaterials {
    /// Adjust the transparency of any water material
    pub fn with_transparency(mut material: StandardMaterial, alpha: f32) -> StandardMaterial {
        material.base_color = material.base_color.with_alpha(alpha.clamp(0.0, 1.0));
        material.alpha_mode = AlphaMode::Blend;
        material
    }

    pub fn with_double_sided(mut material: StandardMaterial) -> StandardMaterial {
        material.double_sided = true;
        material.cull_mode = None;
        material
    }

    pub fn with_single_sided(mut material: StandardMaterial) -> StandardMaterial {
        material.double_sided = false;
        material.cull_mode = Some(bevy::render::render_resource::Face::Back);
        material
    }
}
