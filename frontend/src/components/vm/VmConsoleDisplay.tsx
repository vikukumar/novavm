import { useState, useEffect, useRef, useCallback } from 'react'
import {
  Play, Pause, Square, RotateCcw, Monitor,
  Maximize2, Cpu, Keyboard, MousePointer,
} from 'lucide-react'
import { useVmStore } from '@/stores/vmStore'
import { vmApi } from '@/lib/api'
import { toast } from '@/components/ui/use-toast'
import { cn } from '@/lib/utils'

interface VmConsoleDisplayProps {
  vmId: string
  vmName: string
  vmState: string
  vcpus: number
  memoryMib: number
}

// ─── NovaVM Display Canvas ────────────────────────────────────────────────────
// Renders the real VGA framebuffer from the WHP vCPU thread at ~30fps.
// The backend delivers base64-encoded raw RGBA bytes (width × height × 4).

function VmCanvas({ vmId, active }: { vmId: string; active: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const rafRef    = useRef<number>(0)
  const lastSeq   = useRef<number>(-1)

  const fetchFrame = useCallback(async () => {
    if (!active) return
    try {
      const raw = await vmApi.getFramebuffer(vmId) as {
        available: boolean
        width?: number
        height?: number
        rgba_b64?: string
        seq?: number
      }

      if (raw.available && raw.rgba_b64 && raw.width && raw.height) {
        if (raw.seq === lastSeq.current) return // identical frame — skip draw
        lastSeq.current = raw.seq ?? -1

        const canvas = canvasRef.current
        if (!canvas) return
        const ctx = canvas.getContext('2d')
        if (!ctx) return

        // Decode base64 → Uint8ClampedArray
        const binary = atob(raw.rgba_b64)
        const bytes  = new Uint8ClampedArray(binary.length)
        for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)

        canvas.width  = raw.width
        canvas.height = raw.height
        ctx.putImageData(new ImageData(bytes, raw.width, raw.height), 0, 0)
      }
    } catch { /* ignore poll errors */ }

    rafRef.current = window.setTimeout(fetchFrame, 33) // ~30fps
  }, [vmId, active])

  useEffect(() => {
    if (active) fetchFrame()
    return () => { clearTimeout(rafRef.current) }
  }, [active, fetchFrame])

  return (
    <div className="relative w-full flex items-center justify-center bg-black rounded-xl overflow-hidden"
         style={{ aspectRatio: '8/5', minHeight: 320 }}>
      {/* CRT scanline overlay */}
      <div className="absolute inset-0 pointer-events-none z-10"
           style={{ backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 1px, rgba(0,0,0,0.07) 1px, rgba(0,0,0,0.07) 2px)' }} />
      <canvas
        ref={canvasRef}
        className="w-full h-full object-contain"
        style={{ imageRendering: 'pixelated' }}
      />
      {/* Corner badge */}
      <div className="absolute top-2 right-2 z-20 px-2 py-0.5 rounded text-[9px] font-mono font-semibold
                      bg-violet-500/20 text-violet-300 border border-violet-500/30 backdrop-blur-sm">
        NovaVM Display
      </div>
    </div>
  )
}

// ─── Serial Console ───────────────────────────────────────────────────────────

function SerialConsole({ vmId, active }: { vmId: string; active: boolean }) {
  const [output, setOutput] = useState('')
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!active) return
    const id = setInterval(async () => {
      try {
        const text = await vmApi.getSerialOutput(vmId)
        if (text && text.length > 0) {
          setOutput(prev => {
            const next = prev + text
            return next.length > 524288 ? next.slice(-524288) : next
          })
        }
      } catch { /* ignore */ }
    }, 500)
    return () => clearInterval(id)
  }, [vmId, active])

  // Auto-scroll
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [output])

  return (
    <div className="rounded-xl border border-violet-500/20 bg-[#080808] overflow-hidden">
      <div className="flex items-center gap-2 px-4 py-2 bg-[#101010] border-b border-border/40 text-xs">
        <div className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
        <span className="font-semibold text-foreground">Serial Console — COM1</span>
        <span className="ml-auto text-muted-foreground/50 text-[10px]">real guest UART output</span>
      </div>
      <div className="p-3 h-48 overflow-y-auto font-mono text-[11px] leading-relaxed text-emerald-300/90
                      whitespace-pre-wrap" style={{ scrollbarWidth: 'thin', scrollbarColor: '#333 transparent' }}>
        {output.length === 0
          ? <span className="text-muted-foreground/40 italic">Waiting for guest output…</span>
          : output}
        <div ref={bottomRef} />
      </div>
    </div>
  )
}

