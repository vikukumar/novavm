import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'
import type { VmState } from '@/types'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Format bytes to human-readable string safely. */
export function formatBytes(bytes?: number | null, decimals = 1): string {
  if (bytes === undefined || bytes === null || isNaN(bytes) || bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  const num = bytes / Math.pow(k, i)
  return `${parseFloat(num.toFixed(decimals))} ${sizes[i]}`
}

/** Format MiB value safely. */
export function formatMib(mib?: number | null): string {
  if (mib === undefined || mib === null || isNaN(mib) || mib === 0) return '0 MiB'
  if (mib < 1024) return `${mib} MiB`
  return `${(mib / 1024).toFixed(1)} GiB`
}

/** Format a percentage to one decimal place safely. */
export function formatPercent(value?: number | null): string {
  if (value === undefined || value === null || isNaN(value)) return '0.0%'
  return `${value.toFixed(1)}%`
}

/** Format bytes per second. */
export function formatBps(bps?: number | null): string {
  return `${formatBytes(bps)}/s`
}

/** Format a date string into a locale-relative string safely. */
export function formatRelativeTime(iso?: string | null): string {
  if (!iso) return '—'
  try {
    const date = new Date(iso)
    if (isNaN(date.getTime())) return '—'
    const now = Date.now()
    const diffMs = now - date.getTime()
    const diffSec = Math.floor(diffMs / 1000)
    if (diffSec < 60) return `${Math.max(0, diffSec)}s ago`
    const diffMin = Math.floor(diffSec / 60)
    if (diffMin < 60) return `${diffMin}m ago`
    const diffHour = Math.floor(diffMin / 60)
    if (diffHour < 24) return `${diffHour}h ago`
    const diffDay = Math.floor(diffHour / 24)
    return `${diffDay}d ago`
  } catch {
    return '—'
  }
}

/** Format a date as a short datetime string safely. */
export function formatDateTime(iso?: string | null): string {
  if (!iso) return '—'
  try {
    const d = new Date(iso)
    if (isNaN(d.getTime())) return '—'
    return d.toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return '—'
  }
}

/** Return Tailwind colour class based on VM state. */
export function stateColor(state?: VmState | null): string {
  switch (state) {
    case 'running': return 'text-emerald-500'
    case 'stopped': return 'text-slate-400'
    case 'paused': return 'text-amber-400'
    case 'crashed': return 'text-red-500'
    case 'starting':
    case 'restoring': return 'text-blue-400'
    case 'saving':
    case 'cloning': return 'text-violet-400'
    case 'destroying': return 'text-rose-500'
    default: return 'text-muted-foreground'
  }
}

/** Return the status dot CSS class for a VM state safely. */
export function stateDotClass(state?: VmState | null): string {
  switch (state) {
    case 'running': return 'status-dot status-running'
    case 'stopped': return 'status-dot status-stopped'
    case 'paused': return 'status-dot status-paused'
    case 'crashed': return 'status-dot status-crashed'
    default: return 'status-dot status-starting'
  }
}

/** Generate a consistent colour from a string (for avatars/tags). */
export function stringToHsl(str?: string | null): string {
  if (!str) return 'hsl(210, 60%, 55%)'
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash)
  }
  const h = Math.abs(hash) % 360
  return `hsl(${h}, 60%, 55%)`
}

/** Clamp a number between min and max. */
export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max)
}

/** Debounce a function. */
export function debounce<T extends (...args: unknown[]) => unknown>(
  fn: T,
  wait: number,
): (...args: Parameters<T>) => void {
  let timeout: ReturnType<typeof setTimeout>
  return (...args: Parameters<T>) => {
    clearTimeout(timeout)
    timeout = setTimeout(() => fn(...args), wait)
  }
}
