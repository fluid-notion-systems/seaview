#!/bin/bash

# Simple test script for network mesh send and receive

echo "Starting mesh receiver service..."
mkdir -p "$OUTPUT_DIR"

# Start receiver in background
cargo run --bin mesh_receiver_service --  --verbose &
RECEIVER_PID=$!

# Wait for service to start
sleep 3

echo "Sending test mesh data..."

# Send test data
cargo run --bin mesh_sender_test -- PORT --num-frames 3 --triangles 100 --animate --verbose

echo "Test complete. Generated files:"
ls -la "$OUTPUT_DIR"

# Cleanup
kill $RECEIVER_PID 2>/dev/null || true
