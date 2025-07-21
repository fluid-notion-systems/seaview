//! Network communication module for mesh data transfer

pub mod protocol;
pub mod receiver;

pub use protocol::{MeshData, MessageHeader, MessageType, PROTOCOL_VERSION};
pub use receiver::{MeshReceiver, NonBlockingMeshReceiver, ReceivedMesh};
