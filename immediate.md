# Immediate Task: Extract seaview-network Crate

## Overview

Extract the network receiving and sending code from seaview into a standalone `seaview-network` crate that can be used by both Rust and C/C++ applications. This will enable FluidX3d (g++ based) to directly send meshes to seaview without intermediate file I/O.

## Crate Structure

```
seaview/
├── Cargo.toml (workspace)
├── seaview/           # Main application
│   ├── Cargo.toml
│   └── src/
└── seaview-network/   # Network library
    ├── Cargo.toml
    ├── build.rs       # C header generation
    ├── src/
    │   ├── lib.rs
    │   ├── protocol.rs
    │   ├── sender.rs
    │   ├── receiver.rs
    │   ├── ffi.rs     # C/C++ bindings
    │   └── types.rs   # Shared types with serde
    ├── include/       # Generated C headers
    │   └── seaview_network.h
    └── examples/
        ├── send_mesh.c
        └── receive_mesh.rs
```

## Core Features

### 1. Protocol Definition

```rust
// src/types.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshFrame {
    pub simulation_id: String,      // human-hash ID
    pub frame_number: u32,
    pub timestamp: u64,
    pub domain_bounds: DomainBounds,
    pub vertices: Vec<f32>,         // Flattened x,y,z
    pub normals: Option<Vec<f32>>,  // Optional normals
    pub indices: Option<Vec<u32>>,  // Optional for indexed mesh
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshMetadata {
    pub vertex_count: usize,
    pub face_count: usize,
    pub has_normals: bool,
    pub is_indexed: bool,
}
```

### 2. Sender Implementation (C++ Compatible)

```rust
// src/ffi.rs
use std::ffi::{c_char, CStr};
use std::os::raw::c_float;

#[repr(C)]
pub struct CMeshFrame {
    pub simulation_id: *const c_char,
    pub frame_number: u32,
    pub vertex_count: usize,
    pub vertices: *const c_float,
    pub normals: *const c_float,  // NULL if not present
    pub index_count: usize,
    pub indices: *const u32,       // NULL if polygon soup
}

#[no_mangle]
pub extern "C" fn seaview_network_create_sender(
    host: *const c_char,
    port: u16
) -> *mut NetworkSender {
    // Implementation
}

#[no_mangle]
pub extern "C" fn seaview_network_send_mesh(
    sender: *mut NetworkSender,
    mesh: *const CMeshFrame
) -> i32 {
    // Convert C struct to Rust, serialize, and send
}

#[no_mangle]
pub extern "C" fn seaview_network_destroy_sender(
    sender: *mut NetworkSender
) {
    // Clean up
}
```

### 3. Build Configuration for C++ Linking

```toml
# seaview-network/Cargo.toml
[package]
name = "seaview-network"
version = "0.1.0"
edition = "2021"

[lib]
name = "seaview_network"
crate-type = ["cdylib", "rlib", "staticlib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"        # For binary serialization
thiserror = "1.0"

[build-dependencies]
cbindgen = "0.26"      # Generate C headers
```

## Networking Architecture

### Simple TCP Approach

Using Rust's standard library TCP implementation for simplicity and easy C FFI integration:

**Benefits:**
- Simple, synchronous API
- No runtime dependencies
- Direct C FFI integration
- Predictable behavior
- Easy to debug

**Implementation:**
```rust
use std::net::TcpStream;
use std::io::Write;

pub struct NetworkSender {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl NetworkSender {
    pub fn connect(addr: &str) -> Result<Self, NetworkError> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?; // Low latency
        Ok(Self {
            stream,
            buffer: Vec::with_capacity(1024 * 1024), // 1MB buffer
        })
    }

    pub fn send_mesh(&mut self, mesh: &MeshFrame) -> Result<(), NetworkError> {
        // Serialize with length prefix
        let data = bincode::serialize(mesh)?;
        let len = data.len() as u32;

        self.stream.write_all(&len.to_le_bytes())?;
        self.stream.write_all(&data)?;
        self.stream.flush()?;

        Ok(())
    }
}
```

For the receiver side, we'll use a dedicated thread to avoid blocking the main Bevy app:
```rust
pub fn start_receiver(addr: &str, tx: Sender<MeshFrame>) -> Result<(), NetworkError> {
    let listener = TcpListener::bind(addr)?;

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                // Handle connection in separate thread
                let tx = tx.clone();
                std::thread::spawn(move || {
                    handle_connection(stream, tx);
                });
            }
        }
    });

    Ok(())
}
```

## Serialization Strategy

### Wire Format Options

Nick: Lets go with bincode, and json, configurable

1. **Bincode** (Recommended for performance)
   - Binary format, very fast
   - Compact size
   - Schema evolution challenges

2. **MessagePack** (Balance)
   - Binary with some self-description
   - Good performance
   - Better compatibility

