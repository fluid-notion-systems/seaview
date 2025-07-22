//! Shared types for mesh data transfer

use serde::{Deserialize, Serialize};

/// Frame of mesh data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshFrame {
    /// Human-readable simulation identifier (e.g., "happy-dolphin-42")
    pub simulation_id: String,
    /// Frame number in the simulation sequence
    pub frame_number: u32,
    /// Timestamp in nanoseconds since simulation start
    pub timestamp: u64,
    /// Spatial bounds of the mesh
    pub domain_bounds: DomainBounds,
    /// Flattened vertex positions (x,y,z triplets)
    pub vertices: Vec<f32>,
    /// Optional vertex normals (x,y,z triplets)
    pub normals: Option<Vec<f32>>,
    /// Optional indices for indexed mesh representation
    pub indices: Option<Vec<u32>>,
}

impl MeshFrame {
    /// Create a new mesh frame
    pub fn new(simulation_id: String, frame_number: u32) -> Self {
        Self {
            simulation_id,
            frame_number,
            timestamp: 0,
            domain_bounds: DomainBounds::default(),
            vertices: Vec::new(),
            normals: None,
            indices: None,
        }
    }

    /// Get the number of vertices in the mesh
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 3
    }

    /// Get the number of triangles in the mesh
    pub fn triangle_count(&self) -> usize {
        if let Some(indices) = &self.indices {
            indices.len() / 3
        } else {
            self.vertex_count() / 3
        }
    }

    /// Check if this is an indexed mesh
    pub fn is_indexed(&self) -> bool {
        self.indices.is_some()
    }

    /// Check if normals are present
    pub fn has_normals(&self) -> bool {
        self.normals.is_some()
    }

    /// Validate the mesh data consistency
    pub fn validate(&self) -> Result<(), String> {
        // Check vertices are triplets
        if self.vertices.len() % 3 != 0 {
            return Err(format!(
                "Vertex count {} is not divisible by 3",
                self.vertices.len()
            ));
        }

        // Check normals if present
        if let Some(normals) = &self.normals {
            if normals.len() != self.vertices.len() {
                return Err(format!(
                    "Normal count {} doesn't match vertex count {}",
                    normals.len(),
                    self.vertices.len()
                ));
            }
        }

        // Check indices if present
        if let Some(indices) = &self.indices {
            if indices.len() % 3 != 0 {
                return Err(format!(
                    "Index count {} is not divisible by 3",
                    indices.len()
                ));
            }

            let max_index = *indices.iter().max().unwrap_or(&0) as usize;
            if max_index >= self.vertex_count() {
                return Err(format!(
                    "Index {} exceeds vertex count {}",
                    max_index,
                    self.vertex_count()
                ));
            }
        }

        Ok(())
    }
}

/// Spatial bounds of the mesh domain
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DomainBounds {
    /// Minimum coordinates (x, y, z)
    pub min: [f32; 3],
    /// Maximum coordinates (x, y, z)
    pub max: [f32; 3],
}

impl Default for DomainBounds {
    fn default() -> Self {
        Self {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }
    }
}

impl DomainBounds {
    /// Create new domain bounds
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Calculate the center of the domain
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }

    /// Calculate the size of the domain
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Calculate the diagonal length of the domain
    pub fn diagonal_length(&self) -> f32 {
        let size = self.size();
        (size[0] * size[0] + size[1] * size[1] + size[2] * size[2]).sqrt()
    }

    /// Check if the bounds are valid (min <= max)
    pub fn is_valid(&self) -> bool {
        self.min[0] <= self.max[0] && self.min[1] <= self.max[1] && self.min[2] <= self.max[2]
    }
}

/// Metadata about a mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshMetadata {
    /// Number of vertices in the mesh
    pub vertex_count: usize,
    /// Number of triangular faces
    pub face_count: usize,
    /// Whether the mesh includes normal vectors
    pub has_normals: bool,
    /// Whether the mesh uses indexed representation
    pub is_indexed: bool,
}

impl From<&MeshFrame> for MeshMetadata {
    fn from(frame: &MeshFrame) -> Self {
        Self {
            vertex_count: frame.vertex_count(),
            face_count: frame.triangle_count(),
            has_normals: frame.has_normals(),
            is_indexed: frame.is_indexed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_frame_validation() {
        let mut frame = MeshFrame::new("test".to_string(), 0);

        // Valid triangle mesh
        frame.vertices = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        assert!(frame.validate().is_ok());

        // Invalid vertex count
        frame.vertices.push(1.0);
        assert!(frame.validate().is_err());
        frame.vertices.pop();

        // Invalid normals
        frame.normals = Some(vec![0.0, 0.0, 1.0]); // Only one normal
        assert!(frame.validate().is_err());

        // Valid normals
        frame.normals = Some(vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        assert!(frame.validate().is_ok());

        // Invalid indices
        frame.indices = Some(vec![0, 1, 3]); // Index 3 out of bounds
        assert!(frame.validate().is_err());

        // Valid indices
        frame.indices = Some(vec![0, 1, 2]);
        assert!(frame.validate().is_ok());
    }

    #[test]
    fn test_domain_bounds() {
        let bounds = DomainBounds::new([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);

        assert_eq!(bounds.center(), [1.0, 1.0, 1.0]);
        assert_eq!(bounds.size(), [2.0, 2.0, 2.0]);
        assert!((bounds.diagonal_length() - 3.464).abs() < 0.001);
        assert!(bounds.is_valid());

        let invalid_bounds = DomainBounds::new([1.0, 0.0, 0.0], [0.0, 1.0, 1.0]);
        assert!(!invalid_bounds.is_valid());
    }
}
