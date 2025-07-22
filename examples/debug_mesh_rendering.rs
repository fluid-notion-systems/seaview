use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (print_mesh_info, rotate_meshes))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -45.0_f32.to_radians(),
            45.0_f32.to_radians(),
            0.0,
        )),
    ));

    // Create a simple triangle mesh manually
    let mut triangle_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Define vertices for a triangle
    let positions: Vec<[f32; 3]> = vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]];

    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];

    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let triangle_handle = meshes.add(triangle_mesh);

    // Spawn triangle mesh entity using Mesh3d
    let triangle_entity = commands
        .spawn((
            Mesh3d(triangle_handle.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                ..default()
            })),
            Transform::from_xyz(-3.0, 0.0, 0.0),
            Name::new("Manual Triangle"),
        ))
        .id();

    println!(
        "Spawned triangle entity: {triangle_entity:?} with handle: {triangle_handle:?}"
    );

    // Also spawn a built-in cube for comparison
    let cube_mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    let cube_entity = commands
        .spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                ..default()
            })),
            Transform::from_xyz(3.0, 0.0, 0.0),
            Name::new("Built-in Cube"),
        ))
        .id();

    println!(
        "Spawned cube entity: {cube_entity:?} with handle: {cube_mesh:?}"
    );

    // Create a large triangle mesh (1000 triangles)
    let mut large_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let mut large_positions = Vec::new();
    let mut large_normals = Vec::new();
    let mut large_uvs = Vec::new();

    // Create a grid of triangles
    for i in 0..100 {
        for j in 0..10 {
            let x = i as f32 * 0.1 - 5.0;
            let z = j as f32 * 0.1 - 0.5;

            // Triangle 1
            large_positions.push([x, 0.0, z]);
            large_positions.push([x + 0.1, 0.0, z]);
            large_positions.push([x, 0.0, z + 0.1]);

            // Triangle 2
            large_positions.push([x + 0.1, 0.0, z]);
            large_positions.push([x + 0.1, 0.0, z + 0.1]);
            large_positions.push([x, 0.0, z + 0.1]);

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
    let large_entity = commands
        .spawn((
            Mesh3d(large_handle.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.0, 1.0),
                ..default()
            })),
            Transform::from_xyz(0.0, -1.0, -5.0),
            Name::new("Large Mesh (2000 triangles)"),
        ))
        .id();

    println!(
        "Spawned large mesh entity: {large_entity:?} with handle: {large_handle:?}"
    );
}

fn print_mesh_info(
    meshes: Res<Assets<Mesh>>,
    query: Query<(Entity, &Mesh3d, &Name, &ViewVisibility)>,
    time: Res<Time>,
    mut last_print: Local<f32>,
) {
    let current = time.elapsed_secs();
    if current - *last_print < 2.0 {
        return;
    }
    *last_print = current;

    println!("\n=== Mesh Info ===");
    let mut total_vertices = 0;
    let mut visible_count = 0;
    let mut total_entities = 0;

    for (entity, mesh_handle, name, visibility) in query.iter() {
        total_entities += 1;

        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            let vertex_count = mesh.count_vertices();
            total_vertices += vertex_count;

            if visibility.get() {
                visible_count += 1;
            }

            println!(
                "Entity {:?} - {}: {} vertices, visible: {}",
                entity,
                name.as_str(),
                vertex_count,
                visibility.get()
            );
        } else {
            println!(
                "Entity {:?} - {}: Mesh not loaded yet (handle: {:?})",
                entity,
                name.as_str(),
                mesh_handle.0
            );
        }
    }

    println!("Total entities with Mesh3d: {total_entities}");
    println!("Total vertices: {total_vertices}");
    println!("Visible meshes: {visible_count}");

    // Also print total mesh assets
    println!("Total mesh assets in storage: {}", meshes.len());
    println!("================\n");
}

fn rotate_meshes(mut query: Query<&mut Transform, With<Mesh3d>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}
