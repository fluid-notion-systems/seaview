use std::f32::consts::PI;
use std::fs::{create_dir_all, File};
use stl_io::{write_stl, Triangle, Vector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory
    let output_dir = "test_sequences/cube_rotation";
    create_dir_all(output_dir)?;

    // Generate 60 frames of a rotating cube
    let num_frames = 60;

    for frame in 0..num_frames {
        let angle = (frame as f32 / num_frames as f32) * 2.0 * PI;
        let filename = format!("{}/cube_frame_{:04}.stl", output_dir, frame);

        // Create a rotated cube
        let triangles = create_rotated_cube(angle);

        // Write to file
        let mut file = File::create(&filename)?;
        write_stl(&mut file, triangles.iter())?;

        println!("Created {}", filename);
    }

    // Also create a sequence with particle-like data (just points)
    let particle_dir = "test_sequences/particles";
    create_dir_all(particle_dir)?;

    for frame in 0..30 {
        let filename = format!("{}/particles_t{:03}.stl", particle_dir, frame);
        let triangles = create_particle_frame(frame);

        let mut file = File::create(&filename)?;
        write_stl(&mut file, triangles.iter())?;

        println!("Created {}", filename);
    }

    println!("\nTest sequences created successfully!");
    println!("Try running:");
    println!("  cargo run --bin shoreview test_sequences/cube_rotation");
    println!("  cargo run --bin shoreview test_sequences/particles");

    Ok(())
}

fn create_rotated_cube(angle: f32) -> Vec<Triangle> {
    let mut triangles = Vec::new();

    // Define cube vertices (centered at origin)
    let half_size = 1.0;
    let vertices = [
        [-half_size, -half_size, -half_size],
        [half_size, -half_size, -half_size],
        [half_size, half_size, -half_size],
        [-half_size, half_size, -half_size],
        [-half_size, -half_size, half_size],
        [half_size, -half_size, half_size],
        [half_size, half_size, half_size],
        [-half_size, half_size, half_size],
    ];

    // Apply rotation around Y axis
    let rotated_vertices: Vec<Vector<f32>> = vertices
        .iter()
        .map(|v| {
            let x = v[0] * angle.cos() - v[2] * angle.sin();
            let y = v[1];
            let z = v[0] * angle.sin() + v[2] * angle.cos();
            Vector::new([x, y, z])
        })
        .collect();

    // Create faces with correct winding order
    let v = &rotated_vertices;

    // Front face (z = -1) - counterclockwise from outside
    triangles.push(Triangle {
        normal: calculate_normal(&v[0], &v[2], &v[1]),
        vertices: [v[0], v[2], v[1]],
    });
    triangles.push(Triangle {
        normal: calculate_normal(&v[0], &v[3], &v[2]),
        vertices: [v[0], v[3], v[2]],
    });

    // Back face (z = 1) - counterclockwise from outside
    triangles.push(Triangle {
        normal: calculate_normal(&v[4], &v[5], &v[6]),
        vertices: [v[4], v[5], v[6]],
    });
    triangles.push(Triangle {
        normal: calculate_normal(&v[4], &v[6], &v[7]),
        vertices: [v[4], v[6], v[7]],
    });

    // Left face (x = -1) - counterclockwise from outside
    triangles.push(Triangle {
        normal: calculate_normal(&v[0], &v[4], &v[7]),
        vertices: [v[0], v[4], v[7]],
    });
    triangles.push(Triangle {
        normal: calculate_normal(&v[0], &v[7], &v[3]),
        vertices: [v[0], v[7], v[3]],
    });

    // Right face (x = 1) - counterclockwise from outside
    triangles.push(Triangle {
        normal: calculate_normal(&v[1], &v[2], &v[6]),
        vertices: [v[1], v[2], v[6]],
    });
    triangles.push(Triangle {
        normal: calculate_normal(&v[1], &v[6], &v[5]),
        vertices: [v[1], v[6], v[5]],
    });

    // Bottom face (y = -1) - counterclockwise from below
    triangles.push(Triangle {
        normal: calculate_normal(&v[0], &v[1], &v[5]),
        vertices: [v[0], v[1], v[5]],
    });
    triangles.push(Triangle {
        normal: calculate_normal(&v[0], &v[5], &v[4]),
        vertices: [v[0], v[5], v[4]],
    });

    // Top face (y = 1) - counterclockwise from above
    triangles.push(Triangle {
        normal: calculate_normal(&v[3], &v[7], &v[6]),
        vertices: [v[3], v[7], v[6]],
    });
    triangles.push(Triangle {
        normal: calculate_normal(&v[3], &v[6], &v[2]),
        vertices: [v[3], v[6], v[2]],
    });

    triangles
}

