#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const os = require('os');

// 根据架构选择正确的二进制
const arch = os.arch();
const platform = os.platform();

let binaryPath;

if (platform === 'linux') {
  switch (arch) {
    case 'x64':
      binaryPath = path.join(__dirname, 'bin', 'aiw-linux-x64');
      break;
    default:
      console.error(`❌ Unsupported architecture: ${arch}`);
      console.error('Supported: x64');
      process.exit(1);
  }
} else if (platform === 'win32') {
  switch (arch) {
    case 'x64':
      binaryPath = path.join(__dirname, 'bin', 'aiw-windows-x64.exe');
      break;
    default:
      console.error(`❌ Unsupported architecture: ${arch}`);
      console.error('Supported: x64');
      process.exit(1);
  }
} else {
  console.error(`❌ Unsupported platform: ${platform}`);
  console.error('Supported: linux, win32');
  console.error('\nAlternative options:');
  console.error('1. Using Cargo: cargo install --git https://github.com/putao520/agentic-warden');
  console.error('2. Download pre-built: https://github.com/putao520/agentic-warden/releases');
  process.exit(1);
}

// Execute binary and pass all arguments
const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

process.exit(result.status || 0);
