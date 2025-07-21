# Network Mesh Receiving

Seaview supports real-time mesh reception over TCP, allowing you to stream mesh data directly into the viewer from external applications or simulations.

## Overview

The network receiving feature enables Seaview to act as a mesh visualization server, accepting triangle mesh data over a simple TCP protocol. This is useful for:

- Real-time visualization of simulation results
- Remote mesh viewing
- Integration with external mesh generation tools
- Live debugging of mesh generation algorithms

## Usage

### Starting Seaview with Network Support

To enable network mesh receiving, start Seaview with the `--network-port` flag:

```bash
# Start Seaview listening on port 9877 (default)
seaview --network-port 9877

# Start Seaview listening on a custom port
seaview --network-port 12345

# Start Seaview with both file loading and network support
seaview path/to/mesh.stl --network-port 9877
```

### Protocol

Seaview uses a simple binary protocol for mesh transmission:

#### Message Header (48 bytes)
- Protocol version (u16): Currently 1
- Message type (u16): 1 for mesh data
- Message size (u32): Size of the message body in bytes
- Simulation UUID (36 bytes): ASCII string identifier, zero-padded
- Frame number (u32): Sequential frame identifier

#### Message Body
- Triangle count (u32): Number of triangles in the mesh
- Vertices (f32 array): 9 floats per triangle (3 vertices × 3 coordinates)

All multi-byte values use little-endian byte order.

### Sending Mesh Data

You can use the included `mesh_sender_test` tool for testing:

```bash
# Send a single frame
mesh_sender_test -p 9877

# Send 10 frames of an animated rotating cube
mesh_sender_test -p 9877 -n 10 -a

# Send frames with custom delay (milliseconds)
mesh_sender_test -p 9877 -n 5 -a -d 1000

# Send to a remote host
mesh_sender_test -s 192.168.1.100 -p 9877
```

### Integration Example

Here's a simple example of sending mesh data from your own application:

```rust
use std::net::TcpStream;
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};

fn send_mesh(vertices: &[[f32; 3]], triangles: &[[usize; 3]], frame: u32) -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:9877")?;
    
    // Prepare triangle data (flatten to vertex soup)
    let mut vertex_data = Vec::new();
    for tri in triangles {
        for &idx in tri {
            vertex_data.extend_from_slice(&vertices[idx]);
        }
    }
    
    let triangle_count = triangles.len() as u32;
    let message_size = 4 + (triangle_count * 9 * 4); // count + vertices
    
    // Write header
    stream.write_u16::<LittleEndian>(1)?; // version
    stream.write_u16::<LittleEndian>(1)?; // message type
    stream.write_u32::<LittleEndian>(message_size)?;
    
    // Write UUID (36 bytes)
    let uuid = b"my-simulation-12345678-1234-1234-123";
    stream.write_all(uuid)?;
    
    stream.write_u32::<LittleEndian>(frame)?;
    
    // Write mesh data
    stream.write_u32::<LittleEndian>(triangle_count)?;
    for &v in &vertex_data {
        stream.write_f32::<LittleEndian>(v)?;
    }
    
    stream.flush()?;
    
    // Read acknowledgment
    let mut ack = [0u8];
    stream.read_exact(&mut ack)?;
    
    Ok(())
}
```

### Running a Mesh Receiver Service

For saving received meshes to disk as GLB files, use the standalone `mesh_receiver_service`:

```bash
# Start the receiver service
mesh_receiver_service -p 9876 -o ./received_meshes/

# With custom settings
mesh_receiver_service -p 9876 -o ./output -m 200 -j 8 -v
```

This service will:
- Listen for incoming mesh data on the specified port
- Convert each received mesh to an optimized GLB file
- Save files organized by simulation UUID and frame number
- Handle multiple concurrent connections

### Visualization

Network-received meshes appear in Seaview with:
- Blue-tinted material to distinguish them from loaded files
- Automatic bounds calculation and centering
- Real-time updates as new frames arrive
- Support for multiple simultaneous mesh streams

### Performance Considerations

- The network receiver runs in a separate thread to avoid blocking the render loop
- Meshes are processed and optimized using meshopt for better GPU performance
- Large meshes (>100MB) may cause frame drops during reception
- Consider using the `max_message_size_mb` setting to limit memory usage

### Troubleshooting

If meshes aren't appearing:
1. Check that Seaview started with network support enabled (look for "Network mesh receiving enabled" in logs)
2. Verify the port isn't blocked by a firewall
3. Ensure the sender is using the correct protocol version
4. Check Seaview's console output for error messages

Common issues:
- "Address already in use": Another process is using the port
- "Connection refused": Seaview isn't running or network support isn't enabled
- Inverted normals: The mesh winding order might be incorrect