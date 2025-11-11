#!/usr/bin/env node

const path = require('path');
const { spawn } = require('child_process');
const os = require('os');

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === 'linux' && arch === 'x64') {
    return 'x86_64-unknown-linux-gnu';
  }
  if (platform === 'linux' && arch === 'arm64') {
    return 'aarch64-unknown-linux-gnu';
  }
  if (platform === 'darwin' && arch === 'x64') {
    return 'x86_64-apple-darwin';
  }
  if (platform === 'darwin' && arch === 'arm64') {
    return 'aarch64-apple-darwin';
  }
  if (platform === 'win32' && arch === 'x64') {
    return 'x86_64-pc-windows-msvc';
  }

  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function getBinaryName() {
  return os.platform() === 'win32' ? 'agentic-warden.exe' : 'agentic-warden';
}

function main() {
  try {
    const platform = getPlatform();
    const binaryName = getBinaryName();
    const binaryPath = path.join(__dirname, '..', 'binaries', platform, binaryName);

    // Check if binary exists
    if (!require('fs').existsSync(binaryPath)) {
      console.error(`Binary not found at: ${binaryPath}`);
      console.error('');
      console.error('Please run: npm install');
      console.error('Or install manually from: https://github.com/putao520/agentic-warden/releases');
      process.exit(1);
    }

    // Execute the binary with all arguments
    const child = spawn(binaryPath, process.argv.slice(2), {
      stdio: 'inherit',
      env: process.env
    });

    child.on('exit', (code) => {
      process.exit(code || 0);
    });

    child.on('error', (err) => {
      console.error('Failed to start agentic-warden:', err.message);
      process.exit(1);
    });

  } catch (error) {
    console.error('Error:', error.message);
    process.exit(1);
  }
}

main();
