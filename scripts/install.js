#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import https from 'https';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const BINARY_NAME = 'pull_requests';
const REPO_OWNER = 'adalbertoteixeira'; // Update this to your GitHub username
const REPO_NAME = 'pull_requests'; // Update based on the new repository name

function getBinaryName() {
  const platform = process.platform;
  const arch = process.arch;
  
  const binaryName = `${BINARY_NAME}-${platform}-${arch}${platform === 'win32' ? '.exe' : ''}`;
  return binaryName;
}

function downloadFile(url, destPath) {
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
      } else if (response.statusCode === 200) {
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      } else {
        reject(new Error(`Failed to download: ${response.statusCode}`));
      }
    }).on('error', reject);
  });
}

async function getLatestRelease() {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'api.github.com',
      path: `/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`,
      headers: {
        'User-Agent': 'pull-requests-cli'
      }
    };

    https.get(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        try {
          const release = JSON.parse(data);
          resolve(release);
        } catch (error) {
          reject(error);
        }
      });
    }).on('error', reject);
  });
}

async function downloadBinary() {
  const binaryName = getBinaryName();
  
  console.log(`Downloading ${binaryName} from GitHub releases...`);
  
  try {
    const release = await getLatestRelease();
    const asset = release.assets.find(a => a.name === binaryName);
    
    if (!asset) {
      throw new Error(`Binary not found for platform: ${process.platform}-${process.arch}`);
    }
    
    // Create bin directory
    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
    
    const targetPath = path.join(binDir, BINARY_NAME + (process.platform === 'win32' ? '.exe' : ''));
    
    // Download the binary
    await downloadFile(asset.browser_download_url, targetPath);
    
    // Make it executable
    if (process.platform !== 'win32') {
      fs.chmodSync(targetPath, 0o755);
    }
    
    console.log(`✓ Successfully installed ${BINARY_NAME} v${release.tag_name}`);
  } catch (error) {
    console.error('Failed to download binary:', error.message);
    console.error('\nFalling back to building from source...');
    await buildFromSource();
  }
}

async function buildFromSource() {
  // Check if we're in development (Cargo.toml exists)
  const cargoTomlPath = path.join(__dirname, '..', 'Cargo.toml');
  if (!fs.existsSync(cargoTomlPath)) {
    console.error('Cannot build from source: Cargo.toml not found');
    console.error('Please install Rust and build manually:');
    console.error('1. Install Rust: https://www.rust-lang.org/tools/install');
    console.error('2. Clone the repository');
    console.error('3. Run: cargo build --release');
    process.exit(1);
  }
  
  console.log('Building from source...');
  try {
    execSync('cargo build --release', {
      stdio: 'inherit',
      cwd: path.join(__dirname, '..')
    });
    
    // Create bin directory
    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
    
    // Copy the built binary
    const releaseBinary = path.join(__dirname, '..', 'target', 'release', BINARY_NAME);
    const targetPath = path.join(binDir, BINARY_NAME + (process.platform === 'win32' ? '.exe' : ''));
    
    if (fs.existsSync(releaseBinary)) {
      fs.copyFileSync(releaseBinary, targetPath);
      fs.chmodSync(targetPath, 0o755);
      console.log(`✓ Built and installed ${BINARY_NAME} from source`);
    }
  } catch (error) {
    console.error('Failed to build from source:', error.message);
    console.error('Please ensure Rust is installed: https://www.rust-lang.org/tools/install');
    process.exit(1);
  }
}

// Main installation
downloadBinary().catch((error) => {
  console.error('Installation failed:', error);
  process.exit(1);
});