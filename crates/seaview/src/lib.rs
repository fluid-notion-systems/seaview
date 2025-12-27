//! Seaview mesh viewer library
//!
//! This library provides functionality for viewing and processing 3D mesh data,
//! including network communication for real-time mesh streaming.

pub mod lib {
    pub mod coordinates;
    pub mod network;
    pub mod sequence;
    pub mod session;
}

pub mod app {
    pub mod cli;
    pub mod systems;
    pub mod ui;
}

// Re-export commonly used types from lib modules
pub use app::ui::SeaviewUiPlugin;
pub use lib::coordinates;
pub use lib::network::{self, MeshReceiver, ReceivedMesh};
pub use lib::sequence;
pub use lib::session::{self, Session, SessionManager, SessionPlugin};
