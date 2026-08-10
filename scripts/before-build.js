import { existsSync } from 'node:fs'
import { execSync, spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import os from 'node:os'
import { syncAllMetadata } from './metadata.js'
import { ensureQemuBundled } from './ensure-qemu.js'

const __dirname = dirname(fileURLToPath(import.meta.url))

// 1. Sync all metadata dynamically from Cargo.toml or environment variables
const meta = syncAllMetadata()

const PRODUCT_NAME = 'NovaVM'
const VERSION      = meta.version
const DEVELOPER    = meta.developer
const HOMEPAGE     = meta.homepage
const COPYRIGHT    = meta.copyright

console.log('')
console.log('╔══════════════════════════════════════════════════════╗')
console.log(`║  ${PRODUCT_NAME} v${VERSION.padEnd(8)} — Build Pipeline                   ║`)
console.log(`║  Developer  : ${DEVELOPER.padEnd(39)}║`)
console.log(`║  Homepage   : ${HOMEPAGE.padEnd(39)}║`)
console.log(`║  Copyright  : ${COPYRIGHT.padEnd(39)}║`)
console.log(`║  Platform   : ${(os.platform() + ' ' + os.arch()).padEnd(39)}║`)
console.log('╚══════════════════════════════════════════════════════╝')
console.log('')

// 2. Ensure QEMU is bundled for offline NovaVM installer
try {
  ensureQemuBundled()
} catch (e) {
  console.warn('[NovaVM Build] QEMU bundler notice:', e.message)
}

// 3. Build frontend
const rootFrontend   = resolve(process.cwd(), 'frontend')
const parentFrontend = resolve(process.cwd(), '../frontend')
const targetDir = existsSync(rootFrontend)
  ? rootFrontend
  : existsSync(parentFrontend)
    ? parentFrontend
    : 'frontend'

console.log(`[NovaVM Build] Building frontend in: ${targetDir}`)
try {
  const result = spawnSync('npm', ['run', 'build', '--prefix', 'frontend'], { stdio: 'inherit', shell: true })
  if (result.status !== 0) {
    console.error(`[NovaVM Build] Frontend build failed with exit code ${result.status}`)
    process.exit(result.status || 1)
  }
} catch (e) {
  console.error('[NovaVM Build] Frontend build handle error:', e.message)
  process.exit(1)
}
console.log('[NovaVM Build] Frontend build complete.')

// 3. Run signing setup script
const signScript = resolve(__dirname, '../src-tauri/scripts/sign.js')
const parentSignScript = resolve(__dirname, './sign.js')
const signPath = existsSync(signScript)
  ? signScript
  : existsSync(parentSignScript)
    ? parentSignScript
    : null

if (signPath) {
  console.log('[NovaVM Build] Running code signing setup...')
  try {
    execSync(`node "${signPath}"`, { stdio: 'inherit' })
  } catch (e) {
    console.warn('[NovaVM Build] Signing setup skipped:', e.message)
  }
} else {
  console.log('[NovaVM Build] sign.js not found — skipping signing setup.')
}

console.log('')
console.log('[NovaVM Build] Pre-build steps complete. Tauri will now package the app.')
