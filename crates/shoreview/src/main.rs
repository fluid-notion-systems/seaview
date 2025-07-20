use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

mod cli;
mod sequence;
mod systems;
mod ui;

use cli::Args;
use sequence::{discovery::DiscoverSequenceRequest, SequencePlugin};
use systems::camera::{camera_controller, cursor_grab_system, FpsCamera};
use systems::stl_loader::{StlFilePath, StlLoaderPlugin};
use ui::UIPlugin;

fn main() {
    // Parse command line arguments
    let args = Args::parse_args();

    if args.verbose {
        info!("Starting Shoreview mesh viewer...");
        if let Some(ref path) = args.path {
            info!("Path provided: {:?}", path);
        }
    }

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(StlLoaderPlugin)
        .add_plugins(SequencePlugin)
        .add_plugins(UIPlugin)
        .add_plugins(BrpExtrasPlugin)
        .insert_resource(StlFilePath(args.path.clone()))
        .add_systems(Startup, (setup, handle_input_path))
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
        Transform::from_xyz(100.0, 100.0, 100.0).looking_at(Vec3::new(37.0, 37.0, 27.5), Vec3::Y),
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
}

/// System that handles the input path and decides whether to load a single file or discover a sequence
fn handle_input_path(mut commands: Commands, stl_path: Res<StlFilePath>) {
    if let Some(path) = &stl_path.0 {
        if path.is_dir() {
            // It's a directory - trigger sequence discovery
            info!("Discovering sequences in directory: {:?}", path);
            commands.spawn(DiscoverSequenceRequest {
                directory: path.clone(),
                recursive: true,
            });
        } else if path.is_file() {
            // It's a single file - STL loader will handle it
            info!("Loading single STL file: {:?}", path);
        } else {
            error!("Path does not exist: {:?}", path);
        }
    }
}
