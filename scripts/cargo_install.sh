#!/bin/bash

# Seaview cargo install script
# This script installs the seaview binary using cargo

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Installing seaview..."

# Change to project root
cd "$PROJECT_ROOT"

# Install with optimizations
echo "Building and installing seaview with release optimizations..."
cargo install --path crates/seaview --locked

echo ""
echo "✓ Seaview installed successfully!"
echo ""
echo "You can now run seaview from anywhere with:"
echo "  seaview [OPTIONS]"
echo ""
echo "Examples:"
echo "  seaview                              # Launch with default settings"
echo "  seaview --path model.stl             # Load a specific STL file"
echo "  seaview --network-port 9877          # Enable network mesh receiving"
echo "  seaview --help                       # Show all options"
echo ""

# Check if cargo bin directory is in PATH
if ! echo "$PATH" | grep -q "$HOME/.cargo/bin"; then
    echo "⚠️  Warning: ~/.cargo/bin is not in your PATH"
    echo "   Add the following to your shell configuration:"
    echo "   export PATH=\"\$HOME/.cargo/bin:\$PATH\""
fi
