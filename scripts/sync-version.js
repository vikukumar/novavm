import { syncAllMetadata } from './metadata.js'

const targetVersion = process.argv[2]

if (targetVersion) {
  console.log(`[Version Sync] Explicit version provided: ${targetVersion}`)
} else {
  console.log(`[Version Sync] No explicit version argument provided — resolving dynamically from environment / Cargo.toml...`)
}

const meta = syncAllMetadata(targetVersion)

console.log(`[Version Sync] Successfully synced version ${meta.version} across all manifests and installer configurations!`)
