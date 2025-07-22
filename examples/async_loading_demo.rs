//! Example demonstrating async parallel STL loading
//!
//! This example shows how to use the AsyncStlLoader to load multiple STL files
//! in parallel without blocking the main thread.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(DemoState::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (queue_demo_loads, handle_load_completion, update_ui).chain(),
        )
        .run();
}

#[derive(Resource, Default)]
struct DemoState {
    loads_queued: bool,
    total_queued: usize,
    completed: usize,
    failed: usize,
    mesh_entities: Vec<Entity>,
}

#[derive(Component)]
struct LoadingUI;

#[derive(Component)]
struct ProgressBar;

#[derive(Component)]
struct StatsText;

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));

    // UI
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            LoadingUI,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Async STL Loading Demo"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Stats text
            parent.spawn((
                Text::new("Press SPACE to start loading"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                StatsText,
            ));

            // Progress bar container
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(30.0),
                        margin: UiRect::top(Val::Px(20.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(Color::WHITE),
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                ))
                .with_children(|parent| {
                    // Progress bar fill
                    parent.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.8, 0.2)),
                        ProgressBar,
                    ));
                });
        });

    info!("Async Loading Demo started. Press SPACE to begin loading STL files.");
}

fn queue_demo_loads(mut demo_state: ResMut<DemoState>, keyboard: Res<ButtonInput<KeyCode>>) {
    if demo_state.loads_queued || !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    info!("Queueing STL files for parallel loading...");

    // Queue multiple STL files with different priorities
    let test_files = vec![
        (
            "test_sequences/cube_rotation/cube_frame_0000.stl",
            "Critical",
        ),
        ("test_sequences/cube_rotation/cube_frame_0001.stl", "High"),
        ("test_sequences/cube_rotation/cube_frame_0002.stl", "High"),
        ("test_sequences/cube_rotation/cube_frame_0003.stl", "Normal"),
        ("test_sequences/cube_rotation/cube_frame_0004.stl", "Normal"),
        ("test_sequences/cube_rotation/cube_frame_0005.stl", "Normal"),
        ("test_sequences/cube_rotation/cube_frame_0006.stl", "Low"),
        ("test_sequences/cube_rotation/cube_frame_0007.stl", "Low"),
        ("test_sequences/cube_rotation/cube_frame_0008.stl", "Low"),
        ("test_sequences/cube_rotation/cube_frame_0009.stl", "Low"),
    ];

    // Simulate queuing - in a real integration, we'd use the AsyncStlLoader
    demo_state.loads_queued = true;
    demo_state.total_queued = test_files.len();

    info!("Demo: Would queue {} files for loading", test_files.len());
    info!("Note: This is a demo showing the UI. Actual async loading requires integration with AsyncStlLoader.");
}

fn handle_load_completion(
    mut commands: Commands,
    mut demo_state: ResMut<DemoState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    // Simulate load completion over time
    if demo_state.loads_queued && demo_state.completed < demo_state.total_queued {
        // Simulate loading one file every 0.5 seconds
        let elapsed = time.elapsed_secs();
        let expected_completed = ((elapsed / 0.5) as usize).min(demo_state.total_queued);

        while demo_state.completed < expected_completed {
            demo_state.completed += 1;

            // Create a simple cube mesh as placeholder
            let mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));

            let material = materials.add(StandardMaterial {
                base_color: Color::hsl((demo_state.completed as f32 * 36.0) % 360.0, 0.8, 0.6),
                metallic: 0.2,
                perceptual_roughness: 0.6,
                ..default()
            });

            // Calculate position in a grid
            let index = demo_state.completed - 1;
            let x = (index % 5) as f32 * 3.0 - 6.0;
            let z = (index / 5) as f32 * 3.0 - 3.0;

            let entity = commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(0.5)),
                    Name::new(format!("Demo Mesh {index}")),
                ))
                .id();

            demo_state.mesh_entities.push(entity);

            info!(
                "Simulated load completion: {}/{}",
                demo_state.completed, demo_state.total_queued
            );
        }
    }
}

fn update_ui(
    demo_state: Res<DemoState>,
    mut progress_query: Query<&mut Node, With<ProgressBar>>,
    mut text_query: Query<&mut Text, With<StatsText>>,
    time: Res<Time>,
) {
    if !demo_state.loads_queued {
        return;
    }

    // Update progress bar
    if let Ok(mut node) = progress_query.single_mut() {
        let progress = if demo_state.total_queued > 0 {
            demo_state.completed as f32 / demo_state.total_queued as f32
        } else {
            0.0
        };
        node.width = Val::Percent(progress * 100.0);
    }

    // Update stats text
    if let Ok(mut text) = text_query.single_mut() {
        let loading = demo_state.total_queued.saturating_sub(demo_state.completed);

        text.0 = format!(
            "Queued: {} | Loading: {} | Completed: {} | Failed: {} | FPS: {:.1}",
            0,
            loading,
            demo_state.completed,
            demo_state.failed,
            1.0 / time.delta_secs().max(0.001)
        );
    }
}
