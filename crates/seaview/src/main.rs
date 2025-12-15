use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_brp_extras::BrpExtrasPlugin;
use seaview::{SeaviewUiPlugin, SessionPlugin};

use seaview::app::cli::Args;
use seaview::app::systems::camera::{
    camera_controller, cursor_grab_system, debug_mesh_cache_status, handle_center_on_mesh,
    CenterOnMeshEvent, FpsCamera,
};
use seaview::app::systems::diagnostics::RenderingDiagnosticsPlugin;
use seaview::app::systems::network::NetworkMeshPlugin;
use seaview::app::systems::stl_loader::{StlFilePath, StlLoaderPlugin};
use seaview::lib::coordinates::SourceOrientation;
use seaview::lib::sequence::{discovery::DiscoverSequenceRequest, SequencePlugin};

fn main() {
    // Parse command line arguments
    let args = Args::parse_args();

    if args.verbose {
        info!("Starting Seaview mesh viewer...");
        if let Some(ref path) = args.path {
            info!("Path provided: {:?}", path);
        }
    }

    // Parse source coordinate system
    let source_orientation = match SourceOrientation::from_str(&args.source_coordinates) {
        Ok(orientation) => {
            if args.verbose {
                info!("Using coordinate system: {}", orientation.description());
            }
            orientation
        }
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    // Configure network receiving
    let network_config = if args.network_port.is_some() {
        seaview::lib::network::NetworkConfig {
            enabled: true,
            port: args.network_port.unwrap_or(9877),
            max_message_size_mb: 100,
        }
    } else {
        seaview::lib::network::NetworkConfig::default()
    };

    if network_config.enabled {
        info!(
            "Network mesh receiving enabled on port {}",
            network_config.port
        );
    }

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RenderingDiagnosticsPlugin)
        .add_plugins(seaview::lib::parallel_loader::AsyncStlLoaderPlugin)
        .add_plugins(StlLoaderPlugin)
        .add_plugins(seaview::lib::gltf_loader::GltfLoaderPlugin)
        .add_plugins(SequencePlugin)
        .add_plugins(NetworkMeshPlugin)
        .add_plugins(BrpExtrasPlugin)
        .add_plugins(SessionPlugin)
        .add_plugins(SeaviewUiPlugin)
        .insert_resource(StlFilePath(args.path.clone()))
        .insert_resource(source_orientation)
        .insert_resource(network_config)
        .add_event::<CenterOnMeshEvent>()
        .add_systems(Startup, (setup, handle_input_path, setup_cursor))
        .add_systems(
            Update,
            (
                camera_controller,
                cursor_grab_system,
                handle_center_on_mesh,
                // debug_mesh_cache_status,
            ),
        )
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

/// System to ensure cursor is visible on startup
fn setup_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}

/// System that handles the input path and decides whether to load a single file or discover a sequence
fn handle_input_path(
    mut commands: Commands,
    stl_path: Res<StlFilePath>,
    source_orientation: Res<SourceOrientation>,
) {
    if let Some(path) = &stl_path.0 {
        if path.is_dir() {
            // It's a directory - trigger sequence discovery
            info!("Discovering sequences in directory: {:?}", path);
            commands.spawn(DiscoverSequenceRequest {
                directory: path.clone(),
                recursive: true,
                source_orientation: *source_orientation,
            });
        } else if path.is_file() {
            // It's a single file - loader will handle it based on extension
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            match ext.as_str() {
                "stl" => info!("Loading single STL file: {:?}", path),
                "gltf" | "glb" => info!("Loading single glTF/GLB file: {:?}", path),
                _ => info!("Loading file (unknown format, will try STL): {:?}", path),
            }
        } else {
            error!("Path does not exist: {:?}", path);
        }
    }
}
