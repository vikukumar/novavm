import { useParams, useNavigate } from 'react-router-dom'
import {
  Play, Pause, Square, RotateCcw, Camera,
  Trash2, ArrowLeft, Cpu, MemoryStick,
  Activity, Terminal, Settings2, Code,
} from 'lucide-react'
import { useState, useEffect, useCallback, useRef } from 'react'
import {
  AreaChart, Area, XAxis, YAxis, Tooltip,
  ResponsiveContainer, CartesianGrid,
} from 'recharts'

import { useVmStore } from '@/stores/vmStore'
import { useMetricsStore } from '@/stores/metricsStore'
import { cn, stateDotClass, stateColor, formatPercent, formatMib } from '@/lib/utils'
import { toast } from '@/components/ui/use-toast'
import { VmConsoleDisplay } from '@/components/vm/VmConsoleDisplay'
import { VmScriptingAndUserTab } from '@/components/vm/VmScriptingAndUserTab'
import { settingsApi, VirtualizationInfo } from '@/lib/api'

type Tab = 'overview' | 'console' | 'scripting' | 'snapshots' | 'settings'

export function VmDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const [activeTab, setActiveTab] = useState<Tab>('overview')
  const [loading, setLoading] = useState(true)

  // ── Stable individual selectors — never destructure useVmStore() directly ──
  const vm = useVmStore((s) => s.vms.find((v) => v.id === id))
  const vmMetrics = useMetricsStore((s) => (id ? s.vmMetrics[id] : undefined))

  // Use a ref-stable selector that returns the same empty array reference
  const EMPTY_HISTORY = useRef<import('@/types').VmMetrics[]>([]).current
  const vmHistory = useMetricsStore((s) => (id ? s.vmHistory[id] : undefined) ?? EMPTY_HISTORY)

  const [virtInfo, setVirtInfo] = useState<VirtualizationInfo | null>(null)

  // Fetch VMs & Virt Info on mount
  useEffect(() => {
    let isMounted = true
    useVmStore.getState().fetchVms().finally(() => {
      if (isMounted) setLoading(false)
    })
    settingsApi.getVirtualizationInfo().then((info) => {
      if (isMounted) setVirtInfo(info)
    }).catch(() => {})
    return () => { isMounted = false }
  }, []) // do NOT put id here — fetchVms loads all VMs anyway

  // Stable action handlers using getState() so they never change references
  const handleStart = useCallback(() =>
    handleAction(() => useVmStore.getState().startVm(id!), 'Start'), [id])
  const handlePause = useCallback(() =>
    handleAction(() => useVmStore.getState().pauseVm(id!), 'Pause'), [id])
  const handleResume = useCallback(() =>
    handleAction(() => useVmStore.getState().resumeVm(id!), 'Resume'), [id])
  const handleStop = useCallback(() =>
    handleAction(() => useVmStore.getState().stopVm(id!), 'Stop'), [id])
  const handleReset = useCallback(() =>
    handleAction(() => useVmStore.getState().resetVm(id!), 'Reset'), [id])
  const handleDestroy = useCallback(() =>
    handleAction(async () => { await useVmStore.getState().destroyVm(id!); navigate('/vms') }, 'Destroy'), [id, navigate])

  if (loading && !vm) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-3 text-muted-foreground">
        <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
        <span className="text-sm">Loading virtual machine...</span>
      </div>
    )
  }

  if (!loading && !vm) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-3 text-muted-foreground">
        <span className="text-base font-medium">VM not found</span>
        <button
          onClick={() => navigate('/vms')}
          className="text-sm text-primary hover:underline"
        >
          ← Back to Virtual Machines
        </button>
      </div>
    )
  }

  if (!vm) return null

  const cpuData = vmHistory.map((m, i) => ({ t: i, cpu: m.cpu_percent }))

  const TABS: { id: Tab; label: string; icon: React.ReactNode }[] = [
    { id: 'overview', label: 'Overview', icon: <Activity size={14} /> },
    { id: 'console', label: 'Console', icon: <Terminal size={14} /> },
    { id: 'scripting', label: 'Guest Exec & Users', icon: <Code size={14} /> },
    { id: 'snapshots', label: 'Snapshots', icon: <Camera size={14} /> },
    { id: 'settings', label: 'Settings', icon: <Settings2 size={14} /> },
  ]

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      {/* Breadcrumb */}
      <div className="flex items-center gap-2 text-sm">
        <button
          onClick={() => navigate('/vms')}
          className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft size={14} />
          VMs
        </button>
        <span className="text-muted-foreground">/</span>
        <span className="font-medium">{vm.name}</span>
      </div>

      {/* VM header */}
      <div className="rounded-xl border border-border bg-card p-5">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <span className={stateDotClass(vm.state)} />
            <div>
              <h2 className="text-xl font-bold">{vm.name}</h2>
              <div className="flex items-center gap-3 mt-1 text-sm text-muted-foreground">
                <span className={stateColor(vm.state)}>{vm.state}</span>
                <span>·</span>
                <span>{vm.cpu_vcpus} vCPU</span>
                <span>·</span>
                <span>{formatMib(vm.memory_mib)}</span>
              </div>
            </div>
          </div>

          {/* Action buttons */}
          <div className="flex items-center gap-2 flex-wrap">
            {(vm.state === 'stopped' || vm.state === 'crashed') && (
              <ActionButton label="Start" icon={<Play size={14} />} onClick={handleStart} variant="success" />
            )}
            {vm.state === 'running' && (
              <ActionButton label="Pause" icon={<Pause size={14} />} onClick={handlePause} />
            )}
            {vm.state === 'paused' && (
              <ActionButton label="Resume" icon={<Play size={14} />} onClick={handleResume} variant="success" />
            )}
            {(vm.state === 'running' || vm.state === 'paused') && (
              <ActionButton label="Stop" icon={<Square size={14} />} onClick={handleStop} variant="warning" />
            )}
            {vm.state === 'running' && (
              <ActionButton label="Reset" icon={<RotateCcw size={14} />} onClick={handleReset} />
            )}
            <ActionButton label="Destroy" icon={<Trash2 size={14} />} onClick={handleDestroy} variant="danger" />
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-border">
        <div className="flex gap-1">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={cn(
                'flex items-center gap-2 px-4 py-2.5 text-sm font-medium',
                'border-b-2 transition-colors',
                activeTab === tab.id
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground',
              )}
            >
              {tab.icon}
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Tab content */}
      {activeTab === 'overview' && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* CPU chart */}
          <div className="rounded-xl border border-border bg-card p-4">
            <div className="flex items-center gap-2 mb-3">
              <Cpu size={14} className="text-primary" />
              <span className="text-sm font-medium">CPU</span>
              <span className="ml-auto text-sm font-mono">
                {vmMetrics ? formatPercent(vmMetrics.cpu_percent) : '—'}
              </span>
            </div>
            <ResponsiveContainer width="100%" height={100}>
              <AreaChart data={cpuData}>
                <defs>
                  <linearGradient id="vmCpuGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="hsl(236,72%,65%)" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="hsl(236,72%,65%)" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" opacity={0.3} />
                <XAxis dataKey="t" hide />
                <YAxis domain={[0, 100]} hide />
                <Tooltip
                  contentStyle={{ background: 'hsl(var(--popover))', border: '1px solid hsl(var(--border))', borderRadius: '8px', fontSize: '11px' }}
                  formatter={(v: unknown) => [typeof v === 'number' ? `${v.toFixed(1)}%` : '0%', 'CPU']}
                  labelFormatter={() => ''}
                />
                <Area type="monotone" dataKey="cpu" stroke="hsl(236,72%,65%)" strokeWidth={2} fill="url(#vmCpuGrad)" />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* Memory */}
          <div className="rounded-xl border border-border bg-card p-4">
            <div className="flex items-center gap-2 mb-3">
              <MemoryStick size={14} className="text-emerald-500" />
              <span className="text-sm font-medium">Memory</span>
              <span className="ml-auto text-sm font-mono">
                {vmMetrics ? formatMib(vmMetrics.memory_used_mib) : '—'} / {formatMib(vm.memory_mib)}
              </span>
            </div>
            {vmMetrics && (
              <div className="metric-bar mt-4">
                <div
                  className="metric-bar-fill bg-emerald-500"
                  style={{ width: `${Math.min((vmMetrics.memory_used_mib / vm.memory_mib) * 100, 100)}%` }}
                />
              </div>
            )}
          </div>

          {/* Config summary */}
          <div className="rounded-xl border border-border bg-card p-4 col-span-full">
            <h4 className="text-sm font-medium mb-3">Configuration</h4>
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3 text-sm">
              <ConfigItem label="vCPUs" value={String(vm.cpu_vcpus)} />
              <ConfigItem label="Memory" value={formatMib(vm.memory_mib)} />
              <ConfigItem
                label="Virtualization Engine"
                value={
                  virtInfo
                    ? virtInfo.engine === 'nova_native_kvm'
                      ? 'NovaVM Native (KVM)'
                      : 'NovaVM Native (WHP)'
                    : 'NovaVM Native Engine'
                }
              />
              <ConfigItem
                label="Hardware Acceleration"
                value={
                  virtInfo
                    ? virtInfo.vtx_available
                      ? 'Intel VT-x (Active)'
                      : virtInfo.amd_v_available
                      ? 'AMD-V (Active)'
                      : 'Software Emulation'
                    : 'Hardware Acceleration'
                }
              />
              <ConfigItem label="Group" value={vm.group ?? '—'} />
              <ConfigItem label="Tags" value={(vm.tags || []).join(', ') || '—'} />
            </div>
          </div>
        </div>
      )}

      {activeTab === 'console' && (
        <VmConsoleDisplay vmId={vm.id} vmName={vm.name} vmState={vm.state} vcpus={vm.cpu_vcpus} memoryMib={vm.memory_mib} />
      )}

      {activeTab === 'scripting' && (
        <VmScriptingAndUserTab vmId={vm.id} vmName={vm.name} vmState={vm.state} />
      )}

      {activeTab === 'snapshots' && (
        <div className="rounded-xl border border-dashed border-border p-10 text-center text-muted-foreground text-sm">
          <Camera size={32} className="mx-auto mb-3 opacity-40" />
          No snapshots yet.
          <br />
          <button
            onClick={() => toast({ title: 'Snapshot', description: 'Taking snapshot…' })}
            className="mt-3 text-primary hover:underline"
          >
            Take a snapshot
          </button>
        </div>
      )}

      {activeTab === 'settings' && (
        <div className="rounded-xl border border-border bg-card p-5 text-sm text-muted-foreground">
          VM settings editor — edit CPU, RAM, disks, NICs, firmware. Only available while stopped.
        </div>
      )}
    </div>
  )
}

async function handleAction(action: () => Promise<void>, label: string) {
  try {
    await action()
    toast({ title: `${label} successful` })
  } catch (e) {
    toast({ title: `${label} failed`, description: String(e), variant: 'destructive' })
  }
}

function ActionButton({
  label,
  icon,
  onClick,
  variant = 'default',
}: {
  label: string
  icon: React.ReactNode
  onClick: () => void
  variant?: 'default' | 'success' | 'warning' | 'danger'
}) {
  const styles = {
    default: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
    success: 'bg-emerald-500/10 text-emerald-500 hover:bg-emerald-500/20',
    warning: 'bg-amber-500/10 text-amber-500 hover:bg-amber-500/20',
    danger: 'bg-destructive/10 text-destructive hover:bg-destructive/20',
  }

  return (
    <button
      onClick={onClick}
      className={cn(
        'flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors',
        styles[variant],
      )}
    >
      {icon}
      {label}
    </button>
  )
}

function ConfigItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground mb-0.5">{label}</p>
      <p className="font-medium">{value}</p>
    </div>
  )
}
