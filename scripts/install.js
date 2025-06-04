#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const BINARY_NAME = 'pull_requests';

function getBinaryName() {
  const platform = process.platform;
  const arch = process.arch;
  
  // Map platform names to match our binary naming convention
  let platformName = platform;
  if (platform === 'win32') {
    platformName = 'win32';
  } else if (platform === 'darwin') {
    platformName = 'darwin';
  } else if (platform === 'linux') {
    platformName = 'linux';
  }
  
  // Map architecture names to match our binary naming convention
  let archName = arch;
  if (arch === 'x64') {
    archName = 'x64';
  } else if (arch === 'arm64') {
    archName = 'arm64';
  }
  
  const binaryName = `${BINARY_NAME}-${platformName}-${archName}${platform === 'win32' ? '.exe' : ''}`;
  return binaryName;
}

function installLocalBinary() {
  const binaryName = getBinaryName();
  console.log(`Installing ${binaryName} from local binaries...`);
  
  // Path to the bundled binary
  const sourcePath = path.join(__dirname, '..', 'binaries', binaryName);
  
  // Check if the binary exists
  if (!fs.existsSync(sourcePath)) {
    console.error(`Binary not found for platform: ${process.platform}-${process.arch}`);
    console.error(`Expected binary: ${binaryName}`);
    console.error(`Looked in: ${sourcePath}`);
    console.error('Available binaries:');
    
    const binariesDir = path.join(__dirname, '..', 'binaries');
    if (fs.existsSync(binariesDir)) {
      const availableBinaries = fs.readdirSync(binariesDir);
      availableBinaries.forEach(binary => console.error(`  - ${binary}`));
    } else {
      console.error('  No binaries directory found');
    }
    
    process.exit(1);
  }
  
  // Create bin directory
  const binDir = path.join(__dirname, '..', 'bin');
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  const targetPath = path.join(binDir, BINARY_NAME + (process.platform === 'win32' ? '.exe' : ''));
  
  try {
    // Copy the binary to the bin directory
    fs.copyFileSync(sourcePath, targetPath);
    
    // Make it executable on Unix systems
    if (process.platform !== 'win32') {
      fs.chmodSync(targetPath, 0o755);
    }
    
    console.log(`✓ Successfully installed ${BINARY_NAME}`);
    console.log(`Binary location: ${targetPath}`);
  } catch (error) {
    console.error('Failed to install binary:', error.message);
    process.exit(1);
  }
}

// Main installation
try {
  installLocalBinary();
} catch (error) {
  console.error('Installation failed:', error.message);
  process.exit(1);
}