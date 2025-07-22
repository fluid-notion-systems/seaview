use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<MeshHandleStorage>()
        .add_systems(Startup, setup)
        .add_systems(Update, debug_mesh_info)
        .run();
}

// Resource to store mesh handles
#[derive(Resource, Default)]
struct MeshHandleStorage {
    handles: Vec<(String, Handle<Mesh>)>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_storage: ResMut<MeshHandleStorage>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(Vec3::new(-0.3, -1.0, -0.5), Vec3::Y),
    ));

    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.8, 0.8),
        ..default()
    });

    // Test different RenderAssetUsages configurations
    let test_configs = vec![
        ("RENDER_WORLD", RenderAssetUsages::RENDER_WORLD),
        ("MAIN_WORLD", RenderAssetUsages::MAIN_WORLD),
        (
            "RENDER_WORLD | MAIN_WORLD",
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        ),
        ("all()", RenderAssetUsages::all()),
        ("default()", RenderAssetUsages::default()),
    ];

    for (i, (name, usage)) in test_configs.into_iter().enumerate() {
        // Create a simple triangle mesh with different usage flags
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, usage);

        let positions: Vec<[f32; 3]> = vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]];

        let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];

        let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        let handle = meshes.add(mesh);
        mesh_storage
            .handles
            .push((name.to_string(), handle.clone()));

        // Spawn entity
        let x = (i as f32 - 2.0) * 3.0;
        commands.spawn((
            Mesh3d(handle),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.0, 0.0),
            Name::new(format!("Triangle with {name}")),
        ));

        println!("Created mesh with RenderAssetUsages::{name}");
    }

    // Also test with a built-in mesh for comparison
    let cube_handle = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    mesh_storage
        .handles
        .push(("Built-in Cube".to_string(), cube_handle.clone()));

    commands.spawn((
        Mesh3d(cube_handle),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -3.0, 0.0),
        Name::new("Built-in Cube"),
    ));

    println!(
        "\nSetup complete: Created {} test meshes",
        mesh_storage.handles.len()
    );
}

fn debug_mesh_info(
    meshes: Res<Assets<Mesh>>,
    mesh_storage: Res<MeshHandleStorage>,
    query: Query<(&Name, &Mesh3d, &ViewVisibility)>,
    time: Res<Time>,
    mut last_update: Local<f32>,
) {
    let current = time.elapsed_secs();
    if current - *last_update < 2.0 {
        return;
    }
    *last_update = current;

    println!("\n=== Mesh Debug Info @ {current:.1}s ===");

    // Check stored handles
    println!("Checking stored handles:");
    for (name, handle) in &mesh_storage.handles {
        if let Some(mesh) = meshes.get(handle) {
            println!("  {}: {} vertices ✓", name, mesh.count_vertices());
        } else {
            println!("  {name}: NOT FOUND IN ASSETS ✗");
        }
    }

    // Check entities
    println!("\nChecking entities:");
    let mut found_count = 0;
    let mut visible_count = 0;

    for (name, mesh_component, visibility) in query.iter() {
        if let Some(mesh) = meshes.get(&mesh_component.0) {
            found_count += 1;
            if visibility.get() {
                visible_count += 1;
            }
            println!(
                "  {}: {} vertices, visible: {} ✓",
                name.as_str(),
                mesh.count_vertices(),
                visibility.get()
            );
        } else {
            println!("  {}: MESH NOT FOUND ✗", name.as_str());
        }
    }

    println!("\nSummary:");
    println!(
        "  Entities with valid meshes: {}/{}",
        found_count,
        query.iter().count()
    );
    println!("  Visible entities: {visible_count}");
    println!("  Total mesh assets: {}", meshes.len());

    // Also check what's actually in the mesh assets
    println!("\nAll mesh assets in storage:");
    let mut asset_count = 0;
    for (id, mesh) in meshes.iter() {
        println!("  Asset {:?}: {} vertices", id, mesh.count_vertices());
        asset_count += 1;
        if asset_count >= 10 {
            println!("  ... and {} more", meshes.len() - 10);
            break;
        }
    }

    println!("=============================");
}
