use clap::Parser;
use meshopt::optimize_vertex_cache;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// Import from the parent crate
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

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Number of worker threads
    #[arg(short = 'j', long, default_value = "4")]
    threads: usize,
}

fn convert_to_glb(mesh: &ReceivedMesh, output_path: &Path) -> std::io::Result<()> {
    use byteorder::{LittleEndian, WriteBytesExt};

    // Build vertex and index buffers with deduplication
    let mut unique_vertices = HashMap::new();
    let mut vertex_buffer = Vec::new();
    let mut index_buffer = Vec::new();
    let mut min_pos = [f32::INFINITY; 3];
    let mut max_pos = [f32::NEG_INFINITY; 3];

    // Process each triangle
    for tri_idx in 0..mesh.triangle_count as usize {
        let base_idx = tri_idx * 9;

        // Get triangle vertices
        let v0 = [
            mesh.vertices[base_idx],
            mesh.vertices[base_idx + 1],
            mesh.vertices[base_idx + 2],
        ];
        let v1 = [
            mesh.vertices[base_idx + 3],
            mesh.vertices[base_idx + 4],
            mesh.vertices[base_idx + 5],
        ];
        let v2 = [
            mesh.vertices[base_idx + 6],
            mesh.vertices[base_idx + 7],
            mesh.vertices[base_idx + 8],
        ];

        // Calculate face normal
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let normal = if len > 0.0 {
            [normal[0] / len, normal[1] / len, normal[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };

        // Process each vertex of the triangle
        for &vertex in &[v0, v1, v2] {
            // Update bounds
            for i in 0..3 {
                min_pos[i] = min_pos[i].min(vertex[i]);
                max_pos[i] = max_pos[i].max(vertex[i]);
            }

            // Create vertex key for deduplication
            let key = (
                vertex[0].to_bits(),
                vertex[1].to_bits(),
                vertex[2].to_bits(),
                normal[0].to_bits(),
                normal[1].to_bits(),
                normal[2].to_bits(),
            );

            let index = if let Some(&idx) = unique_vertices.get(&key) {
                idx
            } else {
                let idx = (vertex_buffer.len() / 6) as u32;
                unique_vertices.insert(key, idx);

                // Add vertex position and normal
                vertex_buffer.extend_from_slice(&vertex);
                vertex_buffer.extend_from_slice(&normal);

                idx
            };

            index_buffer.push(index);
        }
    }

    // Optimize vertex cache locality
    let optimized_indices = optimize_vertex_cache(&index_buffer, unique_vertices.len());

    // Build final buffers
    let mut vertex_data = Vec::new();
    let mut index_data = Vec::new();

    // Write vertex data (interleaved position + normal)
    for value in &vertex_buffer {
        vertex_data.write_f32::<LittleEndian>(*value)?;
    }

    // Write index data
    for &idx in &optimized_indices {
        index_data.write_u32::<LittleEndian>(idx)?;
    }

    // Align buffers to 4 bytes
    while vertex_data.len() % 4 != 0 {
        vertex_data.push(0);
    }
    while index_data.len() % 4 != 0 {
        index_data.push(0);
    }

    let vertex_count = vertex_buffer.len() / 6;

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
        vertex_count,
        min_pos[0],
        min_pos[1],
        min_pos[2],
        max_pos[0],
        max_pos[1],
        max_pos[2],
        vertex_count,
        optimized_indices.len(),
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

fn handle_mesh(mesh: ReceivedMesh, args: &Args) -> std::io::Result<()> {
    let start_time = Instant::now();

    if args.verbose {
        println!(
            "Received mesh data: {} triangles, {} vertices",
            mesh.triangle_count,
            mesh.vertices.len()
        );
    }

    // Create output directory for simulation if needed
    let sim_dir = args.output_dir.join(&mesh.simulation_uuid);
    fs::create_dir_all(&sim_dir)?;

    // Generate output filename
    let output_path = sim_dir.join(format!("simulation_{:06}.glb", mesh.frame_number));

    // Convert to GLB
    convert_to_glb(&mesh, &output_path)?;

    let duration = start_time.elapsed();
    println!(
        "Saved frame {} for simulation {} ({} triangles, {:.2}s)",
        mesh.frame_number,
        mesh.simulation_uuid,
        mesh.triangle_count,
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

    // Create mesh receiver
    let mut receiver = MeshReceiver::new(args.port, args.max_size_mb)?;

    println!("Mesh receiver service listening on port {}", args.port);
    println!("Output directory: {:?}", args.output_dir);
    println!("Protocol: version 1");
    println!("Message format:");
    println!("  - Header: version(u16) + type(u16) + size(u32) + uuid(36 bytes) + frame(u32)");
    println!("  - Body: triangle_count(u32) + vertices(f32 array, 9 per triangle)");

    // Run receiver with callback
    receiver.run(|mesh| {
        let args_clone = args.clone();
        pool.execute(move || {
            if let Err(e) = handle_mesh(mesh, &args_clone) {
                eprintln!("Error handling mesh: {}", e);
            }
        });
        true // Continue receiving
    })?;

    Ok(())
}
