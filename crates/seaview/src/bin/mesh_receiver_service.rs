use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::Parser;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Network service that receives triangle mesh data and saves as GLB files"
)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "9876")]
    port: u16,

    /// Output directory for GLB files
    #[arg(short, long, default_value = "./output")]
    output_dir: PathBuf,

    /// Maximum message size in MB
    #[arg(short, long, default_value = "100")]
    max_size_mb: usize,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Number of worker threads
    #[arg(short = 'j', long, default_value = "4")]
    threads: usize,
}

/// Protocol message header
#[derive(Debug, Clone)]
struct MessageHeader {
    /// Protocol version
    version: u16,
    /// Message type (1 = mesh data)
    message_type: u16,
    /// Total message size in bytes (excluding header)
    message_size: u32,
    /// Human-readable UUID for the simulation run
    simulation_uuid: String,
    /// Frame number
    frame_number: u32,
}

impl MessageHeader {
    const HEADER_SIZE: usize = 8 + 36 + 4; // version + type + size + uuid + frame

    fn read_from(stream: &mut TcpStream) -> std::io::Result<Self> {
        let version = stream.read_u16::<LittleEndian>()?;
        if version != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported protocol version: {}", version),
            ));
        }

        let message_type = stream.read_u16::<LittleEndian>()?;
        let message_size = stream.read_u32::<LittleEndian>()?;

        // Read UUID (36 bytes fixed)
        let mut uuid_bytes = [0u8; 36];
        stream.read_exact(&mut uuid_bytes)?;
        let simulation_uuid = String::from_utf8_lossy(&uuid_bytes)
            .trim_end_matches('\0')
            .to_string();

        let frame_number = stream.read_u32::<LittleEndian>()?;

        Ok(MessageHeader {
            version,
            message_type,
            message_size,
            simulation_uuid,
            frame_number,
        })
    }
}

/// Triangle mesh data
#[derive(Debug)]
struct MeshData {
    /// Number of triangles
    triangle_count: u32,
    /// Triangle vertices (flat array of x,y,z coordinates)
    vertices: Vec<f32>,
}

