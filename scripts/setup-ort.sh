#!/bin/bash
set -e

# ONNX Runtime setup script for Agentic-Warden
# Downloads and configures ONNX Runtime for building the project

ORT_VERSION="1.20.0"
VENDOR_DIR="./vendor"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)
        case "$ARCH" in
            x86_64)
                ORT_PACKAGE="onnxruntime-linux-x64-${ORT_VERSION}"
                ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_PACKAGE}.tgz"
                LIB_EXT="so.${ORT_VERSION}"
                ;;
            aarch64)
                ORT_PACKAGE="onnxruntime-linux-aarch64-${ORT_VERSION}"
                ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_PACKAGE}.tgz"
                LIB_EXT="so.${ORT_VERSION}"
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    Darwin*)
        case "$ARCH" in
            x86_64)
                ORT_PACKAGE="onnxruntime-osx-x64-${ORT_VERSION}"
                ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_PACKAGE}.tgz"
                LIB_EXT="dylib"
                ;;
            arm64)
                ORT_PACKAGE="onnxruntime-osx-arm64-${ORT_VERSION}"
                ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_PACKAGE}.tgz"
                LIB_EXT="dylib"
                ;;
            *)
                echo "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        echo "For Windows, please download manually from:"
        echo "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-win-x64-${ORT_VERSION}.zip"
        exit 1
        ;;
esac

ORT_DIR="${VENDOR_DIR}/${ORT_PACKAGE}"
ORT_ARCHIVE="/tmp/${ORT_PACKAGE}.tgz"

# Check if already downloaded
if [ -d "$ORT_DIR" ]; then
    echo "✅ ONNX Runtime already downloaded: $ORT_DIR"
else
    echo "📥 Downloading ONNX Runtime ${ORT_VERSION} for ${OS} ${ARCH}..."
    mkdir -p "$VENDOR_DIR"

    if command -v curl &> /dev/null; then
        curl -L -o "$ORT_ARCHIVE" "$ORT_URL"
    elif command -v wget &> /dev/null; then
        wget -O "$ORT_ARCHIVE" "$ORT_URL"
    else
        echo "❌ Error: Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    echo "📦 Extracting archive..."
    tar -xzf "$ORT_ARCHIVE" -C "$VENDOR_DIR"
    rm "$ORT_ARCHIVE"

    echo "✅ ONNX Runtime downloaded and extracted to: $ORT_DIR"
fi

# Export environment variable
export ORT_LIB_LOCATION="$(cd "$ORT_DIR" && pwd)"

echo ""
echo "🎉 ONNX Runtime setup complete!"
echo ""
echo "To build the project, run:"
echo "  export ORT_LIB_LOCATION=\"$ORT_LIB_LOCATION\""
echo "  cargo build --release"
echo ""
echo "Or source this script to set the environment variable:"
echo "  source $0"
echo ""

# If sourced, the export will persist
if [ "${BASH_SOURCE[0]}" != "${0}" ]; then
    echo "✅ Environment variable set for current shell session"
fi