3. **JSON** (Debug/compatibility)
   - Human readable
   - Larger size
   - Universal support

### Implementation

```rust
// src/protocol.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    MeshFrame = 0x01,
    Metadata = 0x02,
    Checkpoint = 0x03,
    EndOfStream = 0x04,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,  // Serialized content
}

pub fn serialize_mesh(mesh: &MeshFrame) -> Result<Vec<u8>, SerializeError> {
    let payload = bincode::serialize(mesh)?;
    let msg = NetworkMessage {
        msg_type: MessageType::MeshFrame,
        payload,
    };
    bincode::serialize(&msg)
}
```

## Integration with FluidX3d

### C++ Usage Example

```cpp
// FluidX3d integration
#include "seaview_network.h"

class SeaviewMeshSender {
private:
    void* sender;

public:
    SeaviewMeshSender(const char* host, uint16_t port) {
        sender = seaview_network_create_sender(host, port);
    }

    ~SeaviewMeshSender() {
        if (sender) {
            seaview_network_destroy_sender(sender);
        }
    }

    void sendMesh(const std::string& simId,
                  uint32_t frame,
                  const std::vector<float>& vertices,
                  const std::vector<float>& normals) {
        CMeshFrame mesh = {
            .simulation_id = simId.c_str(),
            .frame_number = frame,
            .vertex_count = vertices.size() / 3,
            .vertices = vertices.data(),
            .normals = normals.empty() ? nullptr : normals.data(),
            .index_count = 0,
            .indices = nullptr
        };

        int result = seaview_network_send_mesh(sender, &mesh);
        if (result != 0) {
            // Handle error
        }
    }
};
```

### Build Instructions for FluidX3d

```bash
# Build seaview-network
cd seaview/seaview-network
cargo build --release --features blocking

# Link with FluidX3d (g++)
g++ -o fluidx3d main.cpp \
    -L./target/release -lseaview_network \
    -lpthread -ldl -lm \
    -std=c++17
```

## Migration Tasks

IMPORTANT IMPORTANT IMPORTANT: Please git commit after each STEP, within each phase. VERY IMPORTANT. AGAIN. VERY IMPORTANT.
IMPORTANT IMPORTANT IMPORTANT: Please cargo check, and cargo clippy within each step, and fix errors / warnings. VERY IMPORTANT. AGAIN. VERY IMPORTANT.
also, use thiserror, and tracing for verbose logging, (using info, debug)

### Phase 1: Extract Core Code (Days 1-2)
- [ ] Create seaview-network crate structure
- [ ] Move protocol definitions with serde
- [ ] Implement simple TCP sender
- [ ] Implement threaded TCP receiver
- [ ] Create basic tests

### Phase 2: Add FFI Layer (Days 3-4)
- [ ] Design minimal C API surface
- [ ] Implement FFI functions
- [ ] Generate C headers with cbindgen
- [ ] Create C++ example
- [ ] Test with mock data

### Phase 3: Integrate with FluidX3d (Days 5-7)
- [ ] Build static library
- [ ] Create FluidX3d wrapper class
- [ ] Modify marching cubes output
- [ ] Test end-to-end pipeline
- [ ] Fix any integration issues

### Phase 4: Polish & Document (Week 2)
- [ ] Add connection retry logic
- [ ] Implement basic error recovery
- [ ] Profile performance
- [ ] Write documentation
- [ ] Create usage examples

## Performance Considerations

### Memory Management
- Use arena allocators for C FFI
- Zero-copy where possible
- Careful with string lifetimes
- Pre-allocate buffers

### Network Optimization
- TCP_NODELAY for low latency
- Larger send/receive buffers
- Optional compression
- Frame batching support

### Error Handling
- Graceful degradation
- Automatic reconnection
- Buffering during disconnects
- Clear error codes for C API

## Testing Strategy

### Unit Tests
- Protocol serialization/deserialization
- FFI boundary safety
- Network error handling
- Memory leak detection

### Integration Tests
- C++ example compilation
- End-to-end data flow
- Performance benchmarks
- Stress testing

### Validation
- Compare with file-based pipeline
- Verify mesh integrity
- Check frame ordering
- Memory usage monitoring

## Success Criteria

1. **Functionality**
   - Meshes transfer correctly
   - No data corruption
   - Frame ordering preserved
   - Graceful error handling

2. **Performance**
   - <10ms latency per frame
   - >100MB/s throughput
   - <100MB memory overhead
   - No memory leaks

3. **Usability**
   - Simple C++ integration
   - Clear documentation
   - Example code works
   - Error messages helpful

## Next Steps

1. Create crate structure in seaview workspace
2. Start with Rust-only implementation
3. Add FFI layer incrementally
4. Test with mock C++ application
5. Integrate with FluidX3d

---

*"Building bridges between GPU simulation and real-time visualization"*
