#!/bin/bash
set -e

# Navigate to the script directory (which is inside ios/)
cd "$(dirname "$0")"

# --- BUILD RUST ---

# Rust root is now one level up from here (ios/)
RUST_ROOT="../rust"

# --- GSTREAMER CONFIG ---
# We need to help pkg-config find the iOS GStreamer files
# The GStreamer framework is usually in $(pwd)/Frameworks or /Library/Frameworks
FRAMEWORKS_DIR="$(pwd)/Frameworks"
GST_FRAMEWORK="${FRAMEWORKS_DIR}/GStreamer.framework"

if [ ! -d "$GST_FRAMEWORK" ]; then
    GST_FRAMEWORK="/Library/Frameworks/GStreamer.framework"
fi

if [ ! -d "$GST_FRAMEWORK" ]; then
    echo "Error: GStreamer.framework not found at $GST_FRAMEWORK or system path."
    exit 1
fi

# GStreamer.framework/Resources/lib/pkgconfig contains the .pc files
export PKG_CONFIG_PATH="${GST_FRAMEWORK}/Resources/lib/pkgconfig:${GST_FRAMEWORK}/lib/pkgconfig"
export PKG_CONFIG_ALLOW_CROSS=1

# Verify pkg-config can find gstreamer
# pkg-config --libs gstreamer-1.0

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
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    elif [ -d "$HOME/.cargo/bin" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

pushd "$RUST_ROOT"
cargo build --release --target "$TARGET"
popd

cp "$RUST_ROOT/target/$TARGET/release/libios_stream_handler.a" "libios_stream_handler.a"

echo "Rust build complete."
