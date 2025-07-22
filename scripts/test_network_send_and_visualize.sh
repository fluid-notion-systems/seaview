#!/bin/bash

# Test script for seaview's network visualization capabilities
# Starts seaview in network mode and sends test mesh data to visualize
set -e

PORT=15702  # Default BRP port for seaview
OUTPUT_DIR="./network_test_output"
NUM_FRAMES=20
TRIANGLES=500
DELAY_MS=100

echo "Testing Seaview Network Visualization..."
echo "This will:"
echo "1. Start seaview with network receiver enabled"
echo "2. Send animated mesh data to visualize in real-time"
echo ""

# Create output directory for any saved data
mkdir -p "$OUTPUT_DIR"

# Check if port is available
if netstat -tuln 2>/dev/null | grep -q ":$PORT "; then
    echo "Warning: Port $PORT is already in use"
    echo "Make sure no other seaview instance is running"
fi

echo "Starting seaview with network receiver..."
echo "Press Ctrl+C to stop the test"
echo ""

# Start seaview in background with network mode
cargo run --release --bin seaview -- --network-port $PORT &
SEAVIEW_PID=$!

# Function to cleanup
cleanup() {
    echo ""
    echo "Cleaning up..."
    if [ ! -z "$SEAVIEW_PID" ]; then
        kill $SEAVIEW_PID 2>/dev/null || true
        wait $SEAVIEW_PID 2>/dev/null || true
        echo "Stopped seaview"
    fi
}

# Set trap to cleanup on exit
trap cleanup EXIT INT TERM

# Wait for seaview to start up
echo "Waiting for seaview to initialize..."
sleep 5

# Check if seaview is still running
if ! kill -0 $SEAVIEW_PID 2>/dev/null; then
    echo "Failed to start seaview"
    exit 1
fi

echo "Seaview started! Now sending test mesh data..."
echo "You should see animated meshes appear in the seaview window"
echo ""

# Send continuous stream of test data
cargo run --bin mesh_sender_test -- \
    --server 127.0.0.1 \
    --port $PORT \
    --uuid "network-viz-test-$(date +%s)" \
    --start-frame 0 \
    --num-frames $NUM_FRAMES \
    --triangles $TRIANGLES \
    --delay-ms $DELAY_MS \
    --animate \
    --verbose

echo ""
echo "Finished sending test data."
echo "Seaview should now be displaying the animated mesh sequence."
echo "Use mouse and WASD keys to navigate in the 3D view."
echo ""
echo "Press Enter to stop seaview or Ctrl+C to exit..."
read -r
