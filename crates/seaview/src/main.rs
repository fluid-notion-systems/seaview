use bevy::asset::io::AssetSourceBuilder;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::pbr::{DefaultOpaqueRendererMethod, ScreenSpaceReflections};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use seaview::lib::lighting::GlobalLight;
use seaview::{MeshDimensions, MeshInfoPlugin, NightLightingPlugin, SeaviewUiPlugin, SessionPlugin};

use seaview::app::cli::Args;
use seaview::app::systems::camera::{
    camera_controller, cursor_grab_system, handle_center_on_mesh, CenterOnMeshEvent, FpsCamera,
};
use seaview::app::systems::diagnostics::RenderingDiagnosticsPlugin;

use seaview::lib::coordinates::SourceOrientation;
use seaview::lib::sequence::{
    discovery::DiscoverSequenceRequest, LoadSequenceRequest, SequencePlugin,
};
use seaview::lib::settings::{
    resolve_settings_dir, SaveViewEvent, Settings, SettingsResource,
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

    // Load per-directory settings if a path was provided
    let settings_dir = args.path.as_ref().and_then(|p| resolve_settings_dir(p));
    let dir_settings = settings_dir
        .as_ref()
        .and_then(|dir| match Settings::load_from_dir(dir) {
            Ok(Some(s)) => {
                info!("Loaded seaview.toml from {:?}", dir);
                Some(s)
            }
            Ok(None) => {
                info!("No seaview.toml found in {:?}, using defaults", dir);
                None
            }
            Err(e) => {
                warn!("Failed to load seaview.toml: {}", e);
                None
            }
        });

    // Determine source coordinates: CLI flag > seaview.toml > default
    // clap sets default_value = "yup", so we check if the user explicitly passed it
    let coord_string = {
        let cli_explicit = std::env::args().any(|a| a.starts_with("--source-coordinates"));
        if cli_explicit {
            args.source_coordinates.clone()
        } else if let Some(ref s) = dir_settings {
            s.sequence
                .as_ref()
                .and_then(|seq| seq.source_coordinates.clone())
                .unwrap_or_else(|| args.source_coordinates.clone())
        } else {
            args.source_coordinates.clone()
        }
    };

    // Parse source coordinate system
    let source_orientation = match SourceOrientation::from_str(&coord_string) {
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

    // Build settings resource
    let settings_resource = SettingsResource {
        settings: dir_settings.clone().unwrap_or_default(),
        directory: settings_dir,
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

    app.insert_resource(DefaultOpaqueRendererMethod::deferred())
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RenderingDiagnosticsPlugin)
        .add_plugins(SequencePlugin)
        // .add_plugins(BrpExtrasPlugin)
        .add_plugins(SessionPlugin)
        .add_plugins(SeaviewUiPlugin)
        .add_plugins(NightLightingPlugin)
        .add_plugins(MeshInfoPlugin)
        .insert_resource(args)
        .insert_resource(source_orientation)
        .insert_resource(settings_resource)
        .add_event::<CenterOnMeshEvent>()
        .add_event::<SaveViewEvent>()
        .add_systems(Startup, (setup, handle_input_path, setup_cursor))
        .add_systems(
            Update,
            (
                camera_controller,
                cursor_grab_system,
                handle_center_on_mesh,
                handle_save_view,
                // debug_mesh_cache_status,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    settings_res: Res<SettingsResource>,
    mut ui_state: ResMut<seaview::app::ui::state::UiState>,
) {
    // Determine initial camera transform: seaview.toml > hardcoded default
    let camera_transform = settings_res
        .settings
        .camera
        .as_ref()
        .map(|cam| {
            info!(
                "Applying camera from seaview.toml: pos={:?} rot={:?}",
                cam.position, cam.rotation
            );
            cam.to_transform()
        })
        .unwrap_or_else(|| {
            Transform::from_xyz(100.0, 100.0, 100.0)
                .looking_at(Vec3::new(37.0, 37.0, 27.5), Vec3::Y)
        });

    // Spawn the FPS camera with HDR, MSAA off, and SSR enabled (deferred rendering)
    commands.spawn((
        Camera3d::default(),
        camera_transform,
        FpsCamera::default(),
        Msaa::Off,
        ScreenSpaceReflections::default(),
    ));

    // Apply cached mesh bounds from seaview.toml
    if let Some(ref mesh_bounds) = settings_res.settings.mesh {
        let mut dims = MeshDimensions::from_settings(mesh_bounds);
        info!(
            "Loaded cached mesh bounds from seaview.toml: {:.2} × {:.2} × {:.2} m",
            dims.dimensions.unwrap().x,
            dims.dimensions.unwrap().y,
            dims.dimensions.unwrap().z,
        );
        // Mark as computed so auto-compute doesn't overwrite until recompute is requested
        dims.computed = true;
        commands.insert_resource(dims);
    }

    // Apply playback settings from seaview.toml
    if let Some(ref pb) = settings_res.settings.playback {
        if let Some(speed) = pb.speed {
            ui_state.playback.speed = speed;
            info!("Applied playback speed from seaview.toml: {}", speed);
        }
        if let Some(loop_enabled) = pb.loop_enabled {
            ui_state.playback.loop_enabled = loop_enabled;
            info!("Applied loop setting from seaview.toml: {}", loop_enabled);
        }
    }

    // Add a directional light for overall illumination
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(Vec3::new(-0.3, -1.0, -0.5), Vec3::Y),
        GlobalLight,
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
        GlobalLight,
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
        GlobalLight,
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

/// System that handles save view requests
fn handle_save_view(
    mut save_events: EventReader<SaveViewEvent>,
    mut settings_res: ResMut<SettingsResource>,
    camera_query: Query<&Transform, (With<Camera3d>, With<FpsCamera>)>,
    ui_state: Res<seaview::app::ui::state::UiState>,
    source_orientation: Res<SourceOrientation>,
) {
    for _event in save_events.read() {
        if let Ok(transform) = camera_query.single() {
            // Update settings with current camera, playback, and coordinate state
            settings_res
                .settings
                .set_camera_from_transform(transform);
            settings_res
                .settings
                .set_playback(ui_state.playback.speed, ui_state.playback.loop_enabled);
            settings_res.settings.sequence =
                Some(seaview::lib::settings::SequenceSettings {
                    source_coordinates: Some(source_orientation.to_string()),
                });

            match settings_res.save() {
                Ok(()) => {
                    info!("💾 View saved to seaview.toml");
                }
                Err(e) => {
                    error!("Failed to save view: {}", e);
                }
            }
        } else {
            warn!("No camera found to save");
        }
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
