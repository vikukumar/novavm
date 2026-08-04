import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { HardDrive, Plus, Lock, Zap } from 'lucide-react'
import { storageApi } from '@/lib/api'
import type { DiskMetadata } from '@/types'
import { formatBytes, formatDateTime } from '@/lib/utils'
import { toast } from '@/components/ui/use-toast'

export function StoragePage() {
  const [disks, setDisks] = useState<DiskMetadata[]>([])
  const [loading, setLoading] = useState(true)

  const load = async () => {
    setLoading(true)
    try {
      setDisks(await storageApi.listDisks())
    } catch (e) {
      toast({ title: 'Failed to load disks', description: String(e), variant: 'destructive' })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Storage</h2>
          <p className="text-muted-foreground text-sm mt-0.5">{disks.length} disk image{disks.length !== 1 ? 's' : ''}</p>
        </div>
        <button
          onClick={() => toast({ title: 'Storage', description: 'Disk creation dialog coming soon' })}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          <Plus size={16} />
          New Disk
        </button>
      </div>

      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => <div key={i} className="skeleton h-16 rounded-xl" />)}
        </div>
      ) : disks.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border p-12 text-center">
          <HardDrive size={36} className="mx-auto mb-3 text-muted-foreground/40" />
          <p className="text-sm text-muted-foreground">No disk images yet</p>
          <p className="text-xs text-muted-foreground mt-1">Create a disk image or attach an existing one</p>
        </div>
      ) : (
        <div className="space-y-3">
          {disks.map((disk) => (
            <motion.div key={disk.id} layout className="flex items-center gap-4 p-4 rounded-xl border border-border bg-card">
              <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-muted flex items-center justify-center">
                <HardDrive size={18} className="text-muted-foreground" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm">{disk.name}</span>
                  <span className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                    {disk.format}
                  </span>
                  {disk.encrypted && <Lock size={11} className="text-amber-500" />}
                  {disk.compressed && <Zap size={11} className="text-blue-500" />}
                </div>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {formatBytes(disk.virtual_size_bytes)} virtual · {formatDateTime(disk.created_at)}
                  {disk.thin_provisioned && ' · Thin'}
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </motion.div>
  )
}
