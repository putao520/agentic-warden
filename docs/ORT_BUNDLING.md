# ONNX Runtime Bundling Guide

This document explains how to bundle ONNX Runtime (ORT) with Agentic-Warden to avoid runtime dependencies on user systems.

## Problem

By default, the `ort` crate tries to download ONNX Runtime binaries during the build process. This can fail due to:
- Network connectivity issues
- TLS certificate problems
- Firewall restrictions
- Offline build environments

## Solution: Pre-download and Bundle ORT

### Step 1: Download ONNX Runtime

Download the appropriate ONNX Runtime binary for your target platform:

```bash
# For Linux x86_64
wget https://github.com/microsoft/onnxruntime/releases/download/v1.20.0/onnxruntime-linux-x64-1.20.0.tgz
tar -xzf onnxruntime-linux-x64-1.20.0.tgz

# For macOS x86_64
wget https://github.com/microsoft/onnxruntime/releases/download/v1.20.0/onnxruntime-osx-x64-1.20.0.tgz
tar -xzf onnxruntime-osx-x64-1.20.0.tgz

# For macOS ARM64
wget https://github.com/microsoft/onnxruntime/releases/download/v1.20.0/onnxruntime-osx-arm64-1.20.0.tgz
tar -xzf onnxruntime-osx-arm64-1.20.0.tgz

# For Windows x64
# Download https://github.com/microsoft/onnxruntime/releases/download/v1.20.0/onnxruntime-win-x64-1.20.0.zip
# Extract the zip file
```

### Step 2: Set Environment Variables for Build

Set the `ORT_LIB_LOCATION` environment variable to point to the extracted directory:

```bash
# Linux/macOS
export ORT_LIB_LOCATION=/path/to/onnxruntime-linux-x64-1.20.0

# Windows (PowerShell)
$env:ORT_LIB_LOCATION="C:\path\to\onnxruntime-win-x64-1.20.0"

# Windows (CMD)
set ORT_LIB_LOCATION=C:\path\to\onnxruntime-win-x64-1.20.0
```

### Step 3: Build the Project

```bash
cargo build --release
```

### Step 4: Bundle ORT with the Binary

The `copy-dylibs` feature will automatically copy the ONNX Runtime shared libraries to the target directory.

For distribution, you need to bundle:

**Linux:**
- `libonnxruntime.so.1.20.0` (or current version)
- Set `LD_LIBRARY_PATH` or use `rpath` to find it

**macOS:**
- `libonnxruntime.1.20.0.dylib`
- Set `DYLD_LIBRARY_PATH` or use `@rpath`

**Windows:**
- `onnxruntime.dll`
- Place in the same directory as the executable

### Step 5: Runtime Configuration

If bundling the library in a custom location, set `ORT_DYLIB_PATH` at runtime:

```bash
# Linux/macOS
export ORT_DYLIB_PATH=/opt/agentic-warden/lib/libonnxruntime.so

# Windows
set ORT_DYLIB_PATH=C:\Program Files\agentic-warden\lib\onnxruntime.dll
```

## Alternative: Static Linking

For fully static binaries without runtime dependencies:

1. Build ONNX Runtime from source with static linking
2. Set `ORT_LIB_LOCATION` to the static build directory
3. Use `ORT_STRATEGY=system` environment variable

## Automated Build Script

Create a `build.sh` script for automated builds:

```bash
#!/bin/bash

# Download ORT if not present
ORT_VERSION=1.20.0
ORT_DIR="./vendor/onnxruntime-linux-x64-${ORT_VERSION}"

if [ ! -d "$ORT_DIR" ]; then
    echo "Downloading ONNX Runtime ${ORT_VERSION}..."
    mkdir -p ./vendor
    wget -O /tmp/ort.tgz \
        "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-linux-x64-${ORT_VERSION}.tgz"
    tar -xzf /tmp/ort.tgz -C ./vendor
fi

# Build with bundled ORT
export ORT_LIB_LOCATION="$ORT_DIR"
cargo build --release

# Copy ORT library to target
cp "$ORT_DIR/lib/libonnxruntime.so.${ORT_VERSION}" ./target/release/

echo "Build complete. Binary and ORT library are in ./target/release/"
```

## CI/CD Integration

For GitHub Actions or other CI systems:

```yaml
- name: Download and cache ONNX Runtime
  run: |
    ORT_VERSION=1.20.0
    wget https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-linux-x64-${ORT_VERSION}.tgz
    tar -xzf onnxruntime-linux-x64-${ORT_VERSION}.tgz
    echo "ORT_LIB_LOCATION=$(pwd)/onnxruntime-linux-x64-${ORT_VERSION}" >> $GITHUB_ENV

- name: Build project
  run: cargo build --release

- name: Package with ORT library
  run: |
    mkdir -p dist
    cp target/release/aiw dist/
    cp ${ORT_LIB_LOCATION}/lib/libonnxruntime.so.* dist/
```

## Troubleshooting

### Build fails with "Failed to GET parcel.pyke.io"

This means ORT is trying to download binaries. Solution:
- Set `ORT_LIB_LOCATION` environment variable
- Ensure the path points to the extracted ORT directory (not the lib subdirectory)

### Runtime error: "cannot find libonnxruntime.so"

Solutions:
- Set `ORT_DYLIB_PATH` to the library location
- Copy the library to a system library path (`/usr/local/lib`)
- Set `LD_LIBRARY_PATH` (Linux) or `DYLD_LIBRARY_PATH` (macOS)
- Place the library in the same directory as the binary (Windows)

### Version mismatch errors

Ensure the ORT version matches what the `ort` crate expects:
- Check `Cargo.lock` for the exact `ort-sys` version
- Download the corresponding ORT release

## References

- [ONNX Runtime Releases](https://github.com/microsoft/onnxruntime/releases)
- [ort crate documentation](https://docs.rs/ort)
- [ort linking guide](https://ort.pyke.io/setup/linking)
