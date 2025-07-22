#!/bin/bash
# Test script for C++ integration with seaview-network
#
# This script demonstrates end-to-end functionality by:
# 1. Starting a Rust receiver in the background
# 2. Running the C++ sender example
# 3. Verifying the connection works

set -e  # Exit on error

echo "=== seaview-network C++ Integration Test ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Change to the examples directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Build the Rust library and examples
echo "Building Rust library and examples..."
cd ../../..
cargo build --release -p seaview-network --features ffi --examples
cd "$SCRIPT_DIR"

# Build the C++ example
echo "Building C++ example..."
make clean
make

# Start the Rust receiver in the background
echo
echo "Starting Rust receiver on port 9877..."
../../../target/release/examples/simple_test &
RECEIVER_PID=$!

# Give the receiver time to start
sleep 2

# Function to cleanup on exit
cleanup() {
    echo
    echo "Cleaning up..."
    if [ ! -z "$RECEIVER_PID" ]; then
        kill $RECEIVER_PID 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Run the C++ sender
echo
echo "Running C++ sender..."
if ./send_mesh localhost 9877 5; then
    echo -e "${GREEN}✓ C++ sender completed successfully${NC}"
else
    echo -e "${RED}✗ C++ sender failed${NC}"
    exit 1
fi

# Give time for messages to be processed
sleep 1

echo
echo -e "${GREEN}=== Test completed successfully! ===${NC}"
echo
echo "The test demonstrated:"
echo "  - Building the Rust library with FFI support"
echo "  - Generating C headers with cbindgen"
echo "  - Compiling a C++ application against the library"
echo "  - Successfully sending mesh data from C++ to Rust"
