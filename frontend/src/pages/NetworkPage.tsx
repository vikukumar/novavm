import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { Plus, Network as NetworkIcon, Trash2, Edit3, Shield, Globe, Cpu, Lock } from 'lucide-react'

import { networkApi } from '@/lib/api'
import type { VirtualSwitch, VirtualSwitchMode } from '@/types'
import { cn } from '@/lib/utils'
import { ErrorModal } from '@/components/ui/ErrorModal'
import { ConfirmModal } from '@/components/ui/ConfirmModal'

const MODE_CARDS: { mode: VirtualSwitchMode; vmnet: string; label: string; desc: string; icon: JSX.Element; color: string }[] = [
  {
    mode: 'nat',
    vmnet: 'VMnet8',
    label: 'NAT (Network Address Translation)',
    desc: 'Share host IP address. Guests get automatic internet access and local subnet routing via host NAT.',
    icon: <Globe size={18} />,
    color: 'text-emerald-500 bg-emerald-500/10 border-emerald-500/30',
  },
  {
    mode: 'bridged',
    vmnet: 'VMnet0',
    label: 'Bridged (Direct Network Access)',
    desc: 'Connect directly to physical network interface. Guests appear as separate physical PCs on local LAN.',
    icon: <Cpu size={18} />,
    color: 'text-blue-500 bg-blue-500/10 border-blue-500/30',
  },
  {
    mode: 'host_only',
    vmnet: 'VMnet1',
    label: 'Host-Only (Private Host Connection)',
    desc: 'Private network shared between host and guest VMs. Isolated from external internet.',
    icon: <Lock size={18} />,
    color: 'text-amber-500 bg-amber-500/10 border-amber-500/30',
  },
  {
    mode: 'internal',
    vmnet: 'VMnet2-7',
    label: 'Custom / Internal Private Switch',
    desc: 'Completely isolated internal virtual switch. Interconnects guest VMs only.',
    icon: <Shield size={18} />,
    color: 'text-violet-500 bg-violet-500/10 border-violet-500/30',
  },
]

