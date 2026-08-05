import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { useNavigate } from 'react-router-dom'
import { ChevronRight, ChevronLeft, Check, Server, Cpu, MemoryStick, HardDrive, Network, FolderOpen, Disc } from 'lucide-react'
import { open } from '@tauri-apps/plugin-dialog'

import { useVmStore } from '@/stores/vmStore'
import { networkApi, storageApi } from '@/lib/api'
import { cn } from '@/lib/utils'
import type { DiskBus, DiskMetadata, NicType, VirtualSwitch, VmConfig } from '@/types'
import { ErrorModal } from '@/components/ui/ErrorModal'

const STEPS = [
  { id: 'name', label: 'Name & OS Media', icon: <Server size={14} /> },
  { id: 'cpu', label: 'Processor', icon: <Cpu size={14} /> },
  { id: 'memory', label: 'RAM Memory', icon: <MemoryStick size={14} /> },
  { id: 'storage', label: 'Virtual Disk', icon: <HardDrive size={14} /> },
  { id: 'network', label: 'Network Adapter', icon: <Network size={14} /> },
  { id: 'review', label: 'Summary & Create', icon: <Check size={14} /> },
]

const defaultConfig: VmConfig = {
  name: '',
  description: null,
  cpu: { vcpus: 2, sockets: 1, cores_per_socket: 2, threads_per_core: 1, overcommit_ratio: 1.0 },
  memory: { size_mib: 4096, dynamic_min_mib: 1024, dynamic_max_mib: 8192, ballooning: true, huge_pages: false },
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
  const [errorModal, setErrorModal] = useState<{ title: string; err: unknown } | null>(null)

  // Extra wizard state
  const [isoPath, setIsoPath] = useState<string>('')
  const [guestOs, setGuestOs] = useState<'windows' | 'ubuntu' | 'debian' | 'macos' | 'other'>('ubuntu')
  const [diskOption, setDiskOption] = useState<'new' | 'existing' | 'none'>('new')
  const [newDiskSizeGib, setNewDiskSizeGib] = useState<number>(60)
  const [newDiskBus, setNewDiskBus] = useState<DiskBus>('virtio')
  const [existingDiskPath, setExistingDiskPath] = useState<string>('')
  const [selectedSwitch, setSelectedSwitch] = useState<string>('VMnet8 (NAT)')
  const [nicType, setNicType] = useState<NicType>('virtio')

  // Available options
  const [switches, setSwitches] = useState<VirtualSwitch[]>([])
  const [disks, setDisks] = useState<DiskMetadata[]>([])

  useEffect(() => {
    networkApi.listSwitches().then((s) => {
      setSwitches(s)
      if (s.length > 0) setSelectedSwitch(s[0].name)
    }).catch(() => {})
    storageApi.listDisks().then(setDisks).catch(() => {})
  }, [])

  const isLastStep = step === STEPS.length - 1

  const handleCreate = async () => {
    if (!config.name.trim()) {
      setErrorModal({ title: 'VM Name Required', err: new Error('Please enter a name for your virtual machine.') })
      return
    }

    setCreating(true)

    try {
      const finalDisks = [...config.disks]

      // 1. Handle Boot ISO Media
      if (isoPath) {
        finalDisks.push({
          image_path: isoPath,
          bus: 'ide',
          read_only: true,
          boot: true,
        })
      }

      // 2. Handle Storage Disk
      if (diskOption === 'new') {
        const diskMeta = await storageApi.createDisk({
          name: `${config.name}-hd0`,
          path: '',
          sizeGib: newDiskSizeGib,
          thin_provisioned: true,
        })
        finalDisks.push({
          image_path: diskMeta.path || `${config.name}-hd0.qcow2`,
          bus: newDiskBus,
          read_only: false,
          boot: !isoPath,
        })
      } else if (diskOption === 'existing' && existingDiskPath) {
        finalDisks.push({
          image_path: existingDiskPath,
          bus: newDiskBus,
          read_only: false,
          boot: !isoPath,
        })
      }

      // 3. Handle Network Adapter
      const finalNics = [
        {
          switch_name: selectedSwitch,
          nic_type: nicType,
          mac_address: null,
        },
      ]

      const finalConfig: VmConfig = {
        ...config,
        disks: finalDisks,
        nics: finalNics,
        tags: [guestOs],
      }

      const id = await createVm(finalConfig)
      navigate(`/vms/${id}`)
    } catch (e) {
      setErrorModal({ title: 'VM Creation Failed', err: e })
    } finally {
      setCreating(false)
    }
  }

  return (
    <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} className="max-w-3xl mx-auto">
      <div className="mb-8">
        <h2 className="text-2xl font-bold tracking-tight">New Virtual Machine Wizard</h2>
        <p className="text-muted-foreground text-sm mt-0.5">
          VMware Workstation style step-by-step VM setup
        </p>
      </div>

      {/* Step Indicator */}
      <div className="flex items-center mb-8 overflow-x-auto pb-2">
        {STEPS.map((s, i) => (
          <div key={s.id} className="flex items-center flex-shrink-0">
            <button
              onClick={() => i < step && setStep(i)}
              className={cn(
                'flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs font-medium transition-colors',
                i === step
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : i < step
                    ? 'text-emerald-500 cursor-pointer hover:bg-muted'
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

      {/* Step Content */}
      <div className="rounded-2xl border border-border bg-card p-6 min-h-72 shadow-sm">
        <AnimatePresence mode="wait">
          <motion.div
            key={step}
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            transition={{ duration: 0.15 }}
          >
            {step === 0 && (
              <Step1NameOs
                config={config}
                guestOs={guestOs}
                isoPath={isoPath}
                onChangeConfig={setConfig}
                onChangeGuestOs={setGuestOs}
                onChangeIsoPath={setIsoPath}
              />
            )}
            {step === 1 && (
              <Step2Cpu config={config} onChange={setConfig} />
            )}
            {step === 2 && (
              <Step3Memory config={config} onChange={setConfig} />
            )}
            {step === 3 && (
              <Step4Storage
                diskOption={diskOption}
                newDiskSizeGib={newDiskSizeGib}
                newDiskBus={newDiskBus}
                existingDiskPath={existingDiskPath}
                availableDisks={disks}
                onChangeOption={setDiskOption}
                onChangeSize={setNewDiskSizeGib}
                onChangeBus={setNewDiskBus}
                onChangeExistingPath={setExistingDiskPath}
              />
            )}
            {step === 4 && (
              <Step5Network
                selectedSwitch={selectedSwitch}
                nicType={nicType}
                switches={switches}
                onChangeSwitch={setSelectedSwitch}
                onChangeNicType={setNicType}
              />
            )}
            {step === 5 && (
              <Step6Review
                config={config}
                guestOs={guestOs}
                isoPath={isoPath}
                diskOption={diskOption}
                newDiskSizeGib={newDiskSizeGib}
                newDiskBus={newDiskBus}
                existingDiskPath={existingDiskPath}
                selectedSwitch={selectedSwitch}
              />
            )}
          </motion.div>
        </AnimatePresence>
      </div>

      {/* Navigation Footer */}
      <div className="flex justify-between mt-6">
        <button
          onClick={() => (step === 0 ? navigate('/vms') : setStep(step - 1))}
          className="flex items-center gap-2 px-4 py-2 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors"
        >
          <ChevronLeft size={16} />
          {step === 0 ? 'Cancel' : 'Back'}
        </button>
        <button
          onClick={isLastStep ? handleCreate : () => setStep(step + 1)}
          disabled={creating || (step === 0 && !config.name.trim())}
          className={cn(
            'flex items-center gap-2 px-5 py-2 text-xs font-medium rounded-xl',
            'bg-primary text-primary-foreground',
            'hover:bg-primary/90 transition-colors shadow-sm',
            'disabled:opacity-50 disabled:cursor-not-allowed',
          )}
        >
          {creating ? 'Creating Virtual Machine...' : isLastStep ? 'Finish & Create VM' : 'Next Step'}
          {!isLastStep && <ChevronRight size={16} />}
        </button>
      </div>

      {/* Error Popup Modal */}
      <ErrorModal
        isOpen={Boolean(errorModal)}
        title={errorModal?.title}
        error={errorModal?.err as Error}
        onClose={() => setErrorModal(null)}
      />
    </motion.div>
  )
}

// ─── Step 1: Name, Guest OS & ISO Media ──────────────────────────────────────

function Step1NameOs({
  config,
  guestOs,
  isoPath,
  onChangeConfig,
  onChangeGuestOs,
  onChangeIsoPath,
}: {
  config: VmConfig
  guestOs: string
  isoPath: string
  onChangeConfig: (c: VmConfig) => void
  onChangeGuestOs: (os: 'windows' | 'ubuntu' | 'debian' | 'macos' | 'other') => void
  onChangeIsoPath: (p: string) => void
}) {
  const handleBrowseIso = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [{ name: 'ISO & Disk Images', extensions: ['iso', 'img', 'raw'] }],
      })
      if (selected && typeof selected === 'string') {
        onChangeIsoPath(selected)
      }
    } catch (e) {
      console.error(e)
    }
  }

  return (
    <div className="space-y-5">
      <h3 className="font-bold text-base text-foreground">Name & Operating System Media</h3>

      <div className="space-y-1">
        <label className="text-xs font-medium">Virtual Machine Name *</label>
        <input
          type="text"
          required
          placeholder="e.g. Ubuntu-24.04-DevServer"
          value={config.name}
          onChange={(e) => onChangeConfig({ ...config, name: e.target.value })}
          className="w-full px-3 py-2 text-sm rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
          autoFocus
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-medium">Guest Operating System Type</label>
        <div className="grid grid-cols-4 gap-3">
          {[
            { id: 'ubuntu', label: 'Ubuntu / Debian' },
            { id: 'windows', label: 'Windows 11 / 10' },
            { id: 'macos', label: 'macOS Server' },
            { id: 'other', label: 'Generic Linux' },
          ].map((os) => (
            <button
              key={os.id}
              type="button"
              onClick={() => onChangeGuestOs(os.id as 'windows' | 'ubuntu' | 'debian' | 'macos' | 'other')}
              className={cn(
                'p-3 rounded-xl border text-xs font-medium text-center transition-all',
                guestOs === os.id
                  ? 'bg-primary/10 border-primary text-primary font-semibold'
                  : 'bg-muted border-border hover:bg-accent',
              )}
            >
              {os.label}
            </button>
          ))}
        </div>
      </div>

      {/* Boot ISO Media File Picker */}
      <div className="space-y-1.5 p-4 rounded-2xl bg-muted/40 border border-border">
        <label className="text-xs font-semibold flex items-center gap-1.5 text-foreground">
          <Disc size={16} className="text-primary" />
          <span>Installer ISO Media Image (Optional)</span>
        </label>
        <div className="flex gap-2">
          <input
            type="text"
            placeholder="Select operating system installation ISO file..."
            value={isoPath}
            onChange={(e) => onChangeIsoPath(e.target.value)}
            className="flex-1 px-3 py-2 text-xs font-mono rounded-xl bg-card border border-border focus:outline-none"
          />
          <button
            type="button"
            onClick={handleBrowseIso}
            className="px-3.5 py-2 text-xs font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors flex items-center gap-1.5 shadow-sm"
          >
            <FolderOpen size={14} />
            Browse ISO
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label className="text-xs font-medium">Firmware Type</label>
          <select
            value={config.firmware}
            onChange={(e) => onChangeConfig({ ...config, firmware: e.target.value as 'bios' | 'uefi' })}
            className="w-full px-3 py-2 text-xs rounded-xl bg-muted border border-border focus:outline-none"
          >
            <option value="uefi">UEFI Firmware (Modern)</option>
            <option value="bios">BIOS (Legacy System)</option>
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium">Folder / Group</label>
          <input
            type="text"
            placeholder="e.g. Production"
            value={config.group ?? ''}
            onChange={(e) => onChangeConfig({ ...config, group: e.target.value || null })}
            className="w-full px-3 py-2 text-xs rounded-xl bg-muted border border-border focus:outline-none"
          />
        </div>
      </div>
    </div>
  )
}

