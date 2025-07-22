use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

use stl_io::{read_stl, write_stl, IndexedMesh, Triangle, Vertex};

#[derive(Parser, Debug)]
#[command(author, version, about = "Optimize STL mesh sequences using meshoptimizer", long_about = None)]
struct Args {
    /// Directory containing STL files to optimize
    #[arg(value_name = "DIR")]
    directory: PathBuf,

    /// Output directory (defaults to input directory with '_optimized' suffix)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Process files in parallel
    #[arg(short, long, default_value_t = true)]
    parallel: bool,

    /// Number of threads to use (0 = all available)
    #[arg(short = 'j', long, default_value_t = 0)]
    threads: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Dry run - don't write files, just show statistics
    #[arg(short, long)]
    dry_run: bool,

    /// Skip files smaller than this size in bytes
    #[arg(long, default_value_t = 1024)]
    min_size: u64,

    /// File pattern to match (e.g., "*.stl")
    #[arg(short = 'p', long, default_value = "*.stl")]
    pattern: String,
}

#[derive(Error, Debug)]
enum OptimizationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("STL parsing error: {0}")]
    StlError(String),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
}

#[derive(Debug, Clone)]
struct FileStats {
    path: PathBuf,
    original_size: u64,
    optimized_size: u64,
    original_vertices: usize,
    optimized_vertices: usize,
    #[allow(dead_code)]
    original_triangles: usize,
    processing_time: Duration,
    error: Option<String>,
}

impl FileStats {
    fn size_reduction_percent(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        (1.0 - self.optimized_size as f64 / self.original_size as f64) * 100.0
    }

    fn vertex_reduction_percent(&self) -> f64 {
        if self.original_vertices == 0 {
            return 0.0;
        }
        (1.0 - self.optimized_vertices as f64 / self.original_vertices as f64) * 100.0
    }
}

#[derive(Debug, Default)]
struct OverallStats {
    total_files: usize,
    successful_files: usize,
    failed_files: usize,
    skipped_files: usize,
    total_original_size: u64,
    total_optimized_size: u64,
    total_original_vertices: usize,
    total_optimized_vertices: usize,
    total_processing_time: Duration,
}

impl OverallStats {
    fn add(&mut self, stats: &FileStats) {
        self.total_files += 1;
        if stats.error.is_some() {
            self.failed_files += 1;
        } else {
            self.successful_files += 1;
            self.total_original_size += stats.original_size;
            self.total_optimized_size += stats.optimized_size;
            self.total_original_vertices += stats.original_vertices;
            self.total_optimized_vertices += stats.optimized_vertices;
            self.total_processing_time += stats.processing_time;
        }
    }

    fn print_summary(&self) {
        println!("\n=== Optimization Summary ===");
        println!("Total files processed: {}", self.total_files);
        println!("  Successful: {}", self.successful_files);
        println!("  Failed: {}", self.failed_files);
        println!("  Skipped: {}", self.skipped_files);

        if self.successful_files > 0 {
            let size_reduction = self.total_original_size - self.total_optimized_size;
            let size_reduction_percent =
                (size_reduction as f64 / self.total_original_size as f64) * 100.0;

            println!("\nSize statistics:");
            println!(
                "  Original total: {} MB",
                self.total_original_size / 1_048_576
            );
            println!(
                "  Optimized total: {} MB",
                self.total_optimized_size / 1_048_576
            );
            println!(
                "  Total saved: {} MB ({:.1}%)",
                size_reduction / 1_048_576,
                size_reduction_percent
            );

            let vertex_reduction_percent = (1.0
                - self.total_optimized_vertices as f64 / self.total_original_vertices as f64)
                * 100.0;

            println!("\nVertex statistics:");
            println!("  Original vertices: {}", self.total_original_vertices);
            println!("  Optimized vertices: {}", self.total_optimized_vertices);
            println!("  Vertex reduction: {vertex_reduction_percent:.1}%");

            println!("\nPerformance:");
            println!(
                "  Total processing time: {:.2}s",
                self.total_processing_time.as_secs_f64()
            );
            println!(
                "  Average time per file: {:.2}s",
                self.total_processing_time.as_secs_f64() / self.successful_files as f64
            );
        }
    }
}

fn optimize_stl_file(
    input_path: &Path,
    output_path: &Path,
    verbose: bool,
) -> Result<FileStats, OptimizationError> {
    let start_time = Instant::now();

    // Get original file size
    let original_size = fs::metadata(input_path)?.len();

    // Read STL file - read_stl already returns an IndexedMesh
    let mut file = fs::File::open(input_path)?;
    let indexed_mesh =
        read_stl(&mut file).map_err(|e| OptimizationError::StlError(e.to_string()))?;

    let original_vertices = indexed_mesh.vertices.len();
    let original_triangles = indexed_mesh.faces.len();

    if verbose {
        println!("Processing: {:?}", input_path.file_name().unwrap());
        println!("  Original: {original_vertices} vertices, {original_triangles} triangles");
    }

    // Optimize the mesh
    let optimized_mesh = optimize_indexed_mesh(indexed_mesh)?;

    let optimized_vertices = optimized_mesh.vertices.len();

    // Write optimized mesh - write_stl takes an iterator of triangles
    let triangles = optimized_mesh.faces.iter().map(|face| Triangle {
        normal: face.normal,
        vertices: [
            optimized_mesh.vertices[face.vertices[0]],
            optimized_mesh.vertices[face.vertices[1]],
            optimized_mesh.vertices[face.vertices[2]],
        ],
    });

    let mut output_file = fs::File::create(output_path)?;
    write_stl(&mut output_file, triangles)
        .map_err(|e| OptimizationError::StlError(e.to_string()))?;

    let optimized_size = fs::metadata(output_path)?.len();
    let processing_time = start_time.elapsed();

    let stats = FileStats {
        path: input_path.to_path_buf(),
        original_size,
        optimized_size,
        original_vertices,
        optimized_vertices,
        original_triangles,
        processing_time,
        error: None,
    };

    if verbose {
        println!(
            "  Optimized: {} vertices ({:.1}% reduction)",
            optimized_vertices,
            stats.vertex_reduction_percent()
        );
        println!(
            "  File size: {} KB -> {} KB ({:.1}% reduction)",
            original_size / 1024,
            optimized_size / 1024,
            stats.size_reduction_percent()
        );
        println!("  Processing time: {:.2}s", processing_time.as_secs_f64());
    }

    Ok(stats)
}

