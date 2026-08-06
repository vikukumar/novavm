import { useState, useEffect } from 'react'
import {
  Play, Pause, Square, RotateCcw, Monitor,
  Maximize2, ExternalLink, AlertTriangle, Box, Cpu,
} from 'lucide-react'
import { useVmStore } from '@/stores/vmStore'
import { settingsApi, vmApi } from '@/lib/api'
import { toast } from '@/components/ui/use-toast'
import { cn } from '@/lib/utils'

interface VmConsoleDisplayProps {
  vmId: string
  vmName: string
  vmState: string
  vcpus: number
  memoryMib: number
}

type HypervisorStatus =
  | { kind: 'loading' }
  | { kind: 'native'; backendName: string }  // WHP / KVM / AVF
  | { kind: 'virtualbox'; path: string }
  | { kind: 'qemu'; path: string }
  | { kind: 'none'; installUrl: string }

export function VmConsoleDisplay({ vmId, vmName, vmState, vcpus, memoryMib }: VmConsoleDisplayProps) {
  const [hypervisor, setHypervisor] = useState<HypervisorStatus>({ kind: 'loading' })
  const [openingDisplay, setOpeningDisplay] = useState(false)
  const [displayInfo, setDisplayInfo] = useState<string | null>(null)
  const [serialOutput, setSerialOutput] = useState<string>('')

  // Detect which hypervisor is active
  useEffect(() => {
    settingsApi.getHypervisorInfo()
      .then((info) => {
        const name: string = info.backend_name ?? ''
        if (name.startsWith('NovaVM-')) {
          // Native WHP/KVM/AVF backend — no third-party needed
          setHypervisor({ kind: 'native', backendName: name })
        } else if (name === 'VirtualBox') {
          setHypervisor({ kind: 'virtualbox', path: 'VBoxManage.exe' })
        } else {
          settingsApi.getQemuStatus()
            .then((q) => {
              if (q.installed && q.path) {
                setHypervisor({ kind: 'qemu', path: q.path })
              } else {
                setHypervisor({ kind: 'none', installUrl: 'https://www.virtualbox.org/wiki/Downloads' })
              }
            })
            .catch(() => setHypervisor({ kind: 'none', installUrl: 'https://www.virtualbox.org/wiki/Downloads' }))
        }
      })
      .catch(() => setHypervisor({ kind: 'none', installUrl: 'https://www.virtualbox.org/wiki/Downloads' }))
  }, [])

  // Poll serial output from native backend every 500ms when VM is running
  useEffect(() => {
    if (hypervisor.kind !== 'native') return
    if (vmState !== 'running') return

    const poll = setInterval(async () => {
      try {
        const text = await vmApi.getSerialOutput(vmId)
        if (text && text.length > 0) {
          setSerialOutput(prev => {
            const next = prev + text
            // Cap at 512KB displayed
            return next.length > 524288 ? next.slice(-524288) : next
          })
        }
      } catch { /* ignore poll errors */ }
    }, 500)

    return () => clearInterval(poll)
  }, [vmId, vmState, hypervisor.kind])

  const handleOpenDisplay = async () => {
    if (vmState !== 'running') {
      // Start the VM first
      try {
        await useVmStore.getState().startVm(vmId)
        toast({ title: 'VM Starting', description: 'Starting VM and opening display window...' })
      } catch (e) {
        toast({ title: 'Start Failed', description: String(e), variant: 'destructive' })
        return
      }
    }

    setOpeningDisplay(true)
    try {
      const result = await vmApi.openDisplay(vmId)
      setDisplayInfo(result.info)
      if (result.status === 'opened' || result.status === 'already_running') {
        toast({
          title: 'Display Window Opened',
          description: result.info,
        })
      }
    } catch (e) {
      toast({ title: 'Could Not Open Display', description: String(e), variant: 'destructive' })
    } finally {
      setOpeningDisplay(false)
    }
  }

  const isRunning = vmState === 'running'
  const isStopped = vmState === 'stopped' || vmState === 'crashed'
  const isPaused = vmState === 'paused'

  return (
    <div className="space-y-4">

      {/* ── Hypervisor Status Banner ── */}
      {hypervisor.kind === 'loading' && (
        <div className="flex items-center gap-2 px-4 py-3 rounded-xl bg-muted/60 text-sm text-muted-foreground border border-border">
          <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          Detecting virtualization backend...
        </div>
      )}

      {hypervisor.kind === 'native' && (
        <div className="flex items-center gap-2 px-4 py-2 bg-violet-500/8 border border-violet-500/25 rounded-xl text-xs text-violet-300">
          <Cpu size={13} className="text-violet-400" />
          <span className="font-semibold">{hypervisor.backendName}</span>
          <span className="text-violet-400/60">native hardware virtualization — no third-party software needed</span>
        </div>
      )}

      {hypervisor.kind === 'none' && (
        <div className="flex items-start gap-3 px-4 py-3 bg-amber-500/10 border border-amber-500/30 rounded-xl text-amber-300">
          <AlertTriangle size={18} className="mt-0.5 flex-shrink-0" />
          <div className="flex-1 text-sm">
            <p className="font-semibold">Enable Hardware Virtualization Platform</p>
            <p className="text-amber-400/80 mt-0.5">
              Run in PowerShell (Admin): <code className="bg-black/40 px-1 rounded text-amber-200">Enable-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform</code>
              — then restart to activate NovaVM's native hardware hypervisor.
            </p>
          </div>
        </div>
      )}

      {/* ── Native Serial Console (shown when native backend is active & running) ── */}
      {hypervisor.kind === 'native' && vmState === 'running' && (
        <div className="rounded-2xl border border-violet-500/20 bg-[#080808] overflow-hidden shadow-2xl">
          <div className="flex items-center justify-between px-4 py-2 bg-[#101010] border-b border-border/40 text-xs text-muted-foreground">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span className="font-semibold text-foreground">Serial Console — COM1</span>
            </div>
            <span className="text-[10px] text-muted-foreground/50">real guest output via UART</span>
          </div>
          <div
            className="p-4 h-72 overflow-y-auto font-mono text-xs leading-relaxed text-emerald-300/90 whitespace-pre-wrap"
            style={{ scrollbarWidth: 'thin', scrollbarColor: '#333 transparent' }}
          >
            {serialOutput.length === 0
              ? <span className="text-muted-foreground/40 italic">Waiting for guest output…</span>
              : serialOutput
            }
          </div>
        </div>
      )}

      {/* ── Main Display Card ── */}
      <div className="rounded-2xl border border-border bg-[#0a0a0a] overflow-hidden shadow-2xl">

        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2.5 bg-[#141414] border-b border-border/60 text-xs text-muted-foreground">
          <div className="flex items-center gap-2.5">
            <Monitor size={14} className="text-primary" />
            <span className="font-semibold text-foreground tracking-wide">{vmName}</span>
            <span className={cn(
              'px-2 py-0.5 rounded-full text-[10px] font-semibold border',
              isRunning ? 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30' :
              isPaused  ? 'bg-amber-500/15 text-amber-400 border-amber-500/30' :
                          'bg-slate-500/15 text-slate-400 border-slate-500/30'
            )}>
              {vmState.toUpperCase()}
            </span>
          </div>
          <div className="flex items-center gap-2 text-[11px]">
            <span>{vcpus} vCPU</span>
            <span className="text-border">·</span>
            <span>{memoryMib >= 1024 ? `${(memoryMib / 1024).toFixed(0)} GB` : `${memoryMib} MB`} RAM</span>
          </div>
        </div>

        {/* Display Area */}
        <div className="relative bg-[#050505] min-h-[380px] flex flex-col items-center justify-center gap-6 p-8">

          {/* Stopped state */}
          {isStopped && (
            <>
              <div className="flex flex-col items-center gap-3 text-center">
                <div className="w-20 h-20 rounded-2xl bg-muted/20 border border-border/40 flex items-center justify-center">
                  <Monitor size={40} className="text-primary/40" />
                </div>
                <div>
                  <p className="text-base font-semibold text-foreground/80">Virtual Machine is Powered Off</p>
                  <p className="text-xs text-muted-foreground mt-1">
                    Start the VM to open a real{' '}
                    {hypervisor.kind === 'native' ? hypervisor.backendName :
                     hypervisor.kind === 'virtualbox' ? 'VirtualBox' : 'QEMU'} display window
                  </p>
                </div>
              </div>

              <button
                onClick={handleOpenDisplay}
                disabled={hypervisor.kind === 'none' || hypervisor.kind === 'loading' || openingDisplay}
                className={cn(
                  'flex items-center gap-2.5 px-8 py-3.5 text-sm font-semibold rounded-2xl transition-all shadow-lg',
                  'bg-emerald-500 text-black hover:bg-emerald-400 active:scale-95',
                  'disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100'
                )}
              >
                <Play size={16} />
                {openingDisplay ? 'Opening Display...' : 'Start & Open Display'}
              </button>
            </>
          )}

          {/* Running state */}
          {isRunning && (
            <>
              <div className="flex flex-col items-center gap-3 text-center">
                <div className="relative">
                  <div className="w-20 h-20 rounded-2xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center">
                    <Monitor size={40} className="text-emerald-400" />
                  </div>
                  <span className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-emerald-500 border-2 border-[#050505] animate-pulse" />
                </div>
                <div>
                  <p className="text-base font-semibold text-foreground">VM is Running</p>
                  <p className="text-xs text-muted-foreground mt-1">
                    {hypervisor.kind === 'virtualbox'
                      ? 'Click below to open (or bring to front) the VirtualBox display window'
                      : 'QEMU SDL window should already be open. Click below for VNC info.'}
                  </p>
                </div>
              </div>

              <button
                onClick={handleOpenDisplay}
                disabled={openingDisplay}
                className={cn(
                  'flex items-center gap-2.5 px-8 py-3.5 text-sm font-semibold rounded-2xl transition-all shadow-lg',
                  'bg-primary text-primary-foreground hover:bg-primary/90 active:scale-95',
                  'disabled:opacity-50 disabled:cursor-not-allowed'
                )}
              >
                <Maximize2 size={16} />
                {openingDisplay ? 'Opening...' : 'Open Display Window'}
              </button>

              {displayInfo && (
                <div className="text-xs text-emerald-400/70 bg-emerald-500/5 border border-emerald-500/20 rounded-xl px-4 py-2 max-w-sm text-center">
                  {displayInfo}
                </div>
              )}
            </>
          )}

          {/* Paused state */}
          {isPaused && (
            <>
              <div className="flex flex-col items-center gap-3 text-center">
                <div className="w-20 h-20 rounded-2xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center">
                  <Pause size={40} className="text-amber-400" />
                </div>
                <div>
                  <p className="text-base font-semibold text-foreground">VM is Paused</p>
                  <p className="text-xs text-muted-foreground mt-1">Resume the VM to interact with the display</p>
                </div>
              </div>
              <button
                onClick={() => useVmStore.getState().resumeVm(vmId)}
                className="flex items-center gap-2 px-6 py-3 text-sm font-semibold bg-amber-500 text-black hover:bg-amber-400 rounded-2xl transition-all active:scale-95"
              >
                <Play size={14} />
                Resume VM
              </button>
            </>
          )}
        </div>

        {/* VM Control Toolbar */}
        <div className="flex items-center gap-2 px-4 py-2.5 bg-[#0f0f0f] border-t border-border/40">
          <span className="text-xs text-muted-foreground font-medium mr-1">VM Controls:</span>

          {isStopped && (
            <ToolbarBtn
              onClick={() => useVmStore.getState().startVm(vmId)}
              disabled={hypervisor.kind === 'none'}
              label="Power On"
              icon={<Play size={12} />}
              variant="success"
            />
          )}
          {isRunning && (
            <>
              <ToolbarBtn onClick={() => useVmStore.getState().pauseVm(vmId)} label="Pause" icon={<Pause size={12} />} variant="warning" />
              <ToolbarBtn onClick={() => useVmStore.getState().stopVm(vmId)} label="Power Off" icon={<Square size={12} />} variant="danger" />
              <ToolbarBtn onClick={() => useVmStore.getState().resetVm(vmId)} label="Restart" icon={<RotateCcw size={12} />} />
            </>
          )}
          {isPaused && (
            <ToolbarBtn onClick={() => useVmStore.getState().resumeVm(vmId)} label="Resume" icon={<Play size={12} />} variant="success" />
          )}

          <div className="ml-auto text-[10px] text-muted-foreground/50 font-mono">
            {hypervisor.kind === 'virtualbox' && 'Oracle VirtualBox 7.x'}
            {hypervisor.kind === 'qemu' && 'QEMU SDL + VNC :5900'}
            {hypervisor.kind === 'none' && 'No hypervisor detected'}
          </div>
        </div>
      </div>

      {/* Info Box */}
      <div className="text-xs text-muted-foreground/70 bg-muted/30 border border-border/50 rounded-xl px-4 py-3 leading-relaxed">
        {hypervisor.kind === 'virtualbox' && (
          <>
            <strong className="text-foreground/80">VirtualBox</strong> is detected as your hypervisor. When you start a VM,
            VirtualBox opens its own <strong className="text-foreground/80">native graphical window</strong> with full display support —
            including installer UI, desktop, and mouse/keyboard input. This is a real virtual machine, not a simulation.
          </>
        )}
        {hypervisor.kind === 'qemu' && (
          <>
            <strong className="text-foreground/80">QEMU</strong> is detected as your hypervisor. When you start a VM,
            QEMU opens an <strong className="text-foreground/80">SDL display window</strong> with full graphical output.
            VNC is also available at <code className="font-mono text-primary">127.0.0.1:5900</code> for remote viewing.
          </>
        )}
        {hypervisor.kind === 'none' && (
          <>
            No hypervisor found. Install{' '}
            <a href="https://www.virtualbox.org" target="_blank" rel="noreferrer" className="text-primary hover:underline">VirtualBox</a>
            {' '}or{' '}
            <a href="https://www.qemu.org" target="_blank" rel="noreferrer" className="text-primary hover:underline">QEMU</a>
            {' '}to run real virtual machines with full display.
          </>
        )}
      </div>
    </div>
  )
}

function ToolbarBtn({
  label, icon, onClick, disabled = false, variant = 'default',
}: {
  label: string
  icon: React.ReactNode
  onClick: () => void
  disabled?: boolean
  variant?: 'default' | 'success' | 'warning' | 'danger'
}) {
  const styles: Record<string, string> = {
    default:  'bg-muted/60 hover:bg-accent text-muted-foreground border-border/50',
    success:  'bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-400 border-emerald-500/30',
    warning:  'bg-amber-500/15 hover:bg-amber-500/25 text-amber-400 border-amber-500/30',
    danger:   'bg-rose-500/15 hover:bg-rose-500/25 text-rose-400 border-rose-500/30',
  }
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={cn(
        'flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded border transition-colors disabled:opacity-40 disabled:cursor-not-allowed',
        styles[variant],
      )}
    >
      {icon}{label}
    </button>
  )
}