import { useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { useNavigate } from 'react-router-dom'
import { ChevronRight, ChevronLeft, Check, Server, Cpu, MemoryStick, HardDrive, Network } from 'lucide-react'

import { useVmStore } from '@/stores/vmStore'
import { cn } from '@/lib/utils'
import { toast } from '@/components/ui/use-toast'
import type { VmConfig } from '@/types'

const STEPS = [
  { id: 'name', label: 'Name & OS', icon: <Server size={14} /> },
  { id: 'cpu', label: 'CPU', icon: <Cpu size={14} /> },
  { id: 'memory', label: 'Memory', icon: <MemoryStick size={14} /> },
  { id: 'storage', label: 'Storage', icon: <HardDrive size={14} /> },
  { id: 'network', label: 'Network', icon: <Network size={14} /> },
  { id: 'review', label: 'Review', icon: <Check size={14} /> },
]

const defaultConfig: VmConfig = {
  name: '',
  description: null,
  cpu: { vcpus: 2, sockets: 1, cores_per_socket: 2, threads_per_core: 1, overcommit_ratio: 1.0 },
  memory: { size_mib: 2048, dynamic_min_mib: 512, dynamic_max_mib: 4096, ballooning: true, huge_pages: false },
  firmware: 'uefi',
  secure_boot: false,
  vtpm: false,
  disks: [],
  nics: [],
  shared_folders: [],
  tags: [],
  group: null,
}

export function CreateVmWizard() {
  const navigate = useNavigate()
  const createVm = useVmStore((s) => s.createVm)
  const [step, setStep] = useState(0)
  const [config, setConfig] = useState<VmConfig>(defaultConfig)
  const [creating, setCreating] = useState(false)

  const isLastStep = step === STEPS.length - 1

  const handleCreate = async () => {
    if (!config.name.trim()) {
      toast({ title: 'Name required', description: 'Please enter a VM name', variant: 'destructive' })
      return
    }
    setCreating(true)
    try {
      const id = await createVm(config)
      toast({ title: 'VM created successfully' })
      navigate(`/vms/${id}`)
    } catch (e) {
      toast({ title: 'Failed to create VM', description: String(e), variant: 'destructive' })
    } finally {
      setCreating(false)
    }
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-3xl mx-auto"
    >
      <div className="mb-8">
        <h2 className="text-2xl font-bold tracking-tight">Create Virtual Machine</h2>
        <p className="text-muted-foreground text-sm mt-0.5">Configure your new VM step by step</p>
      </div>

      {/* Step indicator */}
      <div className="flex items-center mb-8 overflow-x-auto pb-2">
        {STEPS.map((s, i) => (
          <div key={s.id} className="flex items-center flex-shrink-0">
            <button
              onClick={() => i < step && setStep(i)}
              className={cn(
                'flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors',
                i === step
                  ? 'bg-primary text-primary-foreground'
                  : i < step
                    ? 'text-emerald-500 cursor-pointer'
                    : 'text-muted-foreground cursor-default',
              )}
            >
              {i < step ? <Check size={12} /> : s.icon}
              {s.label}
            </button>
            {i < STEPS.length - 1 && (
              <ChevronRight size={14} className="text-muted-foreground mx-1" />
            )}
          </div>
        ))}
      </div>

      {/* Step content */}
      <div className="rounded-xl border border-border bg-card p-6 min-h-64">
        <AnimatePresence mode="wait">
          <motion.div
            key={step}
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            transition={{ duration: 0.2 }}
          >
            {step === 0 && (
              <StepName config={config} onChange={(c) => setConfig(c)} />
            )}
            {step === 1 && (
              <StepCpu config={config} onChange={(c) => setConfig(c)} />
            )}
            {step === 2 && (
              <StepMemory config={config} onChange={(c) => setConfig(c)} />
            )}
            {step === 3 && (
              <StepStorage config={config} onChange={(c) => setConfig(c)} />
            )}
            {step === 4 && (
              <StepNetwork config={config} onChange={(c) => setConfig(c)} />
            )}
            {step === 5 && (
              <StepReview config={config} />
            )}
          </motion.div>
        </AnimatePresence>
      </div>

      {/* Navigation */}
      <div className="flex justify-between mt-6">
        <button
          onClick={() => (step === 0 ? navigate('/vms') : setStep(step - 1))}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
        >
          <ChevronLeft size={16} />
          {step === 0 ? 'Cancel' : 'Back'}
        </button>
        <button
          onClick={isLastStep ? handleCreate : () => setStep(step + 1)}
          disabled={creating}
          className={cn(
            'flex items-center gap-2 px-5 py-2 text-sm font-medium rounded-lg',
            'bg-primary text-primary-foreground',
            'hover:bg-primary/90 transition-colors',
            'disabled:opacity-50 disabled:cursor-not-allowed',
          )}
        >
          {creating ? 'Creating…' : isLastStep ? 'Create VM' : 'Next'}
          {!isLastStep && <ChevronRight size={16} />}
        </button>
      </div>
    </motion.div>
  )
}

// ─── Step Components ──────────────────────────────────────────────────────────

function StepName({ config, onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  return (
    <div className="space-y-5">
      <h3 className="font-semibold text-base">Name & Description</h3>
      <div className="space-y-1">
        <label className="text-sm font-medium">VM Name *</label>
        <input
          id="vm-name-input"
          type="text"
          placeholder="e.g. ubuntu-dev"
          value={config.name}
          onChange={(e) => onChange({ ...config, name: e.target.value })}
          className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
          autoFocus
        />
      </div>
      <div className="space-y-1">
        <label className="text-sm font-medium">Description</label>
        <textarea
          placeholder="Optional description…"
          value={config.description ?? ''}
          onChange={(e) => onChange({ ...config, description: e.target.value || null })}
          rows={3}
          className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring resize-none"
        />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label className="text-sm font-medium">Firmware</label>
          <select
            value={config.firmware}
            onChange={(e) => onChange({ ...config, firmware: e.target.value as 'bios' | 'uefi' })}
            className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option value="uefi">UEFI</option>
            <option value="bios">BIOS (Legacy)</option>
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-sm font-medium">Group</label>
          <input
            type="text"
            placeholder="e.g. Development"
            value={config.group ?? ''}
            onChange={(e) => onChange({ ...config, group: e.target.value || null })}
            className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
      </div>
      <div className="flex items-center gap-6">
        <label className="flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            checked={config.secure_boot}
            onChange={(e) => onChange({ ...config, secure_boot: e.target.checked })}
            disabled={config.firmware !== 'uefi'}
            className="rounded"
          />
          Secure Boot
        </label>
        <label className="flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            checked={config.vtpm}
            onChange={(e) => onChange({ ...config, vtpm: e.target.checked })}
            className="rounded"
          />
          Virtual TPM
        </label>
      </div>
    </div>
  )
}

function StepCpu({ config, onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  return (
    <div className="space-y-5">
      <h3 className="font-semibold text-base">CPU Configuration</h3>
      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          <label className="font-medium">vCPUs</label>
          <span className="text-primary font-mono">{config.cpu.vcpus}</span>
        </div>
        <input
          id="vcpu-slider"
          type="range"
          min={1}
          max={32}
          value={config.cpu.vcpus}
          onChange={(e) => onChange({ ...config, cpu: { ...config.cpu, vcpus: Number(e.target.value) } })}
          className="w-full accent-primary"
        />
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>1</span><span>8</span><span>16</span><span>32</span>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label className="text-sm font-medium">Sockets</label>
          <input
            type="number"
            min={1}
            max={8}
            value={config.cpu.sockets}
            onChange={(e) => onChange({ ...config, cpu: { ...config.cpu, sockets: Number(e.target.value) } })}
            className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
        <div className="space-y-1">
          <label className="text-sm font-medium">Overcommit Ratio</label>
          <input
            type="number"
            min={1}
            max={8}
            step={0.5}
            value={config.cpu.overcommit_ratio}
            onChange={(e) => onChange({ ...config, cpu: { ...config.cpu, overcommit_ratio: Number(e.target.value) } })}
            className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
      </div>
    </div>
  )
}

function StepMemory({ config, onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  const values = [512, 1024, 2048, 4096, 8192, 16384, 32768]
  return (
    <div className="space-y-5">
      <h3 className="font-semibold text-base">Memory</h3>
      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          <label className="font-medium">RAM</label>
          <span className="text-primary font-mono">{config.memory.size_mib >= 1024 ? `${config.memory.size_mib / 1024} GiB` : `${config.memory.size_mib} MiB`}</span>
        </div>
        <div className="flex gap-2 flex-wrap">
          {values.map((v) => (
            <button
              key={v}
              onClick={() => onChange({ ...config, memory: { ...config.memory, size_mib: v } })}
              className={cn(
                'px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors',
                config.memory.size_mib === v
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-muted border-border hover:bg-accent',
              )}
            >
              {v >= 1024 ? `${v / 1024} GiB` : `${v} MiB`}
            </button>
          ))}
        </div>
      </div>
      <label className="flex items-center gap-2 text-sm cursor-pointer">
        <input
          type="checkbox"
          checked={config.memory.ballooning}
          onChange={(e) => onChange({ ...config, memory: { ...config.memory, ballooning: e.target.checked } })}
          className="rounded"
        />
        Enable memory ballooning
      </label>
    </div>
  )
}

function StepStorage({ config, onChange: _onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  return (
    <div className="space-y-4">
      <h3 className="font-semibold text-base">Storage</h3>
      <p className="text-sm text-muted-foreground">
        Disk configuration is managed in the Storage page. You can attach disks after VM creation.
      </p>
      {config.disks.length === 0 && (
        <div className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
          No disks attached. VMs without disks boot from network or ISO only.
        </div>
      )}
    </div>
  )
}

function StepNetwork({ config: _config, onChange: _onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  return (
    <div className="space-y-4">
      <h3 className="font-semibold text-base">Network</h3>
      <p className="text-sm text-muted-foreground">
        A NAT network interface will be attached by default. You can customise network
        adapters on the Network page after creation.
      </p>
    </div>
  )
}

function StepReview({ config }: { config: VmConfig }) {
  const items = [
    { label: 'Name', value: config.name || '(unnamed)' },
    { label: 'Firmware', value: config.firmware.toUpperCase() },
    { label: 'vCPUs', value: String(config.cpu.vcpus) },
    { label: 'RAM', value: config.memory.size_mib >= 1024 ? `${config.memory.size_mib / 1024} GiB` : `${config.memory.size_mib} MiB` },
    { label: 'Secure Boot', value: config.secure_boot ? 'Enabled' : 'Disabled' },
    { label: 'vTPM', value: config.vtpm ? 'Enabled' : 'Disabled' },
    { label: 'Ballooning', value: config.memory.ballooning ? 'Enabled' : 'Disabled' },
  ]

  return (
    <div className="space-y-4">
      <h3 className="font-semibold text-base">Review Configuration</h3>
      <div className="divide-y divide-border">
        {items.map((item) => (
          <div key={item.label} className="flex justify-between py-2.5 text-sm">
            <span className="text-muted-foreground">{item.label}</span>
            <span className="font-medium">{item.value}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
