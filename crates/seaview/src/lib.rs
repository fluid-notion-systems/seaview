//! Seaview mesh viewer library
//!
//! This library provides functionality for viewing and processing 3D mesh data,
//! including network communication for real-time mesh streaming.

pub mod coordinates;
pub mod network;
pub mod sequence;
pub mod session;
pub mod systems;
pub mod ui;

// Re-export commonly used types
pub use network::{MeshReceiver, ReceivedMesh};
pub use session::{Session, SessionManager, SessionPlugin};
pub use ui::SeaviewUiPlugin;
