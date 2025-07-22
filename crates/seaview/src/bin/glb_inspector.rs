// use clap::Parser;
// use std::fs::File;
// use std::io::Read;
// use std::path::PathBuf;

// #[derive(Parser, Debug)]
// #[command(author, version, about = "Inspect GLB file contents")]
// struct Args {
//     /// GLB file to inspect
//     file: PathBuf,
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
//     let args = Args::parse();

//     let mut file = File::open(&args.file)?;
//     let mut buffer = Vec::new();
//     file.read_to_end(&mut buffer)?;

//     // Read GLB header
//     if buffer.len() < 12 {
//         eprintln!("File too small to be a valid GLB");
//         return Ok(());
//     }

//     let magic = &buffer[0..4];
//     if magic != b"glTF" {
//         eprintln!("Not a valid GLB file (invalid magic)");
//         return Ok(());
//     }

//     let version = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
//     let total_length = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);

//     println!("GLB Header:");
//     println!("  Version: {}", version);
//     println!("  Total length: {} bytes", total_length);

//     // Read JSON chunk
//     let json_chunk_length = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
//     let json_chunk_type = &buffer[16..20];

//     println!("\nJSON Chunk:");
//     println!("  Length: {} bytes", json_chunk_length);
//     println!("  Type: {}", String::from_utf8_lossy(json_chunk_type));

//     let json_data = &buffer[20..20 + json_chunk_length as usize];
//     let json_str = String::from_utf8_lossy(json_data);

//     // Parse JSON to extract key information
//     if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
//         if let Some(accessors) = json["accessors"].as_array() {
//             println!("\nAccessors:");
//             for (i, accessor) in accessors.iter().enumerate() {
//                 println!("  Accessor {}:", i);
//                 println!("    Count: {}", accessor["count"]);
//                 println!("    Type: {}", accessor["type"]);
//                 if let Some(min) = accessor["min"].as_array() {
//                     println!("    Min: {:?}", min);
//                 }
//                 if let Some(max) = accessor["max"].as_array() {
//                     println!("    Max: {:?}", max);
//                 }
//             }
//         }
//     }

//     // Read binary chunk
//     let bin_offset = 20 + json_chunk_length as usize;
//     let padding = (4 - json_chunk_length % 4) % 4;
//     let bin_chunk_offset = bin_offset + padding as usize;

//     if buffer.len() > bin_chunk_offset + 8 {
//         let bin_chunk_length = u32::from_le_bytes([
//             buffer[bin_chunk_offset],
//             buffer[bin_chunk_offset + 1],
//             buffer[bin_chunk_offset + 2],
//             buffer[bin_chunk_offset + 3],
//         ]);
//         let bin_chunk_type = &buffer[bin_chunk_offset + 4..bin_chunk_offset + 8];

//         println!("\nBinary Chunk:");
//         println!("  Length: {} bytes", bin_chunk_length);
//         println!("  Type: {}", String::from_utf8_lossy(bin_chunk_type));

//         // Read first few vertices
//         let bin_data_start = bin_chunk_offset + 8;
//         if buffer.len() >= bin_data_start + 72 {
//             // At least 3 vertices (3 * 6 * 4 bytes)
//             println!("\nFirst 3 vertices (position + normal):");
//             for i in 0..3 {
//                 let offset = bin_data_start + i * 24;
//                 let x = f32::from_le_bytes([
//                     buffer[offset],
//                     buffer[offset + 1],
//                     buffer[offset + 2],
//                     buffer[offset + 3],
//                 ]);
//                 let y = f32::from_le_bytes([
//                     buffer[offset + 4],
//                     buffer[offset + 5],
//                     buffer[offset + 6],
//                     buffer[offset + 7],
//                 ]);
//                 let z = f32::from_le_bytes([
//                     buffer[offset + 8],
//                     buffer[offset + 9],
//                     buffer[offset + 10],
//                     buffer[offset + 11],
//                 ]);
//                 let nx = f32::from_le_bytes([
//                     buffer[offset + 12],
//                     buffer[offset + 13],
//                     buffer[offset + 14],
//                     buffer[offset + 15],
//                 ]);
//                 let ny = f32::from_le_bytes([
//                     buffer[offset + 16],
//                     buffer[offset + 17],
//                     buffer[offset + 18],
//                     buffer[offset + 19],
//                 ]);
//                 let nz = f32::from_le_bytes([
//                     buffer[offset + 20],
//                     buffer[offset + 21],
//                     buffer[offset + 22],
//                     buffer[offset + 23],
//                 ]);

//                 println!(
//                     "  Vertex {}: pos({:.3}, {:.3}, {:.3}) normal({:.3}, {:.3}, {:.3})",
//                     i, x, y, z, nx, ny, nz
//                 );
//             }

//             // Count unique vertices by checking the vertex buffer size
//             let vertex_stride = 24; // 6 floats * 4 bytes
//             let vertex_count = bin_chunk_length as usize / vertex_stride;
//             println!("\nTotal unique vertices: {}", vertex_count);
//         }
//     }

//     Ok(())
// }
