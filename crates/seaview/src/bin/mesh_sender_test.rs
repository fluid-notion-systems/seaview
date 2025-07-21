use byteorder::{LittleEndian, WriteBytesExt};
use clap::Parser;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Test client for mesh receiver service - sends triangle mesh data"
)]
struct Args {
    /// Server address
    #[arg(short, long, default_value = "127.0.0.1")]
    server: String,

    /// Server port
    #[arg(short, long, default_value = "9876")]
    port: u16,

    /// Simulation UUID
    #[arg(
        short = 'u',
        long,
        default_value = "test-sim-12345678-1234-1234-1234-123"
    )]
    uuid: String,

    /// Starting frame number
    #[arg(short = 'f', long, default_value = "0")]
    start_frame: u32,

    /// Number of frames to send
    #[arg(short = 'n', long, default_value = "1")]
    num_frames: usize,

    /// Number of triangles per frame
    #[arg(short = 't', long, default_value = "100")]
    triangles: u32,

    /// Delay between frames in milliseconds
    #[arg(short = 'd', long, default_value = "100")]
    delay_ms: u64,

    /// Generate animated mesh (rotating cube)
    #[arg(short = 'a', long)]
    animate: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn generate_test_mesh(triangles: u32, frame: u32, animate: bool) -> Vec<f32> {
    let mut vertices = Vec::with_capacity((triangles * 9) as usize);

    if animate {
        // Generate a rotating cube
        let angle = (frame as f32) * 0.1;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Define cube vertices
        let cube_verts = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];

        // Cube faces (2 triangles per face)
        let faces = [
            // Front
            [0, 1, 2],
            [0, 2, 3],
            // Back
            [5, 4, 7],
            [5, 7, 6],
            // Left
            [4, 0, 3],
            [4, 3, 7],
            // Right
            [1, 5, 6],
            [1, 6, 2],
            // Top
            [3, 2, 6],
            [3, 6, 7],
            // Bottom
            [4, 5, 1],
            [4, 1, 0],
        ];

        // Generate triangles
        for i in 0..triangles as usize {
            let face_idx = i % faces.len();
            let face = &faces[face_idx];

            // Use constant scale for all triangles
            let scale = 10.0;

            for &vert_idx in face {
                let v = &cube_verts[vert_idx];

                // Apply rotation around Y axis
                let x = v[0] * cos_a - v[2] * sin_a;
                let y = v[1];
                let z = v[0] * sin_a + v[2] * cos_a;

                // Scale and offset
                vertices.push(x * scale);
                vertices.push(y * scale);
                vertices.push(z * scale);
            }
        }
    } else {
        // Generate a simple flat mesh grid
        let grid_size = (triangles as f32).sqrt() as i32;
        let spacing = 1.0;

        for i in 0..triangles {
            let row = (i as i32) / grid_size;
            let col = (i as i32) % grid_size;

            let x = col as f32 * spacing;
            let z = row as f32 * spacing;

            // First triangle of quad
            if i * 2 < triangles {
                // Vertex 1
                vertices.push(x);
                vertices.push(0.0);
                vertices.push(z);

                // Vertex 2
                vertices.push(x + spacing);
                vertices.push(0.0);
                vertices.push(z);

                // Vertex 3
                vertices.push(x + spacing);
                vertices.push(0.0);
                vertices.push(z + spacing);
            }

            // Second triangle of quad
            if i * 2 + 1 < triangles {
                // Vertex 1
                vertices.push(x);
                vertices.push(0.0);
                vertices.push(z);

                // Vertex 2
                vertices.push(x + spacing);
                vertices.push(0.0);
                vertices.push(z + spacing);

                // Vertex 3
                vertices.push(x);
                vertices.push(0.0);
                vertices.push(z + spacing);
            }
        }

        // Ensure we have exactly the right number of vertices
        vertices.resize((triangles * 9) as usize, 0.0);
    }

    vertices
}

fn send_mesh_data(
    stream: &mut TcpStream,
    uuid: &str,
    frame_number: u32,
    triangles: u32,
    animate: bool,
    verbose: bool,
) -> std::io::Result<()> {
    let start_time = Instant::now();

    // Generate mesh data
    let vertices = generate_test_mesh(triangles, frame_number, animate);

    // Calculate message size
    let message_size = 4 + (triangles * 9 * 4); // triangle count + vertices

    // Write header
    stream.write_u16::<LittleEndian>(1)?; // version
    stream.write_u16::<LittleEndian>(1)?; // message type (mesh data)
    stream.write_u32::<LittleEndian>(message_size)?;

    // Write UUID (padded to 36 bytes)
    let mut uuid_bytes = [0u8; 36];
    let uuid_str = uuid.as_bytes();
    let copy_len = uuid_str.len().min(36);
    uuid_bytes[..copy_len].copy_from_slice(&uuid_str[..copy_len]);
    stream.write_all(&uuid_bytes)?;

    stream.write_u32::<LittleEndian>(frame_number)?;

    // Write mesh data
    stream.write_u32::<LittleEndian>(triangles)?;

    for &v in &vertices {
        stream.write_f32::<LittleEndian>(v)?;
    }

    stream.flush()?;

    // Read acknowledgment
    let mut ack = [0u8];
    stream.read_exact(&mut ack)?;

    let duration = start_time.elapsed();

    if verbose {
        println!(
            "Sent frame {} ({} triangles, {} bytes) in {:.3}s - {}",
            frame_number,
            triangles,
            message_size,
            duration.as_secs_f64(),
            if ack[0] == 1 { "OK" } else { "FAILED" }
        );
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Validate UUID length
    if args.uuid.len() > 36 {
        eprintln!("Error: UUID must be 36 characters or less");
        std::process::exit(1);
    }

    println!("Connecting to {}:{}...", args.server, args.port);

    let server_addr = format!("{}:{}", args.server, args.port);

    for frame in 0..args.num_frames {
        let frame_number = args.start_frame + frame as u32;

        // Create new connection for each frame (simpler protocol)
        match TcpStream::connect(&server_addr) {
            Ok(mut stream) => {
                println!(
                    "Sending frame {} ({}/{})...",
                    frame_number,
                    frame + 1,
                    args.num_frames
                );

                if let Err(e) = send_mesh_data(
                    &mut stream,
                    &args.uuid,
                    frame_number,
                    args.triangles,
                    args.animate,
                    args.verbose,
                ) {
                    eprintln!("Error sending frame {}: {}", frame_number, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                std::process::exit(1);
            }
        }

        // Delay between frames
        if frame + 1 < args.num_frames && args.delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(args.delay_ms));
        }
    }

    println!("Sent {} frames successfully", args.num_frames);

    Ok(())
}
