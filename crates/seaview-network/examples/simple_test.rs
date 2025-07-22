//! Simple test example for seaview-network
//!
//! This example demonstrates basic usage of the mesh sender and receiver.

use seaview_network::{MeshFrame, MeshReceiver, MeshSender};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Start receiver in background
    let receiver_handle = thread::spawn(|| {
        let mut receiver = MeshReceiver::bind("127.0.0.1:9877").expect("Failed to bind receiver");
        println!("Receiver listening on {}", receiver.local_addr().unwrap());

        // Receive one frame
        match receiver.receive_one() {
            Ok(received) => {
                println!(
                    "Received frame {} from {} with {} vertices",
                    received.frame.frame_number,
                    received.source_addr,
                    received.frame.vertex_count()
                );
            }
            Err(e) => {
                eprintln!("Receive error: {}", e);
            }
        }
    });

    // Give receiver time to start
    thread::sleep(Duration::from_millis(100));

    // Connect sender
    let mut sender = MeshSender::connect("127.0.0.1:9877")?;
    println!("Connected to receiver");

    // Create a simple triangle mesh
    let mut mesh = MeshFrame::new("test-simulation".to_string(), 1);
    mesh.vertices = vec![
        0.0, 0.0, 0.0, // Vertex 1
        1.0, 0.0, 0.0, // Vertex 2
        0.0, 1.0, 0.0, // Vertex 3
    ];
    mesh.timestamp = 1000;

    // Send the mesh
    sender.send_mesh(&mesh)?;
    println!("Sent mesh frame");

    // Send end of stream
    sender.send_end_of_stream()?;

    // Wait for receiver to finish
    receiver_handle.join().unwrap();

    println!("Test completed successfully!");
    Ok(())
}
