//! Coordinate system handling for different source orientations

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub enum SourceOrientation {
    /// Y+ is up, Z+ is forward (Bevy default)
    YUp,
    /// Z+ is up, Y+ is forward (common in CAD/GIS)
    ZUp,
    /// X+ is right, Y+ is forward, Z+ is up (common in some fluid simulations)
    FluidX3D,
    /// Custom transformation matrix
    #[allow(dead_code)]
    Custom(Mat3),
}

impl Default for SourceOrientation {
    fn default() -> Self {
        Self::YUp
    }
}

impl SourceOrientation {
    /// Convert the source orientation to a transform that can be applied to meshes
    pub fn to_transform(self) -> Transform {
        match self {
            SourceOrientation::YUp => Transform::IDENTITY,
            SourceOrientation::ZUp => {
                // Rotate -90 degrees around X axis to convert Z-up to Y-up
                Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            }
            SourceOrientation::FluidX3D => {
                // FluidX3D typically uses Z-up with X-right and Y-forward
                // This is the same as ZUp
                Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            }
            SourceOrientation::Custom(mat) => Transform::from_matrix(Mat4::from_mat3(mat)),
        }
    }

    /// Parse from string argument
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "yup" | "y-up" | "y_up" => Ok(Self::YUp),
            "zup" | "z-up" | "z_up" => Ok(Self::ZUp),
            "fluidx3d" | "fluid-x3d" | "fluid_x3d" => Ok(Self::FluidX3D),
            _ => Err(format!(
                "Unknown coordinate system: {s}. Valid options are: yup, zup, fluidx3d"
            )),
        }
    }

    /// Get a description of the coordinate system
    pub fn description(&self) -> &'static str {
        match self {
            SourceOrientation::YUp => "Y+ up, Z+ forward (Bevy default)",
            SourceOrientation::ZUp => "Z+ up, Y+ forward (CAD/GIS standard)",
            SourceOrientation::FluidX3D => "Z+ up, X+ right, Y+ forward (FluidX3D)",
            SourceOrientation::Custom(_) => "Custom transformation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_orientation() {
        assert_eq!(
            SourceOrientation::from_str("yup").unwrap(),
            SourceOrientation::YUp
        );
        assert_eq!(
            SourceOrientation::from_str("ZUP").unwrap(),
            SourceOrientation::ZUp
        );
        assert_eq!(
            SourceOrientation::from_str("z-up").unwrap(),
            SourceOrientation::ZUp
        );
        assert_eq!(
            SourceOrientation::from_str("fluidx3d").unwrap(),
            SourceOrientation::FluidX3D
        );
        assert!(SourceOrientation::from_str("invalid").is_err());
    }

    #[test]
    fn test_transforms() {
        // Y-up should be identity
        let yup_transform = SourceOrientation::YUp.to_transform();
        assert_eq!(yup_transform, Transform::IDENTITY);

        // Z-up should rotate -90 degrees around X
        let zup_transform = SourceOrientation::ZUp.to_transform();
        let expected_quat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        assert!((zup_transform.rotation.x - expected_quat.x).abs() < 0.0001);
        assert!((zup_transform.rotation.y - expected_quat.y).abs() < 0.0001);
        assert!((zup_transform.rotation.z - expected_quat.z).abs() < 0.0001);
        assert!((zup_transform.rotation.w - expected_quat.w).abs() < 0.0001);
    }
}
