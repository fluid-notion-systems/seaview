use bevy::prelude::*;

mod cli;
mod systems;

use cli::Args;
use systems::camera::{camera_controller, cursor_grab_system, FpsCamera};
use systems::stl_loader::{StlFilePath, StlLoaderPlugin};

fn main() {
    // Parse command line arguments
    let args = Args::parse_args();

    if args.verbose {
        info!("Starting Shoreview mesh viewer...");
        if let Some(ref path) = args.stl_file {
            info!("STL file to load: {:?}", path);
        }
    }

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StlLoaderPlugin)
        .insert_resource(StlFilePath(args.stl_file))
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_controller, cursor_grab_system))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the FPS camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        FpsCamera::default(),
    ));

    // Add a light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Add another light from a different angle
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(-4.0, 6.0, -4.0),
        ..default()
    });

    // Add ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 80.0,
    });

    // Add a ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(Plane3d::default().mesh().size(10.0, 10.0))),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -1.0, 0.0),
        ..default()
    });
}
