#!/bin/bash
set -e

# Package script for Agentic-Warden
# Bundles the binary with ONNX Runtime library for distribution

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}📦 Packaging Agentic-Warden${NC}"
echo ""

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)
        PLATFORM="linux"
        LIB_NAME="libonnxruntime.so.1.20.0"
        ;;
    Darwin*)
        PLATFORM="macos"
        LIB_NAME="libonnxruntime.1.20.0.dylib"
        ;;
    *)
        echo "❌ Unsupported OS: $OS"
        exit 1
        ;;
esac

echo -e "${YELLOW}Platform:${NC} $PLATFORM-$ARCH"

# Check if binary exists
BINARY_PATH="target/release/aiw"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    echo "   Run 'cargo build --release' first"
    exit 1
fi

# Find ORT library
ORT_LIB=""
if [ -n "$ORT_LIB_LOCATION" ] && [ -d "$ORT_LIB_LOCATION" ]; then
    ORT_LIB="$ORT_LIB_LOCATION/lib/$LIB_NAME"
elif [ -d "vendor/onnxruntime-${PLATFORM}-${ARCH}-1.20.0" ]; then
    ORT_LIB="vendor/onnxruntime-${PLATFORM}-${ARCH}-1.20.0/lib/$LIB_NAME"
fi

if [ -z "$ORT_LIB" ] || [ ! -f "$ORT_LIB" ]; then
    echo "❌ ORT library not found"
    echo "   Run './scripts/setup-ort.sh' first"
    exit 1
fi

# Create distribution directory
DIST_DIR="dist/aiw-${PLATFORM}-${ARCH}"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo ""
echo -e "${GREEN}📋 Copying files...${NC}"

# Copy binary
cp "$BINARY_PATH" "$DIST_DIR/aiw"
echo "  ✓ Binary: aiw"

# Copy ORT library
cp "$ORT_LIB" "$DIST_DIR/"
echo "  ✓ ORT library: $LIB_NAME"

# Copy documentation
cp README.md "$DIST_DIR/" 2>/dev/null || true
cp LICENSE "$DIST_DIR/" 2>/dev/null || true
echo "  ✓ Documentation"

# Create run script
cat > "$DIST_DIR/aiw-run.sh" << 'EOF'
#!/bin/bash
# Wrapper script to set library path

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Set library path based on platform
case "$(uname -s)" in
    Linux*)
        export LD_LIBRARY_PATH="$SCRIPT_DIR:$LD_LIBRARY_PATH"
        ;;
    Darwin*)
        export DYLD_LIBRARY_PATH="$SCRIPT_DIR:$DYLD_LIBRARY_PATH"
        ;;
esac

# Also set ORT_DYLIB_PATH for extra safety
export ORT_DYLIB_PATH="$SCRIPT_DIR/libonnxruntime"*

# Run the binary
exec "$SCRIPT_DIR/aiw" "$@"
EOF

chmod +x "$DIST_DIR/aiw-run.sh"
echo "  ✓ Run script: aiw-run.sh"

# Create README
cat > "$DIST_DIR/INSTALL.txt" << EOF
Agentic-Warden ${PLATFORM}-${ARCH}

INSTALLATION:
1. Extract this archive to your desired location
2. Run: ./aiw-run.sh

DIRECT USAGE (requires setting library path):
  export LD_LIBRARY_PATH=\$(pwd):\$LD_LIBRARY_PATH
  ./aiw mcp

SYSTEM INSTALL (optional):
  sudo cp aiw /usr/local/bin/
  sudo cp $LIB_NAME /usr/local/lib/
  sudo ldconfig  # Linux only

For more information, see README.md or visit:
https://github.com/putao520/agentic-warden
EOF

echo "  ✓ Install instructions: INSTALL.txt"

# Create tarball
cd dist
TARBALL="aiw-${PLATFORM}-${ARCH}.tar.gz"
tar -czf "$TARBALL" "aiw-${PLATFORM}-${ARCH}"
cd ..

echo ""
echo -e "${GREEN}✅ Package created successfully!${NC}"
echo ""
echo "Distribution: dist/aiw-${PLATFORM}-${ARCH}/"
echo "Tarball:      dist/$TARBALL"
echo ""
echo "Contents:"
ls -lh "dist/aiw-${PLATFORM}-${ARCH}/"
echo ""
echo "To test the package:"
echo "  cd dist/aiw-${PLATFORM}-${ARCH}"
echo "  ./aiw-run.sh --version"
