/**
 * NovaVM Code Signing Script
 * Developer : Vikash Kumar <https://vikukumar.github.io>
 * Product   : NovaVM
 *
 * This script is called by Tauri's build pipeline before packaging.
 * It sets up the correct signing environment variables for each platform
 * with metadata dynamically resolved from Cargo.toml or pipeline env.
 */

import { execSync } from 'node:child_process'
import os from 'node:os'
import { getProjectMetadata } from '../../scripts/metadata.js'

const meta = getProjectMetadata()
const DEVELOPER     = meta.developer
const HOMEPAGE      = meta.homepage
const PRODUCT_NAME  = 'NovaVM'
const VERSION       = meta.version
const COPYRIGHT     = meta.copyright
const TIMESTAMP_URL = process.env.TIMESTAMP_URL || 'http://timestamp.digicert.com'

console.log(`[NovaVM Sign] ${PRODUCT_NAME} v${VERSION} — ${DEVELOPER}`)
console.log(`[NovaVM Sign] Homepage: ${HOMEPAGE}`)
console.log(`[NovaVM Sign] Copyright: ${COPYRIGHT}`)
console.log(`[NovaVM Sign] Platform: ${os.platform()} ${os.arch()}`)

// ── Windows signing ────────────────────────────────────────────────────────────
if (os.platform() === 'win32') {
  const thumbprint = process.env.WINDOWS_CERTIFICATE_THUMBPRINT
  const pfxPath    = process.env.WINDOWS_CERTIFICATE_PATH
  const pfxPass    = process.env.WINDOWS_CERTIFICATE_PASSWORD

  if (thumbprint) {
    console.log(`[NovaVM Sign] Windows: using certificate thumbprint ${thumbprint}`)
    process.env.WINDOWS_CERTIFICATE_THUMBPRINT = thumbprint
    process.env.WINDOWS_DIGEST_ALGORITHM = 'sha256'
    process.env.WINDOWS_TIMESTAMP_URL = TIMESTAMP_URL
  } else if (pfxPath) {
    console.log(`[NovaVM Sign] Windows: using .pfx certificate at ${pfxPath}`)
    process.env.WINDOWS_CERTIFICATE_PATH = pfxPath
    if (pfxPass) process.env.WINDOWS_CERTIFICATE_PASSWORD = pfxPass
    process.env.WINDOWS_DIGEST_ALGORITHM = 'sha256'
    process.env.WINDOWS_TIMESTAMP_URL = TIMESTAMP_URL
  } else {
    console.log('[NovaVM Sign] Windows: no certificate configured in environment.')
    console.log('[NovaVM Sign] Set WINDOWS_CERTIFICATE_THUMBPRINT or WINDOWS_CERTIFICATE_PATH in CI.')
    console.log('[NovaVM Sign] The installer will be unsigned (development build).')
  }
}

// ── macOS signing ──────────────────────────────────────────────────────────────
if (os.platform() === 'darwin') {
  const identity = process.env.APPLE_SIGNING_IDENTITY
  if (identity) {
    console.log(`[NovaVM Sign] macOS: signing with identity "${identity}"`)
    process.env.APPLE_SIGNING_IDENTITY = identity
  } else {
    console.log('[NovaVM Sign] macOS: no signing identity configured (set APPLE_SIGNING_IDENTITY).')
  }

  if (process.env.APPLE_CERTIFICATE) {
    const p12Path = '/tmp/novavm_cert.p12'
    const fs = await import('node:fs')
    fs.writeFileSync(p12Path, Buffer.from(process.env.APPLE_CERTIFICATE, 'base64'))
    try {
      execSync(
        `security import "${p12Path}" -P "${process.env.APPLE_CERTIFICATE_PASSWORD || ''}" ` +
        `-k ~/Library/Keychains/login.keychain-db -A -T /usr/bin/codesign`,
        { stdio: 'inherit' }
      )
      console.log('[NovaVM Sign] macOS: certificate imported to keychain.')
    } catch (e) {
      console.warn('[NovaVM Sign] macOS: certificate import failed:', e.message)
    }
  }
}

// ── Tauri updater signing key ──────────────────────────────────────────────────
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  console.log('[NovaVM Sign] TAURI_SIGNING_PRIVATE_KEY not set — update packages will not be signed.')
}

console.log(`[NovaVM Sign] Signing setup complete for ${PRODUCT_NAME} v${VERSION}.`)
