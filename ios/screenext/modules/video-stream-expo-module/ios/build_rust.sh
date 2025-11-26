#!/bin/bash
set -e

# Navigate to the script directory (which is inside ios/)
cd "$(dirname "$0")"

# --- BUILD RUST ---

# Rust root is now one level up from here (ios/)
RUST_ROOT="../rust"

# Map Xcode ARCHS to Rust targets
ARCH_ARRAY=($ARCHS)
ARCH="${ARCH_ARRAY[0]}"

if [[ "$PLATFORM_NAME" == "iphonesimulator" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        TARGET="x86_64-apple-ios"
    elif [[ "$ARCH" == "arm64" ]]; then
        TARGET="aarch64-apple-ios-sim"
    else
        echo "Unsupported simulator arch: $ARCH"
        exit 1
    fi
else
    TARGET="aarch64-apple-ios"
fi

echo "Building Rust library 'ios_stream_handler' for target: $TARGET"

if ! command -v cargo &> /dev/null; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

pushd "$RUST_ROOT"
cargo build --release --target "$TARGET"
popd

cp "$RUST_ROOT/target/$TARGET/release/libios_stream_handler.a" "libios_stream_handler.a"

echo "Rust build complete."
