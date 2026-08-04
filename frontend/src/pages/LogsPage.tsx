import { useState } from 'react'
import { motion } from 'framer-motion'
import { ScrollText, Search } from 'lucide-react'
import { cn } from '@/lib/utils'

const MOCK_LOGS = [
  { ts: '2026-08-04 14:00:01', level: 'INFO', msg: 'NovaVM engine initialised' },
  { ts: '2026-08-04 14:00:02', level: 'INFO', msg: 'Default NAT switch created' },
  { ts: '2026-08-04 14:00:05', level: 'DEBUG', msg: 'Host metrics sampled: CPU 12.3%, RAM 4.1 GiB / 16 GiB' },
  { ts: '2026-08-04 14:01:00', level: 'INFO', msg: 'NovaVM application ready' },
]

const LEVEL_COLORS: Record<string, string> = {
  INFO: 'text-blue-400',
  DEBUG: 'text-muted-foreground',
  WARN: 'text-amber-400',
  ERROR: 'text-destructive',
}

export function LogsPage() {
  const [search, setSearch] = useState('')
  const filtered = MOCK_LOGS.filter(
    (l) => !search || l.msg.toLowerCase().includes(search.toLowerCase()),
  )

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-5xl mx-auto space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Logs</h2>
          <p className="text-muted-foreground text-sm mt-0.5">Application and VM event logs</p>
        </div>
      </div>

      <div className="relative">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
        <input
          id="log-search"
          type="text"
          placeholder="Search logs…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full pl-9 pr-4 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      <div className="rounded-xl border border-border bg-[#0d0d0d] overflow-hidden">
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border/50 text-xs text-muted-foreground">
          <ScrollText size={12} />
          <span>Application Log</span>
          <span className="ml-auto">{filtered.length} entries</span>
        </div>
        <div className="font-mono text-xs p-4 space-y-1.5 max-h-96 overflow-y-auto">
          {filtered.map((log, i) => (
            <div key={i} className="flex gap-3">
              <span className="text-muted-foreground/50 flex-shrink-0">{log.ts}</span>
              <span className={cn('flex-shrink-0 w-12', LEVEL_COLORS[log.level] ?? 'text-foreground')}>
                {log.level}
              </span>
              <span className="text-foreground/80 break-all">{log.msg}</span>
            </div>
          ))}
          {filtered.length === 0 && (
            <p className="text-muted-foreground">No log entries match your search.</p>
          )}
        </div>
      </div>
    </motion.div>
  )
}
