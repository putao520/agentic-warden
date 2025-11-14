# ONNX Runtime Bundling Guide

This document explains how to bundle ONNX Runtime (ORT) with Agentic-Warden for distribution.

## Background

Agentic-Warden uses `fastembed` for generating embeddings, which depends on ONNX Runtime (ORT). By default, ORT tries to download binaries during build, which can fail due to network/TLS issues.

**Our Solution:** Pre-download ORT library and bundle it with the binary for distribution.

## Quick Start

### 1. One-Time Setup

Download ONNX Runtime for your platform:

```bash
./scripts/setup-ort.sh
```

This downloads ORT to `vendor/onnxruntime-<platform>-<arch>-1.20.0/`

### 2. Build

Set the environment variable and build:

```bash
export ORT_LIB_LOCATION="$(pwd)/vendor/onnxruntime-linux-x64-1.20.0"
cargo build --release
```

### 3. Package for Distribution

Create a distributable package:

```bash
./scripts/package.sh
```

This creates:
- `dist/aiw-<platform>-<arch>/` - Directory with binary + ORT library
- `dist/aiw-<platform>-<arch>.tar.gz` - Tarball for distribution

## How It Works

### Build Time

1. `fastembed` crate depends on `ort`
2. `ort` needs ONNX Runtime libraries
3. By setting `ORT_LIB_LOCATION`, we tell `ort` to use our pre-downloaded library instead of downloading

### Runtime

The packaged distribution includes:
- `aiw` - The binary
- `libonnxruntime.so.1.20.0` (Linux) or `libonnxruntime.1.20.0.dylib` (macOS) - ORT library
- `aiw-run.sh` - Wrapper script that sets library paths

Users can run either:
```bash
# Using the wrapper (recommended)
./aiw-run.sh mcp

# Or set library path manually
export LD_LIBRARY_PATH=$(pwd):$LD_LIBRARY_PATH
./aiw mcp
```

## Platform Support

### Linux (x86_64 / aarch64)

ORT Library: `libonnxruntime.so.1.20.0`

```bash
# Build
export ORT_LIB_LOCATION="$(pwd)/vendor/onnxruntime-linux-x64-1.20.0"
cargo build --release

# Run
export LD_LIBRARY_PATH=./dist/aiw-linux-x86_64:$LD_LIBRARY_PATH
./dist/aiw-linux-x86_64/aiw mcp
```

### macOS (x86_64 / arm64)

ORT Library: `libonnxruntime.1.20.0.dylib`

```bash
# Build
export ORT_LIB_LOCATION="$(pwd)/vendor/onnxruntime-osx-arm64-1.20.0"
cargo build --release

# Run
export DYLD_LIBRARY_PATH=./dist/aiw-macos-arm64:$DYLD_LIBRARY_PATH
./dist/aiw-macos-arm64/aiw mcp
```

### Windows (x64)

ORT Library: `onnxruntime.dll`

Download manually from: https://github.com/microsoft/onnxruntime/releases/download/v1.20.0/onnxruntime-win-x64-1.20.0.zip

```powershell
# Build
$env:ORT_LIB_LOCATION = "C:\path\to\onnxruntime-win-x64-1.20.0"
cargo build --release

# The DLL must be in the same directory as the .exe
copy vendor\onnxruntime-win-x64-1.20.0\lib\onnxruntime.dll target\release\
```

## System Installation (Optional)

For system-wide installation without library path setup:

### Linux
```bash
sudo cp dist/aiw-linux-x86_64/aiw /usr/local/bin/
sudo cp dist/aiw-linux-x86_64/libonnxruntime.so.1.20.0 /usr/local/lib/
sudo ldconfig
```

### macOS
```bash
sudo cp dist/aiw-macos-arm64/aiw /usr/local/bin/
sudo cp dist/aiw-macos-arm64/libonnxruntime.1.20.0.dylib /usr/local/lib/
```

### Windows
Place both `aiw.exe` and `onnxruntime.dll` in the same directory (e.g., `C:\Program Files\aiw\`)

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Setup ONNX Runtime
  run: |
    ./scripts/setup-ort.sh
    echo "ORT_LIB_LOCATION=$(pwd)/vendor/onnxruntime-linux-x64-1.20.0" >> $GITHUB_ENV

- name: Build
  run: cargo build --release

- name: Package
  run: ./scripts/package.sh

- name: Upload Artifact
  uses: actions/upload-artifact@v3
  with:
    name: aiw-linux-x64
    path: dist/aiw-linux-x86_64.tar.gz
```

## Troubleshooting

### Build Error: "Failed to GET parcel.pyke.io"

**Problem:** ORT is trying to download binaries instead of using local library.

**Solution:** Ensure `ORT_LIB_LOCATION` is set:
```bash
export ORT_LIB_LOCATION="$(pwd)/vendor/onnxruntime-linux-x64-1.20.0"
```

### Runtime Error: "cannot find libonnxruntime.so"

**Problem:** System can't find the ORT library.

**Solutions:**

1. Use the wrapper script:
   ```bash
   ./aiw-run.sh mcp
   ```

2. Or set library path manually:
   ```bash
   export LD_LIBRARY_PATH=/path/to/aiw/directory:$LD_LIBRARY_PATH
   ./aiw mcp
   ```

3. Or install system-wide (see System Installation above)

### Version Mismatch

If you see ORT version errors:

1. Check the version in `vendor/`:
   ```bash
   cat vendor/onnxruntime-*/VERSION_NUMBER
   ```

2. Ensure it matches what `fastembed` expects (currently 1.20.0)

## Development vs Distribution

### Development Build
```bash
# One-time setup
./scripts/setup-ort.sh

# Every build
export ORT_LIB_LOCATION="$(pwd)/vendor/onnxruntime-linux-x64-1.20.0"
cargo build --release
```

### Distribution Package
```bash
# Creates self-contained package
./scripts/package.sh

# Test the package
cd dist/aiw-linux-x86_64
./aiw-run.sh --version
```

## Why Not Static Linking?

Microsoft only provides **dynamic libraries** (.so, .dylib, .dll) in official releases. Static linking would require:
- Compiling ORT from source (2-3 hours)
- Separate builds for each platform
- Much larger binaries (~150MB vs ~20MB)

Our dynamic library bundling approach:
- ✅ Uses official prebuilt binaries
- ✅ Fast builds (no ORT compilation)
- ✅ Cross-platform (Linux/macOS/Windows)
- ✅ Reasonable size (~70MB total: 20MB binary + 50MB ORT)
- ✅ Self-contained packages

## References

- [ONNX Runtime Releases](https://github.com/microsoft/onnxruntime/releases)
- [fastembed-rs](https://github.com/Anush008/fastembed-rs)
- [ort crate documentation](https://docs.rs/ort)
