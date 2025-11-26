#!/bin/bash
set -e

# Navigate to the script directory (ios/)
cd "$(dirname "$0")"

# --- CONFIGURATION ---
GST_VERSION="1.26.8"
GST_IOS_PKG_URL="https://gstreamer.freedesktop.org/data/pkg/ios/${GST_VERSION}/gstreamer-1.0-devel-${GST_VERSION}-ios-universal.pkg"
FRAMEWORKS_DIR="$(pwd)/Frameworks"
GST_FRAMEWORK_PATH="${FRAMEWORKS_DIR}/GStreamer.framework"
SYSTEM_GST_PATH="/Library/Frameworks/GStreamer.framework"

# --- SETUP GSTREAMER ---

# 1. Check system install
if [ -d "$SYSTEM_GST_PATH" ]; then
    echo "Found GStreamer in system path: $SYSTEM_GST_PATH"
    exit 0
fi

# 2. Check local install
if [ -d "$GST_FRAMEWORK_PATH" ]; then
    echo "Found GStreamer locally: $GST_FRAMEWORK_PATH"
    exit 0
fi

# 3. Check standard user install location
USER_GST_PATH="$HOME/Library/Developer/GStreamer/iPhone.sdk/GStreamer.framework"
if [ -d "$USER_GST_PATH" ]; then
    echo "Found GStreamer in user path: $USER_GST_PATH"
    echo "Copying to local frameworks..."
    mkdir -p "$FRAMEWORKS_DIR"
    cp -R "$USER_GST_PATH" "$FRAMEWORKS_DIR/"
    exit 0
fi

echo "GStreamer framework not found. Downloading version ${GST_VERSION}..."

mkdir -p "$FRAMEWORKS_DIR"
TMP_DIR=$(mktemp -d)
PKG_FILE="$TMP_DIR/gstreamer.pkg"

# Download
curl -L -o "$PKG_FILE" "$GST_IOS_PKG_URL"

echo "Unpacking GStreamer framework..."
cd "$TMP_DIR"

# Use pkgutil to expand the package (handles pbzx compression correctly)
# This is safer than xar + gunzip as modern pkgs often use pbzx
pkgutil --expand-full "$PKG_FILE" expanded

# Move Framework
SOURCE_FW=$(find expanded -name "GStreamer.framework" | head -n 1)

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
