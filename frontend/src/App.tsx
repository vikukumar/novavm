import { useEffect } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { AnimatePresence } from 'framer-motion'

import { Layout } from '@/components/layout/Layout'
import { CommandPalette } from '@/components/layout/CommandPalette'
import { ThemeProvider } from '@/components/layout/ThemeProvider'
import { ToastProvider } from '@/components/ui/toast-provider'

import { DashboardPage } from '@/pages/DashboardPage'
import { VmListPage } from '@/pages/VmListPage'
import { VmDetailPage } from '@/pages/VmDetailPage'
import { CreateVmWizard } from '@/pages/CreateVmWizard'
import { StoragePage } from '@/pages/StoragePage'
import { NetworkPage } from '@/pages/NetworkPage'
import { SnapshotPage } from '@/pages/SnapshotPage'
import { LogsPage } from '@/pages/LogsPage'
import { SettingsPage } from '@/pages/SettingsPage'

import { useVmStore } from '@/stores/vmStore'
import { useMetricsStore } from '@/stores/metricsStore'
import { useUiStore } from '@/stores/uiStore'

export default function App() {
  const fetchVms = useVmStore((s) => s.fetchVms)
  const startPolling = useMetricsStore((s) => s.startPolling)
  const stopPolling = useMetricsStore((s) => s.stopPolling)

  // Global keyboard shortcuts
  const setCommandPaletteOpen = useUiStore((s) => s.setCommandPaletteOpen)
  const commandPaletteOpen = useUiStore((s) => s.commandPaletteOpen)

  useEffect(() => {
    // Initial data load
    fetchVms()

    // Start metrics polling every second
    startPolling(1000)

    // Refresh VM list every 5 seconds
    const vmRefresh = setInterval(fetchVms, 5000)

    return () => {
      stopPolling()
      clearInterval(vmRefresh)
    }
  }, [fetchVms, startPolling, stopPolling])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setCommandPaletteOpen(!commandPaletteOpen)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [commandPaletteOpen, setCommandPaletteOpen])

  return (
    <ThemeProvider>
      <ToastProvider>
        <BrowserRouter>
          <CommandPalette />
          <Layout>
            <AnimatePresence mode="wait">
              <Routes>
                <Route path="/" element={<Navigate to="/dashboard" replace />} />
                <Route path="/dashboard" element={<DashboardPage />} />
                <Route path="/vms" element={<VmListPage />} />
                <Route path="/vms/create" element={<CreateVmWizard />} />
                <Route path="/vms/:id" element={<VmDetailPage />} />
                <Route path="/storage" element={<StoragePage />} />
                <Route path="/network" element={<NetworkPage />} />
                <Route path="/snapshots" element={<SnapshotPage />} />
                <Route path="/logs" element={<LogsPage />} />
                <Route path="/settings" element={<SettingsPage />} />
              </Routes>
            </AnimatePresence>
          </Layout>
        </BrowserRouter>
      </ToastProvider>
    </ThemeProvider>
  )
}
