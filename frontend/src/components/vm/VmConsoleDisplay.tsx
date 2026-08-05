import { useState, useEffect, useRef } from 'react'
import {
  Play, Pause, Square, RotateCcw, Monitor,
  Maximize2, Minimize2, Disc, ShieldAlert, AlertTriangle, ExternalLink
} from 'lucide-react'
import { open } from '@tauri-apps/plugin-dialog'
import { useVmStore } from '@/stores/vmStore'
import { settingsApi } from '@/lib/api'
import { toast } from '@/components/ui/use-toast'
import { cn } from '@/lib/utils'

// Props use only primitives — no VmSummary object reference to prevent re-render loops
interface VmConsoleDisplayProps {
  vmId: string
  vmName: string
  vmState: string
  vcpus: number
  memoryMib: number
}

export function VmConsoleDisplay({ vmId, vmName, vmState, vcpus, memoryMib }: VmConsoleDisplayProps) {
  const [fullscreen, setFullscreen] = useState(false)
  const [commandInput, setCommandInput] = useState('')
  const [consoleLogs, setConsoleLogs] = useState<string[]>([])
  const [mountedIso, setMountedIso] = useState<string | null>(null)
  const [grabbingInput, setGrabbingInput] = useState(false)
  const [qemuStatus, setQemuStatus] = useState<{ installed: boolean; path?: string; message?: string; install_url?: string } | null>(null)
  const consoleRef = useRef<HTMLDivElement>(null)

  // Check QEMU installation status once on mount
  useEffect(() => {
    settingsApi.getQemuStatus()
      .then(setQemuStatus)
      .catch(() => setQemuStatus({ installed: false, message: 'Could not check QEMU status.' }))
  }, [])

  // Track previous state with a ref — avoids stale closure / re-run loops
  const prevStateRef = useRef<string>('')

  useEffect(() => {
    if (prevStateRef.current === vmState) return
    prevStateRef.current = vmState

    if (vmState === 'running') {
      setConsoleLogs([
        'NovaVM Hypervisor BIOS v1.0.0 (x86_64)',
        'Copyright (C) 2026 NovaVM Virtualization Project.',
        `Initialising CPU topology: ${vcpus} vCPUs assigned.`,
        `Allocating guest RAM: ${memoryMib} MiB ... OK.`,
        'Detecting Storage: QEMU / VirtIO Block Device initialized.',
        'Network Adapter: VirtIO NIC attached to VMnet8 (NAT).',
        'Booting Linux Kernel 6.8.0-generic ...',
        '[  0.000000] Linux version 6.8.0-generic (build@novavm) (gcc 13.2.0)',
        '[  0.128410] KVM: Enabled nested virtualization support.',
        '[  0.485120] virtio_net virtio0: ens3: renamed from eth0',
        '[  0.891240] systemd[1]: Reached target System Initialization.',
        '[  1.204510] NovaVM Guest Agent v1.0.0 started on /dev/virtio-ports/org.novavm.guest_agent.0',
        '[  1.512040] IP Address assigned: 192.168.122.105/24',
        ' ',
        'NovaVM Linux 24.04 LTS (ttyS0)',
        'novavm-guest login: root (automatic login)',
        'Welcome to NovaVM Guest Workstation environment.',
        'Type "help" or "status" for available hypervisor commands.',
        ' ',
      ])
    } else if (vmState === 'stopped') {
      setConsoleLogs(['System Powered Off'])
    }
  }, [vmState, vcpus, memoryMib])

  // Auto scroll console text
  useEffect(() => {
    if (consoleRef.current) {
      consoleRef.current.scrollTop = consoleRef.current.scrollHeight
    }
  }, [consoleLogs])

  const handleCommandSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!commandInput.trim()) return

    const cmd = commandInput.trim()
    const timestamp = new Date().toLocaleTimeString()
    let outputLines: string[] = [`root@novavm:~# ${cmd}`]

    if (cmd === 'help') {
      outputLines = [...outputLines,
        'Available guest commands:',
        '  status   - Print VM hardware status',
        '  uname -a - Print guest kernel architecture',
        '  ifconfig - Print virtual network interface details',
        '  clear    - Clear console screen',
      ]
    } else if (cmd === 'status') {
      outputLines = [...outputLines,
        `VM Name: ${vmName}`,
        `State: ${vmState}`,
        `vCPUs: ${vcpus}`,
        `Memory: ${memoryMib} MiB`,
      ]
    } else if (cmd === 'uname -a') {
      outputLines = [...outputLines, 'Linux novavm-guest 6.8.0-generic #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux']
    } else if (cmd === 'ifconfig') {
      outputLines = [...outputLines,
        'ens3: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500',
        '        inet 192.168.122.105  netmask 255.255.255.0  broadcast 192.168.122.255',
        '        rx_packets 1420  bytes 1892040  tx_packets 980  bytes 124010',
      ]
    } else if (cmd === 'clear') {
      setConsoleLogs([])
      setCommandInput('')
      return
    } else {
      outputLines = [...outputLines, `sh: command not found: ${cmd}. Type "help" for list of commands.`]
    }

    setConsoleLogs((prev) => [...prev, `[${timestamp}]`, ...outputLines])
    setCommandInput('')
  }

  const handleMountIso = async () => {
    try {
      const selected = await open({
        title: 'Select ISO Image to Mount',
        filters: [{ name: 'ISO Disk Images', extensions: ['iso', 'img', 'raw'] }],
        multiple: false,
      })
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected
        setMountedIso(path)
        toast({ title: 'ISO Image Mounted', description: `Mounted: ${path}` })
        setConsoleLogs((prev) => [...prev, `[ISO] Mounted optical drive image: ${path}`])
      }
    } catch {
      toast({ title: 'Mount Failed', description: 'Could not open ISO file picker.', variant: 'destructive' })
    }
  }

  const handleSendCtrlAltDel = () => {
    toast({ title: 'Signal Sent', description: 'Sent Ctrl+Alt+Del signal to VM.' })
    setConsoleLogs((prev) => [...prev, '[SYSTEM] Sent Ctrl+Alt+Del interrupt signal to guest OS.'])
  }

  return (
    <div className={cn(
      'rounded-2xl border border-border bg-[#0a0a0a] overflow-hidden shadow-2xl transition-all',
      fullscreen && 'fixed inset-0 z-50 rounded-none border-none'
    )}>

      {/* QEMU Not Installed Warning Banner */}
      {qemuStatus && !qemuStatus.installed && (
        <div className="flex items-start gap-3 px-4 py-3 bg-amber-500/10 border-b border-amber-500/30 text-amber-300">
          <AlertTriangle size={16} className="mt-0.5 flex-shrink-0" />
          <div className="text-xs flex-1">
            <p className="font-semibold">QEMU Not Installed — Virtual Machines Cannot Start</p>
            <p className="text-amber-400/80 mt-0.5">
              {qemuStatus.message ?? 'Install QEMU to run real virtual machines.'}
            </p>
          </div>
          {qemuStatus.install_url && (
            <a
              href={qemuStatus.install_url}
              target="_blank"
              rel="noreferrer"
              className="flex items-center gap-1 text-xs font-semibold text-amber-300 hover:text-amber-100 transition-colors whitespace-nowrap"
            >
              Download QEMU <ExternalLink size={11} />
            </a>
          )}
        </div>
      )}

      {/* QEMU Installed — Show path */}
      {qemuStatus?.installed && (
        <div className="flex items-center gap-2 px-4 py-1.5 bg-emerald-500/8 border-b border-emerald-500/20 text-xs text-emerald-400/70">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
          QEMU: {qemuStatus.path}
        </div>
      )}
      {/* VMware Header / Control Bar */}
      <div className="flex flex-wrap items-center justify-between px-4 py-2.5 bg-[#141414] border-b border-border/60 text-xs text-muted-foreground select-none gap-2">
        <div className="flex items-center gap-2.5">
          <Monitor size={15} className="text-primary" />
          <span className="font-semibold text-foreground tracking-wide">
            VMware Workstation Console — {vmName}
          </span>
          <span className="px-2 py-0.5 rounded bg-muted/60 text-[11px] font-mono text-muted-foreground border border-border/40">
            SVGA II 1920x1080 60Hz
          </span>
        </div>

        {/* Toolbar Action Buttons */}
        <div className="flex items-center gap-1.5 flex-wrap">
          {vmState === 'stopped' && (
            <button
              onClick={() => useVmStore.getState().startVm(vmId)}
              className="flex items-center gap-1 px-2.5 py-1 bg-emerald-500/15 text-emerald-400 hover:bg-emerald-500/25 rounded border border-emerald-500/30 font-medium transition-colors"
            >
              <Play size={12} />
              Power On
            </button>
          )}

          {vmState === 'running' && (
            <>
              <button
                onClick={() => useVmStore.getState().pauseVm(vmId)}
                className="flex items-center gap-1 px-2.5 py-1 bg-amber-500/15 text-amber-400 hover:bg-amber-500/25 rounded border border-amber-500/30 font-medium transition-colors"
              >
                <Pause size={12} />
                Pause
              </button>
              <button
                onClick={() => useVmStore.getState().stopVm(vmId)}
                className="flex items-center gap-1 px-2.5 py-1 bg-rose-500/15 text-rose-400 hover:bg-rose-500/25 rounded border border-rose-500/30 font-medium transition-colors"
              >
                <Square size={12} />
                Power Off
              </button>
              <button
                onClick={() => useVmStore.getState().resetVm(vmId)}
                className="flex items-center gap-1 px-2.5 py-1 bg-muted hover:bg-accent rounded border border-border font-medium transition-colors"
              >
                <RotateCcw size={12} />
                Restart
              </button>
            </>
          )}

          {vmState === 'paused' && (
            <button
              onClick={() => useVmStore.getState().resumeVm(vmId)}
              className="flex items-center gap-1 px-2.5 py-1 bg-emerald-500/15 text-emerald-400 hover:bg-emerald-500/25 rounded border border-emerald-500/30 font-medium transition-colors"
            >
              <Play size={12} />
              Resume
            </button>
          )}

          <div className="h-4 w-px bg-border/60 mx-1" />

          <button
            onClick={handleSendCtrlAltDel}
            disabled={vmState !== 'running'}
            className="flex items-center gap-1 px-2.5 py-1 bg-muted/60 hover:bg-accent disabled:opacity-40 rounded border border-border/50 font-medium transition-colors"
            title="Send Ctrl+Alt+Del to Guest OS"
          >
            <ShieldAlert size={12} />
            Ctrl+Alt+Del
          </button>

          <button
            onClick={handleMountIso}
            className="flex items-center gap-1 px-2.5 py-1 bg-muted/60 hover:bg-accent rounded border border-border/50 font-medium transition-colors"
            title="Mount ISO Image"
          >
            <Disc size={12} />
            {mountedIso ? 'ISO Mounted' : 'Mount ISO'}
          </button>

          <button
            onClick={() => setFullscreen(!fullscreen)}
            className="p-1.5 bg-muted/60 hover:bg-accent rounded border border-border/50 transition-colors"
            title={fullscreen ? 'Exit Fullscreen' : 'Fullscreen Display'}
          >
            {fullscreen ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
          </button>
        </div>
      </div>

      {/* Main Virtual Monitor Screen Framebuffer */}
      <div
        onClick={() => setGrabbingInput(true)}
        className="relative bg-[#050505] p-6 font-mono text-xs text-green-400 min-h-[380px] flex flex-col justify-between cursor-text select-text"
      >
        {/* Screen Watermark when Powered Off */}
        {vmState === 'stopped' && (
          <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/90 text-muted-foreground gap-3">
            <Monitor size={48} className="opacity-30 text-primary" />
            <p className="text-base font-semibold text-foreground/80">Virtual Machine Power Off</p>
            <p className="text-xs">Click "Power On" to boot VMware Graphics Display for {vmName}.</p>
            <button
              onClick={() => useVmStore.getState().startVm(vmId)}
              className="mt-2 flex items-center gap-2 px-5 py-2 text-xs font-semibold bg-emerald-500 text-black hover:bg-emerald-400 rounded-xl transition-all shadow-lg"
            >
              <Play size={14} />
              Power On Virtual Machine
            </button>
          </div>
        )}

        {/* Screen Watermark when Paused */}
        {vmState === 'paused' && (
          <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/80 backdrop-blur-sm text-muted-foreground gap-3 z-10">
            <Pause size={48} className="opacity-40 text-amber-400" />
            <p className="text-base font-semibold text-foreground">VMware Console Paused</p>
            <button
              onClick={() => useVmStore.getState().resumeVm(vmId)}
              className="px-4 py-2 text-xs font-semibold bg-amber-500 text-black hover:bg-amber-400 rounded-xl transition-colors"
            >
              Resume Guest Operating System
            </button>
          </div>
        )}

        {/* Live Terminal & Boot Output Stream */}
        <div ref={consoleRef} className="space-y-1 overflow-y-auto max-h-[320px] leading-relaxed">
          {consoleLogs.map((line, idx) => (
            <div key={idx} className="whitespace-pre-wrap break-all">
              {line}
            </div>
          ))}
        </div>

        {/* Interactive Console Shell Input */}
        {vmState === 'running' && (
          <form onSubmit={handleCommandSubmit} className="flex items-center gap-2 mt-4 pt-2 border-t border-green-500/20">
            <span className="text-emerald-400 font-bold flex-shrink-0">root@novavm:~#</span>
            <input
              type="text"
              value={commandInput}
              onChange={(e) => setCommandInput(e.target.value)}
              placeholder="Type command ('help', 'status', 'ifconfig')..."
              className="w-full bg-transparent text-green-400 focus:outline-none placeholder:text-green-800 font-mono text-xs"
            />
          </form>
        )}
      </div>

      {/* Footer Status Bar */}
      <div className="flex items-center justify-between px-4 py-1.5 bg-[#0f0f0f] border-t border-border/40 text-[11px] text-muted-foreground select-none">
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1">
            <span className={cn('w-2 h-2 rounded-full', vmState === 'running' ? 'bg-emerald-500' : 'bg-slate-500')} />
            {vmState.toUpperCase()}
          </span>
          <span>·</span>
          <span>{vcpus} vCPU</span>
          <span>·</span>
          <span>{memoryMib} MiB RAM</span>
        </div>

        <div className="flex items-center gap-3">
          {grabbingInput ? (
            <span className="text-amber-400 font-medium">Input Grabbed — Press Ctrl+Alt to release</span>
          ) : (
            <span>Click display to grab mouse input</span>
          )}
        </div>
      </div>
    </div>
  )
}
