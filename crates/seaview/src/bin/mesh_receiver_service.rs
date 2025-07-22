use clap::Parser;
use meshopt::optimize_vertex_cache;

use std::fs;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// Import from the parent crate
use baby_shark::mesh::Mesh as BabySharkMesh;
use seaview::network::{MeshReceiver, ReceivedMesh};

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

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Number of worker threads
    #[arg(short = 'j', long, default_value = "4")]
    threads: usize,
}

fn convert_to_glb(mesh: &ReceivedMesh, output_path: &Path) -> std::io::Result<()> {
    use byteorder::{LittleEndian, WriteBytesExt};

    // Create baby_shark mesh to handle deduplication
    let baby_shark_mesh: BabySharkMesh<f32> = mesh.into();

    // Build vertex and index buffers
    let mut vertex_buffer = Vec::new();
    let mut index_buffer = Vec::new();
    let mut min_pos = [f32::INFINITY; 3];
    let mut max_pos = [f32::NEG_INFINITY; 3];

    // Process deduplicated vertices
    for vertex in baby_shark_mesh.vertices() {
        // Update bounds
        for i in 0..3 {
            min_pos[i] = min_pos[i].min(vertex[i]);
            max_pos[i] = max_pos[i].max(vertex[i]);
        }
    }

    // Build vertex buffer with normals
    let mut normals = vec![[0.0f32; 3]; baby_shark_mesh.vertex_count()];
    let mut normal_counts = vec![0u32; baby_shark_mesh.vertex_count()];

    // Calculate normals by averaging face normals for each vertex
    for tri_idx in 0..(baby_shark_mesh.index_count() / 3) {
        let i0 = baby_shark_mesh.indices()[tri_idx * 3];
        let i1 = baby_shark_mesh.indices()[tri_idx * 3 + 1];
        let i2 = baby_shark_mesh.indices()[tri_idx * 3 + 2];

        let v0 = baby_shark_mesh.vertices()[i0];
        let v1 = baby_shark_mesh.vertices()[i1];
        let v2 = baby_shark_mesh.vertices()[i2];

        // Calculate face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(&edge2);

        // Only normalize if not degenerate
        let len_sq = face_normal.norm_squared();
        if len_sq > 1e-10 {
            let face_normal = face_normal / len_sq.sqrt();

            // Add to vertex normals
            for &idx in &[i0, i1, i2] {
                normals[idx][0] += face_normal[0];
                normals[idx][1] += face_normal[1];
                normals[idx][2] += face_normal[2];
                normal_counts[idx] += 1;
            }
        }
    }

    // Normalize the averaged normals and build vertex buffer
    for (i, vertex) in baby_shark_mesh.vertices().iter().enumerate() {
        // Position
        vertex_buffer.write_f32::<LittleEndian>(vertex[0])?;
        vertex_buffer.write_f32::<LittleEndian>(vertex[1])?;
        vertex_buffer.write_f32::<LittleEndian>(vertex[2])?;

        // Normal (averaged and normalized)
        let count = normal_counts[i] as f32;
        let normal = if count > 0.0 {
            let nx = normals[i][0] / count;
            let ny = normals[i][1] / count;
            let nz = normals[i][2] / count;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len > 0.0 {
                [nx / len, ny / len, nz / len]
            } else {
                [0.0, 1.0, 0.0]
            }
        } else {
            [0.0, 1.0, 0.0]
        };

        vertex_buffer.write_f32::<LittleEndian>(normal[0])?;
        vertex_buffer.write_f32::<LittleEndian>(normal[1])?;
        vertex_buffer.write_f32::<LittleEndian>(normal[2])?;
    }

    // Build index buffer
    for &idx in baby_shark_mesh.indices() {
        index_buffer.write_u32::<LittleEndian>(idx as u32)?;
    }

    // Optimize vertex cache locality
    let indices_u32: Vec<u32> = baby_shark_mesh
        .indices()
        .iter()
        .map(|&i| i as u32)
        .collect();
    let optimized_indices = optimize_vertex_cache(&indices_u32, baby_shark_mesh.vertex_count());

    // Write optimized indices
    let mut optimized_index_buffer = Vec::new();
    for &idx in &optimized_indices {
        optimized_index_buffer.write_u32::<LittleEndian>(idx)?;
    }

    // Align buffers to 4 bytes
    while vertex_buffer.len() % 4 != 0 {
        vertex_buffer.push(0);
    }
    while optimized_index_buffer.len() % 4 != 0 {
        optimized_index_buffer.push(0);
    }

    let vertex_count = baby_shark_mesh.vertex_count();
    let vertex_buffer_size = vertex_buffer.len();
    let index_buffer_size = optimized_index_buffer.len();

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
          "mode": 4
        }}
      ],
      "name": "mesh"
    }}
  ],
  "buffers": [
    {{
      "byteLength": {}
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
  "accessors": [
    {{
      "bufferView": 0,
      "byteOffset": 0,
      "componentType": 5126,
      "count": {},
      "type": "VEC3",
      "max": [{}, {}, {}],
      "min": [{}, {}, {}]
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
  ]
}}"#,
        vertex_buffer_size + index_buffer_size, // total buffer size
        vertex_buffer_size,                     // vertex buffer size
        vertex_buffer_size,                     // index buffer offset
        index_buffer_size,                      // index buffer size
        vertex_count,                           // position accessor count
        max_pos[0],
        max_pos[1],
        max_pos[2], // max bounds
        min_pos[0],
        min_pos[1],
        min_pos[2],              // min bounds
        vertex_count,            // normal accessor count
        optimized_indices.len(), // index count
    );

    let json_aligned = {
        let mut v = json.into_bytes();
        while v.len() % 4 != 0 {
            v.push(b' ');
        }
        v
    };

    // Create GLB file
    let mut glb_data = Vec::new();

    // GLB Header
    glb_data.write_u32::<LittleEndian>(0x46546C67)?; // magic "glTF"
    glb_data.write_u32::<LittleEndian>(2)?; // version
    glb_data.write_u32::<LittleEndian>(
        12 + 8
            + json_aligned.len() as u32
            + 8
            + vertex_buffer_size as u32
            + index_buffer_size as u32,
    )?; // total length

    // JSON chunk
    glb_data.write_u32::<LittleEndian>(json_aligned.len() as u32)?; // chunk length
    glb_data.write_u32::<LittleEndian>(0x4E4F534A)?; // chunk type "JSON"
    glb_data.extend_from_slice(&json_aligned);

    // BIN chunk
    glb_data.write_u32::<LittleEndian>((vertex_buffer_size + index_buffer_size) as u32)?; // chunk length
    glb_data.write_u32::<LittleEndian>(0x004E4942)?; // chunk type "BIN\0"
    glb_data.extend_from_slice(&vertex_buffer);
    glb_data.extend_from_slice(&optimized_index_buffer);

    // Write to file
    fs::write(output_path, glb_data)?;

    Ok(())
}

fn handle_mesh(
    args: &Args,
    received_mesh: ReceivedMesh,
    stats: &mut ConnectionStats,
) -> std::io::Result<()> {
    stats.frames_received += 1;
    stats.total_triangles += received_mesh.triangle_count as u64;

    if args.verbose {
        println!(
            "Received mesh data: {} triangles, {} vertices",
            received_mesh.triangle_count,
            received_mesh.vertices.len()
        );
    }

    // Create output filename
    let filename = format!(
        "{}_{:06}.glb",
        received_mesh.simulation_uuid.replace('-', "_"),
        received_mesh.frame_number
    );
    let output_path = args.output_dir.join(&filename);

    // Convert to GLB
    let start = Instant::now();
    convert_to_glb(&received_mesh, &output_path)?;
    let conversion_time = start.elapsed();

    if args.verbose {
        println!(
            "Saved {} ({} triangles) in {:?}",
            filename, received_mesh.triangle_count, conversion_time
        );
    }

    Ok(())
}

struct ConnectionStats {
    frames_received: u32,
    total_triangles: u64,
    start_time: Instant,
}

impl ConnectionStats {
    fn new() -> Self {
        Self {
            frames_received: 0,
            total_triangles: 0,
            start_time: Instant::now(),
        }
    }

    fn print_summary(&self) {
        let elapsed = self.start_time.elapsed();
        let fps = self.frames_received as f64 / elapsed.as_secs_f64();
        let triangles_per_sec = self.total_triangles as f64 / elapsed.as_secs_f64();

        println!("\nConnection Summary:");
        println!("  Frames received: {}", self.frames_received);
        println!("  Total triangles: {}", self.total_triangles);
        println!("  Average FPS: {fps:.2}");
        println!("  Triangles/sec: {triangles_per_sec:.0}");
        println!("  Total time: {elapsed:?}");
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Create output directory if it doesn't exist
    fs::create_dir_all(&args.output_dir)?;

    // Create mesh receiver
    let mut receiver = MeshReceiver::new(args.port, args.max_size_mb)?;

    println!("Mesh receiver service listening on port {}", args.port);
    println!("Output directory: {:?}", args.output_dir);
    println!("Protocol: version 1");
    println!("Message format:");
    println!("  - Header: version(u16) + type(u16) + size(u32) + uuid(36 bytes) + frame(u32)");
    println!("  - Body: triangle_count(u32) + vertices(f32 array, 9 per triangle)");

    // Run receiver with callback
    let args = Arc::new(args);
    let mut stats = ConnectionStats::new();

    receiver.run(|mesh| {
        if let Err(e) = handle_mesh(&args, mesh, &mut stats) {
            eprintln!("Error handling mesh: {e}");
        }
        true // Continue receiving
    })?;

    // Print final statistics
    if stats.frames_received > 0 {
        stats.print_summary();
    }

    Ok(())
}
