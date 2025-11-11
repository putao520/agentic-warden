#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');

const PACKAGE_VERSION = require('../package.json').version;
const REPO_OWNER = 'putao520';
const REPO_NAME = 'agentic-warden';

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

async function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (redirectResponse) => {
          redirectResponse.pipe(file);
          file.on('finish', () => {
            file.close();
            resolve();
          });
        }).on('error', reject);
      } else {
        if (response.statusCode !== 200) {
          reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
          return;
        }
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      }
    }).on('error', (err) => {
      fs.unlink(destPath, () => {});
      reject(err);
    });

    file.on('error', (err) => {
      fs.unlink(destPath, () => {});
      reject(err);
    });
  });
}

async function findReleaseAsset() {
  const platform = getPlatform();
  const binaryName = getBinaryName();
  const assetName = `agentic-warden-${PACKAGE_VERSION}-${platform}.tar.gz`;

  // 检查是否使用本地预构建的二进制文件（用于开发和测试）
  const localBinaryPath = path.join(__dirname, '..', 'binaries', platform, binaryName);
  if (fs.existsSync(localBinaryPath)) {
    console.log(`Found local binary at: ${localBinaryPath}`);
    return { local: true, path: localBinaryPath };
  }

  // 否则从 GitHub Releases 下载
  const releaseUrl = `https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/v${PACKAGE_VERSION}/${assetName}`;
  console.log(`Downloading from: ${releaseUrl}`);
  return { local: false, url: releaseUrl };
}

async function extractBinary(tarPath, destDir, binaryName) {
  return new Promise((resolve, reject) => {
    try {
      // 创建目标目录
      if (!fs.existsSync(destDir)) {
        fs.mkdirSync(destDir, { recursive: true });
      }

      // 解压 tar.gz
      execSync(`tar -xzf "${tarPath}" -C "${destDir}"`, { stdio: 'inherit' });

      // 设置执行权限
      const binaryPath = path.join(destDir, binaryName);
      if (os.platform() !== 'win32') {
        fs.chmodSync(binaryPath, '755');
      }

      resolve(binaryPath);
    } catch (error) {
      reject(error);
    }
  });
}

async function main() {
  try {
    console.log('Installing agentic-warden...');

    const platform = getPlatform();
    const binaryName = getBinaryName();
    const asset = await findReleaseAsset();

    const binariesDir = path.join(__dirname, '..', 'binaries', platform);
    const binaryPath = path.join(binariesDir, binaryName);

    if (asset.local) {
      // 使用本地预构建的二进制文件
      console.log(`Installing local binary to: ${binaryPath}`);
      if (!fs.existsSync(binariesDir)) {
        fs.mkdirSync(binariesDir, { recursive: true });
      }
      fs.copyFileSync(asset.path, binaryPath);
      fs.chmodSync(binaryPath, '755');
    } else {
      // 从 GitHub Releases 下载
      const tempPath = path.join(os.tmpdir(), `agentic-warden-${Date.now()}.tar.gz`);

      console.log(`Downloading agentic-warden v${PACKAGE_VERSION} for ${platform}...`);
      await downloadFile(asset.url, tempPath);

      console.log(`Extracting to: ${binaryPath}`);
      await extractBinary(tempPath, binariesDir, binaryName);

      // 清理临时文件
      fs.unlinkSync(tempPath);
    }

    console.log(`Successfully installed agentic-warden v${PACKAGE_VERSION}`);
    console.log(`Binary location: ${binaryPath}`);

  } catch (error) {
    console.error('Failed to install agentic-warden:', error.message);
    console.error('\nYou can also install manually:');
    console.error('1. Download the binary from: https://github.com/putao520/agentic-warden/releases');
    console.error('2. Build from source: cargo install --path .');
    process.exit(1);
  }
}

// 如果直接运行此脚本
if (require.main === module) {
  main();
}

module.exports = { getPlatform, getBinaryName, downloadFile };
