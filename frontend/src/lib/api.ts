/**
 * Tauri IPC invoke wrappers.
 *
 * All backend communication goes through this module. Each function maps
 * directly to a Tauri command registered in src-tauri/src/main.rs.
 */

import { invoke } from '@tauri-apps/api/core'
import type {
  ApiError,
  AppSettings,
  DiskMetadata,
  HostMetrics,
  HypervisorInfo,
  SnapshotResult,
  VirtualSwitch,
  VirtualSwitchMode,
  VmConfig,
  VmMetrics,
  VmSummary,
} from '@/types'

// ─── Generic error handling ───────────────────────────────────────────────────

export class NovaError extends Error {
  code: string

  constructor(err: ApiError) {
    super(err.message)
    this.name = 'NovaError'
    this.code = err.code
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (e) {
    if (e && typeof e === 'object' && 'code' in e && 'message' in e) {
      throw new NovaError(e as ApiError)
    }
    throw e
  }
}

// ─── VM Commands ─────────────────────────────────────────────────────────────

export const vmApi = {
  list: (): Promise<VmSummary[]> => call('list_vms'),

  get: (vmId: string): Promise<VmSummary> => call('get_vm', { vmId }),

  create: (config: VmConfig): Promise<{ vm_id: string }> =>
    call('create_vm', { request: { config } }),

  updateConfig: (vmId: string, config: VmConfig): Promise<void> =>
    call('update_vm_config', { vmId, config }),

  start: (vmId: string): Promise<void> => call('start_vm', { vmId }),

  pause: (vmId: string): Promise<void> => call('pause_vm', { vmId }),

  resume: (vmId: string): Promise<void> => call('resume_vm', { vmId }),

  stop: (vmId: string): Promise<void> => call('stop_vm', { vmId }),

  reset: (vmId: string): Promise<void> => call('reset_vm', { vmId }),

  destroy: (vmId: string): Promise<void> => call('destroy_vm', { vmId }),
}

// ─── Monitor Commands ─────────────────────────────────────────────────────────

export const monitorApi = {
  getHostMetrics: (): Promise<HostMetrics> => call('get_host_metrics'),

  getHostHistory: (): Promise<HostMetrics[]> => call('get_host_metrics_history'),

  getVmMetrics: (vmId: string): Promise<VmMetrics> => call('get_vm_metrics', { vmId }),

  getVmHistory: (vmId: string): Promise<VmMetrics[]> =>
    call('get_vm_metrics_history', { vmId }),
}

// ─── Network Commands ─────────────────────────────────────────────────────────

export const networkApi = {
  listSwitches: (): Promise<VirtualSwitch[]> => call('list_switches'),

  createSwitch: (name: string, mode: VirtualSwitchMode): Promise<string> =>
    call('create_switch', { name, mode }),

  deleteSwitch: (name: string): Promise<void> => call('delete_switch', { name }),
}

// ─── Storage Commands ─────────────────────────────────────────────────────────

export const storageApi = {
  listDisks: (): Promise<DiskMetadata[]> => call('list_disks'),

  createDisk: (
    name: string,
    path: string,
    sizeGib: number,
    encrypted: boolean,
    compressed: boolean,
  ): Promise<DiskMetadata> =>
    call('create_disk', { name, path, sizeGib, encrypted, compressed }),
}

// ─── Snapshot Commands ────────────────────────────────────────────────────────

export const snapshotApi = {
  take: (
    vmId: string,
    name: string,
    description?: string,
  ): Promise<SnapshotResult> =>
    call('take_snapshot', { vmId, name, description: description ?? null }),
}

// ─── Settings Commands ────────────────────────────────────────────────────────

export const settingsApi = {
  get: (): Promise<AppSettings> => call('get_settings'),

  update: (settings: AppSettings): Promise<void> =>
    call('update_settings', { settings }),

  getVersion: (): Promise<string> => call('get_app_version'),

  getHypervisorInfo: (): Promise<HypervisorInfo> => call('get_hypervisor_info'),
}
