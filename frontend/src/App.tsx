import { useEffect } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'

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

import { ErrorBoundary } from '@/components/ErrorBoundary'

export default function App() {
  useEffect(() => {
    // Initial data load
    useVmStore.getState().fetchVms()

    // Start metrics polling every second
    useMetricsStore.getState().startPolling(1000)

    // Refresh VM list every 5 seconds
    const vmRefresh = setInterval(() => {
      useVmStore.getState().fetchVms()
    }, 5000)

    return () => {
      useMetricsStore.getState().stopPolling()
      clearInterval(vmRefresh)
    }
  }, [])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        const current = useUiStore.getState().commandPaletteOpen
        useUiStore.getState().setCommandPaletteOpen(!current)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [])

  return (
    <ThemeProvider>
      <ToastProvider>
        <BrowserRouter>
          <CommandPalette />
          <Layout>
            <ErrorBoundary>
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
            </ErrorBoundary>
          </Layout>
        </BrowserRouter>
      </ToastProvider>
    </ThemeProvider>
  )
}