fn create_particle_frame(frame: usize) -> Vec<Triangle> {
    let mut triangles = Vec::new();
    let time = frame as f32 * 0.1;

    // Create a small sphere (icosahedron) at various positions
    let num_particles = 20;
    let particle_radius = 0.05;

    for i in 0..num_particles {
        let angle = (i as f32 / num_particles as f32) * 2.0 * PI;
        let radius = 2.0 + (time * 0.5).sin() * 0.5;

        let x = angle.cos() * radius;
        let y = (time + i as f32 * 0.3).sin() * 0.5;
        let z = angle.sin() * radius;

        // Add a small icosahedron at this position
        triangles.extend(create_icosahedron(x, y, z, particle_radius));
    }

    triangles
}

fn create_icosahedron(cx: f32, cy: f32, cz: f32, radius: f32) -> Vec<Triangle> {
    let mut triangles = Vec::new();

    // Golden ratio
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let a = radius / (phi * phi + 1.0).sqrt();
    let b = a * phi;

    // Vertices of an icosahedron
    let vertices = vec![
        Vector::new([cx - a, cy - b, cz]),
        Vector::new([cx + a, cy - b, cz]),
        Vector::new([cx - a, cy + b, cz]),
        Vector::new([cx + a, cy + b, cz]),
        Vector::new([cx, cy - a, cz - b]),
        Vector::new([cx, cy + a, cz - b]),
        Vector::new([cx, cy - a, cz + b]),
        Vector::new([cx, cy + a, cz + b]),
        Vector::new([cx - b, cy, cz - a]),
        Vector::new([cx - b, cy, cz + a]),
        Vector::new([cx + b, cy, cz - a]),
        Vector::new([cx + b, cy, cz + a]),
    ];

    // Faces of the icosahedron (20 triangular faces)
    let faces = vec![
        [0, 4, 1],
        [0, 9, 4],
        [9, 5, 4],
        [4, 5, 8],
        [4, 8, 1],
        [8, 10, 1],
        [8, 3, 10],
        [5, 3, 8],
        [5, 2, 3],
        [2, 7, 3],
        [7, 10, 3],
        [7, 6, 10],
        [7, 11, 6],
        [11, 0, 6],
        [0, 1, 6],
        [6, 1, 10],
        [9, 0, 11],
        [9, 11, 2],
        [9, 2, 5],
        [7, 2, 11],
    ];

    for face in faces {
        let v0 = &vertices[face[0]];
        let v1 = &vertices[face[1]];
        let v2 = &vertices[face[2]];

        triangles.push(Triangle {
            normal: calculate_normal(v0, v1, v2),
            vertices: [*v0, *v1, *v2],
        });
    }

    triangles
}

fn calculate_normal(v0: &Vector<f32>, v1: &Vector<f32>, v2: &Vector<f32>) -> Vector<f32> {
    // Calculate two edge vectors
    let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    // Cross product
    let normal = [
        edge1[1] * edge2[2] - edge1[2] * edge2[1],
        edge1[2] * edge2[0] - edge1[0] * edge2[2],
        edge1[0] * edge2[1] - edge1[1] * edge2[0],
    ];

    // Normalize
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length > 0.0 {
        Vector::new([normal[0] / length, normal[1] / length, normal[2] / length])
    } else {
        Vector::new([0.0, 1.0, 0.0])
    }
}
