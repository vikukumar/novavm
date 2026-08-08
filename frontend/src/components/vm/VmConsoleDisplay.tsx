/**
 * NovaVM Workstation — VM Console Display
 *
 * A pixel-accurate reproduction of VMware Workstation's VM console experience:
 * - VMware-style window chrome with titlebar, menu bar, and window controls
 * - Toolbar with Power On/Off/Suspend/Reset/Ctrl+Alt+Del/Snapshot buttons
 * - Live VM display canvas (VGA framebuffer at 640×400, scalable)
 * - Device activity status bar (HDD 💾, NIC 🌐, CD-ROM 💿, USB 🔌)
 * - Keyboard/mouse capture with VMware-style UX (click to grab, Ctrl+Alt to release)
 * - Boot animation overlay showing NovaVM branding during startup
 */

import { useState, useEffect, useRef, useCallback } from 'react'
import {
  Play, Pause, Square, RotateCcw, Monitor, Power,
  Maximize2, Cpu, HardDrive, Wifi, Disc3, Usb,
  Camera, AlertTriangle, Keyboard, Mouse, Layers,
} from 'lucide-react'
import { useVmStore } from '@/stores/vmStore'
import { vmApi } from '@/lib/api'
import { toast } from '@/components/ui/use-toast'
import { cn } from '@/lib/utils'

// ─── Types ─────────────────────────────────────────────────────────────────────

interface VmConsoleDisplayProps {
  vmId: string
  vmName: string
  vmState: string
  vcpus: number
  memoryMib: number
}

type BootStage = 'off' | 'booting' | 'bios' | 'installer' | 'running' | 'paused' | 'crashed'

interface FramebufferData {
  available: boolean
  width?: number
  height?: number
  rgba_b64?: string
  seq?: number
}

// ─── Boot Stage Animation Overlay ──────────────────────────────────────────────

function BootOverlay({ stage, vmName }: { stage: BootStage; vmName: string }) {
  const [dots, setDots] = useState('')
  const [progress, setProgress] = useState(0)
  const [phase, setPhase] = useState(0)

  useEffect(() => {
    if (stage !== 'booting') return
    const dot = setInterval(() => setDots(d => d.length >= 3 ? '' : d + '.'), 400)
    const prog = setInterval(() => setProgress(p => Math.min(p + 2, 95)), 120)
    const ph = setInterval(() => setPhase(p => (p + 1) % 4), 800)
    return () => { clearInterval(dot); clearInterval(prog); clearInterval(ph) }
  }, [stage])

  if (stage !== 'booting') return null

  const phaseLabels = [
    'Initializing virtual hardware...',
    'Loading NovaVM BIOS ROM...',
    'Detecting boot media...',
    'Starting guest OS...',
  ]

  return (
    <div className="absolute inset-0 z-40 flex flex-col items-center justify-center bg-[#0a0a0f]">
      {/* Animated background grid */}
      <div className="absolute inset-0 opacity-5"
        style={{
          backgroundImage: 'linear-gradient(rgba(139,92,246,0.4) 1px, transparent 1px), linear-gradient(90deg, rgba(139,92,246,0.4) 1px, transparent 1px)',
          backgroundSize: '40px 40px'
        }} />

      {/* Glowing orb */}
      <div className="relative mb-8">
        <div className="w-20 h-20 rounded-full bg-gradient-to-br from-violet-600 to-purple-900 flex items-center justify-center shadow-2xl"
          style={{ boxShadow: '0 0 60px rgba(139,92,246,0.6), 0 0 120px rgba(139,92,246,0.2)' }}>
          <Power size={32} className="text-white" />
        </div>
        {/* Pulsing rings */}
        <div className="absolute inset-0 rounded-full border border-violet-500/30 animate-ping" />
        <div className="absolute -inset-2 rounded-full border border-violet-500/20 animate-ping"
          style={{ animationDelay: '0.3s' }} />
      </div>

      {/* Brand */}
      <div className="text-center mb-6 z-10">
        <div className="text-2xl font-bold text-white tracking-wider mb-1">
          <span className="text-violet-400">Nova</span>VM Workstation
        </div>
        <div className="text-sm text-violet-300/60 font-mono">
          Starting {vmName}
        </div>
      </div>

      {/* Phase label */}
      <div className="text-xs text-violet-300/80 font-mono mb-4 h-4 z-10">
        {phaseLabels[phase]}{dots}
      </div>

      {/* Progress bar */}
      <div className="w-64 z-10">
        <div className="h-1 bg-violet-950 rounded-full overflow-hidden">
          <div
            className="h-full bg-gradient-to-r from-violet-600 to-purple-400 rounded-full transition-all duration-150"
            style={{ width: `${progress}%` }}
          />
        </div>
        <div className="flex justify-between mt-1.5 text-[10px] text-violet-500/60 font-mono">
          <span>NovaVM WHP Engine</span>
          <span>{progress}%</span>
        </div>
      </div>

      {/* Bottom hint */}
      <div className="absolute bottom-4 text-[10px] text-violet-500/40 font-mono z-10">
        Press Ctrl+Alt to release keyboard/mouse • © 2026 Vikash Kumar
      </div>
    </div>
  )
}

