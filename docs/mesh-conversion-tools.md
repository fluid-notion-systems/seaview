# Mesh Conversion and Network Tools

This document describes the mesh format conversion tools and network services available in Seaview for optimizing and streaming 3D mesh data.

## Overview

Seaview provides several tools for working with 3D mesh data:

1. **STL to glTF/GLB Converter** - Convert STL files to the more efficient glTF format
2. **Mesh Receiver Service** - Network service that accepts triangle mesh data and saves as GLB files
3. **Mesh Optimization Tools** - Utilities for optimizing mesh data for better performance

## STL to glTF/GLB Converter

The `stl_to_gltf` tool converts STL files to glTF or GLB format, which offers better compression and faster loading times.

### Usage

```bash
# Convert a single STL file to GLB
stl_to_gltf input.stl

# Convert with custom output
stl_to_gltf input.stl -o output.glb

# Convert entire directory in parallel
stl_to_gltf /path/to/stl/directory -o /path/to/output/directory

# Convert to text glTF format (with separate .bin file)
stl_to_gltf input.stl -f gltf

# Custom material properties
stl_to_gltf input.stl --base-color 0.2,0.5,0.8 --metallic 0.3 --roughness 0.7
```

### Options

- `-o, --output` - Output file or directory (defaults to input with .glb extension)
- `-f, --format` - Output format: `glb` (default) or `gltf`
- `-p, --parallel` - Process directory files in parallel (default: true)
- `-j, --threads` - Number of threads to use (0 = all available)
- `--pattern` - File pattern to match when processing directories (default: "*.stl")
- `--base-color` - Material base color as R,G,B values 0.0-1.0 (default: 0.8,0.8,0.8)
- `--metallic` - Material metallic value 0.0-1.0 (default: 0.1)
- `--roughness` - Material roughness value 0.0-1.0 (default: 0.8)
- `-v, --verbose` - Verbose output

### Benefits of glTF/GLB

- **Smaller file sizes** - Binary format with efficient data packing
- **Faster loading** - Indexed geometry reduces redundant vertex data
- **Better materials** - PBR material support built-in
- **Industry standard** - Widely supported across tools and engines
- **GPU-ready** - Data layout optimized for GPU upload

## Mesh Receiver Service

The `mesh_receiver_service` is a network service that accepts triangle mesh data over TCP and saves it as GLB files organized by simulation UUID and frame number.

### Starting the Service

```bash
# Start with default settings (port 9876)
mesh_receiver_service

# Custom port and output directory
mesh_receiver_service -p 8080 -o /data/simulations

# Verbose logging
mesh_receiver_service -v

# Set maximum message size (in MB)
mesh_receiver_service --max-size-mb 500
```

### Options

- `-p, --port` - Port to listen on (default: 9876)
- `-o, --output-dir` - Output directory for GLB files (default: ./output)
- `-m, --max-size-mb` - Maximum message size in MB (default: 100)
- `-j, --threads` - Number of worker threads (default: 4)
- `-v, --verbose` - Verbose logging

### Network Protocol

The service uses a simple binary protocol:

#### Message Header (48 bytes)
```
- version      (u16)  - Protocol version (must be 1)
- message_type (u16)  - Message type (1 = mesh data)
- message_size (u32)  - Size of message body in bytes
- uuid         (36B)  - Simulation UUID (ASCII, null-padded)
- frame_number (u32)  - Frame number
```

#### Message Body (for mesh data)
```
- triangle_count (u32)     - Number of triangles
- vertices       (f32[])   - Triangle vertices (9 floats per triangle)
```

All multi-byte values are little-endian.

#### Response
The server sends a single byte response:
- `0x01` - Success
- `0x00` - Failure

### Output Format

Files are saved as:
```
{output_dir}/{simulation_uuid}/simulation_{frame_number:06}.glb
```

Example:
```
./output/my-sim-12345678/simulation_000042.glb
```

## Testing the Network Service

Use the `mesh_sender_test` tool to test the mesh receiver service:

