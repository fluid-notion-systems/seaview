use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

use stl_io::{read_stl, IndexedMesh};

#[derive(Parser, Debug)]
#[command(author, version, about = "Convert STL files to glTF/GLB format", long_about = None)]
struct Args {
    /// Input STL file or directory containing STL files
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output file or directory (defaults to input with .glb extension)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short = 'f', long, value_enum, default_value = "glb")]
    format: OutputFormat,

    /// Process directory files in parallel
    #[arg(short, long, default_value_t = true)]
    parallel: bool,

    /// Number of threads to use (0 = all available)
    #[arg(short = 'j', long, default_value_t = 0)]
    threads: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// File pattern to match when processing directories (e.g., "*.stl")
    #[arg(short = 'p', long, default_value = "*.stl")]
    pattern: String,

    /// Set material base color (R,G,B values 0.0-1.0)
    #[arg(long, value_delimiter = ',', default_value = "0.8,0.8,0.8")]
    base_color: Vec<f32>,

    /// Set material metallic value (0.0-1.0)
    #[arg(long, default_value_t = 0.1)]
    metallic: f32,

    /// Set material roughness value (0.0-1.0)
    #[arg(long, default_value_t = 0.8)]
    roughness: f32,
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    /// Binary glTF format (.glb) - more compact
    Glb,
    /// Text glTF format (.gltf) with separate .bin file
    Gltf,
}

#[derive(Error, Debug)]
enum ConversionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("STL parsing error: {0}")]
    StlError(String),

    #[error("glTF creation error: {0}")]
    GltfError(String),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

#[derive(Debug, Clone)]
struct ConversionStats {
    path: PathBuf,
    original_size: u64,
    output_size: u64,
    original_vertices: usize,
    original_triangles: usize,
    processing_time: Duration,
    error: Option<String>,
}

impl ConversionStats {
    fn size_reduction_percent(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        (1.0 - self.output_size as f64 / self.original_size as f64) * 100.0
    }
}

fn convert_stl_to_gltf(
    input_path: &Path,
    output_path: &Path,
    args: &Args,
) -> Result<ConversionStats, ConversionError> {
    let start_time = Instant::now();

    // Get original file size
    let original_size = fs::metadata(input_path)?.len();

    // Read STL file
    let mut file = fs::File::open(input_path)?;
    let indexed_mesh = read_stl(&mut file).map_err(|e| ConversionError::StlError(e.to_string()))?;

    let original_vertices = indexed_mesh.vertices.len();
    let original_triangles = indexed_mesh.faces.len();

    if args.verbose {
        println!("Converting: {:?}", input_path.file_name().unwrap());
        println!(
            "  Original: {} vertices, {} triangles",
            original_vertices, original_triangles
        );
    }

    // Convert to glTF
    write_gltf_simple(&indexed_mesh, output_path, &args.format, args)?;

    let output_size = fs::metadata(output_path)?.len();
    let processing_time = start_time.elapsed();

    let stats = ConversionStats {
        path: input_path.to_path_buf(),
        original_size,
        output_size,
        original_vertices,
        original_triangles,
        processing_time,
        error: None,
    };

    if args.verbose {
        println!(
            "  File size: {} KB -> {} KB ({:.1}% reduction)",
            original_size / 1024,
            output_size / 1024,
            stats.size_reduction_percent()
        );
        println!("  Processing time: {:.2}s", processing_time.as_secs_f64());
    }

    Ok(stats)
}

