import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { HardDrive, Plus, Lock, Zap, FolderOpen, Trash2 } from 'lucide-react'
import { open } from '@tauri-apps/plugin-dialog'

import { storageApi } from '@/lib/api'
import type { DiskMetadata } from '@/types'
import { formatBytes, formatDateTime } from '@/lib/utils'
import { ErrorModal } from '@/components/ui/ErrorModal'

export function StoragePage() {
  const [disks, setDisks] = useState<DiskMetadata[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [errorModal, setErrorModal] = useState<{ title: string; err: unknown } | null>(null)

  // Form State
  const [name, setName] = useState('')
  const [path, setPath] = useState('')
  const [sizeGib, setSizeGib] = useState(50)
  const [thin, setThin] = useState(true)
  const [encrypted, setEncrypted] = useState(false)
  const [compressed, setCompressed] = useState(false)
  const [submitting, setSubmitting] = useState(false)

  const load = async () => {
    setLoading(true)
    try {
      setDisks(await storageApi.listDisks())
    } catch (e) {
      setErrorModal({ title: 'Failed to Load Storage Library', err: e })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const handleBrowseFolder = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [{ name: 'Disk Image Target', extensions: ['qcow2', 'vmdk', 'vhd', 'raw'] }],
      })
      if (selected && typeof selected === 'string') {
        setPath(selected)
      }
    } catch (e) {
      console.error(e)
    }
  }

  const handleImportIsoDisk = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [{ name: 'Disk & ISO Images', extensions: ['iso', 'img', 'vmdk', 'qcow2', 'vhd', 'raw'] }],
      })
      if (selected && typeof selected === 'string') {
        await storageApi.importDisk(selected)
        await load()
      }
    } catch (e) {
      setErrorModal({ title: 'Import Storage Image Failed', err: e })
    }
  }

  const handleCreateDisk = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return
    setSubmitting(true)
    try {
      await storageApi.createDisk({
        name,
        path,
        sizeGib,
        thin_provisioned: thin,
        encrypted,
        compressed,
      })
      setShowCreateModal(false)
      setName('')
      setPath('')
      await load()
    } catch (err) {
      setErrorModal({ title: 'Create Storage Image Failed', err })
    } finally {
      setSubmitting(false)
    }
  }

  const handleDeleteDisk = async (id: string) => {
    try {
      await storageApi.deleteDisk(id, true)
      await load()
    } catch (e) {
      setErrorModal({ title: 'Delete Disk Image Failed', err: e })
    }
  }

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Storage Library</h2>
          <p className="text-muted-foreground text-sm mt-0.5">
            {disks.length} virtual disk image{disks.length !== 1 ? 's' : ''} & ISO media files
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleImportIsoDisk}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-muted hover:bg-accent text-foreground rounded-xl transition-colors border border-border"
          >
            <FolderOpen size={16} />
            Import Image / ISO
          </button>
          <button
            onClick={() => setShowCreateModal(true)}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors shadow-sm"
          >
            <Plus size={16} />
            Create Virtual Disk
          </button>
        </div>
      </div>

      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => <div key={i} className="skeleton h-20 rounded-xl" />)}
        </div>
      ) : disks.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-border p-12 text-center bg-card/40">
          <HardDrive size={40} className="mx-auto mb-3 text-muted-foreground/40" />
          <h3 className="text-base font-semibold text-foreground">No Disk Images Configured</h3>
          <p className="text-xs text-muted-foreground mt-1 max-w-md mx-auto">
            Create a new virtual disk image (QCOW2, VMDK, VHD, RAW) or import an existing ISO installer media file to attach to VMs.
          </p>
          <div className="flex justify-center gap-3 mt-6">
            <button
              onClick={handleImportIsoDisk}
              className="px-4 py-2 text-xs font-medium bg-muted hover:bg-accent rounded-xl border border-border transition-colors"
            >
              Browse ISO / Disk File
            </button>
            <button
              onClick={() => setShowCreateModal(true)}
              className="px-4 py-2 text-xs font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors"
            >
              Create New Disk
            </button>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {disks.map((disk) => (
            <motion.div key={disk.id} layout className="flex items-center gap-4 p-4 rounded-2xl border border-border bg-card hover:bg-accent/10 transition-colors">
              <div className="flex-shrink-0 w-11 h-11 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
                <HardDrive size={20} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-sm text-foreground">{disk.name}</span>
                  <span className="text-xs font-mono px-2 py-0.5 rounded-full bg-muted text-muted-foreground border border-border uppercase">
                    {disk.format}
                  </span>
                  {disk.thin_provisioned && (
                    <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-500 font-medium">
                      Thin Provisioned
                    </span>
                  )}
                  {disk.encrypted && <Lock size={12} className="text-amber-500" />}
                  {disk.compressed && <Zap size={12} className="text-blue-500" />}
                </div>
                <p className="text-xs text-muted-foreground mt-1 truncate">
                  {disk.path ? disk.path : 'Default Managed Storage'} · Virtual Size: {formatBytes(disk.virtual_size_bytes)} · Created {formatDateTime(disk.created_at)}
                </p>
              </div>
              <button
                onClick={() => handleDeleteDisk(disk.id)}
                title="Delete Disk Image"
                className="p-2.5 rounded-xl text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
              >
                <Trash2 size={16} />
              </button>
            </motion.div>
          ))}
        </div>
      )}

      {/* Create Disk Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
          <div className="relative w-full max-w-lg rounded-2xl bg-card border border-border shadow-2xl p-6">
            <h3 className="text-lg font-bold tracking-tight mb-1">Create Virtual Disk Image</h3>
            <p className="text-xs text-muted-foreground mb-5">
              Allocate a new virtual hard disk image for guest OS installation.
            </p>

            <form onSubmit={handleCreateDisk} className="space-y-4">
              <div className="space-y-1">
                <label className="text-xs font-medium">Disk Image Name *</label>
                <input
                  type="text"
                  required
                  placeholder="e.g. ubuntu-root-disk"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full px-3 py-2 text-sm rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
                />
              </div>

              <div className="space-y-1">
                <label className="text-xs font-medium">File Path (Optional)</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="Leave empty for default VM storage folder"
                    value={path}
                    onChange={(e) => setPath(e.target.value)}
                    className="flex-1 px-3 py-2 text-sm rounded-xl bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                  <button
                    type="button"
                    onClick={handleBrowseFolder}
                    className="px-3 py-2 text-xs font-medium bg-muted hover:bg-accent rounded-xl border border-border transition-colors flex items-center gap-1.5"
                  >
                    <FolderOpen size={14} />
                    Browse
                  </button>
                </div>
              </div>

              <div className="space-y-1.5">
                <div className="flex justify-between text-xs font-medium">
                  <span>Disk Capacity</span>
                  <span className="font-mono text-primary">{sizeGib} GB</span>
                </div>
                <input
                  type="range"
                  min={5}
                  max={1000}
                  step={5}
                  value={sizeGib}
                  onChange={(e) => setSizeGib(Number(e.target.value))}
                  className="w-full accent-primary"
                />
                <div className="flex justify-between text-[10px] text-muted-foreground font-mono">
                  <span>5 GB</span><span>250 GB</span><span>500 GB</span><span>1000 GB</span>
                </div>
              </div>

              <div className="space-y-2 pt-2 border-t border-border">
                <label className="flex items-center gap-2.5 text-xs cursor-pointer">
                  <input
                    type="checkbox"
                    checked={thin}
                    onChange={(e) => setThin(e.target.checked)}
                    className="rounded accent-primary"
                  />
                  <span>Thin Provisioning (Allocate disk space dynamically as needed)</span>
                </label>
                <label className="flex items-center gap-2.5 text-xs cursor-pointer">
                  <input
                    type="checkbox"
                    checked={encrypted}
                    onChange={(e) => setEncrypted(e.target.checked)}
                    className="rounded accent-primary"
                  />
                  <span>Encrypt Disk Image (AES-GCM-256)</span>
                </label>
                <label className="flex items-center gap-2.5 text-xs cursor-pointer">
                  <input
                    type="checkbox"
                    checked={compressed}
                    onChange={(e) => setCompressed(e.target.checked)}
                    className="rounded accent-primary"
                  />
                  <span>Compress Image Clusters</span>
                </label>
              </div>

              <div className="flex justify-end gap-3 pt-4 border-t border-border">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="px-4 py-2 text-xs font-medium text-muted-foreground hover:text-foreground rounded-xl transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={submitting || !name.trim()}
                  className="px-5 py-2 text-xs font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors disabled:opacity-50"
                >
                  {submitting ? 'Creating...' : 'Create Disk Image'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

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
