#!/usr/bin/env node

/**
 * postinstall.js - Platform-specific binary downloader for aiw
 *
 * Downloads the correct binary for the current platform from GitHub Releases.
 * Supports: Linux x64, Windows x64
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

// Configuration
const REPO_OWNER = 'putao520';
const REPO_NAME = 'agentic-warden';
const BIN_DIR = path.join(__dirname, '..', 'bin');

// Platform mapping
const PLATFORM_MAP = {
  'linux-x64': {
    filename: 'aiw-linux-x64',
    asset: 'aiw-linux-x64',
  },
  'win32-x64': {
    filename: 'aiw-windows-x64.exe',
    asset: 'aiw-windows-x64.exe',
  },
};

/**
 * Get package version from package.json
 */
function getPackageVersion() {
  const packageJson = require(path.join(__dirname, '..', 'package.json'));
  return packageJson.version;
}

/**
 * Get platform key for current system
 */
function getPlatformKey() {
  const platform = os.platform();
  const arch = os.arch();
  return `${platform}-${arch}`;
}

/**
 * Download file with redirect support
 */
function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    const request = (url) => {
      const protocol = url.startsWith('https') ? https : require('http');

      protocol.get(url, {
        headers: {
          'User-Agent': 'aiw-npm-installer',
        },
      }, (response) => {
        // Handle redirects (GitHub releases use 302)
        if (response.statusCode === 301 || response.statusCode === 302) {
          const redirectUrl = response.headers.location;
          if (redirectUrl) {
            request(redirectUrl);
            return;
          }
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Download failed with status ${response.statusCode}`));
          return;
        }

        const totalSize = parseInt(response.headers['content-length'], 10);
        let downloadedSize = 0;
        let lastProgress = 0;

        response.on('data', (chunk) => {
          downloadedSize += chunk.length;
          if (totalSize) {
            const progress = Math.floor((downloadedSize / totalSize) * 100);
            if (progress >= lastProgress + 10) {
              process.stdout.write(`\r  Downloading: ${progress}%`);
              lastProgress = progress;
            }
          }
        });

        response.pipe(file);

        file.on('finish', () => {
          file.close();
          console.log('\r  Downloading: 100%');
          resolve();
        });
      }).on('error', (err) => {
        fs.unlink(dest, () => {});
        reject(err);
      });
    };

    request(url);
  });
}

/**
 * Make file executable (Unix only)
 */
function makeExecutable(filepath) {
  if (os.platform() !== 'win32') {
    try {
      fs.chmodSync(filepath, 0o755);
    } catch (err) {
      console.warn(`  Warning: Could not set executable permission: ${err.message}`);
    }
  }
}

/**
 * Get binary version by executing it
 */
function getBinaryVersion(filepath) {
  try {
    const result = execSync(`"${filepath}" --version`, {
      encoding: 'utf8',
      timeout: 5000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    // Extract version number (e.g., "aiw 0.5.26" -> "0.5.26")
    const match = result.match(/(\d+\.\d+\.\d+)/);
    return match ? match[1] : null;
  } catch {
    return null;
  }
}

/**
 * Check if binary already exists and matches current package version
 */
function binaryExists(filepath, expectedVersion) {
  try {
    if (!fs.existsSync(filepath)) {
      return false;
    }

    // Check if file is executable and returns version
    const stat = fs.statSync(filepath);
    if (stat.size < 1000) {
      // File too small, probably corrupted
      return false;
    }

    // Check version match (critical for upgrades!)
    const binaryVersion = getBinaryVersion(filepath);
    if (binaryVersion && binaryVersion !== expectedVersion) {
      console.log(`  Existing binary version: ${binaryVersion}`);
      console.log(`  Expected version: ${expectedVersion}`);
      console.log('  Version mismatch, will download new binary.');
      return false;
    }

    return true;
  } catch {
    return false;
  }
}

/**
 * Main installation function
 */
async function install() {
  console.log('');
  console.log('  aiw postinstall: Setting up platform-specific binary...');
  console.log('');

  const platformKey = getPlatformKey();
  const platformInfo = PLATFORM_MAP[platformKey];

  if (!platformInfo) {
    console.log(`  Platform ${platformKey} is not supported.`);
    console.log('');
    console.log('  Supported platforms:');
    console.log('    - linux-x64');
    console.log('    - win32-x64');
    console.log('');
    console.log('  Alternative installation methods:');
    console.log('    1. Cargo: cargo install --git https://github.com/putao520/agentic-warden');
    console.log('    2. Download: https://github.com/putao520/agentic-warden/releases');
    console.log('');
    process.exit(0); // Don't fail npm install
  }

  // Ensure bin directory exists
  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }

  const binaryPath = path.join(BIN_DIR, platformInfo.filename);
  const version = getPackageVersion();

  // Check if binary already exists AND matches expected version
  if (binaryExists(binaryPath, version)) {
    console.log(`  Binary already exists: ${platformInfo.filename} (v${version})`);
    console.log('  Skipping download.');
    console.log('');
    return;
  }
  const downloadUrl = `https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/v${version}/${platformInfo.asset}`;

  console.log(`  Platform: ${platformKey}`);
  console.log(`  Version: v${version}`);
  console.log(`  Binary: ${platformInfo.filename}`);
  console.log('');

  try {
    console.log(`  Downloading from GitHub Releases...`);
    await downloadFile(downloadUrl, binaryPath);

    makeExecutable(binaryPath);

    console.log('');
    console.log('  Installation complete!');
    console.log(`  Binary installed: ${binaryPath}`);
    console.log('');
  } catch (err) {
    console.error('');
    console.error(`  Download failed: ${err.message}`);
    console.error('');
    console.error('  This might happen if:');
    console.error(`    - Release v${version} doesn't exist yet`);
    console.error('    - GitHub is unreachable');
    console.error('    - Network issues');
    console.error('');
    console.error('  Alternative installation:');
    console.error('    cargo install --git https://github.com/putao520/agentic-warden');
    console.error('');

    // Clean up partial download
    try {
      if (fs.existsSync(binaryPath)) {
        fs.unlinkSync(binaryPath);
      }
    } catch {}

    // Don't fail npm install, just warn
    process.exit(0);
  }
}

// Run installation
install().catch((err) => {
  console.error('  Unexpected error:', err.message);
  process.exit(0); // Don't fail npm install
});
