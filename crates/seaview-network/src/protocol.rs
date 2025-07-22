//! Network protocol definitions for mesh streaming
//!
//! This module defines the wire protocol for transmitting mesh data between
//! simulation and visualization components.

use crate::types::MeshFrame;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use thiserror::Error;
use tracing::{debug, trace};

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u16 = 2;

/// Maximum message size (100MB by default)
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 100 * 1024 * 1024;

/// Message types in the protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    /// Mesh frame data
    MeshFrame = 0x01,
    /// Metadata about the stream
    Metadata = 0x02,
    /// Checkpoint/snapshot marker
    Checkpoint = 0x03,
    /// End of stream marker
    EndOfStream = 0x04,
    /// Heartbeat/keepalive
    Heartbeat = 0x05,
}

impl MessageType {
    /// Convert from u8 representation
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::MeshFrame),
            0x02 => Some(Self::Metadata),
            0x03 => Some(Self::Checkpoint),
            0x04 => Some(Self::EndOfStream),
            0x05 => Some(Self::Heartbeat),
            _ => None,
        }
    }
}

/// Wire format for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    /// Binary format using bincode
    Bincode,
    /// JSON format for debugging
    #[cfg(feature = "json")]
    Json,
}

impl Default for WireFormat {
    fn default() -> Self {
        Self::Bincode
    }
}

/// Protocol error types
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[cfg(feature = "json")]
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Invalid protocol version: expected {expected}, got {received}")]
    InvalidVersion { expected: u16, received: u16 },

    #[error("Invalid message type: {0}")]
    InvalidMessageType(u8),

    #[error("Message too large: {size} bytes exceeds maximum {max_size} bytes")]
    MessageTooLarge { size: usize, max_size: usize },

    #[error("Invalid message format")]
    InvalidFormat,

    #[error("Unexpected end of stream")]
    UnexpectedEof,
}

/// Message envelope containing type and payload
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Protocol version
    pub version: u16,
    /// Message type
    pub msg_type: MessageType,
    /// Serialized payload
    pub payload: Vec<u8>,
}

impl NetworkMessage {
    /// Create a new network message
    pub fn new(msg_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            msg_type,
            payload,
        }
    }

    /// Get the total size of the message when serialized
    pub fn size(&self) -> usize {
        // Version (2) + Type (1) + Length (4) + Payload
        2 + 1 + 4 + self.payload.len()
    }
}

/// Protocol handler for reading and writing messages
pub struct Protocol {
    format: WireFormat,
    max_message_size: usize,
}

impl Default for Protocol {
    fn default() -> Self {
        Self {
            format: WireFormat::default(),
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }
}

impl Protocol {
    /// Create a new protocol handler
    pub fn new(format: WireFormat) -> Self {
        Self {
            format,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }

    /// Set the maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Serialize a mesh frame
    pub fn serialize_mesh(&self, mesh: &MeshFrame) -> Result<NetworkMessage, ProtocolError> {
        debug!(
            "Serializing mesh frame: sim_id={}, frame={}, vertices={}",
            mesh.simulation_id,
            mesh.frame_number,
            mesh.vertex_count()
        );

        let payload = match self.format {
            WireFormat::Bincode => {
                trace!("Using bincode format");
                bincode::serialize(mesh)?
            }
            #[cfg(feature = "json")]
            WireFormat::Json => {
                trace!("Using JSON format");
                serde_json::to_vec(mesh)?
            }
        };

        debug!("Serialized payload size: {} bytes", payload.len());

        if payload.len() > self.max_message_size {
            return Err(ProtocolError::MessageTooLarge {
                size: payload.len(),
                max_size: self.max_message_size,
            });
        }

        Ok(NetworkMessage::new(MessageType::MeshFrame, payload))
    }

    /// Deserialize a mesh frame
    pub fn deserialize_mesh(&self, payload: &[u8]) -> Result<MeshFrame, ProtocolError> {
        trace!("Deserializing mesh from {} bytes", payload.len());

        let mesh = match self.format {
            WireFormat::Bincode => bincode::deserialize(payload)?,
            #[cfg(feature = "json")]
            WireFormat::Json => serde_json::from_slice(payload)?,
        };

        Ok(mesh)
    }

    /// Write a message to a stream
    pub fn write_message<W: Write>(
        &self,
        writer: &mut W,
        message: &NetworkMessage,
    ) -> Result<(), ProtocolError> {
        use byteorder::{LittleEndian, WriteBytesExt};

        debug!(
            "Writing message: type={:?}, payload_size={}",
            message.msg_type,
            message.payload.len()
        );

        // Write header
        writer.write_u16::<LittleEndian>(message.version)?;
        writer.write_u8(message.msg_type as u8)?;
        writer.write_u32::<LittleEndian>(message.payload.len() as u32)?;

        // Write payload
        writer.write_all(&message.payload)?;
        writer.flush()?;

        trace!("Message written successfully");
        Ok(())
    }

    /// Read a message from a stream
    pub fn read_message<R: Read>(
        &self,
        reader: &mut R,
    ) -> Result<NetworkMessage, ProtocolError> {
        use byteorder::{LittleEndian, ReadBytesExt};

        trace!("Reading message header");

        // Read header
        let version = reader.read_u16::<LittleEndian>()?;
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::InvalidVersion {
                expected: PROTOCOL_VERSION,
                received: version,
            });
        }

