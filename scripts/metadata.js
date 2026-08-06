import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

/**
 * Dynamically extract project metadata from Cargo.toml or CI/CD environment variables.
 * Priority order for Version:
 *   1. Explicit version argument passed to script
 *   2. process.env.APP_VERSION
 *   3. process.env.BUILD_VERSION
 *   4. process.env.VERSION
 *   5. process.env.TAG_NAME
 *   6. process.env.GITHUB_REF / GITHUB_REF_NAME (CI pipeline)
 *   7. Cargo.toml [workspace.package] version
 */
export function getProjectMetadata(explicitVersion) {
  let rootDir = path.resolve(__dirname, '..')

  let curr = rootDir
  while (curr !== path.dirname(curr)) {
    const candidate = path.join(curr, 'Cargo.toml')
    if (fs.existsSync(candidate) && fs.readFileSync(candidate, 'utf8').includes('[workspace]')) {
      rootDir = curr
      break
    }
    curr = path.dirname(curr)
  }

  let cargoVersion = '1.0.0'
  let cargoAuthors = 'Vikash Kumar'
  let cargoHomepage = 'https://vikukumar.github.io'
  let cargoRepo = 'https://github.com/vikukumar/novavm'

  const cargoPath = path.join(rootDir, 'Cargo.toml')
  if (fs.existsSync(cargoPath)) {
    const cargoStr = fs.readFileSync(cargoPath, 'utf8')
    const verMatch = cargoStr.match(/version\s*=\s*"([^"]+)"/)
    if (verMatch) cargoVersion = verMatch[1]

    const authorMatch = cargoStr.match(/authors\s*=\s*\[\s*"([^"<]+)(?:<[^>]+>)??"\s*\]/)
    if (authorMatch) cargoAuthors = authorMatch[1].trim()

    const homeMatch = cargoStr.match(/homepage\s*=\s*"([^"]+)"/)
    if (homeMatch) cargoHomepage = homeMatch[1]

    const repoMatch = cargoStr.match(/repository\s*=\s*"([^"]+)"/)
    if (repoMatch) cargoRepo = repoMatch[1]
  }

  let githubRefVer = null
  if (process.env.GITHUB_REF && process.env.GITHUB_REF.includes('/tags/')) {
    githubRefVer = process.env.GITHUB_REF.split('/tags/').pop().replace(/^v/, '')
  } else if (process.env.GITHUB_REF_NAME) {
    githubRefVer = process.env.GITHUB_REF_NAME.replace(/^v/, '')
  }

  const rawVersion = explicitVersion ||
    process.env.APP_VERSION ||
    process.env.BUILD_VERSION ||
    process.env.VERSION ||
    process.env.TAG_NAME ||
    githubRefVer ||
    cargoVersion

  const version = rawVersion.replace(/^v/, '')
  const developer = process.env.AUTHOR || process.env.DEVELOPER || process.env.BUILD_AUTHOR || cargoAuthors || 'Vikash Kumar'
  const homepage = process.env.HOMEPAGE || cargoHomepage || 'https://vikukumar.github.io'
  const repository = process.env.REPOSITORY || cargoRepo || 'https://github.com/vikukumar/novavm'
  const year = process.env.YEAR || process.env.BUILD_YEAR || new Date().getFullYear().toString()
  const copyright = `© ${year} ${developer}. All rights reserved.`

  return {
    rootDir,
    version,
    developer,
    homepage,
    repository,
    year,
    copyright,
  }
}

/**
 * Synchronize version and branding across all manifest files, NSIS installer headers,
 * and Linux post-install scripts dynamically.
 */
