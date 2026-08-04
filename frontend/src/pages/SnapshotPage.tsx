import { motion } from 'framer-motion'
import { Camera } from 'lucide-react'

export function SnapshotPage() {
  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-5xl mx-auto space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Snapshots</h2>
        <p className="text-muted-foreground text-sm mt-0.5">Manage VM snapshots and restore points</p>
      </div>
      <div className="rounded-xl border border-dashed border-border p-16 text-center">
        <Camera size={40} className="mx-auto mb-4 text-muted-foreground/40" />
        <h3 className="font-semibold mb-2">No snapshots</h3>
        <p className="text-sm text-muted-foreground">
          Take a snapshot from the VM detail page to save your VM's state.
        </p>
      </div>
    </motion.div>
  )
}
