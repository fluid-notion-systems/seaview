//! Network receiver for mesh data

use super::protocol::{MeshData, MessageHeader};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Received mesh data with metadata
#[derive(Debug, Clone)]
pub struct ReceivedMesh {
    /// Simulation UUID
    pub simulation_uuid: String,
    /// Frame number
    pub frame_number: u32,
    /// Number of triangles
    pub triangle_count: u32,
    /// Flat array of triangle vertices (x,y,z coordinates)
    pub vertices: Vec<f32>,
}

impl From<(MessageHeader, MeshData)> for ReceivedMesh {
    fn from((header, data): (MessageHeader, MeshData)) -> Self {
        Self {
            simulation_uuid: header.simulation_uuid,
            frame_number: header.frame_number,
            triangle_count: data.triangle_count,
            vertices: data.vertices,
        }
    }
}

impl From<ReceivedMesh> for baby_shark::mesh::Mesh<f32> {
    fn from(received_mesh: ReceivedMesh) -> Self {
        baby_shark::mesh::Mesh::from_iter(received_mesh.vertices.into_iter())
    }
}

impl From<&ReceivedMesh> for baby_shark::mesh::Mesh<f32> {
    fn from(received_mesh: &ReceivedMesh) -> Self {
        //FIXME: I dont like this call to copied(), we'll look into zero-copy later
        baby_shark::mesh::Mesh::from_iter(received_mesh.vertices.iter().copied())
    }
}

/// Mesh receiver that listens for incoming mesh data
pub struct MeshReceiver {
    listener: TcpListener,
    max_message_size: usize,
}

impl MeshReceiver {
    /// Create a new mesh receiver listening on the specified port
    pub fn new(port: u16, max_message_size_mb: usize) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        listener.set_nonblocking(false)?;

        Ok(Self {
            listener,
            max_message_size: max_message_size_mb * 1024 * 1024,
        })
    }

    /// Get the local address the receiver is bound to
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    /// Accept a single connection and receive mesh data
    pub fn receive_one(&mut self) -> std::io::Result<ReceivedMesh> {
        let (mut stream, _addr) = self.listener.accept()?;
        self.handle_connection(&mut stream)
    }

    /// Run the receiver with a callback for each received mesh
    pub fn run<F>(&mut self, mut callback: F) -> std::io::Result<()>
    where
        F: FnMut(ReceivedMesh) -> bool,
    {
        loop {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => match self.handle_connection(&mut stream) {
                    Ok(mesh) => {
                        if !callback(mesh) {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error handling connection: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Start receiver in a background thread with a channel
    pub fn run_async(mut self) -> (mpsc::Receiver<ReceivedMesh>, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            self.run(|mesh| tx.send(mesh).is_ok()).ok();
        });

        (rx, handle)
    }

    /// Handle a single connection
    fn handle_connection(&self, stream: &mut TcpStream) -> std::io::Result<ReceivedMesh> {
        // Set timeout for read operations
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;

        // Read header
        let header = MessageHeader::read_from(stream)?;

        // Validate message size
        if header.message_size as usize > self.max_message_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Message size {} exceeds maximum {} bytes",
                    header.message_size, self.max_message_size
                ),
            ));
        }

        // Read mesh data
        let mesh_data = MeshData::read_from(stream, header.message_size)?;

        // Send acknowledgment
        let _ = stream.write_all(&[1u8]);

        Ok(ReceivedMesh::from((header, mesh_data)))
    }
}

/// Non-blocking mesh receiver that can be polled
pub struct NonBlockingMeshReceiver {
    listener: TcpListener,
    max_message_size: usize,
}

impl NonBlockingMeshReceiver {
    /// Create a new non-blocking mesh receiver
    pub fn new(port: u16, max_message_size_mb: usize) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            listener,
            max_message_size: max_message_size_mb * 1024 * 1024,
        })
    }

    /// Try to receive a mesh without blocking
    pub fn try_receive(&mut self) -> std::io::Result<Option<ReceivedMesh>> {
        match self.listener.accept() {
            Ok((mut stream, _addr)) => {
                stream.set_nonblocking(false)?;
                match self.handle_connection(&mut stream) {
                    Ok(mesh) => Ok(Some(mesh)),
                    Err(e) => Err(e),
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Handle a single connection
    fn handle_connection(&self, stream: &mut TcpStream) -> std::io::Result<ReceivedMesh> {
        // Set timeout for read operations
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;

        // Read header
        let header = MessageHeader::read_from(stream)?;

        // Validate message size
        if header.message_size as usize > self.max_message_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Message size {} exceeds maximum {} bytes",
                    header.message_size, self.max_message_size
                ),
            ));
        }

        // Read mesh data
        let mesh_data = MeshData::read_from(stream, header.message_size)?;

        // Send acknowledgment
        let _ = stream.write_all(&[1u8]);

        Ok(ReceivedMesh::from((header, mesh_data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_received_mesh_conversion() {
        let header = MessageHeader::new(
            super::super::protocol::MessageType::MeshData,
            40,
            "test-uuid".to_string(),
            0,
        );

        let data = MeshData::new(1, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);

        let received = ReceivedMesh::from((header, data));
        assert_eq!(received.simulation_uuid, "test-uuid");
        assert_eq!(received.frame_number, 0);
        assert_eq!(received.triangle_count, 1);
        assert_eq!(received.vertices.len(), 9);
    }
}
