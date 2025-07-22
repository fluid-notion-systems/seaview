use super::parallel_loader::{AsyncStlLoader, LoadPriority};
use crate::lib::sequence::async_cache::AsyncMeshCache;
use bevy::prelude::*;
use std::path::PathBuf;

pub struct StlLoaderPlugin;

impl Plugin for StlLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_initial_stl_file)
            .add_systems(Update, handle_initial_load_complete);
    }
}

#[derive(Resource)]
pub struct StlFilePath(pub Option<PathBuf>);

fn load_initial_stl_file(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stl_path: Res<StlFilePath>,
    mut mesh_cache: ResMut<AsyncMeshCache>,
    async_loader: Res<AsyncStlLoader>,
    _source_orientation: Res<crate::lib::coordinates::SourceOrientation>,
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

        info!("Queueing initial STL file for loading: {:?}", path);

        // Queue the STL file for async loading with critical priority
        mesh_cache.get_or_queue(
            path,
            &async_loader,
            LoadPriority::Critical,
            true, // use fallback
        );

        // Note: The actual mesh spawning will happen when the load completes
        // via the process_completed_loads system
    } else {
        info!("No STL file specified, showing demo scene");

        // If no STL file is provided, create a demo cube
        let mesh_handle = meshes.add(Mesh::from(Cuboid::new(2.0, 2.0, 2.0)));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 1.0),
            metallic: 0.1,
            perceptual_roughness: 0.8,
            double_sided: false,
            cull_mode: Some(bevy::render::render_resource::Face::Back),
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

/// System to handle completion of the initial STL file load
fn handle_initial_load_complete(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<super::parallel_loader::LoadCompleteEvent>,
    mut mesh_cache: ResMut<AsyncMeshCache>,
    source_orientation: Res<crate::lib::coordinates::SourceOrientation>,
) {
    for event in events.read() {
        if event.success && mesh_cache.current_mesh_entity.is_none() {
            // This is our initial load
            if let Some(mesh_handle) = mesh_cache.cache.get(&event.path) {
                info!("Initial STL file loaded successfully: {:?}", event.path);

                // Create a material for the mesh
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.8, 0.8),
                    metallic: 0.1,
                    perceptual_roughness: 0.8,
                    reflectance: 0.5,
                    double_sided: false,
                    cull_mode: Some(bevy::render::render_resource::Face::Back),
                    ..default()
                });

                // Spawn the mesh entity
                let entity = commands
                    .spawn((
                        Mesh3d(mesh_handle.clone()),
                        MeshMaterial3d(material),
                        source_orientation.as_ref().to_transform(),
                        Name::new("Initial STL Model"),
                    ))
                    .id();

                // Track the entity in mesh cache so it can be removed when sequence plays
                mesh_cache.current_mesh_entity = Some(entity);
            }
        }
    }
}
