use crate::sequence::async_cache::AsyncMeshCache;
use crate::systems::parallel_loader::{AsyncStlLoader, LoadPriority};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::Path;

pub struct GltfLoaderPlugin;

impl Plugin for GltfLoaderPlugin {
    fn build(&self, _app: &mut App) {
        // For now, glTF loading is handled through the parallel loader
        // which converts glTF to mesh data
    }
}

/// Load a glTF/GLB file directly (synchronous alternative for simple cases)
pub fn load_gltf_as_mesh(path: &Path) -> Result<(Mesh, Option<StandardMaterial>), String> {
    let (document, buffers, _images) =
        gltf::import(path).map_err(|e| format!("Failed to import glTF: {}", e))?;

    // For simplicity, we'll just load the first primitive of the first mesh
    let gltf_mesh = document
        .meshes()
        .next()
        .ok_or_else(|| "No meshes found in glTF file".to_string())?;

    let primitive = gltf_mesh
        .primitives()
        .next()
        .ok_or_else(|| "No primitives found in mesh".to_string())?;

    // Create Bevy mesh
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    // Extract vertex positions
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

    if let Some(positions_reader) = reader.read_positions() {
        let positions: Vec<[f32; 3]> = positions_reader.collect();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(positions),
        );
    } else {
        return Err("No position data found in glTF primitive".to_string());
    }

    // Extract normals
    if let Some(normals_reader) = reader.read_normals() {
        let normals: Vec<[f32; 3]> = normals_reader.collect();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(normals),
        );
    }

    // Extract UVs
    if let Some(tex_coords_reader) = reader.read_tex_coords(0) {
        let uvs: Vec<[f32; 2]> = tex_coords_reader.into_f32().collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs));
    }

    // Extract indices
    if let Some(indices_reader) = reader.read_indices() {
        let indices: Vec<u32> = indices_reader.into_u32().collect();
        mesh.insert_indices(Indices::U32(indices));
    }

    // Extract material
    let gltf_material = primitive.material();
    let material = if let Some(_gltf_material) = gltf_material.index() {
        let material_ref = primitive.material();
        let pbr = material_ref.pbr_metallic_roughness();
        let base_color = pbr.base_color_factor();

        Some(StandardMaterial {
            base_color: Color::srgba(base_color[0], base_color[1], base_color[2], base_color[3]),
            metallic: pbr.metallic_factor(),
            perceptual_roughness: pbr.roughness_factor(),
            double_sided: material_ref.double_sided(),
            cull_mode: if material_ref.double_sided() {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            ..default()
        })
    } else {
        None
    };

    Ok((mesh, material))
}

/// Check if a file is a glTF/GLB file based on extension
pub fn is_gltf_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        ext == "gltf" || ext == "glb"
    } else {
        false
    }
}

/// Integration with the async loader system
pub fn queue_gltf_load(
    path: &Path,
    mesh_cache: &mut AsyncMeshCache,
    async_loader: &AsyncStlLoader,
    priority: LoadPriority,
) {
    if is_gltf_file(path) {
        info!("Queueing glTF/GLB file for loading: {:?}", path);
        mesh_cache.get_or_queue(&path.to_path_buf(), async_loader, priority, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_gltf_file() {
        assert!(is_gltf_file(Path::new("model.gltf")));
        assert!(is_gltf_file(Path::new("model.GLB")));
        assert!(is_gltf_file(Path::new("path/to/model.glb")));
        assert!(!is_gltf_file(Path::new("model.stl")));
        assert!(!is_gltf_file(Path::new("model")));
    }
}
