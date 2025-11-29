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
    case 'arm64':
      binaryPath = path.join(__dirname, 'bin', 'aiw-linux-arm64');
      break;
    case 'arm':
      binaryPath = path.join(__dirname, 'bin', 'aiw-linux-armv7');
      break;
    default:
      console.error(`❌ Unsupported architecture: ${arch}`);
      console.error('Supported: x64, arm64, arm');
      process.exit(1);
  }
} else {
  console.error(`❌ Unsupported platform: ${platform}`);
  console.error('Currently only Linux is supported.');
  console.error('\nAlternative options:');
  console.error('1. Using Cargo: cargo install --git https://github.com/putao520/agentic-warden');
  console.error('2. Download pre-built: https://github.com/putao520/agentic-warden/releases');
  process.exit(1);
}

// 执行二进制并传递所有参数
const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

process.exit(result.status || 0);
