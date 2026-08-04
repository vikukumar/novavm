import { useState, useMemo } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Plus, Search, Grid, List } from 'lucide-react'
import { useNavigate } from 'react-router-dom'

import { useVmStore } from '@/stores/vmStore'
import { VmCard } from '@/components/vm/VmCard'
import { cn } from '@/lib/utils'
import type { VmState } from '@/types'

const STATE_FILTERS: { label: string; value: VmState | 'all' }[] = [
  { label: 'All', value: 'all' },
  { label: 'Running', value: 'running' },
  { label: 'Stopped', value: 'stopped' },
  { label: 'Paused', value: 'paused' },
  { label: 'Crashed', value: 'crashed' },
]

type ViewMode = 'grid' | 'list'

export function VmListPage() {
  const navigate = useNavigate()
  const vms = useVmStore((s) => s.vms)
  const loading = useVmStore((s) => s.loading)

  const [search, setSearch] = useState('')
  const [stateFilter, setStateFilter] = useState<VmState | 'all'>('all')
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [tagFilter, setTagFilter] = useState('')

  const allTags = useMemo(
    () => [...new Set(vms.flatMap((v) => v.tags))].sort(),
    [vms],
  )

  const filtered = useMemo(
    () =>
      vms.filter((vm) => {
        if (stateFilter !== 'all' && vm.state !== stateFilter) return false
        if (search && !vm.name.toLowerCase().includes(search.toLowerCase())) return false
        if (tagFilter && !vm.tags.includes(tagFilter)) return false
        return true
      }),
    [vms, stateFilter, search, tagFilter],
  )

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="space-y-5 max-w-7xl mx-auto"
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Virtual Machines</h2>
          <p className="text-muted-foreground text-sm mt-0.5">
            {vms.length} total, {vms.filter((v) => v.state === 'running').length} running
          </p>
        </div>
        <button
          id="vm-list-create-btn"
          onClick={() => navigate('/vms/create')}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          <Plus size={16} />
          New VM
        </button>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Search */}
        <div className="relative flex-1 min-w-48">
          <Search
            size={14}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
          />
          <input
            id="vm-search"
            type="text"
            placeholder="Search VMs…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className={cn(
              'w-full pl-9 pr-4 py-2 text-sm rounded-lg',
              'bg-muted border border-border',
              'focus:outline-none focus:ring-2 focus:ring-ring',
              'placeholder:text-muted-foreground',
            )}
          />
        </div>

        {/* State filter pills */}
        <div className="flex items-center gap-1 bg-muted rounded-lg p-1">
          {STATE_FILTERS.map((f) => (
            <button
              key={f.value}
              onClick={() => setStateFilter(f.value)}
              className={cn(
                'px-3 py-1 text-xs font-medium rounded-md transition-colors',
                stateFilter === f.value
                  ? 'bg-card text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {f.label}
            </button>
          ))}
        </div>

        {/* Tag filter */}
        {allTags.length > 0 && (
          <select
            value={tagFilter}
            onChange={(e) => setTagFilter(e.target.value)}
            className="text-xs bg-muted border border-border rounded-lg px-2 py-2 focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option value="">All tags</option>
            {allTags.map((tag) => (
              <option key={tag} value={tag}>
                {tag}
              </option>
            ))}
          </select>
        )}

        {/* View mode */}
        <div className="flex items-center gap-1 bg-muted rounded-lg p-1 ml-auto">
          <button
            onClick={() => setViewMode('grid')}
            className={cn(
              'p-1.5 rounded-md transition-colors',
              viewMode === 'grid' ? 'bg-card shadow-sm' : 'text-muted-foreground',
            )}
            aria-label="Grid view"
          >
            <Grid size={14} />
          </button>
          <button
            onClick={() => setViewMode('list')}
            className={cn(
              'p-1.5 rounded-md transition-colors',
              viewMode === 'list' ? 'bg-card shadow-sm' : 'text-muted-foreground',
            )}
            aria-label="List view"
          >
            <List size={14} />
          </button>
        </div>
      </div>

      {/* VM grid / list */}
      {loading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="skeleton h-32 rounded-xl" />
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border p-16 text-center">
          <p className="text-muted-foreground text-sm">
            {search || stateFilter !== 'all'
              ? 'No VMs match your filters'
              : 'No virtual machines yet'}
          </p>
          {!search && stateFilter === 'all' && (
            <button
              onClick={() => navigate('/vms/create')}
              className="mt-3 text-sm text-primary hover:underline"
            >
              Create your first VM →
            </button>
          )}
        </div>
      ) : (
        <AnimatePresence mode="popLayout">
          <div
            className={cn(
              viewMode === 'grid'
                ? 'grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-4'
                : 'flex flex-col gap-2',
            )}
          >
            {filtered.map((vm) => (
              <VmCard key={vm.id} vm={vm} compact={viewMode === 'list'} />
            ))}
          </div>
        </AnimatePresence>
      )}

      {/* Result count */}
      {filtered.length > 0 && (
        <p className="text-xs text-muted-foreground">
          Showing {filtered.length} of {vms.length} VMs
        </p>
      )}
    </motion.div>
  )
}
