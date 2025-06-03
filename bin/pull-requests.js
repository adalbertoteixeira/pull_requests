#!/usr/bin/env node

import { spawn } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const binaryName = 'pull_requests' + (process.platform === 'win32' ? '.exe' : '');
const binaryPath = path.join(__dirname, binaryName);

// Pass all arguments to the binary
const args = process.argv.slice(2);

const child = spawn(binaryPath, args, {
  stdio: 'inherit',
  env: process.env
});

child.on('error', (err) => {
  if (err.code === 'ENOENT') {
    console.error('pull_requests binary not found!');
    console.error('Please try reinstalling the package:');
    console.error('  npm install @commit-message/cli');
  } else {
    console.error('Failed to start pull_requests:', err.message);
  }
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code);
  }
});
