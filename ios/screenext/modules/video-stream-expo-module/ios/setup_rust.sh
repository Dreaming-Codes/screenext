#!/bin/bash
set -e

# Navigate to the script directory
cd "$(dirname "$0")"

# Check if cargo matches, otherwise try loading from default location
if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    elif [ -d "$HOME/.cargo/bin" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# Install Rust if still not found
if ! command -v cargo &> /dev/null; then
    echo "Rust not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed."
fi

# Verify cargo is available now
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo not found after installation attempt."
    exit 1
fi

echo "Adding iOS architectures to Rust..."
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim

echo "Rust setup complete."