fn write_gltf_simple(
    mesh: &IndexedMesh,
    output_path: &Path,
    format: &OutputFormat,
    args: &Args,
) -> Result<(), ConversionError> {
    use byteorder::{LittleEndian, WriteBytesExt};

    // Build vertex and index buffers
    let mut vertex_data = Vec::new();
    let mut index_data = Vec::new();

    // Calculate bounds
    let mut min_pos = [f32::INFINITY; 3];
    let mut max_pos = [f32::NEG_INFINITY; 3];

    // Write vertices (position + normal)
    for (i, vertex) in mesh.vertices.iter().enumerate() {
        // Update bounds
        for j in 0..3 {
            min_pos[j] = min_pos[j].min(vertex[j]);
            max_pos[j] = max_pos[j].max(vertex[j]);
        }

        // Write position
        vertex_data.write_f32::<LittleEndian>(vertex[0])?;
        vertex_data.write_f32::<LittleEndian>(vertex[1])?;
        vertex_data.write_f32::<LittleEndian>(vertex[2])?;

        // Calculate vertex normal by averaging face normals
        let mut normal = [0.0f32; 3];
        let mut face_count = 0;

        for face in &mesh.faces {
            if face.vertices.contains(&i) {
                normal[0] += face.normal[0];
                normal[1] += face.normal[1];
                normal[2] += face.normal[2];
                face_count += 1;
            }
        }

        if face_count > 0 {
            let inv_count = 1.0 / face_count as f32;
            normal[0] *= inv_count;
            normal[1] *= inv_count;
            normal[2] *= inv_count;

            // Normalize
            let len =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 0.0 {
                let inv_len = 1.0 / len;
                normal[0] *= inv_len;
                normal[1] *= inv_len;
                normal[2] *= inv_len;
            }
        }

        // Write normal
        vertex_data.write_f32::<LittleEndian>(normal[0])?;
        vertex_data.write_f32::<LittleEndian>(normal[1])?;
        vertex_data.write_f32::<LittleEndian>(normal[2])?;
    }

    // Write indices
    for face in &mesh.faces {
        index_data.write_u32::<LittleEndian>(face.vertices[0] as u32)?;
        index_data.write_u32::<LittleEndian>(face.vertices[1] as u32)?;
        index_data.write_u32::<LittleEndian>(face.vertices[2] as u32)?;
    }

    // Align buffers to 4 bytes
    while vertex_data.len() % 4 != 0 {
        vertex_data.push(0);
    }
    while index_data.len() % 4 != 0 {
        index_data.push(0);
    }

    // Create JSON structure manually
    let json = format!(
        r#"{{
  "asset": {{
    "generator": "seaview STL to glTF converter",
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
        "baseColorFactor": [{}, {}, {}, 1.0],
        "metallicFactor": {},
        "roughnessFactor": {}
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
      "byteLength": {}{}
    }}
  ]
}}"#,
        args.base_color[0],
        args.base_color[1],
        args.base_color[2],
        args.metallic,
        args.roughness,
        mesh.vertices.len(),
        min_pos[0],
        min_pos[1],
        min_pos[2],
        max_pos[0],
        max_pos[1],
        max_pos[2],
        mesh.vertices.len(),
        mesh.faces.len() * 3,
        vertex_data.len(),
        vertex_data.len(),
        index_data.len(),
        vertex_data.len() + index_data.len(),
        if matches!(format, OutputFormat::Gltf) {
            format!(
                r#", "uri": "{}.bin""#,
                output_path.file_stem().unwrap().to_string_lossy()
            )
        } else {
            String::new()
        }
    );

    // Combine buffers
    let mut combined_buffer = vertex_data;
    combined_buffer.extend_from_slice(&index_data);

    // Write output
    match format {
        OutputFormat::Glb => {
            let mut writer = fs::File::create(output_path)?;

            let json_bytes = json.as_bytes();
            let json_padding = (4 - json_bytes.len() % 4) % 4;
            let json_length = json_bytes.len() + json_padding;

            // GLB header
            writer.write_all(b"glTF")?; // Magic
            writer.write_u32::<LittleEndian>(2)?; // Version
            writer.write_u32::<LittleEndian>(
                12 + 8 + json_length as u32 + 8 + combined_buffer.len() as u32,
            )?; // Total length

            // JSON chunk
            writer.write_u32::<LittleEndian>(json_length as u32)?;
            writer.write_all(b"JSON")?;
            writer.write_all(json_bytes)?;
            writer.write_all(&vec![0x20; json_padding])?;

            // Binary chunk
            writer.write_u32::<LittleEndian>(combined_buffer.len() as u32)?;
            writer.write_all(b"BIN\0")?;
            writer.write_all(&combined_buffer)?;
        }
        OutputFormat::Gltf => {
            // Write JSON
            fs::write(output_path, json)?;

            // Write binary buffer
            let bin_path = output_path.with_extension("bin");
            fs::write(bin_path, combined_buffer)?;
        }
    }

    Ok(())
}

fn process_file(input_path: &Path, args: &Args) -> Result<ConversionStats, ConversionError> {
    let output_path = determine_output_path(input_path, &args.output, &args.format)?;

    if args.verbose {
        println!("Processing: {:?} -> {:?}", input_path, output_path);
    }

    convert_stl_to_gltf(input_path, &output_path, args)
}

