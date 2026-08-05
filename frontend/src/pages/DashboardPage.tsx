import { motion } from 'framer-motion'
import {
  Server,
  Play,
  Pause,
  AlertTriangle,
  Cpu,
  MemoryStick,
  Plus,
  Zap,
  ShieldCheck,
  Activity,
  Layers,
  Info,
  ExternalLink,
} from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useEffect, useState } from 'react'
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts'

import { useVmStore } from '@/stores/vmStore'
import { useMetricsStore } from '@/stores/metricsStore'
import { VmCard } from '@/components/vm/VmCard'
import { cn, formatMib, formatPercent } from '@/lib/utils'
import { settingsApi, VirtualizationInfo, EngineType } from '@/lib/api'

const fadeInUp = {
  initial: { opacity: 0, y: 16 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: 0.3 },
}

// ─── Engine display helpers ───────────────────────────────────────────────────

function getEngineMeta(engine: EngineType) {
  switch (engine) {
    case 'nova_native_whp':
      return {
        name: 'NovaVM Native',
        badge: 'WHP',
        sub: 'Windows Hypervisor Platform',
        color: 'from-violet-600 to-indigo-600',
        badgeColor: 'bg-violet-500/20 text-violet-300 border-violet-500/30',
        dot: 'bg-emerald-500',
        perfLabel: '★★★★★ Hardware Accelerated',
        perfColor: 'text-emerald-400',
        icon: <Zap size={18} className="text-violet-400" />,
      }
    case 'nova_native_kvm':
      return {
        name: 'NovaVM Native',
        badge: 'KVM',
        sub: 'Linux Kernel Virtual Machine',
        color: 'from-emerald-600 to-teal-600',
        badgeColor: 'bg-emerald-500/20 text-emerald-300 border-emerald-500/30',
        dot: 'bg-emerald-500',
        perfLabel: '★★★★★ Hardware Accelerated',
        perfColor: 'text-emerald-400',
        icon: <Zap size={18} className="text-emerald-400" />,
      }
    case 'nova_qemu_accelerated':
      return {
        name: 'NovaVM + QEMU',
        badge: 'ACCEL',
        sub: 'QEMU with hardware acceleration',
        color: 'from-blue-600 to-cyan-600',
        badgeColor: 'bg-blue-500/20 text-blue-300 border-blue-500/30',
        dot: 'bg-blue-400',
        perfLabel: '★★★★ Accelerated',
        perfColor: 'text-blue-400',
        icon: <Activity size={18} className="text-blue-400" />,
      }
    case 'nova_qemu_software':
      return {
        name: 'NovaVM + QEMU',
        badge: 'TCG',
        sub: 'QEMU software emulation',
        color: 'from-amber-600 to-orange-600',
        badgeColor: 'bg-amber-500/20 text-amber-300 border-amber-500/30',
        dot: 'bg-amber-400',
        perfLabel: '★★ Software Emulation',
        perfColor: 'text-amber-400',
        icon: <Activity size={18} className="text-amber-400" />,
      }
    default:
      return {
        name: 'NovaVM Simulation',
        badge: 'SIM',
        sub: 'No hypervisor available',
        color: 'from-slate-600 to-slate-700',
        badgeColor: 'bg-slate-500/20 text-slate-300 border-slate-500/30',
        dot: 'bg-slate-500',
        perfLabel: '★ Simulation Only',
        perfColor: 'text-slate-400',
        icon: <Layers size={18} className="text-slate-400" />,
      }
  }
}

function getCpuLabel(tech: string) {
  if (tech === 'intel_vtx') return 'Intel VT-x'
  if (tech === 'amd_v') return 'AMD-V (SVM)'
  if (tech === 'arm_hv') return 'ARM Hypervisor'
  return 'No VT Extension'
}

// ─── Dashboard ────────────────────────────────────────────────────────────────

