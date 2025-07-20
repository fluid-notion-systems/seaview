use crate::sequence::loader::{load_stl_file_optimized, MeshCache};
use bevy::prelude::*;
use std::path::PathBuf;

pub struct StlLoaderPlugin;

impl Plugin for StlLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_initial_stl_file);
    }
}

#[derive(Resource)]
pub struct StlFilePath(pub Option<PathBuf>);

fn load_initial_stl_file(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stl_path: Res<StlFilePath>,
    mut mesh_cache: ResMut<MeshCache>,
) {
    if let Some(path) = &stl_path.0 {
        // Only load if it's a file, not a directory
        if path.is_dir() {
            info!("Path is a directory, skipping initial STL load: {:?}", path);
            return;
        }

        if !path.exists() {
            error!("Path does not exist: {:?}", path);
            return;
        }

        info!("Loading initial STL file: {:?}", path);

        // Load the STL file using optimized loader
        match load_stl_file_optimized(path) {
            Ok(mesh) => {
                let mesh_handle = meshes.add(mesh);

                // Create a material for the mesh
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.8, 0.8),
                    metallic: 0.1,
                    perceptual_roughness: 0.8,
                    reflectance: 0.5,
                    ..default()
                });

                // Spawn the mesh entity
                let entity = commands
                    .spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(material),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                        Name::new("Initial STL Model"),
                    ))
                    .id();

                // Track the entity in mesh cache so it can be removed when sequence plays
                mesh_cache.current_mesh_entity = Some(entity);
            }
            Err(e) => {
                error!("Failed to load STL file: {}", e);
            }
        }
    } else {
        info!("No STL file specified, showing demo scene");

        // If no STL file is provided, create a demo cube
        let mesh_handle = meshes.add(Mesh::from(Cuboid::new(2.0, 2.0, 2.0)));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 1.0),
            metallic: 0.1,
            perceptual_roughness: 0.8,
            ..default()
        });

        let entity = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Name::new("Demo Cube"),
            ))
            .id();

        // Track the entity in mesh cache
        mesh_cache.current_mesh_entity = Some(entity);
    }
}
