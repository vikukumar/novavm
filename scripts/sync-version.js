import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const rootDir = path.resolve(__dirname, '..')

const targetVersion = process.argv[2]

if (!targetVersion) {
  console.error('Error: Version argument required (e.g. node ./scripts/sync-version.js 1.0.1)')
  process.exit(1)
}

// Clean version string (remove leading 'v' if present)
const cleanVersion = targetVersion.replace(/^v/, '')

console.log(`[Version Sync] Updating workspace version to ${cleanVersion}...`)

// 1. Update root package.json
const rootPkgPath = path.join(rootDir, 'package.json')
if (fs.existsSync(rootPkgPath)) {
  const pkg = JSON.parse(fs.readFileSync(rootPkgPath, 'utf8'))
  pkg.version = cleanVersion
  fs.writeFileSync(rootPkgPath, JSON.stringify(pkg, null, 2) + '\n')
  console.log(`  ✓ Updated package.json`)
}

// 2. Update frontend/package.json
const frontendPkgPath = path.join(rootDir, 'frontend', 'package.json')
if (fs.existsSync(frontendPkgPath)) {
  const pkg = JSON.parse(fs.readFileSync(frontendPkgPath, 'utf8'))
  pkg.version = cleanVersion
  fs.writeFileSync(frontendPkgPath, JSON.stringify(pkg, null, 2) + '\n')
  console.log(`  ✓ Updated frontend/package.json`)
}

// 3. Update src-tauri/tauri.conf.json
const tauriConfPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json')
if (fs.existsSync(tauriConfPath)) {
  const conf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'))
  conf.version = cleanVersion
  fs.writeFileSync(tauriConfPath, JSON.stringify(conf, null, 2) + '\n')
  console.log(`  ✓ Updated src-tauri/tauri.conf.json`)
}

// 4. Update root Cargo.toml [workspace.package] version
const cargoTomlPath = path.join(rootDir, 'Cargo.toml')
if (fs.existsSync(cargoTomlPath)) {
  let content = fs.readFileSync(cargoTomlPath, 'utf8')
  content = content.replace(
    /(\[workspace\.package\][\s\S]*?version\s*=\s*")[^"]+(")/,
    `$1${cleanVersion}$2`
  )
  fs.writeFileSync(cargoTomlPath, content)
  console.log(`  ✓ Updated Cargo.toml [workspace.package]`)
}

console.log(`[Version Sync] Successfully synced version ${cleanVersion} across all manifests!`)
