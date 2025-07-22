//! Network library for real-time mesh streaming between simulation and visualization
//!
//! This crate provides a high-performance network protocol for streaming triangle mesh
//! data from simulations to visualization tools. It supports both Rust and C/C++ clients
//! through FFI bindings.

pub mod protocol;
pub mod receiver;
pub mod sender;
pub mod types;

#[cfg(feature = "ffi")]
pub mod ffi;

// Re-export commonly used types
pub use protocol::{MessageType, Protocol, ProtocolError, WireFormat, PROTOCOL_VERSION};
pub use receiver::{
    MeshReceiver, NonBlockingMeshReceiver, ReceiveError, ReceivedMesh, ReceiverConfig,
};
pub use sender::{MeshSender, NetworkError, SenderConfig};
pub use types::{DomainBounds, MeshFrame, MeshMetadata};

/// Result type for network operations
pub type Result<T> = std::result::Result<T, NetworkError>;
