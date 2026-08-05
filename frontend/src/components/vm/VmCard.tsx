import { useNavigate } from 'react-router-dom'
import { motion } from 'framer-motion'
import {
  Play,
  Pause,
  Square,
  Trash2,
  Camera,
  Cpu,
  MemoryStick,
} from 'lucide-react'

import { useState } from 'react'
import { useVmStore } from '@/stores/vmStore'
import { cn, stateDotClass, stateColor, formatMib } from '@/lib/utils'
import type { VmSummary } from '@/types'
import { toast } from '@/components/ui/use-toast'
import { ConfirmModal } from '@/components/ui/ConfirmModal'

interface VmCardProps {
  vm: VmSummary
  compact?: boolean
}

export function VmCard({ vm, compact = false }: VmCardProps) {
  const navigate = useNavigate()
  const [confirmDestroy, setConfirmDestroy] = useState(false)

  const handleAction = async (
    action: () => Promise<void>,
    label: string,
  ) => {
    try {
      await action()
      toast({ title: `${label} successful`, variant: 'default' })
    } catch (e) {
      toast({ title: `${label} failed`, description: String(e), variant: 'destructive' })
    }
  }

  const canStart = vm.state === 'stopped' || vm.state === 'crashed'
  const canPause = vm.state === 'running'
  const canResume = vm.state === 'paused'
  const canStop = vm.state === 'running' || vm.state === 'paused'

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.96 }}
      transition={{ duration: 0.2 }}
      onClick={() => navigate(`/vms/${vm.id}`)}
      className={cn(
        'group relative cursor-pointer rounded-xl border border-border',
        'bg-card hover:bg-accent/30 transition-all duration-200',
        'hover:border-primary/30 hover:shadow-md hover:shadow-primary/5',
        compact ? 'p-3' : 'p-4',
      )}
    >
      {/* Status dot */}
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2.5">
          <span className={stateDotClass(vm.state)} />
          <span className="font-semibold text-sm truncate max-w-[140px]">{vm.name}</span>
        </div>
        <span className={cn('text-xs font-medium capitalize', stateColor(vm.state))}>
          {vm.state}
        </span>
      </div>

      {/* Specs */}
      {!compact && (
        <div className="flex items-center gap-4 mb-3 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Cpu size={11} />
            {vm.cpu_vcpus} vCPU
          </span>
          <span className="flex items-center gap-1">
            <MemoryStick size={11} />
            {formatMib(vm.memory_mib)}
          </span>
        </div>
      )}

      {/* Tags */}
      {!compact && vm.tags && vm.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 mb-3">
          {vm.tags.slice(0, 3).map((tag) => (
            <span
              key={tag}
              className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* Actions — visible on hover */}
      <div
        className={cn(
          'flex items-center gap-1',
          'opacity-0 group-hover:opacity-100 transition-opacity',
        )}
        onClick={(e) => e.stopPropagation()}
      >
        {canStart && (
          <ActionBtn
            icon={<Play size={12} />}
            label="Start"
            onClick={() => handleAction(() => useVmStore.getState().startVm(vm.id), 'Start')}
            className="hover:bg-emerald-500/20 hover:text-emerald-500"
          />
        )}
        {canPause && (
          <ActionBtn
            icon={<Pause size={12} />}
            label="Pause"
            onClick={() => handleAction(() => useVmStore.getState().pauseVm(vm.id), 'Pause')}
          />
        )}
        {canResume && (
          <ActionBtn
            icon={<Play size={12} />}
            label="Resume"
            onClick={() => handleAction(() => useVmStore.getState().resumeVm(vm.id), 'Resume')}
            className="hover:bg-emerald-500/20 hover:text-emerald-500"
          />
        )}
        {canStop && (
          <ActionBtn
            icon={<Square size={12} />}
            label="Stop"
            onClick={() => handleAction(() => useVmStore.getState().stopVm(vm.id), 'Stop')}
            className="hover:bg-amber-500/20 hover:text-amber-500"
          />
        )}
        <ActionBtn
          icon={<Camera size={12} />}
          label="Snapshot"
          onClick={() => {}}
        />
        <ActionBtn
          icon={<Trash2 size={12} />}
          label="Destroy"
          onClick={() => setConfirmDestroy(true)}
          className="hover:bg-destructive/20 hover:text-destructive"
        />
      </div>

      <ConfirmModal
        isOpen={confirmDestroy}
        title={`Destroy Virtual Machine '${vm.name}'?`}
        description="Are you sure you want to permanently destroy this virtual machine? All unpersisted state and running processes will be terminated."
        confirmText="Destroy VM"
        variant="danger"
        onConfirm={async () => {
          setConfirmDestroy(false)
          await handleAction(() => useVmStore.getState().destroyVm(vm.id), 'Destroy')
        }}
        onClose={() => setConfirmDestroy(false)}
      />
    </motion.div>
  )
}

function ActionBtn({
  icon,
  label,
  onClick,
  className,
}: {
  icon: React.ReactNode
  label: string
  onClick: () => void
  className?: string
}) {
  return (
    <button
      title={label}
      onClick={onClick}
      className={cn(
        'p-1.5 rounded-md text-muted-foreground transition-colors',
        'hover:bg-accent',
        className,
      )}
    >
      {icon}
    </button>
  )
}