export function NetworkPage() {
  const [switches, setSwitches] = useState<VirtualSwitch[]>([])
  const [physicalAdapters, setPhysicalAdapters] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [modalMode, setModalMode] = useState<'create' | 'edit' | null>(null)
  const [errorModal, setErrorModal] = useState<{ title: string; err: unknown } | null>(null)
  const [deleteConfirmSwitch, setDeleteConfirmSwitch] = useState<string | null>(null)

  // Form State
  const [selectedSwitchName, setSelectedSwitchName] = useState('')
  const [mode, setMode] = useState<VirtualSwitchMode>('nat')
  const [subnet, setSubnet] = useState('192.168.128.0/24')
  const [gateway, setGateway] = useState('192.168.128.1')
  const [dhcpEnabled, setDhcpEnabled] = useState(true)
  const [dhcpStart, setDhcpStart] = useState('192.168.128.128')
  const [dhcpEnd, setDhcpEnd] = useState('192.168.128.254')
  const [adapterName, setAdapterName] = useState<string>('')
  const [submitting, setSubmitting] = useState(false)

  const load = async () => {
    setLoading(true)
    try {
      const [swList, adapters] = await Promise.all([
        networkApi.listSwitches(),
        networkApi.listPhysicalAdapters().catch(() => []),
      ])
      setSwitches(swList)
      setPhysicalAdapters(adapters)
    } catch (e) {
      setErrorModal({ title: 'Failed to Load Virtual Networks', err: e })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const openCreateModal = (presetMode: VirtualSwitchMode = 'nat') => {
    setMode(presetMode)
    const card = MODE_CARDS.find((c) => c.mode === presetMode)
    const count = switches.filter((s) => s.mode === presetMode).length + 1
    setSelectedSwitchName(`${card?.vmnet ?? 'VMnet'}_${count}`)
    setSubnet(presetMode === 'nat' ? '192.168.128.0/24' : presetMode === 'host_only' ? '192.168.192.0/24' : '192.168.1.0/24')
    setGateway(presetMode === 'nat' ? '192.168.128.1' : presetMode === 'host_only' ? '192.168.192.1' : '192.168.1.1')
    setDhcpEnabled(presetMode === 'nat' || presetMode === 'host_only')
    setDhcpStart(presetMode === 'nat' ? '192.168.128.128' : '192.168.192.128')
    setDhcpEnd(presetMode === 'nat' ? '192.168.128.254' : '192.168.192.254')
    setAdapterName(physicalAdapters[0] ?? '')
    setModalMode('create')
  }

  const openEditModal = (sw: VirtualSwitch) => {
    setSelectedSwitchName(sw.name)
    setMode(sw.mode)
    setSubnet(sw.subnet)
    setGateway(sw.gateway)
    setDhcpEnabled(sw.dhcp_enabled)
    setDhcpStart(sw.dhcp_range_start)
    setDhcpEnd(sw.dhcp_range_end)
    setAdapterName(sw.adapter_name ?? physicalAdapters[0] ?? '')
    setModalMode('edit')
  }

  const handleSaveSwitch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedSwitchName.trim()) return
    setSubmitting(true)
    try {
      const params = {
        name: selectedSwitchName,
        mode,
        subnet,
        gateway,
        dhcp_enabled: dhcpEnabled,
        dhcp_range_start: dhcpStart,
        dhcp_range_end: dhcpEnd,
        adapter_name: mode === 'bridged' ? adapterName : null,
      }
      if (modalMode === 'create') {
        await networkApi.createSwitch(params)
      } else {
        await networkApi.updateSwitch(params)
      }
      setModalMode(null)
      await load()
    } catch (err) {
      setErrorModal({ title: modalMode === 'create' ? 'Create Virtual Network Failed' : 'Update Virtual Network Failed', err })
    } finally {
      setSubmitting(false)
    }
  }

  const handleDelete = async (name: string) => {
    try {
      await networkApi.deleteSwitch(name)
      await load()
    } catch (e) {
      setErrorModal({ title: `Failed to Delete Switch '${name}'`, err: e })
    }
  }

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Virtual Network Editor</h2>
          <p className="text-muted-foreground text-sm mt-0.5">
            VMware Workstation style virtual networks ({switches.length} active switch{switches.length !== 1 ? 'es' : ''})
          </p>
        </div>
        <button
          onClick={() => openCreateModal('nat')}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors shadow-sm"
        >
          <Plus size={16} />
          Add Virtual Network
        </button>
      </div>

      {/* Network Mode Presets */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {MODE_CARDS.map((card) => (
          <div
            key={card.mode}
            className={cn('p-4 rounded-2xl border bg-card transition-all flex flex-col justify-between', card.color)}
          >
            <div>
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2 font-semibold text-sm text-foreground">
                  {card.icon}
                  <span>{card.label}</span>
                </div>
                <span className="text-[11px] font-mono px-2 py-0.5 rounded-md bg-muted text-muted-foreground border border-border font-medium">
                  {card.vmnet}
                </span>
              </div>
              <p className="text-xs text-muted-foreground leading-relaxed">{card.desc}</p>
            </div>
            <div className="mt-4 pt-3 border-t border-border/40 flex justify-end">
              <button
                onClick={() => openCreateModal(card.mode)}
                className="text-xs font-medium text-primary hover:underline flex items-center gap-1"
              >
                + Add {card.vmnet} Network
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* Active Virtual Switches */}
      <div className="space-y-3">
        <h3 className="text-base font-semibold text-foreground">Active Virtual Switches</h3>

        {loading ? (
          <div className="space-y-3">
            {Array.from({ length: 3 }).map((_, i) => <div key={i} className="skeleton h-20 rounded-2xl" />)}
          </div>
        ) : switches.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-border p-12 text-center bg-card/40">
            <NetworkIcon size={40} className="mx-auto mb-3 text-muted-foreground/40" />
            <h4 className="text-base font-semibold text-foreground">No Virtual Networks Configured</h4>
            <p className="text-xs text-muted-foreground mt-1">
              Create a VMware NAT (VMnet8), Bridged (VMnet0), or Host-Only (VMnet1) switch to connect VMs.
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {switches.map((sw) => {
              const card = MODE_CARDS.find((c) => c.mode === sw.mode)
              return (
                <motion.div
                  key={sw.id}
                  layout
                  className="flex items-center gap-4 p-4 rounded-2xl border border-border bg-card hover:bg-accent/10 transition-colors"
                >
                  <div className="flex-shrink-0 w-11 h-11 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
                    <NetworkIcon size={20} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-bold text-sm text-foreground">{sw.name}</span>
                      <span className={cn('text-xs px-2.5 py-0.5 rounded-full font-medium border', card?.color)}>
                        {sw.mode.toUpperCase().replace('_', '-')}
                      </span>
                      {sw.dhcp_enabled && (
                        <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-500 font-medium">
                          DHCP Server Enabled
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                      Subnet: <span className="font-mono text-foreground">{sw.subnet}</span> · Gateway: <span className="font-mono text-foreground">{sw.gateway}</span>
                      {sw.dhcp_enabled && ` · DHCP Pool: ${sw.dhcp_range_start} - ${sw.dhcp_range_end}`}
                      {sw.mode === 'bridged' && sw.adapter_name && ` · Adapter: ${sw.adapter_name}`}
                    </p>
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => openEditModal(sw)}
                      className="p-2.5 rounded-xl text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                      title="Edit Network Settings"
                    >
                      <Edit3 size={16} />
                    </button>
                    <button
                      onClick={() => setDeleteConfirmSwitch(sw.name)}
                      className="p-2.5 rounded-xl text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
                      title="Delete Virtual Switch"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </motion.div>
              )
            })}
          </div>
        )}
      </div>

      {/* Create / Edit Switch Modal */}
      {modalMode && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
          <div className="relative w-full max-w-xl rounded-2xl bg-card border border-border shadow-2xl p-6">
            <h3 className="text-lg font-bold tracking-tight mb-1">
              {modalMode === 'create' ? 'Add Virtual Network Switch' : `Edit Virtual Switch '${selectedSwitchName}'`}
            </h3>
            <p className="text-xs text-muted-foreground mb-5">
              Configure VMware Workstation style virtual networking modes, IPv4 subnets, and DHCP options.
            </p>

            <form onSubmit={handleSaveSwitch} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1">
                  <label className="text-xs font-medium">Network Switch Name *</label>
                  <input
                    type="text"
                    required
                    disabled={modalMode === 'edit'}
                    placeholder="e.g. VMnet8"
                    value={selectedSwitchName}
                    onChange={(e) => setSelectedSwitchName(e.target.value)}
                    className="w-full px-3 py-2 text-sm rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-60"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-xs font-medium">Network Mode</label>
                  <select
                    value={mode}
                    onChange={(e) => setMode(e.target.value as VirtualSwitchMode)}
                    className="w-full px-3 py-2 text-sm rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
                  >
                    <option value="nat">NAT (Shared Host IP - VMnet8)</option>
                    <option value="bridged">Bridged (Physical Adapter - VMnet0)</option>
                    <option value="host_only">Host-Only (Private - VMnet1)</option>
                    <option value="internal">Custom / Internal Isolated</option>
                  </select>
                </div>
              </div>

              {/* Physical Adapter binding for Bridged mode */}
              {mode === 'bridged' && (
                <div className="space-y-1 p-3 rounded-xl bg-blue-500/10 border border-blue-500/20">
                  <label className="text-xs font-semibold text-blue-500">Physical Host Network Adapter (Bridging)</label>
                  <select
                    value={adapterName}
                    onChange={(e) => setAdapterName(e.target.value)}
                    className="w-full px-3 py-2 text-sm rounded-xl bg-card border border-border focus:outline-none focus:ring-2 focus:ring-ring"
                  >
                    {physicalAdapters.map((a) => (
                      <option key={a} value={a}>{a}</option>
                    ))}
                  </select>
                </div>
              )}

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1">
                  <label className="text-xs font-medium">Subnet CIDR</label>
                  <input
                    type="text"
                    required
                    placeholder="e.g. 192.168.128.0/24"
                    value={subnet}
                    onChange={(e) => setSubnet(e.target.value)}
                    className="w-full px-3 py-2 text-sm font-mono rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-xs font-medium">Gateway IP Address</label>
                  <input
                    type="text"
                    required
                    placeholder="e.g. 192.168.128.1"
                    value={gateway}
                    onChange={(e) => setGateway(e.target.value)}
                    className="w-full px-3 py-2 text-sm font-mono rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                </div>
              </div>

              <div className="space-y-3 pt-2 border-t border-border">
                <label className="flex items-center gap-2.5 text-xs font-semibold cursor-pointer">
                  <input
                    type="checkbox"
                    checked={dhcpEnabled}
                    onChange={(e) => setDhcpEnabled(e.target.checked)}
                    className="rounded accent-primary"
                  />
                  <span>Use Local DHCP Service to Distribute IP Addresses to VMs</span>
                </label>

                {dhcpEnabled && (
                  <div className="grid grid-cols-2 gap-4 pl-6">
                    <div className="space-y-1">
                      <label className="text-xs font-medium text-muted-foreground">DHCP Range Start</label>
                      <input
                        type="text"
                        value={dhcpStart}
                        onChange={(e) => setDhcpStart(e.target.value)}
                        className="w-full px-3 py-2 text-sm font-mono rounded-xl bg-muted border border-border focus:outline-none"
                      />
                    </div>
                    <div className="space-y-1">
                      <label className="text-xs font-medium text-muted-foreground">DHCP Range End</label>
                      <input
                        type="text"
                        value={dhcpEnd}
                        onChange={(e) => setDhcpEnd(e.target.value)}
                        className="w-full px-3 py-2 text-sm font-mono rounded-xl bg-muted border border-border focus:outline-none"
                      />
                    </div>
                  </div>
                )}
              </div>

              <div className="flex justify-end gap-3 pt-4 border-t border-border">
                <button
                  type="button"
                  onClick={() => setModalMode(null)}
                  className="px-4 py-2 text-xs font-medium text-muted-foreground hover:text-foreground rounded-xl transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={submitting}
                  className="px-5 py-2 text-xs font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors disabled:opacity-50"
                >
                  {submitting ? 'Saving...' : modalMode === 'create' ? 'Create Network' : 'Save Changes'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Confirmation Modal */}
      <ConfirmModal
        isOpen={Boolean(deleteConfirmSwitch)}
        title={`Delete Switch '${deleteConfirmSwitch}'?`}
        description="Are you sure you want to delete this virtual switch? Connected VMs will lose network connectivity on this switch."
        confirmText="Delete Switch"
        variant="danger"
        onConfirm={async () => {
          if (deleteConfirmSwitch) {
            const name = deleteConfirmSwitch
            setDeleteConfirmSwitch(null)
            await handleDelete(name)
          }
        }}
        onClose={() => setDeleteConfirmSwitch(null)}
      />

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