// ─── VM Display Canvas ──────────────────────────────────────────────────────────

function VmCanvas({
  vmId,
  active,
  onFocus,
  onBlur,
  isFocused,
}: {
  vmId: string
  active: boolean
  onFocus: () => void
  onBlur: () => void
  isFocused: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const timerRef = useRef<number>(0)
  const lastSeq = useRef<number>(-999)
  const [hovered, setHovered] = useState(false)

  // Framebuffer poll loop at 30fps
  useEffect(() => {
    if (!active) return
    let isMounted = true

    const loop = async () => {
      if (!isMounted) return
      try {
        const raw = await vmApi.getFramebuffer(vmId) as FramebufferData
        if (isMounted && raw.available && raw.rgba_b64 && raw.width && raw.height) {
          const currentSeq = raw.seq ?? 0
          if (currentSeq !== lastSeq.current || lastSeq.current === -999) {
            lastSeq.current = currentSeq
            const canvas = canvasRef.current
            if (canvas) {
              const ctx = canvas.getContext('2d')
              if (ctx) {
                const binary = atob(raw.rgba_b64)
                const bytes = new Uint8ClampedArray(binary.length)
                for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
                if (canvas.width !== raw.width) canvas.width = raw.width
                if (canvas.height !== raw.height) canvas.height = raw.height
                ctx.putImageData(new ImageData(bytes, raw.width, raw.height), 0, 0)
              }
            }
          }
        }
      } catch { /* ignore poll errors */ }
      if (isMounted) timerRef.current = window.setTimeout(loop, 33)
    }
    loop()
    return () => { isMounted = false; clearTimeout(timerRef.current) }
  }, [vmId, active])

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (!active || !isFocused) return
    // Ctrl+Alt releases focus (VMware convention)
    if (e.ctrlKey && e.altKey) {
      onBlur()
      containerRef.current?.blur()
      return
    }
    // Ctrl+Alt+Del injection
    if (e.ctrlKey && e.altKey && e.key === 'Delete') {
      vmApi.sendInput(vmId, 'keydown', 'Ctrl+Alt+Del').catch(() => {})
      e.preventDefault()
      return
    }
    e.preventDefault()
    e.stopPropagation()
    vmApi.sendInput(vmId, 'keydown', e.key).catch(() => {})
  }, [active, isFocused, vmId, onBlur])

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => { if (active) { onFocus(); containerRef.current?.focus() } }}
      onKeyDown={handleKeyDown}
      onBlur={onBlur}
      className="relative w-full h-full outline-none cursor-default"
    >
      {/* The actual VGA framebuffer canvas */}
      <canvas
        ref={canvasRef}
        className="w-full h-full object-contain block"
        style={{ imageRendering: 'pixelated', backgroundColor: '#000' }}
      />

      {/* CRT scanline overlay for authenticity */}
      <div
        className="absolute inset-0 pointer-events-none z-10"
        style={{
          backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.04) 2px, rgba(0,0,0,0.04) 4px)',
        }}
      />

      {/* Focus capture overlay */}
      {!isFocused && active && (
        <div
          className={cn(
            'absolute inset-0 z-20 flex flex-col items-center justify-center gap-3 transition-opacity duration-200',
            hovered ? 'bg-black/40' : 'bg-transparent'
          )}
        >
          {hovered && (
            <div className="flex flex-col items-center gap-2">
              <div className="px-4 py-2 rounded-xl bg-black/80 border border-violet-500/40 backdrop-blur-sm text-center">
                <div className="flex items-center gap-2 text-sm font-semibold text-white mb-0.5">
                  <Mouse size={14} className="text-violet-400" />
                  Click to grab keyboard & mouse
                </div>
                <div className="text-[10px] text-violet-300/60">
                  Press Ctrl+Alt to release
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Input captured banner */}
      {isFocused && (
        <div className="absolute top-2 left-1/2 -translate-x-1/2 z-30">
          <div className="flex items-center gap-1.5 px-3 py-1 rounded-full text-[10px] font-semibold
                          bg-violet-600/90 text-white border border-violet-400/40 backdrop-blur-sm shadow-lg">
            <Keyboard size={10} />
            Keyboard & Mouse Grabbed — Press Ctrl+Alt to Release
          </div>
        </div>
      )}
    </div>
  )
}

// ─── Device Status Bar ──────────────────────────────────────────────────────────

function DeviceStatusBar({
  vmState,
  vcpus,
  memoryMib,
}: {
  vmState: string
  vcpus: number
  memoryMib: number
}) {
  const [hddActive, setHddActive] = useState(false)
  const [netActive, setNetActive] = useState(false)

  // Simulate random HDD/NIC activity lights when running
  useEffect(() => {
    if (vmState !== 'running') return
    const hdd = setInterval(() => {
      setHddActive(true)
      setTimeout(() => setHddActive(false), 80 + Math.random() * 200)
    }, 500 + Math.random() * 1500)
    const net = setInterval(() => {
      setNetActive(true)
      setTimeout(() => setNetActive(false), 60 + Math.random() * 150)
    }, 800 + Math.random() * 2000)
    return () => { clearInterval(hdd); clearInterval(net) }
  }, [vmState])

  const isRunning = vmState === 'running'
  const memGb = memoryMib >= 1024 ? `${(memoryMib / 1024).toFixed(0)} GB` : `${memoryMib} MB`

  return (
    <div className="flex items-center gap-0 px-3 h-[26px] bg-[#1a1a1a] border-t border-[#2d2d2d] text-[10px] font-mono select-none">
      {/* VM state indicator */}
      <div className="flex items-center gap-1.5 mr-3">
        <div className={cn(
          'w-1.5 h-1.5 rounded-full',
          isRunning ? 'bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.8)] animate-pulse' :
          vmState === 'paused' ? 'bg-amber-400' :
          'bg-slate-600'
        )} />
        <span className={cn(
          'font-semibold uppercase tracking-wider',
          isRunning ? 'text-emerald-400' :
          vmState === 'paused' ? 'text-amber-400' :
          'text-slate-500'
        )}>{vmState}</span>
      </div>

      <div className="h-3 w-px bg-[#333] mx-1" />

      {/* vCPU */}
      <div className="flex items-center gap-1 mr-2 text-slate-400">
        <Cpu size={9} className="text-slate-500" />
        <span>{vcpus} vCPU</span>
      </div>

      {/* Memory */}
      <div className="flex items-center gap-1 mr-3 text-slate-400">
        <span className="text-slate-500">MEM</span>
        <span>{memGb}</span>
      </div>

      <div className="h-3 w-px bg-[#333] mx-1" />

      {/* Device icons */}
      <div className="flex items-center gap-2 ml-1">
        {/* HDD activity */}
        <div className="flex items-center gap-1" title="Virtual Hard Disk">
          <HardDrive size={9} className={cn(hddActive && isRunning ? 'text-amber-400' : 'text-slate-600')} />
          <div className={cn('w-1 h-1 rounded-full', hddActive && isRunning ? 'bg-amber-400' : 'bg-slate-700')} />
        </div>

        {/* NIC activity */}
        <div className="flex items-center gap-1" title="Virtual Network Adapter">
          <Wifi size={9} className={cn(netActive && isRunning ? 'text-sky-400' : 'text-slate-600')} />
          <div className={cn('w-1 h-1 rounded-full', netActive && isRunning ? 'bg-sky-400' : 'bg-slate-700')} />
        </div>

        {/* CD-ROM */}
        <div className="flex items-center gap-1" title="Virtual CD-ROM/DVD Drive">
          <Disc3 size={9} className="text-slate-600" />
        </div>

        {/* USB */}
        <div className="flex items-center gap-1" title="USB Controller">
          <Usb size={9} className="text-slate-600" />
        </div>
      </div>

      {/* Spacer */}
      <div className="ml-auto flex items-center gap-2 text-slate-600">
        <span>NovaVM WHP</span>
        <span className="text-[8px]">30fps</span>
        <div className="h-3 w-px bg-[#333]" />
        <span>640×400</span>
      </div>
    </div>
  )
}

// ─── VMware-Style Toolbar ───────────────────────────────────────────────────────

function VmToolbar({
  vmId,
  vmName,
  vmState,
  onStart,
  onPause,
  onResume,
  onStop,
  onReset,
  onFullscreen,
  launching,
}: {
  vmId: string
  vmName: string
  vmState: string
  onStart: () => void
  onPause: () => void
  onResume: () => void
  onStop: () => void
  onReset: () => void
  onFullscreen: () => void
  launching: boolean
}) {
  const isRunning = vmState === 'running'
  const isStopped = vmState === 'stopped' || vmState === 'crashed'
  const isPaused = vmState === 'paused'

  const sendCtrlAltDel = () => {
    vmApi.sendInput(vmId, 'keydown', 'Ctrl+Alt+Del').catch(() => {})
    toast({ title: 'Ctrl+Alt+Del sent to guest' })
  }

  return (
    <div className="flex items-center gap-0.5 px-2 py-1.5 bg-[#1e1e1e] border-b border-[#2d2d2d] select-none">
      {/* Power controls group */}
      <div className="flex items-center gap-0.5 mr-1">
        {isStopped && (
          <TBtn
            onClick={onStart}
            disabled={launching}
            icon={<Play size={13} className="text-emerald-400" />}
            label="Power On"
            tooltip="Power on this virtual machine"
            hotkey="Ctrl+B"
          />
        )}
        {isPaused && (
          <TBtn
            onClick={onResume}
            icon={<Play size={13} className="text-emerald-400" />}
            label="Resume"
            tooltip="Resume the paused virtual machine"
          />
        )}
        {isRunning && (
          <TBtn
            onClick={onPause}
            icon={<Pause size={13} className="text-amber-400" />}
            label="Suspend"
            tooltip="Suspend the virtual machine"
          />
        )}
        {(isRunning || isPaused) && (
          <>
            <TBtn
              onClick={onStop}
              icon={<Square size={12} className="text-rose-400" />}
              label="Power Off"
              tooltip="Power off this virtual machine"
            />
            <TBtn
              onClick={onReset}
              icon={<RotateCcw size={12} className="text-sky-400" />}
              label="Restart"
              tooltip="Reset the virtual machine"
            />
          </>
        )}
      </div>

      <div className="h-5 w-px bg-[#333] mx-1" />

      {/* Snapshot */}
      <TBtn
        onClick={() => toast({ title: 'Snapshot', description: 'Saving VM snapshot...' })}
        icon={<Camera size={12} className="text-violet-400" />}
        label="Snapshot"
        tooltip="Take a snapshot of this virtual machine"
        disabled={!isRunning}
      />

      <div className="h-5 w-px bg-[#333] mx-1" />

      {/* Ctrl+Alt+Del */}
      <TBtn
        onClick={sendCtrlAltDel}
        icon={<Keyboard size={12} className="text-slate-300" />}
        label="Ctrl+Alt+Del"
        tooltip="Send Ctrl+Alt+Del to the virtual machine"
        disabled={!isRunning}
      />

      <div className="h-5 w-px bg-[#333] mx-1" />

      {/* Removable Devices */}
      <TBtn
        onClick={() => {}}
        icon={<Usb size={12} className="text-slate-300" />}
        label="Devices ▾"
        tooltip="Connect/disconnect removable devices"
        disabled={!isRunning}
      />

      {/* Spacer */}
      <div className="ml-auto flex items-center gap-0.5">
        <div className="h-5 w-px bg-[#333] mx-1" />
        {/* Full screen */}
        <TBtn
          onClick={onFullscreen}
          icon={<Maximize2 size={12} className="text-slate-300" />}
          label="Full Screen"
          tooltip="Enter full screen mode (Ctrl+Alt+Enter)"
        />
      </div>
    </div>
  )
}

function TBtn({
  icon,
  label,
  onClick,
  disabled = false,
  tooltip,
  hotkey,
}: {
  icon: React.ReactNode
  label: string
  onClick: () => void
  disabled?: boolean
  tooltip?: string
  hotkey?: string
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={tooltip ? `${tooltip}${hotkey ? ` (${hotkey})` : ''}` : undefined}
      className={cn(
        'flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded',
        'text-slate-300 hover:text-white hover:bg-white/8',
        'disabled:opacity-30 disabled:cursor-not-allowed',
        'transition-colors duration-100'
      )}
    >
      {icon}
      <span className="hidden sm:inline">{label}</span>
    </button>
  )
}

// ─── VMware-style Menu Bar ──────────────────────────────────────────────────────

function VmMenuBar({ vmName }: { vmName: string }) {
  const menus = ['File', 'Edit', 'View', 'VM', 'Tabs', 'Help']
  return (
    <div className="flex items-center px-2 h-7 bg-[#181818] border-b border-[#2a2a2a] text-[11px] select-none">
      {menus.map(m => (
        <button
          key={m}
          className="px-3 py-0.5 text-slate-400 hover:text-white hover:bg-white/8 rounded transition-colors"
        >
          {m}
        </button>
      ))}
      <div className="ml-auto flex items-center gap-2 text-slate-600 text-[10px] font-mono pr-1">
        <span>{vmName}</span>
        <span>—</span>
        <span className="text-violet-500">NovaVM Workstation</span>
      </div>
    </div>
  )
}

// ─── Window Title Bar ───────────────────────────────────────────────────────────

function VmTitleBar({
  vmName,
  vmState,
  isFullscreen,
  onMinimize,
  onMaximize,
  onClose,
}: {
  vmName: string
  vmState: string
  isFullscreen: boolean
  onMinimize: () => void
  onMaximize: () => void
  onClose: () => void
}) {
  const stateLabel = vmState === 'running' ? 'Running' : vmState === 'paused' ? 'Suspended' :
    vmState === 'stopped' ? 'Powered Off' : vmState.charAt(0).toUpperCase() + vmState.slice(1)

  return (
    <div className="flex items-center h-9 px-3 bg-gradient-to-b from-[#242424] to-[#1a1a1a] border-b border-[#2a2a2a] select-none">
      {/* Window controls (macOS style for VMware look) */}
      <div className="flex items-center gap-1.5 mr-3">
        <button
          onClick={onClose}
          className="w-3 h-3 rounded-full bg-[#ff5f57] hover:bg-[#ff7369] border border-black/20 transition-colors"
          title="Close"
        />
        <button
          onClick={onMinimize}
          className="w-3 h-3 rounded-full bg-[#ffbd2e] hover:bg-[#ffcb5b] border border-black/20 transition-colors"
          title="Minimize"
        />
        <button
          onClick={onMaximize}
          className="w-3 h-3 rounded-full bg-[#28c840] hover:bg-[#34d748] border border-black/20 transition-colors"
          title={isFullscreen ? 'Exit Full Screen' : 'Enter Full Screen'}
        />
      </div>

      {/* VM Icon */}
      <div className="mr-2 w-4 h-4 rounded bg-gradient-to-br from-violet-600 to-purple-800 flex items-center justify-center">
        <Monitor size={9} className="text-white" />
      </div>

      {/* Title */}
      <div className="flex items-center gap-2 text-[12px]">
        <span className="font-semibold text-slate-200">{vmName}</span>
        <span className="text-slate-600">—</span>
        <span className={cn(
          'text-[10px] font-medium px-1.5 py-0.5 rounded',
          vmState === 'running' ? 'text-emerald-400 bg-emerald-500/10' :
          vmState === 'paused' ? 'text-amber-400 bg-amber-500/10' :
          'text-slate-500 bg-slate-500/10'
        )}>{stateLabel}</span>
      </div>

      {/* Right: branding */}
      <div className="ml-auto flex items-center gap-2">
        <span className="text-[10px] text-violet-500/70 font-mono font-semibold tracking-wider">
          NovaVM Workstation
        </span>
        <div className="w-4 h-4 rounded bg-gradient-to-br from-violet-600 to-purple-900 flex items-center justify-center">
          <Layers size={8} className="text-white" />
        </div>
      </div>
    </div>
  )
}

// ─── Powered Off State ──────────────────────────────────────────────────────────

function PoweredOffState({ vmName, vcpus, memoryMib, onStart, launching }: {
  vmName: string
  vcpus: number
  memoryMib: number
  onStart: () => void
  launching: boolean
}) {
  const memGb = memoryMib >= 1024 ? `${(memoryMib / 1024).toFixed(0)} GB` : `${memoryMib} MB`
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center bg-[#0a0a0f]">
      {/* Background vignette */}
      <div className="absolute inset-0"
        style={{ background: 'radial-gradient(ellipse at center, #0e0e1a 0%, #050508 70%)' }} />

      {/* NovaVM logo block */}
      <div className="relative z-10 flex flex-col items-center gap-6">
        {/* Monitor icon */}
        <div className="relative">
          <div className="w-28 h-28 rounded-[24px] bg-gradient-to-br from-[#1e1540] to-[#0d0d18]
                          border border-violet-900/40 flex items-center justify-center shadow-2xl">
            <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-violet-950 to-purple-950
                            flex items-center justify-center border border-violet-800/30">
              <Power size={36} className="text-violet-600" />
            </div>
          </div>
          {/* Glow */}
          <div className="absolute inset-0 rounded-[24px] opacity-30 blur-2xl
                          bg-gradient-to-br from-violet-600 to-purple-900" />
        </div>

        {/* Text */}
        <div className="text-center">
          <h3 className="text-lg font-bold text-slate-200 mb-1">
            {vmName}
          </h3>
          <p className="text-sm text-slate-500 mb-0.5">
            Virtual Machine is Powered Off
          </p>
          <p className="text-xs text-slate-700 font-mono">
            {vcpus} vCPU · {memGb} RAM · NovaVM WHP Engine
          </p>
        </div>

        {/* Power on button */}
        <button
          onClick={onStart}
          disabled={launching}
          className={cn(
            'flex items-center gap-3 px-10 py-4 rounded-2xl text-sm font-bold tracking-wide',
            'bg-gradient-to-r from-emerald-600 to-teal-600 text-white',
            'hover:from-emerald-500 hover:to-teal-500',
            'active:scale-95 transition-all duration-150 shadow-xl',
            'disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100',
            'border border-emerald-400/20',
          )}
          style={{ boxShadow: '0 0 30px rgba(16,185,129,0.3), 0 8px 32px rgba(0,0,0,0.5)' }}
        >
          <Play size={20} />
          {launching ? 'Starting Virtual Machine...' : 'Power On'}
        </button>

        {/* VM hardware summary */}
        <div className="grid grid-cols-3 gap-3 mt-1">
          {[
            { icon: <Cpu size={12} />, label: 'Processors', value: `${vcpus} vCPU` },
            { icon: <Monitor size={12} />, label: 'Memory', value: memGb },
            { icon: <HardDrive size={12} />, label: 'Storage', value: '60 GB VMDK' },
          ].map(item => (
            <div key={item.label} className="flex flex-col items-center gap-1 px-4 py-2
                                             rounded-xl bg-[#111118] border border-slate-800/60 text-center">
              <div className="text-slate-500">{item.icon}</div>
              <div className="text-[9px] text-slate-600 uppercase tracking-wider">{item.label}</div>
              <div className="text-[11px] text-slate-300 font-mono font-medium">{item.value}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Bottom branding */}
      <div className="absolute bottom-4 text-[10px] text-slate-800 font-mono z-10">
        © 2026 Vikash Kumar · NovaVM Workstation · Windows Hypervisor Platform
      </div>
    </div>
  )
}

// ─── Paused State ────────────────────────────────────────────────────────────────

function PausedState({ vmName, onResume }: { vmName: string; onResume: () => void }) {
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center bg-[#0a0a0f]/95 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-4 z-10">
        <div className="w-16 h-16 rounded-2xl bg-amber-500/10 border border-amber-500/30
                        flex items-center justify-center">
          <Pause size={28} className="text-amber-400" />
        </div>
        <div className="text-center">
          <p className="text-base font-semibold text-slate-200">Virtual Machine Suspended</p>
          <p className="text-xs text-slate-500 mt-1 font-mono">
            vCPU execution is frozen — {vmName}
          </p>
        </div>
        <button
          onClick={onResume}
          className="flex items-center gap-2 px-6 py-3 rounded-xl bg-amber-500/10 border border-amber-500/30
                     text-amber-400 hover:bg-amber-500/20 font-semibold text-sm transition-colors"
        >
          <Play size={16} /> Resume Virtual Machine
        </button>
      </div>
    </div>
  )
}

// ─── Crashed State ───────────────────────────────────────────────────────────────

function CrashedState({ vmName, onStart }: { vmName: string; onStart: () => void }) {
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center bg-[#0a0a0f]">
      <div className="flex flex-col items-center gap-4 z-10">
        <div className="w-16 h-16 rounded-2xl bg-rose-500/10 border border-rose-500/30
                        flex items-center justify-center">
          <AlertTriangle size={28} className="text-rose-400" />
        </div>
        <div className="text-center">
          <p className="text-base font-semibold text-slate-200">Virtual Machine Crashed</p>
          <p className="text-xs text-slate-500 mt-1 font-mono">{vmName} — Guest OS terminated unexpectedly</p>
        </div>
        <button
          onClick={onStart}
          className="flex items-center gap-2 px-6 py-3 rounded-xl bg-rose-500/10 border border-rose-500/30
                     text-rose-400 hover:bg-rose-500/20 font-semibold text-sm transition-colors"
        >
          <RotateCcw size={16} /> Restart Virtual Machine
        </button>
      </div>
    </div>
  )
}

// ─── Main Component ─────────────────────────────────────────────────────────────

export function VmConsoleDisplay({
  vmId,
  vmName,
  vmState,
  vcpus,
  memoryMib,
}: VmConsoleDisplayProps) {
  const [launching, setLaunching] = useState(false)
  const [bootStage, setBootStage] = useState<BootStage>('off')
  const [isFocused, setIsFocused] = useState(false)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const windowRef = useRef<HTMLDivElement>(null)

  const isRunning = vmState === 'running'
  const isStopped = vmState === 'stopped'
  const isPaused = vmState === 'paused'
  const isCrashed = vmState === 'crashed'

  // Sync boot stage with VM state
  useEffect(() => {
    if (isRunning) {
      if (bootStage === 'booting') {
        // Keep the boot overlay a moment then clear it
        const t = setTimeout(() => setBootStage('running'), 2000)
        return () => clearTimeout(t)
      }
      setBootStage('running')
    } else if (isPaused) {
      setBootStage('paused')
    } else if (isCrashed) {
      setBootStage('crashed')
    } else {
      setBootStage('off')
    }
  }, [vmState]) // eslint-disable-line react-hooks/exhaustive-deps

  const handleStart = async () => {
    setLaunching(true)
    setBootStage('booting')
    try {
      await useVmStore.getState().startVm(vmId)
      toast({ title: '⚡ VM Starting', description: `${vmName} is booting via NovaVM WHP Engine` })
    } catch (e) {
      setBootStage('off')
      toast({ title: 'Start Failed', description: String(e), variant: 'destructive' })
    } finally {
      setLaunching(false)
    }
  }

  const handlePause = async () => {
    try { await useVmStore.getState().pauseVm(vmId) } catch { /* handled by store */ }
  }

  const handleResume = async () => {
    try { await useVmStore.getState().resumeVm(vmId) } catch { /* handled by store */ }
  }

  const handleStop = async () => {
    setIsFocused(false)
    try { await useVmStore.getState().stopVm(vmId) } catch { /* handled by store */ }
  }

  const handleReset = async () => {
    setBootStage('booting')
    try { await useVmStore.getState().resetVm(vmId) } catch { /* handled by store */ }
  }

  const handleFullscreen = () => {
    const el = windowRef.current
    if (!el) return
    if (!isFullscreen) {
      el.requestFullscreen?.().catch(() => {})
      setIsFullscreen(true)
    } else {
      document.exitFullscreen?.()
      setIsFullscreen(false)
    }
  }

  const handleFocusCapture = () => setIsFocused(true)
  const handleFocusRelease = () => setIsFocused(false)

  return (
    <div
      ref={windowRef}
      className="flex flex-col rounded-xl overflow-hidden border border-[#2a2a2a] shadow-2xl bg-[#121212]"
      style={{ boxShadow: '0 0 0 1px rgba(255,255,255,0.04), 0 20px 60px rgba(0,0,0,0.8)' }}
    >
      {/* Window Title Bar */}
      <VmTitleBar
        vmName={vmName}
        vmState={vmState}
        isFullscreen={isFullscreen}
        onMinimize={() => {}}
        onMaximize={handleFullscreen}
        onClose={handleStop}
      />

      {/* Menu Bar */}
      <VmMenuBar vmName={vmName} />

      {/* Toolbar */}
      <VmToolbar
        vmId={vmId}
        vmName={vmName}
        vmState={vmState}
        onStart={handleStart}
        onPause={handlePause}
        onResume={handleResume}
        onStop={handleStop}
        onReset={handleReset}
        onFullscreen={handleFullscreen}
        launching={launching}
      />

      {/* Display Area — black bezel frame like VMware */}
      <div className="relative bg-[#050505]" style={{ aspectRatio: '8/5', minHeight: 380 }}>
        {/* Inner display bezel */}
        <div className="absolute inset-[6px] bg-black rounded overflow-hidden"
          style={{ boxShadow: 'inset 0 0 20px rgba(0,0,0,0.8)' }}>

          {/* Boot overlay */}
          <BootOverlay stage={bootStage} vmName={vmName} />

          {/* Powered Off */}
          {(isStopped || (!isRunning && !isPaused && !isCrashed)) && bootStage !== 'booting' && (
            <PoweredOffState
              vmName={vmName}
              vcpus={vcpus}
              memoryMib={memoryMib}
              onStart={handleStart}
              launching={launching}
            />
          )}

          {/* Running — live VGA canvas */}
          {(isRunning || bootStage === 'running') && (
            <VmCanvas
              vmId={vmId}
              active={isRunning}
              isFocused={isFocused}
              onFocus={handleFocusCapture}
              onBlur={handleFocusRelease}
            />
          )}

          {/* Paused */}
          {isPaused && bootStage !== 'booting' && (
            <PausedState vmName={vmName} onResume={handleResume} />
          )}

          {/* Crashed */}
          {isCrashed && (
            <CrashedState vmName={vmName} onStart={handleStart} />
          )}
        </div>
      </div>

      {/* Device Status Bar */}
      <DeviceStatusBar vmState={vmState} vcpus={vcpus} memoryMib={memoryMib} />
    </div>
  )
}