fn determine_output_path(
    input_path: &Path,
    output_option: &Option<PathBuf>,
    format: &OutputFormat,
) -> Result<PathBuf, ConversionError> {
    let extension = match format {
        OutputFormat::Glb => "glb",
        OutputFormat::Gltf => "gltf",
    };

    match output_option {
        Some(output) => {
            if output.is_dir() {
                Ok(output
                    .join(input_path.file_stem().unwrap())
                    .with_extension(extension))
            } else {
                Ok(output.clone())
            }
        }
        None => Ok(input_path.with_extension(extension)),
    }
}

fn process_directory(args: &Args) -> Result<Vec<ConversionStats>, ConversionError> {
    let pattern = glob::Pattern::new(&args.pattern)
        .map_err(|e| ConversionError::PathError(format!("Invalid pattern: {}", e)))?;

    let stl_files: Vec<PathBuf> = fs::read_dir(&args.input)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && pattern.matches_path(path))
        .collect();

    if stl_files.is_empty() {
        println!(
            "No STL files found matching pattern '{}' in {:?}",
            args.pattern, args.input
        );
        return Ok(Vec::new());
    }

    println!("Found {} STL files to convert", stl_files.len());

    // Set up thread pool if needed
    if args.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()
            .map_err(|e| ConversionError::GltfError(format!("Failed to set thread pool: {}", e)))?;
    }

    let results = if args.parallel {
        stl_files
            .par_iter()
            .map(|path| process_file(path, args))
            .collect::<Vec<_>>()
    } else {
        stl_files
            .iter()
            .map(|path| process_file(path, args))
            .collect::<Vec<_>>()
    };

    let mut stats = Vec::new();
    for result in results {
        match result {
            Ok(stat) => stats.push(stat),
            Err(e) => {
                eprintln!("Error: {}", e);
                stats.push(ConversionStats {
                    path: args.input.clone(),
                    original_size: 0,
                    output_size: 0,
                    original_vertices: 0,
                    original_triangles: 0,
                    processing_time: Duration::from_secs(0),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(stats)
}

fn print_summary(stats: &[ConversionStats]) {
    let successful = stats.iter().filter(|s| s.error.is_none()).count();
    let failed = stats.len() - successful;

    println!("\n=== Conversion Summary ===");
    println!("Total files: {}", stats.len());
    println!("  Successful: {}", successful);
    println!("  Failed: {}", failed);

    if successful > 0 {
        let total_original_size: u64 = stats
            .iter()
            .filter(|s| s.error.is_none())
            .map(|s| s.original_size)
            .sum();
        let total_output_size: u64 = stats
            .iter()
            .filter(|s| s.error.is_none())
            .map(|s| s.output_size)
            .sum();
        let total_processing_time: Duration = stats
            .iter()
            .filter(|s| s.error.is_none())
            .map(|s| s.processing_time)
            .sum();

        let size_reduction_percent = if total_original_size > 0 {
            (1.0 - total_output_size as f64 / total_original_size as f64) * 100.0
        } else {
            0.0
        };

        println!("\nSize statistics:");
        println!("  Total original: {} MB", total_original_size / 1_048_576);
        println!("  Total output: {} MB", total_output_size / 1_048_576);
        println!("  Size reduction: {:.1}%", size_reduction_percent);

        println!("\nPerformance:");
        println!("  Total time: {:.2}s", total_processing_time.as_secs_f64());
        println!(
            "  Average time per file: {:.2}s",
            total_processing_time.as_secs_f64() / successful as f64
        );
    }

    // Show failed files
    for stat in stats.iter().filter(|s| s.error.is_some()) {
        if let Some(error) = &stat.error {
            eprintln!("\nFailed: {:?}", stat.path);
            eprintln!("  Error: {}", error);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate base color
    if args.base_color.len() != 3 {
        eprintln!("Error: base_color must have exactly 3 values (R,G,B)");
        std::process::exit(1);
    }
    for &val in &args.base_color {
        if val < 0.0 || val > 1.0 {
            eprintln!("Error: base_color values must be between 0.0 and 1.0");
            std::process::exit(1);
        }
    }

    // Check if input exists
    if !args.input.exists() {
        eprintln!("Error: Input path does not exist: {:?}", args.input);
        std::process::exit(1);
    }

    let stats = if args.input.is_dir() {
        process_directory(&args)?
    } else {
        vec![process_file(&args.input, &args)?]
    };

    print_summary(&stats);

    Ok(())
}
