//! Seaview mesh viewer library
//!
//! This library provides functionality for viewing and processing 3D mesh data,
//! including network communication for real-time mesh streaming.

pub mod network;

// Re-export commonly used types
pub use network::{MeshReceiver, ReceivedMesh};
