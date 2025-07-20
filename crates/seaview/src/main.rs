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
        info!("Starting Seaview mesh viewer...");
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
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the FPS camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(100.0, 100.0, 100.0).looking_at(Vec3::new(37.0, 37.0, 27.5), Vec3::Y),
        FpsCamera::default(),
    ));

    // Add a directional light for overall illumination
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(Vec3::new(-0.3, -1.0, -0.5), Vec3::Y),
    ));

    // Add a point light from above
    commands.spawn((
        PointLight {
            intensity: 50000.0,
            shadows_enabled: false,
            range: 1000.0,
            ..default()
        },
        Transform::from_xyz(50.0, 150.0, 50.0),
    ));

    // Add another point light from a different angle for better surface visibility
    commands.spawn((
        PointLight {
            intensity: 30000.0,
            shadows_enabled: false,
            range: 1000.0,
            ..default()
        },
        Transform::from_xyz(-50.0, 100.0, -50.0),
    ));

    // Add ambient light for overall brightness
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
        ..default()
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
