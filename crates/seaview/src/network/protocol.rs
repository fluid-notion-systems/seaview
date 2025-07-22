//! Protocol definitions for mesh data transfer

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

pub const PROTOCOL_VERSION: u16 = 1;

/// Protocol message types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageType {
    MeshData = 1,
}

/// Protocol message header
#[derive(Debug, Clone)]
pub struct MessageHeader {
    /// Protocol version
    pub version: u16,
    /// Message type
    pub message_type: MessageType,
    /// Total message size in bytes (excluding header)
    pub message_size: u32,
    /// Human-readable UUID for the simulation run
    pub simulation_uuid: String,
    /// Frame number
    pub frame_number: u32,
}

impl MessageHeader {
    pub const HEADER_SIZE: usize = 8 + 36 + 4; // version + type + size + uuid + frame

    pub fn new(
        message_type: MessageType,
        message_size: u32,
        simulation_uuid: String,
        frame_number: u32,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type,
            message_size,
            simulation_uuid,
            frame_number,
        }
    }

    pub fn read_from<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let version = reader.read_u16::<LittleEndian>()?;
        if version != PROTOCOL_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported protocol version: {version}"),
            ));
        }

        let message_type_raw = reader.read_u16::<LittleEndian>()?;
        let message_type = match message_type_raw {
            1 => MessageType::MeshData,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown message type: {message_type_raw}"),
                ))
            }
        };

        let message_size = reader.read_u32::<LittleEndian>()?;

        // Read UUID (36 bytes fixed)
        let mut uuid_bytes = [0u8; 36];
        reader.read_exact(&mut uuid_bytes)?;
        let simulation_uuid = String::from_utf8_lossy(&uuid_bytes)
            .trim_end_matches('\0')
            .to_string();

        let frame_number = reader.read_u32::<LittleEndian>()?;

        Ok(MessageHeader {
            version,
            message_type,
            message_size,
            simulation_uuid,
            frame_number,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_u16::<LittleEndian>(self.version)?;
        writer.write_u16::<LittleEndian>(self.message_type as u16)?;
        writer.write_u32::<LittleEndian>(self.message_size)?;

        // Write UUID (padded to 36 bytes)
        let mut uuid_bytes = [0u8; 36];
        let uuid_str = self.simulation_uuid.as_bytes();
        let copy_len = uuid_str.len().min(36);
        uuid_bytes[..copy_len].copy_from_slice(&uuid_str[..copy_len]);
        writer.write_all(&uuid_bytes)?;

        writer.write_u32::<LittleEndian>(self.frame_number)?;
        writer.flush()?;

        Ok(())
    }
}

/// Triangle mesh data
#[derive(Debug)]
pub struct MeshData {
    /// Number of triangles
    pub triangle_count: u32,
    /// Triangle vertices (flat array of x,y,z coordinates)
    pub vertices: Vec<f32>,
}

impl MeshData {
    pub fn new(triangle_count: u32, vertices: Vec<f32>) -> Self {
        assert_eq!(vertices.len(), (triangle_count * 9) as usize);
        Self {
            triangle_count,
            vertices,
        }
    }

    pub fn read_from<R: Read>(reader: &mut R, message_size: u32) -> std::io::Result<Self> {
        // Read triangle count
        let triangle_count = reader.read_u32::<LittleEndian>()?;

        // Validate size
        let expected_size = 4 + (triangle_count * 9 * 4); // 4 bytes for count + 9 floats per triangle
        if expected_size != message_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid message size. Expected {expected_size} bytes, got {message_size}"
                ),
            ));
        }

        // Read vertices (3 vertices per triangle, 3 floats per vertex)
        let vertex_count = (triangle_count * 9) as usize;
        let mut vertices = Vec::with_capacity(vertex_count);

        for _ in 0..vertex_count {
            vertices.push(reader.read_f32::<LittleEndian>()?);
        }

        Ok(MeshData {
            triangle_count,
            vertices,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_u32::<LittleEndian>(self.triangle_count)?;

        for &v in &self.vertices {
            writer.write_f32::<LittleEndian>(v)?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn message_size(&self) -> u32 {
        4 + (self.triangle_count * 9 * 4)
    }
}