        let msg_type_raw = reader.read_u8()?;
        let msg_type = MessageType::from_u8(msg_type_raw)
            .ok_or(ProtocolError::InvalidMessageType(msg_type_raw))?;

        let payload_size = reader.read_u32::<LittleEndian>()? as usize;

        debug!(
            "Message header: version={}, type={:?}, size={}",
            version, msg_type, payload_size
        );

        if payload_size > self.max_message_size {
            return Err(ProtocolError::MessageTooLarge {
                size: payload_size,
                max_size: self.max_message_size,
            });
        }

        // Read payload
        let mut payload = vec![0u8; payload_size];
        reader.read_exact(&mut payload)?;

        trace!("Message read successfully");

        Ok(NetworkMessage {
            version,
            msg_type,
            payload,
        })
    }

    /// Send a heartbeat message
    pub fn create_heartbeat(&self) -> NetworkMessage {
        NetworkMessage::new(MessageType::Heartbeat, Vec::new())
    }

    /// Create an end-of-stream message
    pub fn create_end_of_stream(&self) -> NetworkMessage {
        NetworkMessage::new(MessageType::EndOfStream, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_message_serialization() {
        let protocol = Protocol::default();
        let mesh = MeshFrame::new("test-sim".to_string(), 42);

        let message = protocol.serialize_mesh(&mesh).unwrap();
        assert_eq!(message.version, PROTOCOL_VERSION);
        assert_eq!(message.msg_type, MessageType::MeshFrame);

        // Test round-trip
        let deserialized = protocol.deserialize_mesh(&message.payload).unwrap();
        assert_eq!(deserialized.simulation_id, mesh.simulation_id);
        assert_eq!(deserialized.frame_number, mesh.frame_number);
    }

    #[test]
    fn test_message_write_read() {
        let protocol = Protocol::default();
        let message = protocol.create_heartbeat();

        let mut buffer = Vec::new();
        protocol.write_message(&mut buffer, &message).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_message = protocol.read_message(&mut cursor).unwrap();

        assert_eq!(read_message.version, message.version);
        assert_eq!(read_message.msg_type, message.msg_type);
        assert_eq!(read_message.payload, message.payload);
    }

    #[test]
    fn test_message_too_large() {
        let protocol = Protocol::default().with_max_message_size(100);
        let mut mesh = MeshFrame::new("test".to_string(), 0);
        mesh.vertices = vec![0.0; 1000]; // Large mesh

        let result = protocol.serialize_mesh(&mesh);
        assert!(matches!(result, Err(ProtocolError::MessageTooLarge { .. })));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_format() {
        let protocol = Protocol::new(WireFormat::Json);
        let mesh = MeshFrame::new("test-json".to_string(), 1);

        let message = protocol.serialize_mesh(&mesh).unwrap();
        let deserialized = protocol.deserialize_mesh(&message.payload).unwrap();

        assert_eq!(deserialized.simulation_id, mesh.simulation_id);
    }
}
