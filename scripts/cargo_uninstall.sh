#!/bin/bash

# Seaview cargo uninstall script
# This script uninstalls the seaview binary

set -e

echo "Uninstalling seaview..."

# Uninstall the binary
cargo uninstall seaview

echo ""
echo "✓ Seaview uninstalled successfully!"
echo ""
