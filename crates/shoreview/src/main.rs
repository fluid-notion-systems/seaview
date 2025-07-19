use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

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
        .add_plugins(BrpExtrasPlugin)
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
        Camera3d::default(),
        Transform::from_xyz(-2.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        FpsCamera::default(),
    ));

    // Add a light
    commands.spawn((
        PointLight {
            intensity: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Add another light from a different angle
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-4.0, 6.0, -4.0),
    ));

    // Add a directional light for better overall illumination
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    // Add ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 80.0,
        affects_lightmapped_meshes: false,
    });

    // Add a ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));
}