impl MeshData {
    fn read_from(stream: &mut TcpStream, message_size: u32) -> std::io::Result<Self> {
        // Read triangle count
        let triangle_count = stream.read_u32::<LittleEndian>()?;

        // Validate size
        let expected_size = 4 + (triangle_count * 9 * 4); // 4 bytes for count + 9 floats per triangle
        if expected_size != message_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid message size. Expected {} bytes, got {}",
                    expected_size, message_size
                ),
            ));
        }

        // Read vertices (3 vertices per triangle, 3 floats per vertex)
        let vertex_count = (triangle_count * 9) as usize;
        let mut vertices = Vec::with_capacity(vertex_count);

        for _ in 0..vertex_count {
            vertices.push(stream.read_f32::<LittleEndian>()?);
        }

        Ok(MeshData {
            triangle_count,
            vertices,
        })
    }

    fn to_glb(&self, output_path: &Path) -> std::io::Result<()> {
        // Calculate bounds
        let mut min_pos = [f32::INFINITY; 3];
        let mut max_pos = [f32::NEG_INFINITY; 3];

        // Build vertex and index buffers
        let mut vertex_data = Vec::new();
        let mut index_data = Vec::new();
        let mut unique_vertices = std::collections::HashMap::new();
        let mut vertex_index = 0u32;

        // Process triangles
        for tri_idx in 0..self.triangle_count as usize {
            let base_idx = tri_idx * 9;

            // Calculate triangle normal
            let v0 = [
                self.vertices[base_idx],
                self.vertices[base_idx + 1],
                self.vertices[base_idx + 2],
            ];
            let v1 = [
                self.vertices[base_idx + 3],
                self.vertices[base_idx + 4],
                self.vertices[base_idx + 5],
            ];
            let v2 = [
                self.vertices[base_idx + 6],
                self.vertices[base_idx + 7],
                self.vertices[base_idx + 8],
            ];

            // Calculate normal
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let mut normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];

            let len =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 0.0 {
                normal[0] /= len;
                normal[1] /= len;
                normal[2] /= len;
            }

            // Process each vertex of the triangle
            for i in 0..3 {
                let vert_idx = base_idx + i * 3;
                let pos = [
                    self.vertices[vert_idx],
                    self.vertices[vert_idx + 1],
                    self.vertices[vert_idx + 2],
                ];

                // Update bounds
                for j in 0..3 {
                    min_pos[j] = min_pos[j].min(pos[j]);
                    max_pos[j] = max_pos[j].max(pos[j]);
                }

                // Create vertex key for deduplication
                let key = (
                    pos[0].to_bits(),
                    pos[1].to_bits(),
                    pos[2].to_bits(),
                    normal[0].to_bits(),
                    normal[1].to_bits(),
                    normal[2].to_bits(),
                );

                let idx = if let Some(&existing_idx) = unique_vertices.get(&key) {
                    existing_idx
                } else {
                    let idx = vertex_index;
                    vertex_index += 1;

                    // Write vertex data (position + normal)
                    vertex_data.write_f32::<LittleEndian>(pos[0])?;
                    vertex_data.write_f32::<LittleEndian>(pos[1])?;
                    vertex_data.write_f32::<LittleEndian>(pos[2])?;
                    vertex_data.write_f32::<LittleEndian>(normal[0])?;
                    vertex_data.write_f32::<LittleEndian>(normal[1])?;
                    vertex_data.write_f32::<LittleEndian>(normal[2])?;

                    unique_vertices.insert(key, idx);
                    idx
                };

                index_data.write_u32::<LittleEndian>(idx)?;
            }
        }

        let unique_vertex_count = vertex_index as usize;

        // Align buffers to 4 bytes
        while vertex_data.len() % 4 != 0 {
            vertex_data.push(0);
        }
        while index_data.len() % 4 != 0 {
            index_data.push(0);
        }

        // Create glTF JSON
        let json = format!(
            r#"{{
  "asset": {{
    "generator": "mesh_receiver_service",
    "version": "2.0"
  }},
  "scene": 0,
  "scenes": [
    {{
      "nodes": [0]
    }}
  ],
  "nodes": [
    {{
      "mesh": 0
    }}
  ],
  "meshes": [
    {{
      "primitives": [
        {{
          "attributes": {{
            "POSITION": 0,
            "NORMAL": 1
          }},
          "indices": 2,
          "material": 0,
          "mode": 4
        }}
      ]
    }}
  ],
  "materials": [
    {{
      "pbrMetallicRoughness": {{
        "baseColorFactor": [0.8, 0.8, 0.8, 1.0],
        "metallicFactor": 0.1,
        "roughnessFactor": 0.8
      }},
      "doubleSided": false
    }}
  ],
  "accessors": [
    {{
      "bufferView": 0,
      "byteOffset": 0,
      "componentType": 5126,
      "count": {},
      "type": "VEC3",
      "min": [{}, {}, {}],
      "max": [{}, {}, {}]
    }},
    {{
      "bufferView": 0,
      "byteOffset": 12,
      "componentType": 5126,
      "count": {},
      "type": "VEC3"
    }},
    {{
      "bufferView": 1,
      "byteOffset": 0,
      "componentType": 5125,
      "count": {},
      "type": "SCALAR"
    }}
  ],
  "bufferViews": [
    {{
      "buffer": 0,
      "byteOffset": 0,
      "byteLength": {},
      "byteStride": 24,
      "target": 34962
    }},
    {{
      "buffer": 0,
      "byteOffset": {},
      "byteLength": {},
      "target": 34963
    }}
  ],
  "buffers": [
    {{
      "byteLength": {}
    }}
  ]
}}"#,
            unique_vertex_count,
            min_pos[0],
            min_pos[1],
            min_pos[2],
            max_pos[0],
            max_pos[1],
            max_pos[2],
            unique_vertex_count,
            self.triangle_count * 3,
            vertex_data.len(),
            vertex_data.len(),
            index_data.len(),
            vertex_data.len() + index_data.len(),
        );

        // Combine buffers
        let mut combined_buffer = vertex_data;
        combined_buffer.extend_from_slice(&index_data);

        // Write GLB file
        let mut writer = fs::File::create(output_path)?;

        let json_bytes = json.as_bytes();
        let json_padding = (4 - json_bytes.len() % 4) % 4;
        let json_length = json_bytes.len() + json_padding;

        // GLB header
        writer.write_all(b"glTF")?;
        writer.write_u32::<LittleEndian>(2)?;
        writer.write_u32::<LittleEndian>(
            12 + 8 + json_length as u32 + 8 + combined_buffer.len() as u32,
        )?;

        // JSON chunk
        writer.write_u32::<LittleEndian>(json_length as u32)?;
        writer.write_all(b"JSON")?;
        writer.write_all(json_bytes)?;
        writer.write_all(&vec![0x20; json_padding])?;

        // Binary chunk
        writer.write_u32::<LittleEndian>(combined_buffer.len() as u32)?;
        writer.write_all(b"BIN\0")?;
        writer.write_all(&combined_buffer)?;

        Ok(())
    }
}

