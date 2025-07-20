use byteorder::{LittleEndian, ReadBytesExt};
use clap::{arg, Command};
use log::warn;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct StlInfo {
    format: StlFormat,
    num_triangles: u32,
    bounds: Option<Bounds>,
}

#[derive(Debug, PartialEq)]
enum StlFormat {
    Binary,
    Ascii,
}

#[derive(Debug)]
struct Bounds {
    min: [f32; 3],
    max: [f32; 3],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    let matches = Command::new("stl_info")
        .version("1.0")
        .author("Seaview STL Info Tool")
        .about("Display information about STL files and convert between formats")
        .arg(arg!(<FILE> "STL file to analyze"))
        .arg(arg!(--"convert-ascii" "Convert binary STL to ASCII format"))
        .arg(arg!(--"fix-ascii" "Fix NaN/Inf values in ASCII STL files"))
        .get_matches();

    let file_path = matches.get_one::<String>("FILE").unwrap();
    let convert_ascii = matches.get_flag("convert-ascii");
    let fix_ascii = matches.get_flag("fix-ascii");

    let path = Path::new(file_path);
    if !path.exists() {
        eprintln!("Error: File '{}' does not exist", file_path);
        std::process::exit(1);
    }

    // Analyze the STL file
    let info = analyze_stl(path)?;

    // Display information
    println!("STL File Information:");
    println!("====================");
    println!("File: {}", path.display());
    println!("Format: {:?}", info.format);
    println!("Number of triangles: {}", info.num_triangles);

    if let Some(bounds) = &info.bounds {
        println!("\nBounding Box:");
        println!(
            "  Min: [{:.6}, {:.6}, {:.6}]",
            bounds.min[0], bounds.min[1], bounds.min[2]
        );
        println!(
            "  Max: [{:.6}, {:.6}, {:.6}]",
            bounds.max[0], bounds.max[1], bounds.max[2]
        );
        println!(
            "  Size: [{:.6}, {:.6}, {:.6}]",
            bounds.max[0] - bounds.min[0],
            bounds.max[1] - bounds.min[1],
            bounds.max[2] - bounds.min[2]
        );
    }

    // Convert to ASCII if requested
    if convert_ascii {
        if info.format == StlFormat::Binary {
            println!("\nConverting to ASCII format...");
            let output_path = convert_to_ascii(path)?;
            println!("ASCII file saved to: {}", output_path.display());
        } else {
            println!("\nFile is already in ASCII format, skipping conversion.");
        }
    }

    // Fix ASCII if requested
    if fix_ascii {
        if info.format == StlFormat::Ascii {
            println!("\nFixing NaN/Inf values in ASCII file...");
            let output_path = fix_ascii_stl(path)?;
            println!("Fixed ASCII file saved to: {}", output_path.display());
        } else {
            println!("\nFile is in binary format. Use --convert-ascii first to convert to ASCII.");
        }
    }

    Ok(())
}

fn analyze_stl(path: &Path) -> Result<StlInfo, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 5];
    file.read_exact(&mut buffer)?;

    let format = if &buffer == b"solid" {
        StlFormat::Ascii
    } else {
        StlFormat::Binary
    };

    match format {
        StlFormat::Binary => analyze_binary_stl(path),
        StlFormat::Ascii => analyze_ascii_stl(path),
    }
}

