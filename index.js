#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

const arch = os.arch();
const platform = os.platform();

let binaryName;
let binaryPath;

// Determine binary name based on platform
if (platform === 'linux' && arch === 'x64') {
  binaryName = 'aiw-linux-x64';
} else if (platform === 'win32' && arch === 'x64') {
  binaryName = 'aiw-windows-x64.exe';
} else {
  console.error(`Error: Unsupported platform: ${platform}-${arch}`);
  console.error('');
  console.error('Supported platforms:');
  console.error('  - linux-x64');
  console.error('  - win32-x64');
  console.error('');
  console.error('Alternative installation:');
  console.error('  cargo install --git https://github.com/putao520/agentic-warden');
  console.error('  https://github.com/putao520/agentic-warden/releases');
  process.exit(1);
}

binaryPath = path.join(__dirname, 'bin', binaryName);

// Check if binary exists
if (!fs.existsSync(binaryPath)) {
  console.error(`Error: Binary not found: ${binaryPath}`);
  console.error('');
  console.error('The postinstall script may have failed to download the binary.');
  console.error('');
  console.error('Try reinstalling:');
  console.error('  npm uninstall -g @putao520/aiw && npm install -g @putao520/aiw');
  console.error('');
  console.error('Or run postinstall manually:');
  console.error(`  node ${path.join(__dirname, 'scripts', 'postinstall.js')}`);
  console.error('');
  console.error('Alternative installation:');
  console.error('  cargo install --git https://github.com/putao520/agentic-warden');
  process.exit(1);
}

// Execute binary and pass all arguments
const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

process.exit(result.status || 0);
