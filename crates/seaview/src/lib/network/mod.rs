//! Network communication module for mesh data transfer

pub mod protocol;
pub mod receiver;

#[allow(unused_imports)]
pub use protocol::{MeshData, MessageHeader, MessageType, PROTOCOL_VERSION};
#[allow(unused_imports)]
pub use receiver::{MeshReceiver, NonBlockingMeshReceiver, ReceivedMesh};

use bevy::prelude::*;

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
            port: 9877,
            max_message_size_mb: 100,
        }
    }
}

/// Event emitted when a new mesh is received over the network
#[derive(Event, Debug)]
pub struct NetworkMeshReceived {
    pub entity: Entity,
    pub simulation_uuid: String,
    pub frame_number: u32,
    pub triangle_count: u32,
}