fn analyze_binary_stl(path: &Path) -> Result<StlInfo, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Skip 80-byte header
    let mut header = [0u8; 80];
    reader.read_exact(&mut header)?;

    // Read number of triangles
    let num_triangles = reader.read_u32::<LittleEndian>()?;

    let mut bounds = Bounds {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    // Read triangles and calculate bounds
    for _ in 0..num_triangles {
        // Read normal (3 floats)
        let _normal = [
            reader.read_f32::<LittleEndian>()?,
            reader.read_f32::<LittleEndian>()?,
            reader.read_f32::<LittleEndian>()?,
        ];

        // Read vertices (3 vertices × 3 floats each)
        for _ in 0..3 {
            let vertex = [
                reader.read_f32::<LittleEndian>()?,
                reader.read_f32::<LittleEndian>()?,
                reader.read_f32::<LittleEndian>()?,
            ];

            for i in 0..3 {
                bounds.min[i] = bounds.min[i].min(vertex[i]);
                bounds.max[i] = bounds.max[i].max(vertex[i]);
            }
        }

        // Skip attribute byte count
        reader.read_u16::<LittleEndian>()?;
    }

    Ok(StlInfo {
        format: StlFormat::Binary,
        num_triangles,
        bounds: Some(bounds),
    })
}

fn analyze_ascii_stl(path: &Path) -> Result<StlInfo, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut num_triangles = 0;
    let mut bounds = Bounds {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    let mut in_facet = false;
    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("facet normal") {
            in_facet = true;
            num_triangles += 1;
        } else if line.starts_with("endfacet") {
            in_facet = false;
        } else if in_facet && line.starts_with("vertex") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    bounds.min[0] = bounds.min[0].min(x);
                    bounds.min[1] = bounds.min[1].min(y);
                    bounds.min[2] = bounds.min[2].min(z);
                    bounds.max[0] = bounds.max[0].max(x);
                    bounds.max[1] = bounds.max[1].max(y);
                    bounds.max[2] = bounds.max[2].max(z);
                }
            }
        }
    }

    Ok(StlInfo {
        format: StlFormat::Ascii,
        num_triangles,
        bounds: if num_triangles > 0 {
            Some(bounds)
        } else {
            None
        },
    })
}

fn convert_to_ascii(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Create output filename
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let output_path = path.with_file_name(format!("{}-ascii.stl", stem));

    let output_file = File::create(&output_path)?;
    let mut writer = BufWriter::new(output_file);

    // Skip 80-byte header
    let mut header = [0u8; 80];
    reader.read_exact(&mut header)?;

    // Read number of triangles
    let num_triangles = reader.read_u32::<LittleEndian>()?;

    // Write ASCII header
    writeln!(writer, "solid {}", stem)?;

    // Read and convert triangles
    for i in 0..num_triangles {
        // Read normal (often garbage in binary STL files)
        let _stored_normal = [
            reader.read_f32::<LittleEndian>()?,
            reader.read_f32::<LittleEndian>()?,
            reader.read_f32::<LittleEndian>()?,
        ];

        // Read vertices
        let mut vertices = [[0.0f32; 3]; 3];
        for j in 0..3 {
            vertices[j] = [
                reader.read_f32::<LittleEndian>()?,
                reader.read_f32::<LittleEndian>()?,
                reader.read_f32::<LittleEndian>()?,
            ];
        }

        // Skip attribute byte count
        reader.read_u16::<LittleEndian>()?;

        // Calculate normal from vertices
        let v0 = vertices[0];
        let v1 = vertices[1];
        let v2 = vertices[2];

        // Edge vectors
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        // Cross product
        let mut normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        // Normalize
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();

        // Check for degenerate triangles or NaN values
        if length > 0.0 && length.is_finite() {
            normal[0] /= length;
            normal[1] /= length;
            normal[2] /= length;
        } else {
            // Default normal for degenerate triangles
            normal = [0.0, 0.0, 1.0];
            warn!(
                "Triangle {} has degenerate or invalid normal, using default",
                i
            );
        }

        // Check vertices for NaN or infinity
        let mut valid = true;
        for vertex in &vertices {
            for &coord in vertex {
                if !coord.is_finite() {
                    warn!("Triangle {} has invalid vertex coordinate: {}", i, coord);
                    valid = false;
                }
            }
        }

        if !valid {
            warn!("Skipping triangle {} due to invalid coordinates", i);
            continue;
        }

        // Write ASCII facet with proper formatting
        writeln!(
            writer,
            "  facet normal {:.6} {:.6} {:.6}",
            normal[0], normal[1], normal[2]
        )?;
        writeln!(writer, "    outer loop")?;
        for vertex in &vertices {
            writeln!(
                writer,
                "      vertex {:.6} {:.6} {:.6}",
                vertex[0], vertex[1], vertex[2]
            )?;
        }
        writeln!(writer, "    endloop")?;
        writeln!(writer, "  endfacet")?;
    }

    writeln!(writer, "endsolid {}", stem)?;
    writer.flush()?;

    Ok(output_path)
}

