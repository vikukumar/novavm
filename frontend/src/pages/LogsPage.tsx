import { useState, useEffect, useRef } from 'react'
import { ScrollText, Search, Trash2, Copy, RefreshCw, Check } from 'lucide-react'
import { logsApi, type LogEntry } from '@/lib/api'
import { cn } from '@/lib/utils'
import { toast } from '@/components/ui/use-toast'

const LEVEL_COLORS: Record<string, string> = {
  INFO: 'text-blue-400 font-medium',
  DEBUG: 'text-purple-400 font-medium',
  WARN: 'text-amber-400 font-semibold',
  ERROR: 'text-destructive font-bold',
}

const LOG_LEVELS = ['ALL', 'INFO', 'WARN', 'ERROR', 'DEBUG']

export function LogsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [search, setSearch] = useState('')
  const [selectedLevel, setSelectedLevel] = useState('ALL')
  const [autoScroll, setAutoScroll] = useState(true)
  const [copied, setCopied] = useState(false)
  const logContainerRef = useRef<HTMLDivElement>(null)

  const fetchLogs = async () => {
    try {
      const liveLogs = await logsApi.getLogs()
      setLogs(liveLogs)
    } catch {
      // Ignore IPC poll errors
    }
  }

  useEffect(() => {
    fetchLogs()
    const interval = setInterval(fetchLogs, 1000)
    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight
    }
  }, [logs, autoScroll])

  const handleClearLogs = async () => {
    try {
      await logsApi.clearLogs()
      setLogs([])
      toast({ title: 'Logs Cleared', description: 'Application log buffer cleared.' })
    } catch (e) {
      toast({ title: 'Clear Failed', description: String(e), variant: 'destructive' })
    }
  }

  const handleCopyLogs = () => {
    const text = filtered
      .map((l) => `[${l.timestamp}] [${l.level}] [${l.target}]: ${l.message}`)
      .join('\n')
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const filtered = logs.filter((l) => {
    if (selectedLevel !== 'ALL' && l.level.toUpperCase() !== selectedLevel) return false
    if (!search) return true
    const query = search.toLowerCase()
    return (
      l.message.toLowerCase().includes(query) ||
      l.target.toLowerCase().includes(query) ||
      l.timestamp.toLowerCase().includes(query) ||
      l.level.toLowerCase().includes(query)
    )
  })

  return (
    <div className="max-w-6xl mx-auto space-y-5">
      {/* Page Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Real-Time Application Logs</h2>
          <p className="text-muted-foreground text-sm mt-0.5">
            Live hypervisor engine and system log event stream
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={fetchLogs}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-muted hover:bg-accent rounded-lg border border-border transition-colors"
          >
            <RefreshCw size={13} />
            Refresh
          </button>
          <button
            onClick={handleCopyLogs}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-muted hover:bg-accent rounded-lg border border-border transition-colors"
          >
            {copied ? <Check size={13} className="text-emerald-400" /> : <Copy size={13} />}
            {copied ? 'Copied!' : 'Copy Logs'}
          </button>
          <button
            onClick={handleClearLogs}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-destructive/15 text-destructive hover:bg-destructive/25 rounded-lg border border-destructive/30 transition-colors"
          >
            <Trash2 size={13} />
            Clear
          </button>
        </div>
      </div>

      {/* Controls Bar */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Search Input */}
        <div className="relative flex-1 min-w-48">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <input
            id="log-search"
            type="text"
            placeholder="Search real-time log entries…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-4 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring placeholder:text-muted-foreground"
          />
        </div>

        {/* Level Filters */}
        <div className="flex items-center gap-1 bg-muted rounded-lg p-1">
          {LOG_LEVELS.map((lvl) => (
            <button
              key={lvl}
              onClick={() => setSelectedLevel(lvl)}
              className={cn(
                'px-3 py-1 text-xs font-medium rounded-md transition-colors',
                selectedLevel === lvl
                  ? 'bg-card text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {lvl}
            </button>
          ))}
        </div>

        {/* Auto Scroll Toggle */}
        <label className="flex items-center gap-2 text-xs text-muted-foreground cursor-pointer select-none ml-auto">
          <input
            type="checkbox"
            checked={autoScroll}
            onChange={(e) => setAutoScroll(e.target.checked)}
            className="rounded border-border bg-muted accent-primary"
          />
          Auto-scroll
        </label>
      </div>

      {/* Real-time Log Console Output */}
      <div className="rounded-2xl border border-border bg-[#0d0d0d] overflow-hidden shadow-xl">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border/50 bg-card/40 text-xs text-muted-foreground">
          <div className="flex items-center gap-2">
            <ScrollText size={14} className="text-primary" />
            <span className="font-medium text-foreground">Live Hypervisor Log Output</span>
          </div>
          <span className="font-mono text-[11px] bg-muted/60 px-2 py-0.5 rounded border border-border/50">
            {filtered.length} / {logs.length} entries
          </span>
        </div>

        <div
          ref={logContainerRef}
          className="font-mono text-xs p-4 space-y-1.5 min-h-80 max-h-[30rem] overflow-y-auto"
        >
          {filtered.map((log, i) => (
            <div key={i} className="flex flex-wrap sm:flex-nowrap items-start gap-3 hover:bg-white/5 p-1 rounded transition-colors">
              <span className="text-muted-foreground/60 flex-shrink-0 text-[11px] font-mono">
                {log.timestamp}
              </span>
              <span className={cn('flex-shrink-0 w-14 text-[11px]', LEVEL_COLORS[log.level.toUpperCase()] ?? 'text-foreground')}>
                [{log.level}]
              </span>
              <span className="flex-shrink-0 text-muted-foreground/80 font-semibold text-[11px]">
                [{log.target}]
              </span>
              <span className="text-foreground/90 break-all flex-1">{log.message}</span>
            </div>
          ))}

          {filtered.length === 0 && (
            <div className="flex flex-col items-center justify-center py-16 text-muted-foreground gap-2">
              <ScrollText size={24} className="opacity-40" />
              <p className="text-sm">No real-time application logs matching current filters.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