export function DashboardPage() {
  const navigate = useNavigate()
  const vms = useVmStore((s) => s.vms)
  const hostMetrics = useMetricsStore((s) => s.hostMetrics)
  const hostHistory = useMetricsStore((s) => s.hostHistory)
  const [virtInfo, setVirtInfo] = useState<VirtualizationInfo | null>(null)
  const [virtLoading, setVirtLoading] = useState(true)

  useEffect(() => {
    settingsApi.getVirtualizationInfo()
      .then(setVirtInfo)
      .catch(() => setVirtInfo(null))
      .finally(() => setVirtLoading(false))
  }, [])

  const runningVms = vms.filter((v) => v.state === 'running')
  const pausedVms = vms.filter((v) => v.state === 'paused')
  const stoppedVms = vms.filter((v) => v.state === 'stopped')
  const crashedVms = vms.filter((v) => v.state === 'crashed')

  const cpuData = (hostHistory || []).map((m, i) => ({
    t: i,
    cpu: m?.cpu_percent ?? 0,
    mem: hostMetrics && hostMetrics.memory_total_mib
      ? ((m?.memory_used_mib ?? 0) / (hostMetrics.memory_total_mib || 1)) * 100
      : 0,
  }))

  const meta = virtInfo ? getEngineMeta(virtInfo.engine) : null

  return (
    <div className="space-y-6 max-w-7xl mx-auto">
      {/* Header */}
      <motion.div {...fadeInUp} className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Dashboard</h2>
          <p className="text-muted-foreground text-sm mt-0.5">
            {vms.length} virtual machine{vms.length !== 1 ? 's' : ''} •{' '}
            {runningVms.length} running
          </p>
        </div>
        <button
          id="dashboard-create-vm"
          onClick={() => navigate('/vms/create')}
          className={cn(
            'flex items-center gap-2 px-4 py-2 text-sm font-medium',
            'bg-primary text-primary-foreground rounded-lg',
            'hover:bg-primary/90 transition-colors shadow-sm',
          )}
        >
          <Plus size={16} />
          New VM
        </button>
      </motion.div>

      {/* Stats grid */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          label="Running"
          value={runningVms.length}
          icon={<Play size={18} className="text-emerald-500" />}
          color="emerald"
          delay={0}
        />
        <StatCard
          label="Paused"
          value={pausedVms.length}
          icon={<Pause size={18} className="text-amber-500" />}
          color="amber"
          delay={0.05}
        />
        <StatCard
          label="Stopped"
          value={stoppedVms.length}
          icon={<Server size={18} className="text-slate-400" />}
          color="slate"
          delay={0.1}
        />
        <StatCard
          label="Crashed"
          value={crashedVms.length}
          icon={<AlertTriangle size={18} className="text-destructive" />}
          color="red"
          delay={0.15}
        />
      </div>

      {/* Virtualization Engine Card */}
      <motion.div {...fadeInUp} transition={{ delay: 0.18 }}>
        {virtLoading ? (
          <div className="rounded-2xl border border-border bg-card p-6 animate-pulse h-40" />
        ) : virtInfo && meta ? (
          <div className="relative rounded-2xl border border-border bg-card overflow-hidden">
            {/* Gradient accent bar */}
            <div className={cn('absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r', meta.color)} />

            <div className="p-6">
              <div className="flex items-start justify-between gap-4 flex-wrap">
                {/* Left: Engine identity */}
                <div className="flex items-center gap-4">
                  <div className={cn(
                    'w-12 h-12 rounded-xl flex items-center justify-center',
                    'bg-gradient-to-br shrink-0',
                    meta.color.replace('from-', 'from-').replace('to-', 'to-') + '/20',
                    'border border-white/5'
                  )}>
                    {meta.icon}
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <h3 className="font-bold text-base">{meta.name}</h3>
                      <span className={cn(
                        'text-[10px] font-bold px-1.5 py-0.5 rounded border',
                        meta.badgeColor
                      )}>
                        {meta.badge}
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground mt-0.5">{meta.sub}</p>
                    <p className={cn('text-xs font-medium mt-1', meta.perfColor)}>
                      {meta.perfLabel}
                    </p>
                  </div>
                </div>

                {/* Right: Spec pills */}
                <div className="flex flex-wrap gap-2">
                  <SpecPill
                    icon={<Cpu size={11} />}
                    label={getCpuLabel(virtInfo.cpu_virt)}
                    active={virtInfo.cpu_virt !== 'none'}
                  />
                  <SpecPill
                    icon={<ShieldCheck size={11} />}
                    label={`${virtInfo.cpu_cores} vCPU max`}
                    active
                  />
                  <SpecPill
                    icon={<MemoryStick size={11} />}
                    label={formatMib(virtInfo.max_guest_ram_mib) + ' max RAM'}
                    active
                  />
                  <SpecPill
                    icon={<Zap size={11} />}
                    label={(virtInfo.vtx_available || virtInfo.amd_v_available) ? 'HW Accel' : 'SW Emulation'}
                    active={virtInfo.vtx_available || virtInfo.amd_v_available}
                  />
                </div>
              </div>

              {/* Description */}
              <p className="text-xs text-muted-foreground mt-4 leading-relaxed max-w-3xl">
                {virtInfo.description}
              </p>

              {/* Platform details row */}
              <div className="flex items-center gap-4 mt-4 pt-4 border-t border-border/50 flex-wrap">
                <Detail label="Platform" value={virtInfo.hypervisor_platform} />
                <Detail label="Engine Version" value={`NovaVM ${virtInfo.engine_version}`} />
                <Detail label="Host RAM" value={formatMib(virtInfo.total_ram_mib)} />
                <Detail label="CPU Cores" value={String(virtInfo.cpu_cores)} />
                {virtInfo.qemu_available && (
                  <Detail label="QEMU" value={virtInfo.qemu_path?.split(/[/\\]/).pop() ?? 'Available'} />
                )}
                {!virtInfo.qemu_available && virtInfo.engine === 'nova_simulation' && (
                  <a
                    href="https://www.qemu.org/download/#windows"
                    target="_blank"
                    rel="noreferrer"
                    className="flex items-center gap-1 text-xs text-primary hover:underline ml-auto"
                  >
                    <ExternalLink size={11} />
                    Install QEMU for real VMs
                  </a>
                )}
              </div>
            </div>
          </div>
        ) : (
          <div className="rounded-2xl border border-border bg-card p-6 flex items-center gap-3 text-sm text-muted-foreground">
            <Info size={16} />
            Unable to detect virtualization engine
          </div>
        )}
      </motion.div>

      {/* Host metrics row */}
      {hostMetrics && (
        <motion.div
          {...fadeInUp}
          transition={{ delay: 0.22 }}
          className="grid grid-cols-1 lg:grid-cols-3 gap-4"
        >
          {/* CPU chart */}
          <div className="lg:col-span-2 rounded-xl border border-border bg-card p-5">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <Cpu size={16} className="text-primary" />
                <span className="font-semibold text-sm">Host CPU & Memory</span>
              </div>
              <div className="flex items-center gap-3 text-xs text-muted-foreground">
                <span className="flex items-center gap-1">
                  <span className="w-2 h-2 rounded-full bg-primary inline-block" />
                  CPU {formatPercent(hostMetrics.cpu_percent)}
                </span>
                <span className="flex items-center gap-1">
                  <span className="w-2 h-2 rounded-full bg-emerald-500 inline-block" />
                  RAM{' '}
                  {formatPercent(
                    (hostMetrics.memory_used_mib / hostMetrics.memory_total_mib) * 100,
                  )}
                </span>
              </div>
            </div>
            <ResponsiveContainer width="100%" height={160}>
              <AreaChart data={cpuData} margin={{ top: 0, right: 0, bottom: 0, left: 0 }}>
                <defs>
                  <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="hsl(236,72%,65%)" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="hsl(236,72%,65%)" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="memGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" opacity={0.5} />
                <XAxis dataKey="t" hide />
                <YAxis domain={[0, 100]} hide />
                <Tooltip
                  contentStyle={{
                    background: 'hsl(var(--popover))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: '8px',
                    fontSize: '12px',
                  }}
                  formatter={(v: number) => [`${v.toFixed(1)}%`]}
                  labelFormatter={() => ''}
                />
                <Area
                  type="monotone"
                  dataKey="cpu"
                  stroke="hsl(236,72%,65%)"
                  strokeWidth={2}
                  fill="url(#cpuGradient)"
                  name="CPU"
                />
                <Area
                  type="monotone"
                  dataKey="mem"
                  stroke="#10b981"
                  strokeWidth={2}
                  fill="url(#memGradient)"
                  name="RAM"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* Memory breakdown */}
          <div className="rounded-xl border border-border bg-card p-5 flex flex-col gap-4">
            <div className="flex items-center gap-2">
              <MemoryStick size={16} className="text-primary" />
              <span className="font-semibold text-sm">Memory</span>
            </div>
            <div className="space-y-3 flex-1">
              <MemBar
                label="Used"
                used={hostMetrics.memory_used_mib}
                total={hostMetrics.memory_total_mib}
                color="bg-primary"
              />
              <MemBar
                label="Available"
                used={hostMetrics.memory_total_mib - hostMetrics.memory_used_mib}
                total={hostMetrics.memory_total_mib}
                color="bg-emerald-500"
              />
              {hostMetrics.swap_total_mib > 0 && (
                <MemBar
                  label="Swap"
                  used={hostMetrics.swap_used_mib}
                  total={hostMetrics.swap_total_mib}
                  color="bg-amber-500"
                />
              )}
            </div>
            <div className="text-xs text-muted-foreground">
              Total: {formatMib(hostMetrics.memory_total_mib)}
            </div>
          </div>
        </motion.div>
      )}

      {/* VM grid */}
      <motion.div {...fadeInUp} transition={{ delay: 0.27 }}>
        <div className="flex items-center justify-between mb-3">
          <h3 className="font-semibold text-sm">Virtual Machines</h3>
          <button
            onClick={() => navigate('/vms')}
            className="text-xs text-primary hover:underline"
          >
            View all
          </button>
        </div>
        {vms.length === 0 ? (
          <EmptyState onCreateVm={() => navigate('/vms/create')} />
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-4">
            {vms.slice(0, 8).map((vm) => (
              <VmCard key={vm.id} vm={vm} />
            ))}
          </div>
        )}
      </motion.div>
    </div>
  )
}

