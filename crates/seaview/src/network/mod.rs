//! Network communication module for mesh data transfer

pub mod protocol;
pub mod receiver;

#[allow(unused_imports)]
pub use protocol::{MeshData, MessageHeader, MessageType, PROTOCOL_VERSION};
#[allow(unused_imports)]
pub use receiver::{MeshReceiver, NonBlockingMeshReceiver, ReceivedMesh};
