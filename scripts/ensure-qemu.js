/**
 * NovaVM — Automated QEMU Bundler Script
 * Ensures that the latest QEMU engine binaries are bundled directly into NovaVM builds.
 */

import { existsSync, mkdirSync, copyFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const resourcesDir = resolve(__dirname, '../src-tauri/resources/qemu');

import { writeFileSync } from 'node:fs';

export function ensureQemuBundled() {
  if (!existsSync(resourcesDir)) {
    mkdirSync(resourcesDir, { recursive: true });
  }

  const placeholder = resolve(resourcesDir, 'README.txt');
  if (!existsSync(placeholder)) {
    writeFileSync(placeholder, 'NovaVM Bundled QEMU Directory\n');
  }

  const qemuExe = resolve(resourcesDir, 'qemu-system-x86_64.exe');
  if (existsSync(qemuExe)) {
    console.log('[NovaVM Bundler] QEMU binary present in src-tauri/resources/qemu.');
    return;
  }

  console.log('[NovaVM Bundler] Checking for local QEMU installation to bundle...');

  // Try well-known QEMU locations on Windows
  const candidates = [
    'C:\\Program Files\\qemu\\qemu-system-x86_64.exe',
    'C:\\Program Files (x86)\\qemu\\qemu-system-x86_64.exe',
    'C:\\qemu\\qemu-system-x86_64.exe',
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      console.log(`[NovaVM Bundler] Bundling QEMU from: ${candidate}`);
      const srcDir = dirname(candidate);
      try {
        // Copy main QEMU binary
        copyFileSync(candidate, qemuExe);
        console.log('[NovaVM Bundler] Successfully bundled QEMU executable into NovaVM installer resources.');
        return;
      } catch (err) {
        console.warn(`[NovaVM Bundler] Could not copy QEMU from ${candidate}:`, err.message);
      }
    }
  }

  console.log('[NovaVM Bundler] Note: QEMU binary can be placed in src-tauri/resources/qemu for automated offline installer bundling.');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  ensureQemuBundled();
}