fn handle_client(mut stream: TcpStream, args: Arc<Args>) -> std::io::Result<()> {
    let peer_addr = stream.peer_addr()?;
    let start_time = Instant::now();

    if args.verbose {
        println!("Connection from: {}", peer_addr);
    }

    // Read header
    let header = MessageHeader::read_from(&mut stream)?;

    if args.verbose {
        println!("Received header: {:?}", header);
    }

    // Validate message size
    let max_size = args.max_size_mb * 1024 * 1024;
    if header.message_size as usize > max_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Message size {} exceeds maximum {} bytes",
                header.message_size, max_size
            ),
        ));
    }

    // Only handle mesh data messages
    if header.message_type != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unknown message type: {}", header.message_type),
        ));
    }

    // Read mesh data
    let mesh_data = MeshData::read_from(&mut stream, header.message_size)?;

    if args.verbose {
        println!(
            "Received mesh data: {} triangles, {} vertices",
            mesh_data.triangle_count,
            mesh_data.vertices.len()
        );
    }

    // Create output directory for simulation if needed
    let sim_dir = args.output_dir.join(&header.simulation_uuid);
    fs::create_dir_all(&sim_dir)?;

    // Generate output filename
    let output_path = sim_dir.join(format!("simulation_{:06}.glb", header.frame_number));

    // Convert to GLB
    mesh_data.to_glb(&output_path)?;

    // Send acknowledgment
    stream.write_u8(1)?; // Success
    stream.flush()?;

    let duration = start_time.elapsed();
    println!(
        "Saved frame {} for simulation {} ({} triangles, {:.2}s)",
        header.frame_number,
        header.simulation_uuid,
        mesh_data.triangle_count,
        duration.as_secs_f64()
    );

    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Arc::new(Args::parse());

    // Create output directory
    fs::create_dir_all(&args.output_dir)?;

    // Create thread pool
    let pool = threadpool::ThreadPool::new(args.threads);

    // Start listening
    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.port))?;
    println!("Mesh receiver service listening on port {}", args.port);
    println!("Output directory: {:?}", args.output_dir);
    println!("Protocol: version 1");
    println!("Message format:");
    println!("  - Header: version(u16) + type(u16) + size(u32) + uuid(36 bytes) + frame(u32)");
    println!("  - Body: triangle_count(u32) + vertices(f32 array, 9 per triangle)");

    // Accept connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let args_clone = args.clone();
                pool.execute(move || {
                    if let Err(e) = handle_client(stream, args_clone) {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}
