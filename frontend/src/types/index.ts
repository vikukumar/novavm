// ─── VM Types ────────────────────────────────────────────────────────────────

export type VmState =
  | 'stopped'
  | 'starting'
  | 'running'
  | 'paused'
  | 'crashed'
  | 'saving'
  | 'restoring'
  | 'cloning'
  | 'destroying'

export type FirmwareType = 'bios' | 'uefi'

export type DiskBus = 'virtio' | 'scsi' | 'ide' | 'nvme'

export type NicType = 'virtio' | 'e1000' | 'rtl8139'

export type VirtualSwitchMode = 'nat' | 'bridged' | 'host_only' | 'internal'

export interface CpuConfig {
  vcpus: number
  sockets: number
  cores_per_socket: number
  threads_per_core: number
  overcommit_ratio: number
}

export interface MemoryConfig {
  size_mib: number
  dynamic_min_mib: number
  dynamic_max_mib: number
  ballooning: boolean
  huge_pages: boolean
}

export interface DiskConfig {
  image_path: string
  bus: DiskBus
  read_only: boolean
  boot: boolean
}

export interface NicConfig {
  switch_name: string
  nic_type: NicType
  mac_address: string | null
}

export interface SharedFolderConfig {
  name: string
  host_path: string
  read_only: boolean
  auto_mount: boolean
}

export interface VmConfig {
  name: string
  description: string | null
  cpu: CpuConfig
  memory: MemoryConfig
  firmware: FirmwareType
  secure_boot: boolean
  vtpm: boolean
  disks: DiskConfig[]
  nics: NicConfig[]
  shared_folders: SharedFolderConfig[]
  tags: string[]
  group: string | null
}

export interface VmSummary {
  id: string
  name: string
  state: VmState
  cpu_vcpus: number
  memory_mib: number
  tags: string[]
  group: string | null
  created_at: string
  updated_at: string
}

// ─── Metrics Types ───────────────────────────────────────────────────────────

export interface HostMetrics {
  cpu_percent: number
  memory_total_mib: number
  memory_used_mib: number
  swap_total_mib: number
  swap_used_mib: number
  per_cpu_percent: number[]
  disk_read_bytes: number
  disk_write_bytes: number
  net_rx_bytes: number
  net_tx_bytes: number
  timestamp: number
}

export interface VmMetrics {
  vm_id: string
  cpu_percent: number
  memory_used_mib: number
  disk_read_bytes: number
  disk_write_bytes: number
  net_rx_bytes: number
  net_tx_bytes: number
  timestamp: number
}

// ─── Storage Types ───────────────────────────────────────────────────────────

export type DiskFormat = 'nova_disk' | 'raw' | 'qcow2'

export interface DiskMetadata {
  id: string
  name: string
  virtual_size_bytes: number
  cluster_size_bytes: number
  allocated_clusters: number
  total_clusters: number
  format: DiskFormat
  encrypted: boolean
  compressed: boolean
  thin_provisioned: boolean
  created_at: string
  updated_at: string
  parent_snapshot_id: string | null
}

// ─── Network Types ───────────────────────────────────────────────────────────

export interface BandwidthLimit {
  rx_mbps: number
  tx_mbps: number
}

export interface VirtualSwitch {
  id: string
  name: string
  mode: VirtualSwitchMode
  subnet: string
  gateway: string
  dhcp_enabled: boolean
  dhcp_range_start: string
  dhcp_range_end: string
  dns_servers: string[]
  bandwidth_limit: BandwidthLimit | null
  connected_vms: number
  ipv6_enabled: boolean
}

// ─── Snapshot Types ──────────────────────────────────────────────────────────

export interface SnapshotMetadata {
  id: string
  disk_id: string
  name: string
  description: string | null
  taken_at: string
  parent_id: string | null
  shared_clusters: number
  private_clusters: number
  exported: boolean
}

export interface SnapshotResult {
  vm_id: string
  snapshot_id: string
  name: string
  duration_ms: number
}

// ─── Settings Types ──────────────────────────────────────────────────────────

export type Theme = 'light' | 'dark' | 'system'

export interface AppSettings {
  theme: Theme
  default_storage_dir: string
  default_iso_dir: string
  auto_start_service: boolean
  metrics_interval_secs: number
  telemetry_enabled: boolean
  language: string
}

// ─── API Types ───────────────────────────────────────────────────────────────

export interface ApiError {
  code: string
  message: string
}

export interface HypervisorInfo {
  backend_name: string
  backend_version: string
  secure_boot: boolean
  vtpm: boolean
  nested_virt: boolean
  huge_pages: boolean
  memory_ballooning: boolean
  memory_dedup: boolean
  usb_redirection: boolean
}

// ─── UI Types ────────────────────────────────────────────────────────────────

export interface NavItem {
  id: string
  label: string
  icon: string
  path: string
  badge?: number
}

export interface Notification {
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  title: string
  message: string
  timestamp: number
  read: boolean
}
