import { create } from 'zustand'
import { monitorApi } from '@/lib/api'
import type { HostMetrics, VmMetrics } from '@/types'

const MAX_HISTORY = 60

interface MetricsStore {
  hostMetrics: HostMetrics | null
  hostHistory: HostMetrics[]
  vmMetrics: Record<string, VmMetrics>
  vmHistory: Record<string, VmMetrics[]>
  pollingActive: boolean
  intervalId: ReturnType<typeof setInterval> | null

  fetchHostMetrics: () => Promise<void>
  fetchVmMetrics: (vmId: string) => Promise<void>
  startPolling: (intervalMs?: number) => void
  stopPolling: () => void
}

export const useMetricsStore = create<MetricsStore>((set, get) => ({
  hostMetrics: null,
  hostHistory: [],
  vmMetrics: {},
  vmHistory: {},
  pollingActive: false,
  intervalId: null,

  fetchHostMetrics: async () => {
    try {
      const metrics = await monitorApi.getHostMetrics()
      set((state) => ({
        hostMetrics: metrics,
        hostHistory: [...state.hostHistory.slice(-MAX_HISTORY + 1), metrics],
      }))
    } catch {
      // Silently ignore — metrics failures shouldn't crash the UI
    }
  },

  fetchVmMetrics: async (vmId: string) => {
    try {
      const metrics = await monitorApi.getVmMetrics(vmId)
      set((state) => ({
        vmMetrics: { ...state.vmMetrics, [vmId]: metrics },
        vmHistory: {
          ...state.vmHistory,
          [vmId]: [...(state.vmHistory[vmId] ?? []).slice(-MAX_HISTORY + 1), metrics],
        },
      }))
    } catch {
      // VM may not be running yet
    }
  },

  startPolling: (intervalMs = 1000) => {
    const { pollingActive, stopPolling } = get()
    if (pollingActive) stopPolling()

    const id = setInterval(() => {
      get().fetchHostMetrics()
    }, intervalMs)

    set({ pollingActive: true, intervalId: id })
  },

  stopPolling: () => {
    const { intervalId } = get()
    if (intervalId !== null) clearInterval(intervalId)
    set({ pollingActive: false, intervalId: null })
  },
}))