fn fix_ascii_stl(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;

    // Create output filename
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let output_path = path.with_file_name(format!("{}-fixed.stl", stem));

    let output_file = File::create(&output_path)?;
    let mut writer = BufWriter::new(output_file);

    let mut fixed_count = 0;
    let mut line_number = 0;

    for line in content.lines() {
        line_number += 1;
        let trimmed = line.trim();

        if trimmed.starts_with("facet normal") {
            // Parse the normal values
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5 {
                let mut normal = [0.0f32; 3];
                let mut needs_fix = false;

                for i in 0..3 {
                    if let Ok(val) = parts[i + 2].parse::<f32>() {
                        if val.is_finite() {
                            normal[i] = val;
                        } else {
                            needs_fix = true;
                            normal[i] = 0.0;
                        }
                    } else if parts[i + 2] == "-nan"
                        || parts[i + 2] == "nan"
                        || parts[i + 2] == "-inf"
                        || parts[i + 2] == "inf"
                    {
                        needs_fix = true;
                        normal[i] = 0.0;
                    } else {
                        // Try to parse anyway, might be a formatting issue
                        normal[i] = 0.0;
                        needs_fix = true;
                    }
                }

                if needs_fix {
                    // For now, use a default up normal for invalid normals
                    // In a more sophisticated version, we could calculate from vertices
                    writeln!(writer, "  facet normal 0.000000 0.000000 1.000000")?;
                    fixed_count += 1;
                    warn!("Fixed NaN/Inf normal at line {}", line_number);
                } else {
                    // Write the original line if no fix was needed
                    writeln!(writer, "{}", line)?;
                }
            } else {
                // Malformed line, write as-is
                writeln!(writer, "{}", line)?;
                warn!(
                    "Malformed facet normal line at line {}: {}",
                    line_number, line
                );
            }
        } else if trimmed.starts_with("vertex") {
            // Check vertices for NaN/Inf as well
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 4 {
                let mut vertex = [0.0f32; 3];
                let mut needs_fix = false;

                for i in 0..3 {
                    if let Ok(val) = parts[i + 1].parse::<f32>() {
                        if val.is_finite() {
                            vertex[i] = val;
                        } else {
                            needs_fix = true;
                            warn!("Found NaN/Inf vertex coordinate at line {}", line_number);
                            // For vertices, we can't just use 0, so skip this triangle
                        }
                    } else {
                        needs_fix = true;
                        warn!(
                            "Found invalid vertex coordinate at line {}: {}",
                            line_number,
                            parts[i + 1]
                        );
                    }
                }

                if !needs_fix {
                    writeln!(writer, "{}", line)?;
                } else {
                    // Skip vertices with NaN/Inf - this will corrupt the triangle
                    // In a real implementation, we'd need to skip the entire facet
                    warn!("Skipping vertex with NaN/Inf at line {}", line_number);
                }
            } else {
                writeln!(writer, "{}", line)?;
            }
        } else {
            // Write all other lines as-is
            writeln!(writer, "{}", line)?;
        }
    }

    writer.flush()?;

    if fixed_count > 0 {
        println!("Fixed {} facet normals with NaN/Inf values", fixed_count);
    } else {
        println!("No NaN/Inf values found in normals");
    }

    Ok(output_path)
}
