//! Mesh optimization utilities for improving mesh performance

use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::HashMap;

/// Optimize a mesh by generating indices to reduce vertex duplication
pub fn optimize_mesh(mesh: &mut Mesh) -> Result<OptimizationStats, String> {
    // Get vertex positions
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err("Mesh missing position attribute".to_string());
    };

    // Get vertex normals
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        return Err("Mesh missing normal attribute".to_string());
    };

    // Get vertex UVs
    let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
        return Err("Mesh missing UV attribute".to_string());
    };

    let vertex_count = positions.len();

    // Check if mesh already has indices
    if mesh.indices().is_some() {
        return Ok(OptimizationStats {
            original_vertices: vertex_count,
            optimized_vertices: vertex_count,
            original_triangles: vertex_count / 3,
            vertex_reduction_percentage: 0.0,
        });
    }

    // Create a map to deduplicate vertices
    let mut vertex_map: HashMap<VertexKey, u32> = HashMap::new();
    let mut unique_positions = Vec::new();
    let mut unique_normals = Vec::new();
    let mut unique_uvs = Vec::new();
    let mut indices = Vec::new();

    for i in 0..vertex_count {
        let key = VertexKey {
            position: positions[i],
            normal: normals[i],
            uv: uvs[i],
        };

        let index = match vertex_map.get(&key) {
            Some(&idx) => idx,
            None => {
                let idx = unique_positions.len() as u32;
                vertex_map.insert(key, idx);
                unique_positions.push(positions[i]);
                unique_normals.push(normals[i]);
                unique_uvs.push(uvs[i]);
                idx
            }
        };

        indices.push(index);
    }

    let unique_vertex_count = unique_positions.len();
    let stats = OptimizationStats {
        original_vertices: vertex_count,
        optimized_vertices: unique_vertex_count,
        original_triangles: vertex_count / 3,
        vertex_reduction_percentage: if vertex_count > 0 {
            (1.0 - unique_vertex_count as f32 / vertex_count as f32) * 100.0
        } else {
            0.0
        },
    };

    info!(
        "Mesh optimization: {} vertices -> {} unique vertices ({:.1}% reduction)",
        vertex_count, unique_vertex_count, stats.vertex_reduction_percentage
    );

    // Skip optimization if no significant reduction
    if stats.vertex_reduction_percentage < 5.0 {
        info!("Skipping mesh optimization - minimal vertex duplication");
        return Ok(stats);
    }

    // Update the mesh with optimized data
    mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION);
    mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL);
    mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0);

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, unique_positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, unique_normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, unique_uvs);
    mesh.insert_indices(Indices::U32(indices));

    Ok(stats)
}

/// Create an optimized mesh from raw vertex data
pub fn create_optimized_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
) -> Result<Mesh, String> {
    let vertex_count = positions.len();

    if vertex_count == 0 {
        return Err("No vertices provided".to_string());
    }

    if normals.len() != vertex_count || uvs.len() != vertex_count {
        return Err("Vertex attribute arrays have different lengths".to_string());
    }

    // Create initial mesh
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Optimize it
    match optimize_mesh(&mut mesh) {
        Ok(stats) => {
            debug!("Optimization stats: {:?}", stats);
        }
        Err(e) => {
            warn!("Failed to optimize mesh: {}", e);
        }
    }

    Ok(mesh)
}

/// Key for vertex deduplication
#[derive(Debug, Clone, Copy, PartialEq)]
struct VertexKey {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

impl Eq for VertexKey {}

impl std::hash::Hash for VertexKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Convert floats to bits for hashing
        for &v in &self.position {
            v.to_bits().hash(state);
        }
        for &v in &self.normal {
            v.to_bits().hash(state);
        }
        for &v in &self.uv {
            v.to_bits().hash(state);
        }
    }
}

/// Statistics about mesh optimization
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    pub original_vertices: usize,
    pub optimized_vertices: usize,
    pub original_triangles: usize,
    pub vertex_reduction_percentage: f32,
}

impl OptimizationStats {
    pub fn calculate(original_vertices: usize, optimized_vertices: usize) -> Self {
        let original_triangles = original_vertices / 3;
        let vertex_reduction_percentage = if original_vertices > 0 {
            (1.0 - optimized_vertices as f32 / original_vertices as f32) * 100.0
        } else {
            0.0
        };

        Self {
            original_vertices,
            optimized_vertices,
            original_triangles,
            vertex_reduction_percentage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_stats() {
        let stats = OptimizationStats::calculate(1000, 400);
        assert_eq!(stats.original_vertices, 1000);
        assert_eq!(stats.optimized_vertices, 400);
        assert_eq!(stats.original_triangles, 333);
        assert!((stats.vertex_reduction_percentage - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_vertex_key_equality() {
        let key1 = VertexKey {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 0.5],
        };

        let key2 = VertexKey {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 0.5],
        };

        assert_eq!(key1, key2);
    }
}
