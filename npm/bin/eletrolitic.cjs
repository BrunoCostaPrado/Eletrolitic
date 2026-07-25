#!/usr/bin/env node
const { execFileSync } = require('child_process');
const { existsSync } = require('fs');
const { join } = require('path');
const { platform, arch } = require('os');

const ext = platform() === 'win32' ? '.exe' : '';
const os = platform();
const cpu = arch() === 'arm64' ? 'arm64' : 'x64';
const binDir = join(__dirname, '..', 'bin', `${os}-${cpu}`);
const binPath = join(binDir, `eletrolitic${ext}`);

if (!existsSync(binPath)) {
  console.error(`eletrolitic: binary not found for ${os}-${cpu}. Please install from source.`);
  process.exit(1);
}

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
} catch (e) {
  process.exit(e.status || 1);
}