// ─── Sub-components ───────────────────────────────────────────────────────────

function StatCard({
  label,
  value,
  icon,
  color,
  delay,
}: {
  label: string
  value: number
  icon: React.ReactNode
  color: string
  delay: number
}) {
  void color // reserved for future coloured indicators
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay, duration: 0.3 }}
      className="rounded-xl border border-border bg-card p-4"
    >
      <div className="flex items-center justify-between mb-3">
        <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider">
          {label}
        </span>
        {icon}
      </div>
      <p className="text-3xl font-bold tracking-tight">{value}</p>
    </motion.div>
  )
}

function SpecPill({
  icon,
  label,
  active,
}: {
  icon: React.ReactNode
  label: string
  active: boolean
}) {
  return (
    <span className={cn(
      'flex items-center gap-1.5 text-[11px] px-2.5 py-1 rounded-full border font-medium',
      active
        ? 'bg-primary/10 text-primary border-primary/20'
        : 'bg-muted/40 text-muted-foreground border-border',
    )}>
      {icon}
      {label}
    </span>
  )
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</span>
      <span className="text-xs font-mono font-medium">{value}</span>
    </div>
  )
}

function MemBar({
  label,
  used,
  total,
  color,
}: {
  label: string
  used: number
  total: number
  color: string
}) {
  const pct = total > 0 ? (used / total) * 100 : 0
  return (
    <div>
      <div className="flex justify-between text-xs mb-1.5">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-mono">{formatMib(used)}</span>
      </div>
      <div className="metric-bar">
        <div
          className={cn('metric-bar-fill', color)}
          style={{ width: `${Math.min(pct, 100)}%` }}
        />
      </div>
    </div>
  )
}

function EmptyState({ onCreateVm }: { onCreateVm: () => void }) {
  return (
    <div className="rounded-xl border border-dashed border-border p-12 text-center">
      <Server size={40} className="mx-auto text-muted-foreground/40 mb-4" />
      <h3 className="font-semibold mb-1">No virtual machines yet</h3>
      <p className="text-sm text-muted-foreground mb-4">
        Create your first VM to get started with NovaVM
      </p>
      <button
        id="empty-state-create-vm"
        onClick={onCreateVm}
        className="px-4 py-2 bg-primary text-primary-foreground text-sm rounded-lg hover:bg-primary/90 transition-colors"
      >
        Create VM
      </button>
    </div>
  )
}
