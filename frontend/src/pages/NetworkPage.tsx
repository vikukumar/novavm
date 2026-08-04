import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { Plus, Network as NetworkIcon, Trash2 } from 'lucide-react'
import { networkApi } from '@/lib/api'
import type { VirtualSwitch } from '@/types'
import { cn } from '@/lib/utils'
import { toast } from '@/components/ui/use-toast'

export function NetworkPage() {
  const [switches, setSwitches] = useState<VirtualSwitch[]>([])
  const [loading, setLoading] = useState(true)

  const load = async () => {
    setLoading(true)
    try {
      setSwitches(await networkApi.listSwitches())
    } catch (e) {
      toast({ title: 'Failed to load switches', description: String(e), variant: 'destructive' })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const handleDelete = async (name: string) => {
    try {
      await networkApi.deleteSwitch(name)
      toast({ title: `Switch '${name}' deleted` })
      await load()
    } catch (e) {
      toast({ title: 'Delete failed', description: String(e), variant: 'destructive' })
    }
  }

  const modeColor: Record<string, string> = {
    nat: 'text-emerald-500 bg-emerald-500/10',
    bridged: 'text-blue-500 bg-blue-500/10',
    host_only: 'text-amber-500 bg-amber-500/10',
    internal: 'text-violet-500 bg-violet-500/10',
  }

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Network</h2>
          <p className="text-muted-foreground text-sm mt-0.5">{switches.length} virtual switch{switches.length !== 1 ? 'es' : ''}</p>
        </div>
        <button
          onClick={async () => {
            await networkApi.createSwitch(`switch-${Date.now()}`, 'nat')
            await load()
          }}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          <Plus size={16} />
          New Switch
        </button>
      </div>

      {loading ? (
        <div className="grid gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="skeleton h-20 rounded-xl" />
          ))}
        </div>
      ) : switches.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border p-12 text-center">
          <NetworkIcon size={36} className="mx-auto mb-3 text-muted-foreground/40" />
          <p className="text-sm text-muted-foreground">No virtual switches configured</p>
        </div>
      ) : (
        <div className="space-y-3">
          {switches.map((sw) => (
            <motion.div
              key={sw.id}
              layout
              className="flex items-center gap-4 p-4 rounded-xl border border-border bg-card hover:bg-accent/20 transition-colors"
            >
              <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-muted flex items-center justify-center">
                <NetworkIcon size={18} className="text-muted-foreground" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm">{sw.name}</span>
                  <span className={cn('text-xs px-2 py-0.5 rounded-full font-medium', modeColor[sw.mode] ?? 'text-muted-foreground bg-muted')}>
                    {sw.mode.replace('_', '-')}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {sw.subnet} · Gateway: {sw.gateway} · {sw.connected_vms} VMs connected
                  {sw.dhcp_enabled && ' · DHCP enabled'}
                </p>
              </div>
              <button
                onClick={() => handleDelete(sw.name)}
                className="p-2 rounded-lg text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
              >
                <Trash2 size={14} />
              </button>
            </motion.div>
          ))}
        </div>
      )}
    </motion.div>
  )
}