// ─── Step 2: CPU Cores ────────────────────────────────────────────────────────

function Step2Cpu({ config, onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  return (
    <div className="space-y-6">
      <h3 className="font-bold text-base text-foreground">Processor Allocation</h3>
      <div className="space-y-2">
        <div className="flex justify-between text-xs font-semibold">
          <span>Number of vCPUs</span>
          <span className="text-primary font-mono text-sm">{config.cpu.vcpus} Cores</span>
        </div>
        <input
          type="range"
          min={1}
          max={32}
          value={config.cpu.vcpus}
          onChange={(e) => onChange({ ...config, cpu: { ...config.cpu, vcpus: Number(e.target.value) } })}
          className="w-full accent-primary"
        />
        <div className="flex justify-between text-[10px] text-muted-foreground font-mono">
          <span>1 Core</span><span>4 Cores</span><span>16 Cores</span><span>32 Cores</span>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label className="text-xs font-medium">CPU Sockets</label>
          <input
            type="number"
            min={1}
            max={4}
            value={config.cpu.sockets}
            onChange={(e) => onChange({ ...config, cpu: { ...config.cpu, sockets: Number(e.target.value) } })}
            className="w-full px-3 py-2 text-xs rounded-xl bg-muted border border-border"
          />
        </div>
        <div className="space-y-1">
          <label className="text-xs font-medium">Overcommit Ratio</label>
          <input
            type="number"
            min={1}
            max={4}
            step={0.5}
            value={config.cpu.overcommit_ratio}
            onChange={(e) => onChange({ ...config, cpu: { ...config.cpu, overcommit_ratio: Number(e.target.value) } })}
            className="w-full px-3 py-2 text-xs rounded-xl bg-muted border border-border"
          />
        </div>
      </div>
    </div>
  )
}

// ─── Step 3: RAM Memory ──────────────────────────────────────────────────────

function Step3Memory({ config, onChange }: { config: VmConfig; onChange: (c: VmConfig) => void }) {
  const ramPresets = [2048, 4096, 8192, 16384, 32768]
  return (
    <div className="space-y-6">
      <h3 className="font-bold text-base text-foreground">RAM Memory Allocation</h3>
      <div className="space-y-3">
        <div className="flex justify-between text-xs font-semibold">
          <span>Allocated Memory</span>
          <span className="text-primary font-mono text-sm">
            {config.memory.size_mib >= 1024 ? `${config.memory.size_mib / 1024} GB` : `${config.memory.size_mib} MB`}
          </span>
        </div>
        <div className="flex gap-2 flex-wrap">
          {ramPresets.map((v) => (
            <button
              key={v}
              type="button"
              onClick={() => onChange({ ...config, memory: { ...config.memory, size_mib: v } })}
              className={cn(
                'px-4 py-2 text-xs font-medium rounded-xl border transition-colors',
                config.memory.size_mib === v
                  ? 'bg-primary text-primary-foreground border-primary font-semibold shadow-sm'
                  : 'bg-muted border-border hover:bg-accent',
              )}
            >
              {v >= 1024 ? `${v / 1024} GB` : `${v} MB`}
            </button>
          ))}
        </div>
      </div>
      <label className="flex items-center gap-2.5 text-xs font-medium cursor-pointer">
        <input
          type="checkbox"
          checked={config.memory.ballooning}
          onChange={(e) => onChange({ ...config, memory: { ...config.memory, ballooning: e.target.checked } })}
          className="rounded accent-primary"
        />
        <span>Enable Dynamic Memory Ballooning</span>
      </label>
    </div>
  )
}

// ─── Step 4: Storage Virtual Hard Disk ───────────────────────────────────────

function Step4Storage({
  diskOption,
  newDiskSizeGib,
  newDiskBus,
  existingDiskPath,
  availableDisks,
  onChangeOption,
  onChangeSize,
  onChangeBus,
  onChangeExistingPath,
}: {
  diskOption: 'new' | 'existing' | 'none'
  newDiskSizeGib: number
  newDiskBus: DiskBus
  existingDiskPath: string
  availableDisks: DiskMetadata[]
  onChangeOption: (opt: 'new' | 'existing' | 'none') => void
  onChangeSize: (sz: number) => void
  onChangeBus: (bus: DiskBus) => void
  onChangeExistingPath: (path: string) => void
}) {
  const handleBrowseExistingDisk = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [{ name: 'Virtual Disk Files', extensions: ['qcow2', 'vmdk', 'vhd', 'raw'] }],
      })
      if (selected && typeof selected === 'string') {
        onChangeExistingPath(selected)
      }
    } catch (e) {
      console.error(e)
    }
  }

  return (
    <div className="space-y-5">
      <h3 className="font-bold text-base text-foreground">Virtual Hard Disk Storage</h3>

      <div className="space-y-3">
        {/* Option 1: Create New Disk */}
        <label className={cn(
          'flex items-start gap-3 p-4 rounded-2xl border cursor-pointer transition-all',
          diskOption === 'new' ? 'bg-primary/10 border-primary' : 'bg-muted/40 border-border hover:bg-accent/40',
        )}>
          <input
            type="radio"
            name="diskOption"
            checked={diskOption === 'new'}
            onChange={() => onChangeOption('new')}
            className="mt-1 accent-primary"
          />
          <div className="flex-1 min-w-0">
            <span className="text-xs font-bold text-foreground block">Create a New Virtual Hard Disk Now</span>
            <p className="text-[11px] text-muted-foreground mt-0.5">
              Create a fresh QCOW2 dynamic disk image allocated for guest OS installation.
            </p>
          </div>
        </label>

        {diskOption === 'new' && (
          <div className="pl-7 space-y-4 pt-1">
            <div className="space-y-1.5">
              <div className="flex justify-between text-xs font-semibold">
                <span>Disk Size</span>
                <span className="text-primary font-mono">{newDiskSizeGib} GB</span>
              </div>
              <input
                type="range"
                min={10}
                max={500}
                step={10}
                value={newDiskSizeGib}
                onChange={(e) => onChangeSize(Number(e.target.value))}
                className="w-full accent-primary"
              />
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium">Virtual Controller Bus</label>
              <select
                value={newDiskBus}
                onChange={(e) => onChangeBus(e.target.value as DiskBus)}
                className="w-full px-3 py-2 text-xs rounded-xl bg-muted border border-border"
              >
                <option value="virtio">VirtIO High-Performance Disk (Recommended)</option>
                <option value="nvme">NVMe Controller</option>
                <option value="sata">SATA Controller</option>
                <option value="ide">IDE Legacy Controller</option>
              </select>
            </div>
          </div>
        )}

        {/* Option 2: Existing Disk */}
        <label className={cn(
          'flex items-start gap-3 p-4 rounded-2xl border cursor-pointer transition-all',
          diskOption === 'existing' ? 'bg-primary/10 border-primary' : 'bg-muted/40 border-border hover:bg-accent/40',
        )}>
          <input
            type="radio"
            name="diskOption"
            checked={diskOption === 'existing'}
            onChange={() => onChangeOption('existing')}
            className="mt-1 accent-primary"
          />
          <div className="flex-1 min-w-0">
            <span className="text-xs font-bold text-foreground block">Use an Existing Disk Image</span>
            <p className="text-[11px] text-muted-foreground mt-0.5">
              Attach an existing QCOW2, VMDK, or VHD disk file from your library or local drive.
            </p>
          </div>
        </label>

        {diskOption === 'existing' && (
          <div className="pl-7 space-y-3 pt-1">
            {availableDisks.length > 0 && (
              <div className="space-y-1">
                <label className="text-xs font-medium">Select from Storage Library</label>
                <select
                  value={existingDiskPath}
                  onChange={(e) => onChangeExistingPath(e.target.value)}
                  className="w-full px-3 py-2 text-xs rounded-xl bg-muted border border-border"
                >
                  <option value="">-- Choose from Storage Library --</option>
                  {availableDisks.map((d) => (
                    <option key={d.id} value={d.path || d.name}>{d.name} ({d.format.toUpperCase()})</option>
                  ))}
                </select>
              </div>
            )}
            <div className="space-y-1">
              <label className="text-xs font-medium">Or Browse Disk Image File</label>
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder="Select disk image path..."
                  value={existingDiskPath}
                  onChange={(e) => onChangeExistingPath(e.target.value)}
                  className="flex-1 px-3 py-2 text-xs font-mono rounded-xl bg-muted border border-border"
                />
                <button
                  type="button"
                  onClick={handleBrowseExistingDisk}
                  className="px-3 py-2 text-xs font-medium bg-muted hover:bg-accent rounded-xl border border-border"
                >
                  Browse
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

// ─── Step 5: Network Adapter Assignment ──────────────────────────────────────

function Step5Network({
  selectedSwitch,
  nicType,
  switches,
  onChangeSwitch,
  onChangeNicType,
}: {
  selectedSwitch: string
  nicType: NicType
  switches: VirtualSwitch[]
  onChangeSwitch: (sw: string) => void
  onChangeNicType: (nic: NicType) => void
}) {
  return (
    <div className="space-y-5">
      <h3 className="font-bold text-base text-foreground">Network Adapter Assignment</h3>

      <div className="space-y-4">
        <div className="space-y-1.5">
          <label className="text-xs font-semibold">Assign Virtual Network Switch</label>
          <select
            value={selectedSwitch}
            onChange={(e) => onChangeSwitch(e.target.value)}
            className="w-full px-3 py-2.5 text-sm rounded-xl bg-muted border border-border font-medium focus:outline-none"
          >
            {switches.map((s) => (
              <option key={s.id} value={s.name}>
                {s.name} ({s.mode.toUpperCase().replace('_', '-')}) — Subnet: {s.subnet}
              </option>
            ))}
          </select>
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold">Network Interface Card (NIC) Driver</label>
          <select
            value={nicType}
            onChange={(e) => onChangeNicType(e.target.value as NicType)}
            className="w-full px-3 py-2.5 text-sm rounded-xl bg-muted border border-border font-medium focus:outline-none"
          >
            <option value="virtio">VirtIO Paravirtualized NIC (Highest Performance)</option>
            <option value="e1000">Intel e1000 (Universal Compatibility)</option>
            <option value="rtl8139">Realtek RTL8139 (Legacy Guest Support)</option>
          </select>
        </div>
      </div>
    </div>
  )
}

// ─── Step 6: Review & Finalize ────────────────────────────────────────────────

function Step6Review({
  config,
  guestOs,
  isoPath,
  diskOption,
  newDiskSizeGib,
  newDiskBus,
  existingDiskPath,
  selectedSwitch,
}: {
  config: VmConfig
  guestOs: string
  isoPath: string
  diskOption: string
  newDiskSizeGib: number
  newDiskBus: string
  existingDiskPath: string
  selectedSwitch: string
}) {
  const items = [
    { label: 'VM Name', value: config.name || '(Unnamed)' },
    { label: 'Guest OS Type', value: guestOs.toUpperCase() },
    { label: 'Boot Media ISO', value: isoPath ? isoPath : 'None (Hard Disk Boot)' },
    { label: 'Processor Cores', value: `${config.cpu.vcpus} vCPUs (${config.cpu.sockets} Socket)` },
    { label: 'RAM Memory', value: `${config.memory.size_mib / 1024} GB` },
    {
      label: 'Virtual Disk',
      value:
        diskOption === 'new'
          ? `New ${newDiskSizeGib} GB QCOW2 (${newDiskBus.toUpperCase()})`
          : diskOption === 'existing'
            ? `Attached ${existingDiskPath}`
            : 'No Hard Disk',
    },
    { label: 'Network Adapter', value: `Connected to ${selectedSwitch}` },
  ]

  return (
    <div className="space-y-4">
      <h3 className="font-bold text-base text-foreground">Review Virtual Machine Configuration</h3>
      <div className="divide-y divide-border rounded-2xl bg-muted/40 border border-border p-4">
        {items.map((item) => (
          <div key={item.label} className="flex justify-between py-2 text-xs">
            <span className="text-muted-foreground font-medium">{item.label}</span>
            <span className="font-semibold text-foreground truncate max-w-xs">{item.value}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
