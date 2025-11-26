#!/bin/bash
set -e

# Navigate to the script directory (which is inside ios/)
cd "$(dirname "$0")"

# --- CONFIGURATION ---
GST_VERSION="1.26.8"
GST_IOS_PKG_URL="https://gstreamer.freedesktop.org/data/pkg/ios/${GST_VERSION}/gstreamer-1.0-devel-${GST_VERSION}-ios-universal.pkg"
# Frameworks dir is now current dir since we are in ios/
FRAMEWORKS_DIR="$(pwd)/Frameworks"
GST_FRAMEWORK_PATH="${FRAMEWORKS_DIR}/GStreamer.framework"
SYSTEM_GST_PATH="/Library/Frameworks/GStreamer.framework"

# --- SETUP GSTREAMER ---

setup_gstreamer() {
    # 1. Check system install
    if [ -d "$SYSTEM_GST_PATH" ]; then
        echo "Found GStreamer in system path: $SYSTEM_GST_PATH"
        return 0
    fi

    # 2. Check local install
    if [ -d "$GST_FRAMEWORK_PATH" ]; then
        echo "Found GStreamer locally: $GST_FRAMEWORK_PATH"
        return 0
    fi

    echo "GStreamer framework not found. Downloading version ${GST_VERSION}..."
    
    mkdir -p "$FRAMEWORKS_DIR"
    TMP_DIR=$(mktemp -d)
    PKG_FILE="$TMP_DIR/gstreamer.pkg"

    # Download
    curl -L -o "$PKG_FILE" "$GST_IOS_PKG_URL"

    echo "Unpacking GStreamer framework..."
    # Unpack using pkgutil/cpio technique (works on macOS)
    # The pkg is usually an xar archive containing a Payload
    
    cd "$TMP_DIR"
    xar -xf "$PKG_FILE"
    
    # The payload is usually inside a sub-package folder, e.g., 'gstreamer-1.0-devel-ios-universal.pkg/Payload'
    # Find the Payload file
    PAYLOAD_FILE=$(find . -name "Payload" | head -n 1)
    
    if [ -z "$PAYLOAD_FILE" ]; then
        echo "Error: Could not find Payload in downloaded package."
        exit 1
    fi

    # Extract Payload (cpio gz)
    mkdir -p extracted
    cd extracted
    cat "../$PAYLOAD_FILE" | gunzip -dc | cpio -i 2>/dev/null

    # Move Framework to destination
    # The internal structure usually has 'Library/Frameworks/GStreamer.framework'
    SOURCE_FW=$(find . -name "GStreamer.framework" | head -n 1)

    if [ -z "$SOURCE_FW" ]; then
        echo "Error: Could not find GStreamer.framework in unpacked payload."
        exit 1
    fi

    echo "Installing GStreamer to $FRAMEWORKS_DIR..."
    mv "$SOURCE_FW" "$FRAMEWORKS_DIR/"
    
    # Cleanup
    cd ..
    rm -rf "$TMP_DIR"
    
    echo "GStreamer setup complete."
}

# Run setup
setup_gstreamer

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
