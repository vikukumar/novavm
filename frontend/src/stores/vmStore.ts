import { create } from 'zustand'
import { vmApi } from '@/lib/api'
import type { VmSummary, VmConfig } from '@/types'

interface VmStore {
  vms: VmSummary[]
  selectedVmId: string | null
  loading: boolean
  error: string | null

  // Actions
  fetchVms: () => Promise<void>
  selectVm: (id: string | null) => void
  createVm: (config: VmConfig) => Promise<string>
  startVm: (id: string) => Promise<void>
  pauseVm: (id: string) => Promise<void>
  resumeVm: (id: string) => Promise<void>
  stopVm: (id: string) => Promise<void>
  resetVm: (id: string) => Promise<void>
  destroyVm: (id: string) => Promise<void>
}

export const useVmStore = create<VmStore>((set, get) => ({
  vms: [],
  selectedVmId: null,
  loading: false,
  error: null,

  fetchVms: async () => {
    set({ loading: true, error: null })
    try {
      const vms = await vmApi.list()
      set({ vms, loading: false })
    } catch (e) {
      set({ error: String(e), loading: false })
    }
  },

  selectVm: (id) => set({ selectedVmId: id }),

  createVm: async (config) => {
    const result = await vmApi.create(config)
    await get().fetchVms()
    const id = result?.vm_id || (result as unknown as { vmId?: string })?.vmId
    if (!id) {
      throw new Error('VM creation completed but backend did not return a valid VM ID.')
    }
    return String(id)
  },

  startVm: async (id) => {
    await vmApi.start(id)
    await get().fetchVms()
  },

  pauseVm: async (id) => {
    await vmApi.pause(id)
    await get().fetchVms()
  },

  resumeVm: async (id) => {
    await vmApi.resume(id)
    await get().fetchVms()
  },

  stopVm: async (id) => {
    await vmApi.stop(id)
    await get().fetchVms()
  },

  resetVm: async (id) => {
    await vmApi.reset(id)
    await get().fetchVms()
  },

  destroyVm: async (id) => {
    await vmApi.destroy(id)
    set((state) => ({
      vms: state.vms.filter((v) => v.id !== id),
      selectedVmId: state.selectedVmId === id ? null : state.selectedVmId,
    }))
  },
}))