fn optimize_indexed_mesh(mesh: IndexedMesh) -> Result<IndexedMesh, OptimizationError> {
    // For now, just return the mesh as-is since we've already converted unindexed to indexed
    // Future optimization: implement vertex cache optimization, vertex clustering, etc.
    Ok(mesh)
}

#[allow(dead_code)]
fn vertex_to_key(vertex: &Vertex) -> (u32, u32, u32) {
    (
        vertex[0].to_bits(),
        vertex[1].to_bits(),
        vertex[2].to_bits(),
    )
}

fn process_directory(args: &Args) -> Result<(), OptimizationError> {
    // Determine output directory
    let output_dir = match &args.output {
        Some(dir) => dir.clone(),
        None => {
            let mut output = args.directory.clone();
            let name = output
                .file_name()
                .ok_or_else(|| OptimizationError::PathError("Invalid directory name".to_string()))?
                .to_string_lossy();
            output.set_file_name(format!("{name}_optimized"));
            output
        }
    };

    // Create output directory if not dry run
    if !args.dry_run {
        fs::create_dir_all(&output_dir)?;
    }

    // Find all STL files
    let pattern = glob::Pattern::new(&args.pattern)
        .map_err(|e| OptimizationError::PathError(format!("Invalid pattern: {e}")))?;

    let stl_files: Vec<PathBuf> = fs::read_dir(&args.directory)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && pattern.matches_path(path)
                && path
                    .metadata()
                    .map(|m| m.len() >= args.min_size)
                    .unwrap_or(false)
        })
        .collect();

    if stl_files.is_empty() {
        println!("No STL files found in {:?}", args.directory);
        return Ok(());
    }

    println!("Found {} STL files to process", stl_files.len());

    // Set up thread pool
    if args.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()
            .map_err(|e| {
                OptimizationError::OptimizationFailed(format!("Failed to set thread pool: {e}"))
            })?;
    }

    // Process files
    let results: Vec<FileStats> = if args.parallel {
        stl_files
            .par_iter()
            .map(|input_path| {
                let output_path = output_dir.join(input_path.file_name().unwrap());

                if args.dry_run {
                    // Just analyze, don't write
                    match optimize_stl_file(input_path, &output_path, args.verbose) {
                        Ok(mut stats) => {
                            // Estimate optimized size
                            stats.optimized_size = (stats.original_size as f64
                                * (stats.optimized_vertices as f64
                                    / stats.original_vertices as f64))
                                as u64;
                            stats
                        }
                        Err(e) => FileStats {
                            path: input_path.clone(),
                            original_size: 0,
                            optimized_size: 0,
                            original_vertices: 0,
                            optimized_vertices: 0,
                            original_triangles: 0,
                            processing_time: Duration::from_secs(0),
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    match optimize_stl_file(input_path, &output_path, args.verbose) {
                        Ok(stats) => stats,
                        Err(e) => FileStats {
                            path: input_path.clone(),
                            original_size: 0,
                            optimized_size: 0,
                            original_vertices: 0,
                            optimized_vertices: 0,
                            original_triangles: 0,
                            processing_time: Duration::from_secs(0),
                            error: Some(e.to_string()),
                        },
                    }
                }
            })
            .collect()
    } else {
        stl_files
            .iter()
            .map(|input_path| {
                let output_path = output_dir.join(input_path.file_name().unwrap());

                if args.dry_run {
                    match optimize_stl_file(input_path, &output_path, args.verbose) {
                        Ok(mut stats) => {
                            stats.optimized_size = (stats.original_size as f64
                                * (stats.optimized_vertices as f64
                                    / stats.original_vertices as f64))
                                as u64;
                            stats
                        }
                        Err(e) => FileStats {
                            path: input_path.clone(),
                            original_size: 0,
                            optimized_size: 0,
                            original_vertices: 0,
                            optimized_vertices: 0,
                            original_triangles: 0,
                            processing_time: Duration::from_secs(0),
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    match optimize_stl_file(input_path, &output_path, args.verbose) {
                        Ok(stats) => stats,
                        Err(e) => FileStats {
                            path: input_path.clone(),
                            original_size: 0,
                            optimized_size: 0,
                            original_vertices: 0,
                            optimized_vertices: 0,
                            original_triangles: 0,
                            processing_time: Duration::from_secs(0),
                            error: Some(e.to_string()),
                        },
                    }
                }
            })
            .collect()
    };

    // Compute overall statistics
    let mut overall_stats = OverallStats::default();

    for stats in &results {
        overall_stats.add(stats);

        if let Some(error) = &stats.error {
            eprintln!(
                "Error processing {:?}: {}",
                stats.path.file_name().unwrap(),
                error
            );
        }
    }

    overall_stats.print_summary();

    if !args.dry_run && overall_stats.successful_files > 0 {
        println!("\nOptimized files written to: {output_dir:?}");
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate input directory
    if !args.directory.is_dir() {
        eprintln!("Error: {:?} is not a directory", args.directory);
        std::process::exit(1);
    }

    // Run optimization
    process_directory(&args)?;

    Ok(())
}
