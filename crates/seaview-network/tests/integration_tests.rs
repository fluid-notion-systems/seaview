//! Integration tests for seaview-network

use seaview_network::{
    DomainBounds, MeshFrame, MeshReceiver, MeshSender, NonBlockingMeshReceiver, ReceiverConfig,
};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_simple_send_receive() {
    // Start receiver in background
    let (tx, rx) = mpsc::channel();
    let receiver_thread = thread::spawn(move || {
        let mut receiver = MeshReceiver::bind("127.0.0.1:0").expect("Failed to bind");
        let addr = receiver.local_addr().expect("Failed to get address");
        tx.send(addr).expect("Failed to send address");

        // Receive one mesh
        receiver.receive_one()
    });

    // Get the receiver's address
    let addr = rx.recv().expect("Failed to get receiver address");

    // Connect sender
    let mut sender = MeshSender::connect(addr).expect("Failed to connect");

    // Create test mesh
    let mut mesh = MeshFrame::new("test-sim".to_string(), 42);
    mesh.vertices = vec![
        0.0, 0.0, 0.0, // Triangle 1
        1.0, 0.0, 0.0, 1.0, 1.0, 0.0, // Triangle 2
        2.0, 0.0, 0.0, 2.0, 1.0, 0.0, 2.0, 1.0, 1.0,
    ];
    mesh.timestamp = 123456;
    mesh.domain_bounds = DomainBounds::new([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]);

    // Send mesh
    sender.send_mesh(&mesh).expect("Failed to send mesh");

    // Wait for receiver to get the mesh
    let received = receiver_thread.join().expect("Receiver thread failed");
    let received = received.expect("Failed to receive mesh");

    // Verify received data
    assert_eq!(received.frame.simulation_id, "test-sim");
    assert_eq!(received.frame.frame_number, 42);
    assert_eq!(received.frame.vertices, mesh.vertices);
    assert_eq!(received.frame.timestamp, 123456);
}

#[test]
fn test_multiple_frames() {
    // Test sending multiple frames using separate connections,
    // which matches real-world usage where clients send one frame then EOF

    let mut receiver = MeshReceiver::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = receiver.local_addr().expect("Failed to get address");

    // Start receiver that will handle multiple connections
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for _i in 0..5 {
            match receiver.receive_one() {
                Ok(received) => {
                    tx.send(received).expect("Failed to send");
                }
                Err(e) => {
                    eprintln!("Receive error: {e}");
                    break;
                }
            }
        }
    });

    // Send 5 frames, each with its own connection
    for i in 0..5 {
        let mut sender = MeshSender::connect(addr).expect("Failed to connect");
        let mut mesh = MeshFrame::new("multi-test".to_string(), i);
        mesh.vertices = vec![
            i as f32,
            0.0,
            0.0,
            i as f32 + 1.0,
            0.0,
            0.0,
            i as f32,
            1.0,
            0.0,
        ];
        sender.send_mesh(&mesh).expect("Failed to send mesh");
        // Connection closes automatically when sender is dropped
    }

    // Collect received frames
    let mut frames = Vec::new();
    for _ in 0..5 {
        if let Ok(received) = rx.recv_timeout(Duration::from_secs(1)) {
            frames.push(received);
        }
    }

    // Verify we got all frames
    assert_eq!(frames.len(), 5);
    for (i, received) in frames.iter().enumerate() {
        assert_eq!(received.frame.frame_number, i as u32);
        assert_eq!(received.frame.vertices[0], i as f32);
    }
}

#[test]
fn test_non_blocking_receiver() {
    let mut receiver = NonBlockingMeshReceiver::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = receiver.local_addr().expect("Failed to get address");

    // Initially should return None
    assert!(receiver
        .try_receive()
        .expect("Try receive failed")
        .is_none());

    // Connect and send
    let mut sender = MeshSender::connect(addr).expect("Failed to connect");
    let mut mesh = MeshFrame::new("non-blocking".to_string(), 1);
    mesh.vertices = vec![0.0; 9];
    sender.send_mesh(&mesh).expect("Failed to send");

    // Give it a moment to arrive
    thread::sleep(Duration::from_millis(50));

    // Now should receive
    let received = receiver
        .try_receive()
        .expect("Try receive failed")
        .expect("Expected mesh");
    assert_eq!(received.frame.simulation_id, "non-blocking");
}

#[test]
fn test_heartbeat() {
    let mut receiver = MeshReceiver::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = receiver.local_addr().expect("Failed to get address");

    thread::spawn(move || {
        let mut sender = MeshSender::connect(addr).expect("Failed to connect");

        // Send heartbeat followed by mesh
        sender.send_heartbeat().expect("Failed to send heartbeat");

        let mut mesh = MeshFrame::new("heartbeat-test".to_string(), 0);
        mesh.vertices = vec![0.0; 9];
        sender.send_mesh(&mesh).expect("Failed to send mesh");
    });

    // Should receive the mesh (heartbeat is filtered out)
    let received = receiver.receive_one().expect("Failed to receive");
    assert_eq!(received.frame.simulation_id, "heartbeat-test");
}

