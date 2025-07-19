use bevy::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use stl_io::read_stl;

pub struct StlLoaderPlugin;

impl Plugin for StlLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_stl_file);
    }
}

#[derive(Resource)]
pub struct StlFilePath(pub Option<PathBuf>);

fn load_stl_file(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stl_path: Res<StlFilePath>,
) {
    if let Some(path) = &stl_path.0 {
        // Only load if it's a file, not a directory
        if !path.is_file() {
            return;
        }

        info!("Loading STL file: {:?}", path);

        // Open and read the STL file
        match File::open(path) {
            Ok(mut file) => {
                match read_stl(&mut file) {
                    Ok(stl) => {
                        info!(
                            "STL file loaded successfully with {} triangles",
                            stl.faces.len()
                        );

                        // Convert STL triangles to Bevy mesh
                        let mesh = stl_to_mesh(stl);

                        // Create a material for the mesh
                        let material = materials.add(StandardMaterial {
                            base_color: Color::srgb(0.8, 0.8, 0.8),
                            metallic: 0.1,
                            perceptual_roughness: 0.8,
                            reflectance: 0.5,
                            ..default()
                        });

                        // Spawn the mesh entity
                        commands.spawn((
                            Mesh3d(meshes.add(mesh)),
                            MeshMaterial3d(material),
                            Transform::from_xyz(0.0, 0.0, 0.0),
                            Name::new("STL Model"),
                        ));
                    }
                    Err(e) => {
                        error!("Failed to parse STL file: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to open STL file: {}", e);
            }
        }
    } else {
        info!("No STL file specified, showing demo scene");

        // If no STL file is provided, create a demo cube
        let mesh = meshes.add(Mesh::from(Cuboid::new(2.0, 2.0, 2.0)));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 1.0),
            metallic: 0.1,
            perceptual_roughness: 0.8,
            ..default()
        });

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Name::new("Demo Cube"),
        ));
    }
}

fn stl_to_mesh(stl: stl_io::IndexedMesh) -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    // Extract vertices from the STL
    let vertices: Vec<[f32; 3]> = stl.vertices.iter().map(|v| [v[0], v[1], v[2]]).collect();

    // Extract normals - STL provides face normals, we need to convert to vertex normals
    let mut vertex_normals: Vec<Vec3> = vec![Vec3::ZERO; vertices.len()];
    let mut vertex_face_count: Vec<u32> = vec![0; vertices.len()];

    // Accumulate normals for each vertex
    for face in &stl.faces {
        let normal = Vec3::new(face.normal[0], face.normal[1], face.normal[2]);
        for &vertex_idx in &face.vertices {
            vertex_normals[vertex_idx] += normal;
            vertex_face_count[vertex_idx] += 1;
        }
    }

    // Average the normals
    let normals: Vec<[f32; 3]> = vertex_normals
        .iter()
        .zip(vertex_face_count.iter())
        .map(|(normal, &count)| {
            if count > 0 {
                let averaged = normal.normalize();
                [averaged.x, averaged.y, averaged.z]
            } else {
                [0.0, 1.0, 0.0] // Default up normal
            }
        })
        .collect();

    // Generate simple UV coordinates based on vertex positions
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .map(|v| {
            // Simple planar mapping
            [v[0] * 0.5 + 0.5, v[2] * 0.5 + 0.5]
        })
        .collect();

    // Create indices for the mesh
    let indices: Vec<u32> = stl
        .faces
        .iter()
        .flat_map(|face| face.vertices.iter().map(|&v| v as u32))
        .collect();

    // Set mesh attributes
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    // Generate tangents for proper lighting
    mesh.generate_tangents().ok();

    mesh
}
