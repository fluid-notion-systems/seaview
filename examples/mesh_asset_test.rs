use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (check_mesh_loading, update_mesh_info))
        .run();
}

#[derive(Component)]
struct MeshInfo {
    name: String,
    expected_vertices: usize,
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
    let cube_mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(green_material),
        Transform::from_xyz(3.0, 0.0, 0.0),
        MeshInfo {
            name: "Built-in Cube".to_string(),
            expected_vertices: 24,
        },
    ));

    // Test 2: Simple triangle mesh
    let mut triangle_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let positions: Vec<[f32; 3]> = vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]];

    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];

    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    triangle_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let triangle_handle = meshes.add(triangle_mesh);

    commands.spawn((
        Mesh3d(triangle_handle),
        MeshMaterial3d(red_material),
        Transform::from_xyz(-3.0, 0.0, 0.0),
        MeshInfo {
            name: "Manual Triangle".to_string(),
            expected_vertices: 3,
        },
    ));

    // Test 3: Medium-sized mesh (100 triangles)
    let mut medium_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let mut medium_positions = Vec::new();
    let mut medium_normals = Vec::new();
    let mut medium_uvs = Vec::new();

    // Create a grid of triangles
    for i in 0..10 {
        for j in 0..10 {
            let x = i as f32 * 0.2 - 1.0;
            let z = j as f32 * 0.2 - 1.0;

            // Add a single triangle per grid cell
            medium_positions.push([x, 0.0, z]);
            medium_positions.push([x + 0.2, 0.0, z]);
            medium_positions.push([x + 0.1, 0.1, z + 0.1]);

            // Normals
            for _ in 0..3 {
                medium_normals.push([0.0, 1.0, 0.0]);
            }

            // UVs
            medium_uvs.push([0.0, 0.0]);
            medium_uvs.push([1.0, 0.0]);
            medium_uvs.push([0.5, 1.0]);
        }
    }

    medium_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, medium_positions);
    medium_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, medium_normals);
    medium_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, medium_uvs);

    let medium_handle = meshes.add(medium_mesh);

    commands.spawn((
        Mesh3d(medium_handle),
        MeshMaterial3d(blue_material),
        Transform::from_xyz(0.0, -1.0, -3.0),
        MeshInfo {
            name: "Medium Mesh (100 triangles)".to_string(),
            expected_vertices: 300,
        },
    ));

    println!("Setup complete: Created 3 test meshes");
}

fn check_mesh_loading(
    asset_server: Res<AssetServer>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(&Mesh3d, &MeshInfo)>,
) {
    static mut CHECKED: bool = false;

    unsafe {
        if CHECKED {
            return;
        }
    }

    println!("\n=== Mesh Loading Status ===");
    println!("Total meshes in asset storage: {}", meshes.len());

    for (mesh_component, info) in query.iter() {
        let handle = &mesh_component.0;
        let load_state = asset_server.get_load_state(handle.id());

        match load_state {
            Some(LoadState::Loaded) => {
                if let Some(mesh) = meshes.get(handle) {
                    let vertex_count = mesh.count_vertices();
                    println!(
                        "{}: LOADED - {} vertices (expected: {})",
                        info.name, vertex_count, info.expected_vertices
                    );
                } else {
                    println!(
                        "{}: LoadState is Loaded but mesh not in Assets<Mesh>!",
                        info.name
                    );
                }
            }
            Some(LoadState::Loading) => {
                println!("{}: Still loading...", info.name);
            }
            Some(LoadState::Failed(_)) => {
                println!("{}: Failed to load!", info.name);
            }
            Some(LoadState::NotLoaded) => {
                println!("{}: Not loaded", info.name);
            }
            None => {
                // For programmatically created meshes, check if they're in the asset storage
                if let Some(mesh) = meshes.get(handle) {
                    let vertex_count = mesh.count_vertices();
                    println!(
                        "{}: No LoadState (programmatic) - {} vertices (expected: {})",
                        info.name, vertex_count, info.expected_vertices
                    );
                } else {
                    println!("{}: No LoadState and not in Assets<Mesh>", info.name);
                }
            }
        }
    }

    println!("===========================\n");

    unsafe {
        CHECKED = true;
    }
}

fn update_mesh_info(
    meshes: Res<Assets<Mesh>>,
    query: Query<(&Mesh3d, &MeshInfo, &ViewVisibility)>,
    time: Res<Time>,
    mut last_update: Local<f32>,
) {
    let current = time.elapsed_secs();
    if current - *last_update < 3.0 {
        return;
    }
    *last_update = current;

    println!("\n=== Periodic Mesh Info Update ===");
    let mut total_vertices = 0;
    let mut visible_count = 0;
    let mut loaded_count = 0;

    for (mesh_handle, info, visibility) in query.iter() {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            let vertex_count = mesh.count_vertices();
            total_vertices += vertex_count;
            loaded_count += 1;

            if visibility.get() {
                visible_count += 1;
            }

            println!(
                "{}: {} vertices, visible: {}",
                info.name,
                vertex_count,
                visibility.get()
            );
        } else {
            println!("{}: Mesh not accessible", info.name);
        }
    }

    println!("\nSummary:");
    println!("  Loaded meshes: {}/{}", loaded_count, query.iter().count());
    println!("  Total vertices: {total_vertices}");
    println!("  Visible meshes: {visible_count}");
    println!("  Asset storage size: {}", meshes.len());

    // Also check FPS
    let fps = 1.0 / time.delta_secs();
    println!("  Current FPS: {fps:.1}");
    println!("================================\n");
}