export function syncAllMetadata(explicitVersion) {
  const meta = getProjectMetadata(explicitVersion)
  const { rootDir, version, developer, homepage, repository, copyright } = meta

  console.log(`[Metadata Sync] Resolving project metadata...`)
  console.log(`  • Version    : ${version}`)
  console.log(`  • Developer  : ${developer}`)
  console.log(`  • Homepage   : ${homepage}`)
  console.log(`  • Copyright  : ${copyright}`)

  // 1. Root package.json
  const rootPkgPath = path.join(rootDir, 'package.json')
  if (fs.existsSync(rootPkgPath)) {
    const pkg = JSON.parse(fs.readFileSync(rootPkgPath, 'utf8'))
    pkg.version = version
    pkg.author = developer
    pkg.homepage = homepage
    fs.writeFileSync(rootPkgPath, JSON.stringify(pkg, null, 2) + '\n')
    console.log(`  ✓ Updated package.json`)
  }

  // 2. Frontend package.json
  const frontendPkgPath = path.join(rootDir, 'frontend', 'package.json')
  if (fs.existsSync(frontendPkgPath)) {
    const pkg = JSON.parse(fs.readFileSync(frontendPkgPath, 'utf8'))
    pkg.version = version
    pkg.author = developer
    pkg.homepage = homepage
    fs.writeFileSync(frontendPkgPath, JSON.stringify(pkg, null, 2) + '\n')
    console.log(`  ✓ Updated frontend/package.json`)
  }

  // 3. src-tauri/tauri.conf.json
  const tauriConfPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json')
  if (fs.existsSync(tauriConfPath)) {
    const conf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'))
    conf.version = version
    conf.bundle = conf.bundle || {}
    conf.bundle.copyright = copyright
    conf.bundle.publisher = developer
    conf.bundle.longDescription = `NovaVM is a hardware-accelerated virtual machine manager built by ${developer} (${homepage}). It uses Windows Hypervisor Platform (WHP) on Windows, KVM on Linux, and Apple Virtualization Framework on macOS — no third-party hypervisor required.`
    conf.plugins = conf.plugins || {}
    conf.plugins.updater = conf.plugins.updater || {}
    conf.plugins.updater.endpoints = [
      `${homepage.replace(/\/$/, '')}/novavm/releases/update/{{target}}/{{arch}}/{{current_version}}.json`
    ]
    fs.writeFileSync(tauriConfPath, JSON.stringify(conf, null, 2) + '\n')
    console.log(`  ✓ Updated src-tauri/tauri.conf.json`)
  }

  // 4. Cargo.toml [workspace.package]
  const cargoTomlPath = path.join(rootDir, 'Cargo.toml')
  if (fs.existsSync(cargoTomlPath)) {
    let content = fs.readFileSync(cargoTomlPath, 'utf8')
    content = content.replace(
      /(\[workspace\.package\][\s\S]*?version\s*=\s*")[^"]+(")/,
      `$1${version}$2`
    )
    content = content.replace(
      /(\[workspace\.package\][\s\S]*?homepage\s*=\s*")[^"]+(")/,
      `$1${homepage}$2`
    )
    content = content.replace(
      /(\[workspace\.package\][\s\S]*?repository\s*=\s*")[^"]+(")/,
      `$1${repository}$2`
    )
    fs.writeFileSync(cargoTomlPath, content)
    console.log(`  ✓ Updated Cargo.toml [workspace.package]`)
  }

  // 5. Generate src-tauri/nsis/generated_branding.nsh
  const nsisDir = path.join(rootDir, 'src-tauri', 'nsis')
  if (!fs.existsSync(nsisDir)) {
    fs.mkdirSync(nsisDir, { recursive: true })
  }
  const generatedNshPath = path.join(nsisDir, 'generated_branding.nsh')
  const nshContent = `; Auto-generated by NovaVM metadata sync script — DO NOT EDIT MANUALLY
!define NOVAVM_PUBLISHER    "${developer}"
!define NOVAVM_URL          "${homepage}"
!define NOVAVM_SUPPORT_URL  "${homepage.replace(/\/$/, '')}/support"
!define NOVAVM_UPDATE_URL   "${homepage.replace(/\/$/, '')}/releases"
!define NOVAVM_ABOUT_URL    "${homepage}"
!define NOVAVM_VERSION      "${version}"
!define NOVAVM_COPYRIGHT    "${copyright}"
!define NOVAVM_REG_KEY      "Software\\VikashKumar\\NovaVM"
`
  fs.writeFileSync(generatedNshPath, nshContent)
  console.log(`  ✓ Generated src-tauri/nsis/generated_branding.nsh`)

  // 6. Update src-tauri/scripts/linux-post-install.sh
  const linuxScriptPath = path.join(rootDir, 'src-tauri', 'scripts', 'linux-post-install.sh')
  if (fs.existsSync(linuxScriptPath)) {
    let scriptContent = fs.readFileSync(linuxScriptPath, 'utf8')
    scriptContent = scriptContent.replace(/PRODUCT_NAME="[^"]*"/, `PRODUCT_NAME="NovaVM"`)
    scriptContent = scriptContent.replace(/DEVELOPER="[^"]*"/, `DEVELOPER="${developer}"`)
    scriptContent = scriptContent.replace(/HOMEPAGE="[^"]*"/, `HOMEPAGE="${homepage}"`)
    scriptContent = scriptContent.replace(/VERSION="[^"]*"/, `VERSION="${version}"`)
    fs.writeFileSync(linuxScriptPath, scriptContent)
    console.log(`  ✓ Updated src-tauri/scripts/linux-post-install.sh`)
  }

  return meta
}