// ─── Main Component ───────────────────────────────────────────────────────────

export function VmConsoleDisplay({ vmId, vmName, vmState, vcpus, memoryMib }: VmConsoleDisplayProps) {
  const [launching, setLaunching] = useState(false)
  const [activeTab, setActiveTab] = useState<'display' | 'serial'>('display')

  const isRunning = vmState === 'running'
  const isStopped = vmState === 'stopped' || vmState === 'crashed'
  const isPaused  = vmState === 'paused'

  const handleStart = async () => {
    setLaunching(true)
    try {
      await useVmStore.getState().startVm(vmId)
      toast({ title: 'VM Starting', description: `${vmName} is booting…` })
    } catch (e) {
      toast({ title: 'Start Failed', description: String(e), variant: 'destructive' })
    } finally {
      setLaunching(false)
    }
  }

  return (
    <div className="space-y-3">

      {/* ── Backend Badge ── */}
      <div className="flex items-center gap-2 px-3 py-2 rounded-xl
                      bg-violet-500/8 border border-violet-500/25 text-xs text-violet-300">
        <Cpu size={12} className="text-violet-400" />
        <span className="font-semibold">NovaVM-WHP</span>
        <span className="text-violet-400/60">native hardware virtualisation · Windows Hypervisor Platform</span>
        <span className={cn(
          'ml-auto px-2 py-0.5 rounded-full text-[9px] font-bold border',
          isRunning ? 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30 animate-pulse' :
          isPaused  ? 'bg-amber-500/15  text-amber-400  border-amber-500/30' :
                      'bg-slate-500/15  text-slate-400  border-slate-500/30'
        )}>
          {vmState.toUpperCase()}
        </span>
      </div>

      {/* ── Tab Bar ── */}
      {isRunning && (
        <div className="flex gap-1 p-1 bg-muted/40 rounded-xl border border-border/40">
          {(['display', 'serial'] as const).map(tab => (
            <button key={tab} onClick={() => setActiveTab(tab)}
              className={cn(
                'flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs font-medium rounded-lg transition-all',
                activeTab === tab
                  ? 'bg-background text-foreground shadow-sm border border-border/60'
                  : 'text-muted-foreground hover:text-foreground'
              )}>
              {tab === 'display' ? <Monitor size={11} /> : <Keyboard size={11} />}
              {tab === 'display' ? 'Display' : 'Serial Console'}
            </button>
          ))}
        </div>
      )}

      {/* ── Display / Console Content ── */}
      <div className="rounded-2xl border border-border bg-[#0a0a0a] overflow-hidden shadow-2xl">

        {/* Header */}
        <div className="flex items-center gap-2.5 px-4 py-2.5 bg-[#141414] border-b border-border/60 text-xs">
          <Monitor size={13} className="text-primary" />
          <span className="font-semibold text-foreground tracking-wide">{vmName}</span>
          <div className="flex items-center gap-1.5 ml-auto text-muted-foreground/70 text-[11px]">
            <Cpu size={10} />
            <span>{vcpus} vCPU</span>
            <span className="text-border">·</span>
            <span>{memoryMib >= 1024 ? `${(memoryMib / 1024).toFixed(0)} GB` : `${memoryMib} MB`} RAM</span>
            <MousePointer size={10} className="ml-1 text-violet-400/50" />
            <span className="text-violet-400/50 text-[9px]">click to capture input</span>
          </div>
        </div>

        {/* Content Area */}
        <div className="relative min-h-[360px]">

          {/* ── Running: real display or serial ── */}
          {isRunning && activeTab === 'display' && (
            <div className="p-3">
              <VmCanvas vmId={vmId} active={true} />
            </div>
          )}

          {isRunning && activeTab === 'serial' && (
            <div className="p-3">
              <SerialConsole vmId={vmId} active={true} />
            </div>
          )}

          {/* ── Stopped ── */}
          {isStopped && (
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-5 p-8">
              {/* NovaVM logo / power-off art */}
              <div className="relative">
                <div className="w-24 h-24 rounded-3xl bg-gradient-to-br from-violet-900/40 to-slate-900/40
                                border border-violet-500/20 flex items-center justify-center shadow-2xl">
                  <Monitor size={44} className="text-violet-400/50" />
                </div>
                <div className="absolute inset-0 rounded-3xl bg-violet-500/5 blur-xl" />
              </div>
              <div className="text-center space-y-1">
                <p className="text-sm font-semibold text-foreground/80">Virtual Machine is Powered Off</p>
                <p className="text-xs text-muted-foreground">
                  NovaVM will boot the guest on your CPU using Windows Hypervisor Platform
                </p>
              </div>
              <button onClick={handleStart} disabled={launching}
                className="flex items-center gap-2.5 px-8 py-3.5 text-sm font-semibold rounded-2xl
                           bg-gradient-to-r from-emerald-500 to-teal-500 text-black
                           hover:from-emerald-400 hover:to-teal-400 transition-all active:scale-95 shadow-lg
                           disabled:opacity-60 disabled:cursor-not-allowed disabled:active:scale-100">
                <Play size={16} />
                {launching ? 'Starting…' : 'Power On'}
              </button>
            </div>
          )}

          {/* ── Paused ── */}
          {isPaused && (
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-5 p-8">
              <div className="w-20 h-20 rounded-2xl bg-amber-500/10 border border-amber-500/30
                              flex items-center justify-center">
                <Pause size={36} className="text-amber-400" />
              </div>
              <div className="text-center">
                <p className="text-sm font-semibold text-foreground">VM is Paused</p>
                <p className="text-xs text-muted-foreground mt-1">vCPU execution is frozen</p>
              </div>
              <button onClick={() => useVmStore.getState().resumeVm(vmId)}
                className="flex items-center gap-2 px-6 py-3 text-sm font-semibold
                           bg-amber-500 text-black hover:bg-amber-400 rounded-2xl transition-all active:scale-95">
                <Play size={14} /> Resume
              </button>
            </div>
          )}
        </div>

        {/* VM Control Toolbar */}
        <div className="flex items-center gap-2 px-4 py-2.5 bg-[#0f0f0f] border-t border-border/40">
          <span className="text-[11px] text-muted-foreground font-medium mr-1">VM Controls:</span>

          {isStopped && (
            <ToolbarBtn onClick={handleStart} disabled={launching}
              label="Power On" icon={<Play size={11} />} variant="success" />
          )}
          {isRunning && (
            <>
              <ToolbarBtn onClick={() => useVmStore.getState().pauseVm(vmId)}
                label="Pause" icon={<Pause size={11} />} variant="warning" />
              <ToolbarBtn onClick={() => useVmStore.getState().stopVm(vmId)}
                label="Power Off" icon={<Square size={11} />} variant="danger" />
              <ToolbarBtn onClick={() => useVmStore.getState().resetVm(vmId)}
                label="Restart" icon={<RotateCcw size={11} />} />
            </>
          )}
          {isPaused && (
            <ToolbarBtn onClick={() => useVmStore.getState().resumeVm(vmId)}
              label="Resume" icon={<Play size={11} />} variant="success" />
          )}

          <div className="ml-auto flex items-center gap-1 text-[9px] font-mono text-violet-400/50">
            <Maximize2 size={9} />
            NovaVM WHP · 30fps
          </div>
        </div>
      </div>
    </div>
  )
}

function ToolbarBtn({
  label, icon, onClick, disabled = false, variant = 'default',
}: {
  label: string; icon: React.ReactNode; onClick: () => void
  disabled?: boolean; variant?: 'default' | 'success' | 'warning' | 'danger'
}) {
  const styles: Record<string, string> = {
    default:  'bg-muted/60 hover:bg-accent text-muted-foreground border-border/50',
    success:  'bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-400 border-emerald-500/30',
    warning:  'bg-amber-500/15 hover:bg-amber-500/25 text-amber-400 border-amber-500/30',
    danger:   'bg-rose-500/15 hover:bg-rose-500/25 text-rose-400 border-rose-500/30',
  }
  return (
    <button onClick={onClick} disabled={disabled}
      className={cn(
        'flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded border transition-colors',
        'disabled:opacity-40 disabled:cursor-not-allowed',
        styles[variant],
      )}>
      {icon}{label}
    </button>
  )
}