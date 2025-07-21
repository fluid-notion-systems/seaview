use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

use stl_io::{read_stl, IndexedMesh, Vertex};

#[derive(Parser, Debug)]
#[command(author, version, about = "Convert STL files to glTF/GLB format with optimization", long_about = None)]
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

    /// Enable meshoptimizer optimization
    #[arg(long, default_value_t = true)]
    optimize: bool,

    /// Optimize for vertex cache
    #[arg(long, default_value_t = true)]
    optimize_vertex_cache: bool,

    /// Optimize for overdraw
    #[arg(long, default_value_t = true)]
    optimize_overdraw: bool,

    /// Optimize vertex fetch
    #[arg(long, default_value_t = true)]
    optimize_vertex_fetch: bool,

    /// Simplify mesh (0.0-1.0, where 1.0 = no simplification)
    #[arg(long, default_value_t = 1.0)]
    simplify_ratio: f32,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Dry run - don't write files, just show statistics
    #[arg(short, long)]
    dry_run: bool,

    /// File pattern to match when processing directories (e.g., "*.stl")
    #[arg(short = 'p', long, default_value = "*.stl")]
    pattern: String,

    /// Add metadata to glTF
    #[arg(long)]
    add_metadata: bool,

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
    output_vertices: usize,
    original_triangles: usize,
    output_triangles: usize,
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

    fn vertex_reduction_percent(&self) -> f64 {
        if self.original_vertices == 0 {
            return 0.0;
        }
        (1.0 - self.output_vertices as f64 / self.original_vertices as f64) * 100.0
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

    // Convert to glTF mesh data
    let (vertices, indices) = convert_indexed_mesh_to_buffers(&indexed_mesh);

    // Apply meshoptimizer optimizations if enabled
    let (optimized_vertices, optimized_indices) = if args.optimize {
        optimize_mesh_data(vertices, indices, args)?
    } else {
        (vertices, indices)
    };

    let output_vertices = optimized_vertices.len() / 8; // 8 floats per vertex (3 pos + 3 norm + 2 uv)
    let output_triangles = optimized_indices.len() / 3;

    // Create glTF file
    if !args.dry_run {
        write_gltf(
            &optimized_vertices,
            &optimized_indices,
            output_path,
            &args.format,
            args,
        )?;
    }

    let output_size = if args.dry_run {
        // Estimate output size
        let vertex_data_size = optimized_vertices.len() * 4; // 4 bytes per f32
        let index_data_size = optimized_indices.len() * 4; // 4 bytes per u32
        let gltf_overhead = 1024; // Rough estimate for glTF JSON structure
        (vertex_data_size + index_data_size + gltf_overhead) as u64
    } else {
        fs::metadata(output_path)?.len()
    };

    let processing_time = start_time.elapsed();

    let stats = ConversionStats {
        path: input_path.to_path_buf(),
        original_size,
        output_size,
        original_vertices,
        output_vertices,
        original_triangles,
        output_triangles,
        processing_time,
        error: None,
    };

    if args.verbose {
        println!(
            "  Output: {} vertices, {} triangles",
            output_vertices, output_triangles
        );
        println!(
            "  Vertex reduction: {:.1}%",
            stats.vertex_reduction_percent()
        );
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

fn convert_indexed_mesh_to_buffers(mesh: &IndexedMesh) -> (Vec<f32>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(mesh.vertices.len() * 8);
    let mut indices = Vec::with_capacity(mesh.faces.len() * 3);

    // Build vertex buffer (position + normal + uv)
    for (i, vertex) in mesh.vertices.iter().enumerate() {
        // Position
        vertices.extend_from_slice(&vertex[..]);

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

        vertices.extend_from_slice(&normal);

        // Simple UV mapping (can be improved)
        vertices.push(0.0);
        vertices.push(0.0);
    }

    // Build index buffer
    for face in &mesh.faces {
        indices.push(face.vertices[0] as u32);
        indices.push(face.vertices[1] as u32);
        indices.push(face.vertices[2] as u32);
    }

    (vertices, indices)
}

fn optimize_mesh_data(
    mut vertices: Vec<f32>,
    mut indices: Vec<u32>,
    args: &Args,
) -> Result<(Vec<f32>, Vec<u32>), ConversionError> {
    let vertex_count = vertices.len() / 8;
    let vertex_stride = 8 * std::mem::size_of::<f32>();

    // Apply simplification if requested
    if args.simplify_ratio < 1.0 {
        let target_index_count = ((indices.len() as f32) * args.simplify_ratio) as usize;
        let target_index_count = (target_index_count / 3) * 3; // Ensure multiple of 3

        let mut result = vec![0u32; indices.len()];
        let new_len = meshopt::simplify(
            &mut result,
            &indices,
            &vertices,
            vertex_count,
            vertex_stride,
            target_index_count,
            1e-2, // Target error
            meshopt::SimplifyOptions::empty(),
            None,
        );

        indices = result[..new_len].to_vec();
    }

    // Optimize vertex cache
    if args.optimize_vertex_cache {
        meshopt::optimize_vertex_cache_in_place(&mut indices, vertex_count);
    }

    // Optimize overdraw
    if args.optimize_overdraw {
        let threshold = 1.05; // Allow up to 5% worse ACMR to get better overdraw
        meshopt::optimize_overdraw_in_place(
            &mut indices,
            &vertices,
            vertex_count,
            vertex_stride,
            threshold,
        );
    }

    // Optimize vertex fetch
    if args.optimize_vertex_fetch {
        let (optimized_vertices, vertex_remap) =
            meshopt::optimize_vertex_fetch(&mut indices, &vertices, vertex_count, vertex_stride);
        vertices = optimized_vertices;

        // Update indices with remapped values
        for index in &mut indices {
            *index = vertex_remap[*index as usize] as u32;
        }
    }

    Ok((vertices, indices))
}

fn write_gltf(
    vertices: &[f32],
    indices: &[u32],
    output_path: &Path,
    format: &OutputFormat,
    args: &Args,
) -> Result<(), ConversionError> {
    use gltf::json;

    let vertex_count = vertices.len() / 8;
    let vertex_buffer_len = vertices.len() * std::mem::size_of::<f32>();
    let index_buffer_len = indices.len() * std::mem::size_of::<u32>();

    // Convert vertices to bytes
    let mut vertex_bytes = Vec::with_capacity(vertex_buffer_len);
    for &v in vertices {
        vertex_bytes.extend_from_slice(&v.to_le_bytes());
    }

    // Convert indices to bytes
    let mut index_bytes = Vec::with_capacity(index_buffer_len);
    for &i in indices {
        index_bytes.extend_from_slice(&i.to_le_bytes());
    }

    // Calculate bounds
    let (min_pos, max_pos) = calculate_bounds(vertices);

    // Create glTF JSON structure
    let mut root = json::Root::default();

    // Add metadata if requested
    if args.add_metadata {
        root.asset.generator = Some("seaview STL to glTF converter".to_string());
        root.asset.version = "2.0".to_string();
    }

    // Create buffers
    let buffer_idx = root.push(json::Buffer {
        byte_length: json::buffer::ByteLength((vertex_buffer_len + index_buffer_len) as u32),
        uri: if matches!(format, OutputFormat::Gltf) {
            Some(format!(
                "{}.bin",
                output_path.file_stem().unwrap().to_string_lossy()
            ))
        } else {
            None
        },
        ..Default::default()
    });

    // Create buffer views
    let vertex_buffer_view = root.push(json::buffer::View {
        buffer: buffer_idx,
        byte_length: json::buffer::ByteLength(vertex_buffer_len as u32),
        byte_offset: Some(json::buffer::ByteOffset(0)),
        byte_stride: Some(json::buffer::ByteStride(32)), // 8 floats * 4 bytes
        ..Default::default()
    });

    let index_buffer_view = root.push(json::buffer::View {
        buffer: buffer_idx,
        byte_length: json::buffer::ByteLength(index_buffer_len as u32),
        byte_offset: Some(json::buffer::ByteOffset(vertex_buffer_len as u32)),
        ..Default::default()
    });

    // Create accessors
    let position_accessor = root.push(json::Accessor {
        buffer_view: Some(vertex_buffer_view),
        byte_offset: Some(json::buffer::ByteOffset(0)),
        count: json::buffer::Count(vertex_count as u32),
        component_type: json::validation::Checked::Valid(json::accessor::GenericComponentType(
            gltf::json::accessor::ComponentType::F32,
        )),
        type_: json::validation::Checked::Valid(json::accessor::Type::Vec3),
        min: Some(json::Value::Array(vec![
            json::Value::Number(min_pos[0].into()),
            json::Value::Number(min_pos[1].into()),
            json::Value::Number(min_pos[2].into()),
        ])),
        max: Some(json::Value::Array(vec![
            json::Value::Number(max_pos[0].into()),
            json::Value::Number(max_pos[1].into()),
            json::Value::Number(max_pos[2].into()),
        ])),
        ..Default::default()
    });

    let normal_accessor = root.push(json::Accessor {
        buffer_view: Some(vertex_buffer_view),
        byte_offset: Some(json::buffer::ByteOffset(12)), // 3 floats * 4 bytes
        count: json::buffer::Count(vertex_count as u32),
        component_type: json::validation::Checked::Valid(json::accessor::GenericComponentType(
            gltf::json::accessor::ComponentType::F32,
        )),
        type_: json::validation::Checked::Valid(json::accessor::Type::Vec3),
        ..Default::default()
    });

    let uv_accessor = root.push(json::Accessor {
        buffer_view: Some(vertex_buffer_view),
        byte_offset: Some(json::buffer::ByteOffset(24)), // 6 floats * 4 bytes
        count: json::buffer::Count(vertex_count as u32),
        component_type: json::validation::Checked::Valid(json::accessor::GenericComponentType(
            gltf::json::accessor::ComponentType::F32,
        )),
        type_: json::validation::Checked::Valid(json::accessor::Type::Vec2),
        ..Default::default()
    });

    let index_accessor = root.push(json::Accessor {
        buffer_view: Some(index_buffer_view),
        byte_offset: Some(json::buffer::ByteOffset(0)),
        count: json::buffer::Count(indices.len() as u32),
        component_type: json::validation::Checked::Valid(json::accessor::GenericComponentType(
            gltf::json::accessor::ComponentType::U32,
        )),
        type_: json::validation::Checked::Valid(json::accessor::Type::Scalar),
        ..Default::default()
    });

    // Create material
    let material = root.push(json::Material {
        pbr_metallic_roughness: json::material::PbrMetallicRoughness {
            base_color_factor: json::material::PbrBaseColorFactor([
                args.base_color[0],
                args.base_color[1],
                args.base_color[2],
                1.0,
            ]),
            metallic_factor: json::material::StrengthFactor(args.metallic),
            roughness_factor: json::material::StrengthFactor(args.roughness),
            ..Default::default()
        },
        ..Default::default()
    });

    // Create primitive
    let primitive = json::mesh::Primitive {
        attributes: {
            let mut map = std::collections::BTreeMap::new();
            map.insert(
                json::validation::Checked::Valid(json::mesh::Semantic::Positions),
                position_accessor,
            );
            map.insert(
                json::validation::Checked::Valid(json::mesh::Semantic::Normals),
                normal_accessor,
            );
            map.insert(
                json::validation::Checked::Valid(json::mesh::Semantic::TexCoords(0)),
                uv_accessor,
            );
            map
        },
        indices: Some(index_accessor),
        material: Some(material),
        mode: json::validation::Checked::Valid(json::mesh::Mode::Triangles),
        ..Default::default()
    };

    // Create mesh
    let mesh = root.push(json::Mesh {
        primitives: vec![primitive],
        ..Default::default()
    });

    // Create node
    let node = root.push(json::Node {
        mesh: Some(mesh),
        ..Default::default()
    });

    // Create scene
    let scene = root.push(json::Scene {
        nodes: vec![node],
        ..Default::default()
    });

    root.default_scene = Some(scene);

    // Combine buffers
    let mut combined_buffer = vertex_bytes;
    combined_buffer.extend_from_slice(&index_bytes);

    // Write output
    match format {
        OutputFormat::Glb => {
            let writer = fs::File::create(output_path)?;
            gltf::binary::to_writer(writer, &root, &[combined_buffer.as_slice()])
                .map_err(|e| ConversionError::GltfError(e.to_string()))?;
        }
        OutputFormat::Gltf => {
            // Write JSON
            let writer = fs::File::create(output_path)?;
            json::serialize::to_writer_pretty(writer, &root)
                .map_err(|e| ConversionError::GltfError(e.to_string()))?;

            // Write binary buffer
            let bin_path = output_path.with_extension("bin");
            fs::write(bin_path, combined_buffer)?;
        }
    }

    Ok(())
}

fn calculate_bounds(vertices: &[f32]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for chunk in vertices.chunks(8) {
        for i in 0..3 {
            min[i] = min[i].min(chunk[i]);
            max[i] = max[i].max(chunk[i]);
        }
    }

    (min, max)
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
                // Output is a directory, use input filename with new extension
                Ok(output
                    .join(input_path.file_stem().unwrap())
                    .with_extension(extension))
            } else {
                // Output is a file path
                Ok(output.clone())
            }
        }
        None => {
            // No output specified, replace input extension
            Ok(input_path.with_extension(extension))
        }
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
                // Create error stat
                stats.push(ConversionStats {
                    path: args.input.clone(),
                    original_size: 0,
                    output_size: 0,
                    original_vertices: 0,
                    output_vertices: 0,
                    original_triangles: 0,
                    output_triangles: 0,
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
