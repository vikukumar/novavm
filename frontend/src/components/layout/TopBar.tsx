import { useLocation } from 'react-router-dom'
import { Search, Bell, Sun, Moon, Monitor, Plus } from 'lucide-react'
import { useNavigate } from 'react-router-dom'

import { useUiStore } from '@/stores/uiStore'
import { useMetricsStore } from '@/stores/metricsStore'
import { cn, formatPercent, formatMib } from '@/lib/utils'
import type { Theme } from '@/types'

const PAGE_TITLES: [string, string][] = [
  ['/vms/create', 'Create VM'],
  ['/vms', 'Virtual Machines'],
  ['/dashboard', 'Dashboard'],
  ['/storage', 'Storage'],
  ['/network', 'Network'],
  ['/snapshots', 'Snapshots'],
  ['/logs', 'Logs'],
  ['/settings', 'Settings'],
]

export function TopBar() {
  const location = useLocation()
  const navigate = useNavigate()
  const setCommandPaletteOpen = useUiStore((s) => s.setCommandPaletteOpen)
  const theme = useUiStore((s) => s.theme)
  const setTheme = useUiStore((s) => s.setTheme)
  const notifications = useUiStore((s) => s.notifications)
  const hostMetrics = useMetricsStore((s) => s.hostMetrics)

  const title =
    PAGE_TITLES.find(([path]) =>
      location.pathname === path || location.pathname.startsWith(path + '/'),
    )?.[1] ?? 'NovaVM'

  const unreadCount = notifications.filter((n) => !n.read).length

  const themeIcons: Record<Theme, React.ReactNode> = {
    light: <Sun size={16} />,
    dark: <Moon size={16} />,
    system: <Monitor size={16} />,
  }

  const cycleTheme = () => {
    const order: Theme[] = ['dark', 'light', 'system']
    const next = order[(order.indexOf(theme) + 1) % order.length]
    setTheme(next)
  }

  return (
    <header className="flex items-center justify-between h-14 px-4 border-b border-border bg-card/80 backdrop-blur-sm flex-shrink-0">
      {/* Page title */}
      <h1 className="text-base font-semibold tracking-tight">{title}</h1>

      {/* Host metrics mini-bar */}
      {hostMetrics && (
        <div className="hidden md:flex items-center gap-4 text-xs text-muted-foreground">
          <MetricChip
            label="CPU"
            value={formatPercent(hostMetrics.cpu_percent)}
            percent={hostMetrics.cpu_percent}
          />
          <MetricChip
            label="RAM"
            value={`${formatMib(hostMetrics.memory_used_mib)} / ${formatMib(hostMetrics.memory_total_mib)}`}
            percent={(hostMetrics.memory_used_mib / hostMetrics.memory_total_mib) * 100}
          />
        </div>
      )}

      {/* Right actions */}
      <div className="flex items-center gap-1">
        {/* Search / command palette */}
        <button
          id="command-palette-trigger"
          onClick={() => setCommandPaletteOpen(true)}
          className={cn(
            'flex items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground',
            'bg-muted/50 rounded-lg border border-border/50',
            'hover:bg-muted transition-colors',
          )}
        >
          <Search size={14} />
          <span className="hidden sm:inline">Search…</span>
          <kbd className="hidden sm:inline text-[10px] bg-background border border-border px-1 py-0.5 rounded">
            ⌘K
          </kbd>
        </button>

        {/* Create VM shortcut */}
        <button
          id="create-vm-btn"
          onClick={() => navigate('/vms/create')}
          className={cn(
            'ml-1 flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium',
            'bg-primary text-primary-foreground rounded-lg',
            'hover:bg-primary/90 transition-colors',
          )}
        >
          <Plus size={14} />
          <span className="hidden sm:inline">New VM</span>
        </button>

        {/* Notifications */}
        <button
          id="notifications-btn"
          className="relative p-2 rounded-lg hover:bg-accent transition-colors"
          aria-label="Notifications"
        >
          <Bell size={16} />
          {unreadCount > 0 && (
            <span className="absolute top-1 right-1 w-2 h-2 bg-destructive rounded-full" />
          )}
        </button>

        {/* Theme toggle */}
        <button
          id="theme-toggle"
          onClick={cycleTheme}
          className="p-2 rounded-lg hover:bg-accent transition-colors"
          title={`Theme: ${theme}`}
          aria-label="Toggle theme"
        >
          {themeIcons[theme]}
        </button>
      </div>
    </header>
  )
}

function MetricChip({
  label,
  value,
  percent,
}: {
  label: string
  value: string
  percent: number
}) {
  const color =
    percent > 90
      ? 'bg-destructive'
      : percent > 70
        ? 'bg-amber-500'
        : 'bg-emerald-500'

  return (
    <div className="flex items-center gap-2">
      <span className="text-muted-foreground/70">{label}</span>
      <div className="w-16 metric-bar">
        <div
          className={cn('metric-bar-fill', color)}
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>
      <span className="font-mono text-[11px]">{value}</span>
    </div>
  )
}
