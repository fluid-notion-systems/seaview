//! Network system for receiving mesh data in real-time

use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::sync::{Arc, Mutex};

use crate::network::{NonBlockingMeshReceiver, ReceivedMesh};

pub struct NetworkMeshPlugin;

impl Plugin for NetworkMeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkConfig>()
            .init_resource::<NetworkReceiver>()
            .add_event::<NetworkMeshReceived>()
            .add_systems(Startup, setup_network_receiver)
            .add_systems(Update, poll_network_meshes);
    }
}

/// Configuration for network mesh receiving
#[derive(Resource)]
pub struct NetworkConfig {
    pub enabled: bool,
    pub port: u16,
    pub max_message_size_mb: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9877, // Different from mesh_receiver_service default (9876)
            max_message_size_mb: 100,
        }
    }
}

/// Resource that holds the network receiver
#[derive(Resource, Default)]
pub struct NetworkReceiver {
    receiver: Option<Arc<Mutex<NonBlockingMeshReceiver>>>,
}

/// Component to mark entities that were created from network data
#[derive(Component)]
pub struct NetworkMesh {
    pub simulation_uuid: String,
    pub frame_number: u32,
}

/// Event emitted when a new mesh is received over the network
#[derive(Event)]
pub struct NetworkMeshReceived {
    pub entity: Entity,
    pub simulation_uuid: String,
    pub frame_number: u32,
    pub triangle_count: u32,
}

fn setup_network_receiver(
    config: Res<NetworkConfig>,
    mut network_receiver: ResMut<NetworkReceiver>,
) {
    if !config.enabled {
        info!("Network mesh receiving is disabled");
        return;
    }

    match NonBlockingMeshReceiver::new(config.port, config.max_message_size_mb) {
        Ok(receiver) => {
            info!("Network mesh receiver listening on port {}", config.port);
            network_receiver.receiver = Some(Arc::new(Mutex::new(receiver)));
        }
        Err(e) => {
            error!(
                "Failed to start network receiver on port {}: {}",
                config.port, e
            );
        }
    }
}

fn poll_network_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    network_receiver: Res<NetworkReceiver>,
    mut mesh_received_events: EventWriter<NetworkMeshReceived>,
) {
    if let Some(receiver) = &network_receiver.receiver {
        // Try to lock the receiver
        if let Ok(mut receiver_guard) = receiver.try_lock() {
            // Process available meshes
            loop {
                match receiver_guard.try_receive() {
                    Ok(Some(received_mesh)) => {
                        info!(
                            "Received mesh via network: {} triangles for simulation {} frame {}",
                            received_mesh.triangle_count,
                            received_mesh.simulation_uuid,
                            received_mesh.frame_number
                        );

                        // Convert received mesh to Bevy mesh
                        match convert_to_bevy_mesh(&received_mesh) {
                            Ok(mesh) => {
                                let mesh_handle = meshes.add(mesh);

                                // Create a blue-ish material for network meshes
                                let material = materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.3, 0.5, 0.8),
                                    metallic: 0.1,
                                    perceptual_roughness: 0.8,
                                    ..default()
                                });

                                // Spawn the mesh entity
                                let entity = commands
                                    .spawn((
                                        Mesh3d(mesh_handle),
                                        MeshMaterial3d(material),
                                        Transform::default(),
                                        NetworkMesh {
                                            simulation_uuid: received_mesh.simulation_uuid.clone(),
                                            frame_number: received_mesh.frame_number,
                                        },
                                    ))
                                    .id();

                                // Emit event
                                mesh_received_events.write(NetworkMeshReceived {
                                    entity,
                                    simulation_uuid: received_mesh.simulation_uuid,
                                    frame_number: received_mesh.frame_number,
                                    triangle_count: received_mesh.triangle_count,
                                });
                            }
                            Err(e) => {
                                error!("Failed to convert received mesh: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // No more meshes available
                        break;
                    }
                    Err(e) => {
                        error!("Error receiving mesh: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

fn convert_to_bevy_mesh(received: &ReceivedMesh) -> Result<Mesh, String> {
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    // Extract vertices and calculate normals
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Process each triangle
    for tri_idx in 0..received.triangle_count as usize {
        let base_idx = tri_idx * 9;

        // Get triangle vertices
        let v0 = [
            received.vertices[base_idx],
            received.vertices[base_idx + 1],
            received.vertices[base_idx + 2],
        ];
        let v1 = [
            received.vertices[base_idx + 3],
            received.vertices[base_idx + 4],
            received.vertices[base_idx + 5],
        ];
        let v2 = [
            received.vertices[base_idx + 6],
            received.vertices[base_idx + 7],
            received.vertices[base_idx + 8],
        ];

        // Calculate face normal
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let normal = if len > 0.0 {
            [normal[0] / len, normal[1] / len, normal[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };

        // Add vertices with shared normal
        let start_idx = positions.len() as u32;
        positions.push(v0);
        positions.push(v1);
        positions.push(v2);

        normals.push(normal);
        normals.push(normal);
        normals.push(normal);

        // Add indices
        indices.push(start_idx);
        indices.push(start_idx + 1);
        indices.push(start_idx + 2);
    }

    // Create dummy UVs (could be improved later)
    let uvs = vec![[0.0, 0.0]; positions.len()];

    // Set mesh attributes
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(positions),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs));
    mesh.insert_indices(Indices::U32(indices));

    Ok(mesh)
}