#[test]
fn test_large_mesh() {
    let mut receiver = MeshReceiver::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = receiver.local_addr().expect("Failed to get address");

    thread::spawn(move || {
        let mut sender = MeshSender::connect(addr).expect("Failed to connect");

        // Create large mesh (1M vertices = ~12MB)
        let mut mesh = MeshFrame::new("large-mesh".to_string(), 0);
        mesh.vertices = vec![0.5; 1_000_000 * 3];
        mesh.normals = Some([0.0, 0.0, 1.0].repeat(1_000_000));

        sender.send_mesh(&mesh).expect("Failed to send large mesh");
    });

    let received = receiver.receive_one().expect("Failed to receive");
    assert_eq!(received.frame.vertex_count(), 1_000_000);
    assert!(received.frame.has_normals());
}

#[test]
fn test_custom_config() {
    // Test with JSON format if available
    #[cfg(feature = "json")]
    {
        let config = ReceiverConfig {
            format: WireFormat::Json,
            max_message_size: 50 * 1024 * 1024,
            tcp_nodelay: false,
            recv_buffer_size: Some(2 * 1024 * 1024),
            read_timeout: Some(Duration::from_secs(10)),
            accept_timeout: Some(Duration::from_secs(5)),
        };

        let mut receiver =
            MeshReceiver::bind_with_config("127.0.0.1:0", config).expect("Failed to bind");
        let addr = receiver.local_addr().expect("Failed to get address");

        let sender_config = SenderConfig {
            format: WireFormat::Json,
            max_message_size: 50 * 1024 * 1024,
            tcp_nodelay: false,
            send_buffer_size: Some(2 * 1024 * 1024),
            connect_timeout: Some(Duration::from_secs(5)),
            write_timeout: Some(Duration::from_secs(10)),
        };

        thread::spawn(move || {
            let mut sender =
                MeshSender::connect_with_config(addr, sender_config).expect("Failed to connect");
            let mut mesh = MeshFrame::new("json-test".to_string(), 0);
            mesh.vertices = vec![1.0, 2.0, 3.0];
            sender.send_mesh(&mesh).expect("Failed to send");
        });

        let received = receiver.receive_one().expect("Failed to receive");
        assert_eq!(received.frame.simulation_id, "json-test");
        assert_eq!(received.frame.vertices, vec![1.0, 2.0, 3.0]);
    }
}

#[test]
fn test_mesh_validation() {
    let mut mesh = MeshFrame::new("validation".to_string(), 0);

    // Valid mesh
    mesh.vertices = vec![0.0; 9];
    assert!(mesh.validate().is_ok());

    // Invalid vertex count
    mesh.vertices = vec![0.0; 10];
    assert!(mesh.validate().is_err());

    // Valid with normals
    mesh.vertices = vec![0.0; 9];
    mesh.normals = Some(vec![0.0; 9]);
    assert!(mesh.validate().is_ok());

    // Invalid normal count
    mesh.normals = Some(vec![0.0; 6]);
    assert!(mesh.validate().is_err());

    // Valid indexed mesh
    mesh.normals = None;
    mesh.indices = Some(vec![0, 1, 2]);
    assert!(mesh.validate().is_ok());

    // Invalid index
    mesh.indices = Some(vec![0, 1, 5]); // Index 5 out of bounds
    assert!(mesh.validate().is_err());
}

#[test]
fn test_domain_bounds() {
    let bounds = DomainBounds::new([-1.0, -2.0, -3.0], [1.0, 2.0, 3.0]);

    assert_eq!(bounds.center(), [0.0, 0.0, 0.0]);
    assert_eq!(bounds.size(), [2.0, 4.0, 6.0]);
    assert!((bounds.diagonal_length() - 7.483).abs() < 0.001);
    assert!(bounds.is_valid());

    // Invalid bounds
    let invalid = DomainBounds::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    assert!(!invalid.is_valid());
}

#[test]
fn test_sender_stats() {
    let mut receiver = MeshReceiver::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = receiver.local_addr().expect("Failed to get address");

    // Send 3 frames, each with its own connection
    for i in 0..3 {
        let mut sender = MeshSender::connect(addr).expect("Failed to connect");
        let mut mesh = MeshFrame::new("stats".to_string(), i);
        mesh.vertices = vec![0.0; 9];
        sender.send_mesh(&mesh).expect("Failed to send");

        if i == 2 {
            // Check stats on the last sender
            let stats = sender.stats();
            assert_eq!(stats.frames_sent, 1);
            assert!(stats.bytes_sent > 0);
        }
    }

    // Consume the messages
    for _ in 0..3 {
        receiver.receive_one().expect("Failed to receive");
    }

    let stats = receiver.stats();
    assert_eq!(stats.frames_received, 3);
    assert!(stats.bytes_received > 0);
}

#[test]
fn test_connection_refused() {
    // Try to connect to a port that's not listening
    let result = MeshSender::connect("127.0.0.1:1");
    assert!(result.is_err());
}

#[test]
fn test_timeout_config() {
    let config = ReceiverConfig {
        accept_timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    let mut receiver =
        MeshReceiver::bind_with_config("127.0.0.1:0", config).expect("Failed to bind");

    // Should timeout since no one connects
    let start = std::time::Instant::now();
    let result = receiver.receive_one();
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed >= Duration::from_millis(100));
    assert!(elapsed < Duration::from_secs(1));
}
