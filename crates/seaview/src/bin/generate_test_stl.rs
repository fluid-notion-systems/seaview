use std::fs::File;
use stl_io::{write_stl, Triangle, Vector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple cube made of triangles
    let mut triangles = Vec::new();

    // Define cube vertices
    let v0 = Vector::new([-1.0, -1.0, -1.0]);
    let v1 = Vector::new([1.0, -1.0, -1.0]);
    let v2 = Vector::new([1.0, 1.0, -1.0]);
    let v3 = Vector::new([-1.0, 1.0, -1.0]);
    let v4 = Vector::new([-1.0, -1.0, 1.0]);
    let v5 = Vector::new([1.0, -1.0, 1.0]);
    let v6 = Vector::new([1.0, 1.0, 1.0]);
    let v7 = Vector::new([-1.0, 1.0, 1.0]);

    // Front face (z = -1) - facing viewer, counterclockwise from outside
    triangles.push(Triangle {
        normal: Vector::new([0.0, 0.0, -1.0]),
        vertices: [v0, v2, v1],
    });
    triangles.push(Triangle {
        normal: Vector::new([0.0, 0.0, -1.0]),
        vertices: [v0, v3, v2],
    });

    // Back face (z = 1) - facing away, counterclockwise from outside
    triangles.push(Triangle {
        normal: Vector::new([0.0, 0.0, 1.0]),
        vertices: [v4, v5, v6],
    });
    triangles.push(Triangle {
        normal: Vector::new([0.0, 0.0, 1.0]),
        vertices: [v4, v6, v7],
    });

    // Left face (x = -1) - counterclockwise from outside
    triangles.push(Triangle {
        normal: Vector::new([-1.0, 0.0, 0.0]),
        vertices: [v0, v4, v7],
    });
    triangles.push(Triangle {
        normal: Vector::new([-1.0, 0.0, 0.0]),
        vertices: [v0, v7, v3],
    });

    // Right face (x = 1) - counterclockwise from outside
    triangles.push(Triangle {
        normal: Vector::new([1.0, 0.0, 0.0]),
        vertices: [v1, v2, v6],
    });
    triangles.push(Triangle {
        normal: Vector::new([1.0, 0.0, 0.0]),
        vertices: [v1, v6, v5],
    });

    // Bottom face (y = -1) - counterclockwise from below
    triangles.push(Triangle {
        normal: Vector::new([0.0, -1.0, 0.0]),
        vertices: [v0, v1, v5],
    });
    triangles.push(Triangle {
        normal: Vector::new([0.0, -1.0, 0.0]),
        vertices: [v0, v5, v4],
    });

    // Top face (y = 1) - counterclockwise from above
    triangles.push(Triangle {
        normal: Vector::new([0.0, 1.0, 0.0]),
        vertices: [v3, v7, v6],
    });
    triangles.push(Triangle {
        normal: Vector::new([0.0, 1.0, 0.0]),
        vertices: [v3, v6, v2],
    });

    // Create directory if it doesn't exist
    std::fs::create_dir_all("test_models")?;

    // Write to file
    let mut file = File::create("test_models/cube.stl")?;
    write_stl(&mut file, triangles.iter())?;

    println!("Created test_models/cube.stl");

    // Also create a simple pyramid
    let mut pyramid_triangles = Vec::new();

    // Pyramid vertices
    let base1 = Vector::new([-1.0, 0.0, -1.0]);
    let base2 = Vector::new([1.0, 0.0, -1.0]);
    let base3 = Vector::new([1.0, 0.0, 1.0]);
    let base4 = Vector::new([-1.0, 0.0, 1.0]);
    let apex = Vector::new([0.0, 2.0, 0.0]);

    // Base (two triangles) - facing down, clockwise from below (counterclockwise from above)
    pyramid_triangles.push(Triangle {
        normal: Vector::new([0.0, -1.0, 0.0]),
        vertices: [base1, base2, base3],
    });
    pyramid_triangles.push(Triangle {
        normal: Vector::new([0.0, -1.0, 0.0]),
        vertices: [base1, base3, base4],
    });

    // Side faces - counterclockwise from outside
    pyramid_triangles.push(Triangle {
        normal: calculate_normal(&base1, &apex, &base2),
        vertices: [base1, apex, base2],
    });
    pyramid_triangles.push(Triangle {
        normal: calculate_normal(&base2, &apex, &base3),
        vertices: [base2, apex, base3],
    });
    pyramid_triangles.push(Triangle {
        normal: calculate_normal(&base3, &apex, &base4),
        vertices: [base3, apex, base4],
    });
    pyramid_triangles.push(Triangle {
        normal: calculate_normal(&base4, &apex, &base1),
        vertices: [base4, apex, base1],
    });

    let mut pyramid_file = File::create("test_models/pyramid.stl")?;
    write_stl(&mut pyramid_file, pyramid_triangles.iter())?;

    println!("Created test_models/pyramid.stl");

    Ok(())
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