```bash
# Send a single frame
mesh_sender_test

# Send animated sequence
mesh_sender_test -n 100 -a -d 33 -u my-simulation-001

# Custom server and parameters
mesh_sender_test -s 192.168.1.100 -p 8080 -t 1000 -f 0 -n 10
```

### Test Client Options

- `-s, --server` - Server address (default: 127.0.0.1)
- `-p, --port` - Server port (default: 9876)
- `-u, --uuid` - Simulation UUID (max 36 characters)
- `-f, --start-frame` - Starting frame number (default: 0)
- `-n, --num-frames` - Number of frames to send (default: 1)
- `-t, --triangles` - Number of triangles per frame (default: 100)
- `-d, --delay-ms` - Delay between frames in milliseconds (default: 100)
- `-a, --animate` - Generate animated mesh (rotating cube)
- `-v, --verbose` - Verbose output

## Viewer Support

The Seaview viewer supports loading both STL and glTF/GLB files:

```bash
# View a single glTF file
seaview model.glb

# View a directory of glTF files as a sequence
seaview /path/to/glb/sequence/

# The viewer automatically detects file format based on extension
```

Supported patterns for sequences:
- `simulation_000001.glb`, `simulation_000002.glb`, ...
- `frame_001.glb`, `frame_002.glb`, ...
- `mesh_t0001.glb`, `mesh_t0002.glb`, ...

## Performance Considerations

### File Size Comparison

Typical size reductions when converting from STL to GLB:
- Simple meshes: 30-50% reduction
- Complex meshes with many duplicated vertices: 60-80% reduction
- Already optimized STL files: 10-20% reduction

### Loading Performance

GLB files load significantly faster than STL:
- Binary format requires no text parsing
- Indexed geometry reduces memory bandwidth
- Data layout is GPU-friendly

### Network Streaming

The mesh receiver service is designed for real-time streaming:
- Efficient binary protocol
- Parallel processing with thread pool
- Automatic deduplication of vertices
- Direct GLB generation without intermediate formats

## Integration Example

Here's a Python example for sending mesh data to the receiver service:

```python
import struct
import socket
import numpy as np

def send_mesh(host, port, uuid, frame, triangles):
    """Send triangle mesh data to mesh receiver service"""
    
    # Prepare header
    version = 1
    msg_type = 1
    msg_size = 4 + len(triangles) * 9 * 4
    
    # Pack UUID (36 bytes, padded with nulls)
    uuid_bytes = uuid.encode('ascii')[:36].ljust(36, b'\0')
    
    # Create connection
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))
    
    # Send header
    header = struct.pack('<HHI', version, msg_type, msg_size)
    sock.send(header)
    sock.send(uuid_bytes)
    sock.send(struct.pack('<I', frame))
    
    # Send mesh data
    sock.send(struct.pack('<I', len(triangles)))
    for tri in triangles:
        for vertex in tri:
            sock.send(struct.pack('<fff', *vertex))
    
    # Read response
    response = sock.recv(1)
    sock.close()
    
    return response[0] == 1

# Example usage
triangles = np.array([
    [[0, 0, 0], [1, 0, 0], [0, 1, 0]],  # Triangle 1
    [[1, 0, 0], [1, 1, 0], [0, 1, 0]],  # Triangle 2
])

success = send_mesh('localhost', 9876, 'my-sim-001', 42, triangles)
```

## Future Enhancements

Planned improvements for mesh conversion and streaming:

1. **Compression Support**
   - Draco geometry compression for glTF
   - Streaming compression (zstd, lz4)
   - Quantization for reduced precision

2. **Advanced Optimization**
   - Meshoptimizer integration for better vertex cache optimization
   - Level-of-detail (LOD) generation
   - Mesh simplification

3. **Protocol Extensions**
   - Metadata support (simulation parameters, timestamps)
   - Incremental updates (only changed vertices)
   - Multi-mesh support in single message

4. **Performance**
   - GPU-accelerated mesh processing
   - Memory-mapped file support for large datasets
   - Adaptive streaming based on network conditions