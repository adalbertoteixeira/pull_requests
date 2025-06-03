#!/usr/bin/env node

import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const BINARY_NAME = 'pull_requests';

console.log('Building pull_requests for local development...\n');

try {
  // Build the binary
  execSync('cargo build --release', {
    stdio: 'inherit',
    cwd: path.join(__dirname, '..')
  });
  
  // Create bin directory if it doesn't exist
  const binDir = path.join(__dirname, '..', 'bin');
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  // Copy the binary to bin directory
  const sourcePath = path.join(__dirname, '..', 'target', 'release', BINARY_NAME);
  const targetPath = path.join(binDir, BINARY_NAME + (process.platform === 'win32' ? '.exe' : ''));
  
  if (fs.existsSync(sourcePath)) {
    fs.copyFileSync(sourcePath, targetPath);
    fs.chmodSync(targetPath, 0o755);
    console.log(`✓ Built ${BINARY_NAME} for local development`);
    console.log(`Binary location: ${targetPath}`);
  } else {
    console.error(`✗ Binary not found at ${sourcePath}`);
    process.exit(1);
  }
} catch (error) {
  console.error(`✗ Failed to build:`, error.message);
  process.exit(1);
}

console.log('\nNote: For production releases, use GitHub Actions to build and upload binaries.');