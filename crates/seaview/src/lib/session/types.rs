//! Session types and data structures
//!
//! This module defines the core types used throughout the session management system.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// A session represents a collection of mesh frames from a specific source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for the session
    pub id: Uuid,

    /// Human-readable name for the session
    pub name: String,

    /// When the session was created
    pub created_at: chrono::DateTime<chrono::Local>,

    /// When the session was last accessed
    pub last_accessed: chrono::DateTime<chrono::Local>,

    /// Source of the mesh data
    pub source: SessionSource,

    /// Metadata about the session
    pub metadata: SessionMetadata,

    /// Frame storage information
    #[serde(skip)]
    pub frames: FrameStorage,
}

impl Session {
    /// Create a new session with the given name and source
    pub fn new(name: String, source: SessionSource) -> Self {
        let now = chrono::Local::now();
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: now,
            last_accessed: now,
            source,
            metadata: SessionMetadata::default(),
            frames: FrameStorage::new(),
        }
    }

    /// Get the number of frames in this session
    pub fn frame_count(&self) -> usize {
        self.frames.count()
    }

    /// Update the last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Local::now();
    }

    /// Add a mesh frame to the session
    pub fn add_frame(&mut self, mesh: Mesh) -> usize {
        let index = self.frames.add(mesh);
        self.metadata.frames_received += 1;
        self.touch();
        index
    }

    /// Get a mesh frame by index
    pub fn get_frame(&self, index: usize) -> Option<&Mesh> {
        self.frames.get(index)
    }
}

/// Source of session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionSource {
    /// Network source with port and optional address
    Network {
        port: u16,
        source_address: Option<String>,
    },

    /// File source with path
    File { path: PathBuf },

    /// Data lake source (future)
    DataLake { connection_string: String },
}

impl SessionSource {
    /// Get a display string for the source
    pub fn display_string(&self) -> String {
        match self {
            SessionSource::Network {
                port,
                source_address,
            } => {
                if let Some(addr) = source_address {
                    format!("Network ({}:{})", addr, port)
                } else {
                    format!("Network (port {})", port)
                }
            }
            SessionSource::File { path } => {
                format!("File ({})", path.display())
            }
            SessionSource::DataLake { connection_string } => {
                format!("Data Lake ({})", connection_string)
            }
        }
    }
}

/// Metadata about a session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    /// Total number of frames expected (if known)
    pub total_frames_expected: Option<usize>,

    /// Number of frames received so far
    pub frames_received: usize,

    /// Simulation UUID if provided
    pub simulation_uuid: Option<String>,

    /// Timestep of the simulation
    pub timestep: Option<f32>,

    /// Spatial bounds of the simulation
    pub spatial_bounds: Option<SpatialBounds>,

    /// Additional metadata as key-value pairs
    pub custom: HashMap<String, String>,
}

/// Spatial bounds of a simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialBounds {
    pub min: Vec3,
    pub max: Vec3,
}

/// In-memory storage for mesh frames
#[derive(Debug, Default, Clone)]
pub struct FrameStorage {
    /// Stored mesh frames
    frames: Vec<Mesh>,
}

impl FrameStorage {
    /// Create a new frame storage
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    /// Add a mesh frame and return its index
    pub fn add(&mut self, mesh: Mesh) -> usize {
        self.frames.push(mesh);
        self.frames.len() - 1
    }

    /// Get a mesh frame by index
    pub fn get(&self, index: usize) -> Option<&Mesh> {
        self.frames.get(index)
    }

    /// Get the number of stored frames
    pub fn count(&self) -> usize {
        self.frames.len()
    }

    /// Clear all frames
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Session creation parameters
#[derive(Debug, Clone)]
pub struct CreateSessionParams {
    pub name: String,
    pub source: SessionSource,
    pub metadata: Option<SessionMetadata>,
}

/// Session query filter
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// Filter by source type
    pub source_type: Option<SessionSourceType>,

    /// Filter by name (substring match)
    pub name_contains: Option<String>,

    /// Filter by minimum frame count
    pub min_frames: Option<usize>,
}

/// Type of session source for filtering
#[derive(Debug, Clone, PartialEq)]
pub enum SessionSourceType {
    Network,
    File,
    DataLake,
}

impl SessionSourceType {
    /// Check if a session source matches this type
    pub fn matches(&self, source: &SessionSource) -> bool {
        match (self, source) {
            (SessionSourceType::Network, SessionSource::Network { .. }) => true,
            (SessionSourceType::File, SessionSource::File { .. }) => true,
            (SessionSourceType::DataLake, SessionSource::DataLake { .. }) => true,
            _ => false,
        }
    }
}
