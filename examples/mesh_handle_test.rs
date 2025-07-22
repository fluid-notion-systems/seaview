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

// Resource to store mesh handles and prevent them from being dropped
#[derive(Resource, Default)]
struct MeshHandleStorage {
    handles: Vec<Handle<Mesh>>,
}

#[derive(Component)]
struct MeshDebugInfo {
    name: String,
    expected_vertices: usize,
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
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(Vec3::new(-0.3, -1.0, -0.5), Vec3::Y),
    ));

    // Create materials
    let red_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        ..default()
    });

    let green_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.0),
        ..default()
    });

    let blue_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.0, 1.0),
        ..default()
    });

    // Test 1: Built-in cube mesh
    let cube_handle = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    mesh_storage.handles.push(cube_handle.clone());

    commands.spawn((
        Mesh3d(cube_handle),
        MeshMaterial3d(green_material),
        Transform::from_xyz(3.0, 0.0, 0.0),
        MeshDebugInfo {
            name: "Built-in Cube".to_string(),
            expected_vertices: 24,
        },
    ));

    // Test 2: Simple triangle mesh
    let mut triangle_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let positions: Vec<[f32; 3]> = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
    ];

    let normals: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];

    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [0.5, 1.0],
    ];

    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let triangle_handle = meshes.add(triangle_mesh);
    mesh_storage.handles.push(triangle_handle.clone());

    commands.spawn((
        Mesh3d(triangle_handle),
        MeshMaterial3d(red_material),
        Transform::from_xyz(-3.0, 0.0, 0.0),
        MeshDebugInfo {
            name: "Manual Triangle".to_string(),
            expected_vertices: 3,
        },
    ));

    // Test 3: Large mesh to test performance
    let mut large_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let mut large_positions = Vec::new();
    let mut large_normals = Vec::new();
    let mut large_uvs = Vec::new();

    // Create a 1000x1000 triangle grid (2 million triangles, 6 million vertices)
    for i in 0..1000 {
        for j in 0..1000 {
            let x = i as f32 * 0.01 - 5.0;
            let z = j as f32 * 0.01 - 5.0;

            // First triangle
            large_positions.push([x, 0.0, z]);
            large_positions.push([x + 0.01, 0.0, z]);
            large_positions.push([x, 0.0, z + 0.01]);

            // Second triangle
            large_positions.push([x + 0.01, 0.0, z]);
            large_positions.push([x + 0.01, 0.0, z + 0.01]);
            large_positions.push([x, 0.0, z + 0.01]);

            // Normals (pointing up)
            for _ in 0..6 {
                large_normals.push([0.0, 1.0, 0.0]);
            }

            // UVs
            large_uvs.push([0.0, 0.0]);
            large_uvs.push([1.0, 0.0]);
            large_uvs.push([0.0, 1.0]);
            large_uvs.push([1.0, 0.0]);
            large_uvs.push([1.0, 1.0]);
            large_uvs.push([0.0, 1.0]);
        }
    }

    large_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, large_positions);
    large_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, large_normals);
    large_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, large_uvs);

    let large_handle = meshes.add(large_mesh);
    mesh_storage.handles.push(large_handle.clone());

    commands.spawn((
        Mesh3d(large_handle),
        MeshMaterial3d(blue_material),
        Transform::from_xyz(0.0, -2.0, -10.0),
        MeshDebugInfo {
            name: "Large Mesh (2M triangles)".to_string(),
            expected_vertices: 6_000_000,
        },
    ));

    println!("Setup complete: Created 3 test meshes including a 6M vertex mesh");
}

fn debug_mesh_info(
    meshes: Res<Assets<Mesh>>,
    mesh_storage: Res<MeshHandleStorage>,
    query: Query<(&Mesh3d, &MeshDebugInfo, &ViewVisibility)>,
    time: Res<Time>,
    mut last_update: Local<f32>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
) {
    let current = time.elapsed_secs();
    if current - *last_update < 2.0 {
        return;
    }
    *last_update = current;

    println!("\n=== Mesh Debug Info @ {current:.1}s ===");

    // Check stored handles
    println!("Stored mesh handles: {}", mesh_storage.handles.len());
    for (i, handle) in mesh_storage.handles.iter().enumerate() {
        if let Some(mesh) = meshes.get(handle) {
            println!("  Handle {}: {} vertices", i, mesh.count_vertices());
        } else {
            println!("  Handle {i}: NOT FOUND IN ASSETS");
        }
    }

    // Check entities
    println!("\nEntities with meshes:");
    let mut total_vertices = 0;
    let mut visible_count = 0;

    for (mesh_component, info, visibility) in query.iter() {
        let handle = &mesh_component.0;

        if let Some(mesh) = meshes.get(handle) {
            let vertex_count = mesh.count_vertices();
            total_vertices += vertex_count;

            if visibility.get() {
                visible_count += 1;
            }

            println!(
                "  {}: {} vertices (expected: {}), visible: {}",
                info.name,
                vertex_count,
                info.expected_vertices,
                visibility.get()
            );
        } else {
            println!(
                "  {}: MESH NOT FOUND (expected: {} vertices)",
                info.name,
                info.expected_vertices
            );
        }
    }

    println!("\nSummary:");
    println!("  Total vertices: {total_vertices}");
    println!("  Total vertices (M): {:.2}", total_vertices as f32 / 1_000_000.0);
    println!("  Visible meshes: {visible_count}");
    println!("  Mesh assets in storage: {}", meshes.len());

    // Get FPS from diagnostics
    if let Some(fps_diagnostic) = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
    {
        if let Some(fps) = fps_diagnostic.smoothed() {
            println!("  FPS: {fps:.1}");
        }
    }

    println!("=============================");
}
