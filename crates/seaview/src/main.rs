use bevy::asset::io::AssetSourceBuilder;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use seaview::{SeaviewUiPlugin, SessionPlugin};

use seaview::app::cli::Args;
use seaview::app::systems::camera::{
    camera_controller, cursor_grab_system, handle_center_on_mesh, CenterOnMeshEvent, FpsCamera,
};
use seaview::app::systems::diagnostics::RenderingDiagnosticsPlugin;

use seaview::lib::coordinates::SourceOrientation;
use seaview::lib::sequence::{
    discovery::DiscoverSequenceRequest, LoadSequenceRequest, SequencePlugin,
};

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

    let mut app = App::new();

    // Register asset source for the sequence directory if provided
    if let Some(ref path) = args.path {
        // Canonicalize to get absolute path
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if canonical_path.is_dir() {
            let path_str = canonical_path.to_string_lossy().to_string();
            info!(
                "Registering asset source 'seq' for sequence directory: {:?}",
                canonical_path
            );
            info!("Asset source path: {}", path_str);
            app.register_asset_source("seq", AssetSourceBuilder::platform_default(&path_str, None));
            info!("Asset source 'seq' registered successfully");
        } else if canonical_path.is_file() {
            // Register parent directory for single file loads
            if let Some(parent) = canonical_path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                info!(
                    "Registering asset source 'seq' for file directory: {:?}",
                    parent
                );
                info!("Asset source path: {}", parent_str);
                app.register_asset_source(
                    "seq",
                    AssetSourceBuilder::platform_default(&parent_str, None),
                );
                info!("Asset source 'seq' registered successfully");
            } else {
                error!(
                    "Could not get parent directory for file: {:?}",
                    canonical_path
                );
            }
        }
    } else {
        warn!("No path provided - asset source 'seq' not registered");
        warn!("Provide a path via CLI to load meshes");
    }

    app.add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RenderingDiagnosticsPlugin)
        .add_plugins(SequencePlugin)
        // .add_plugins(BrpExtrasPlugin)
        .add_plugins(SessionPlugin)
        .add_plugins(SeaviewUiPlugin)
        .insert_resource(args)
        .insert_resource(source_orientation)
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
    args: Res<Args>,
    source_orientation: Res<SourceOrientation>,
    mut load_events: EventWriter<LoadSequenceRequest>,
) {
    if let Some(path) = &args.path {
        if path.is_dir() {
            // It's a directory - trigger sequence discovery
            info!("Discovering sequences in directory: {:?}", path);
            commands.spawn(DiscoverSequenceRequest {
                directory: path.clone(),
                recursive: false,
                source_orientation: *source_orientation,
            });
        } else if path.is_file() {
            // It's a single file - load it directly using the sequence loader
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            match ext.as_str() {
                "glb" | "gltf" => {
                    info!("Loading single glTF/GLB file: {:?}", path);
                    // Load as a single-frame "sequence"
                    // Use the seq:// asset source that was registered at startup
                    load_events.write(LoadSequenceRequest {
                        frame_paths: vec![path.clone()],
                    });
                }
                _ => {
                    warn!("Unsupported file type: {:?}", path);
                }
            }
        } else {
            error!("Path does not exist: {:?}", path);
        }
    }
}
