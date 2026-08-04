import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { Command } from 'cmdk'
import { motion, AnimatePresence } from 'framer-motion'
import {
  Search,
  Server,
  LayoutDashboard,
  HardDrive,
  Network,
  Settings,
  Plus,
} from 'lucide-react'

import { useUiStore } from '@/stores/uiStore'
import { useVmStore } from '@/stores/vmStore'
import { cn } from '@/lib/utils'

export function CommandPalette() {
  const open = useUiStore((s) => s.commandPaletteOpen)
  const setOpen = useUiStore((s) => s.setCommandPaletteOpen)
  const navigate = useNavigate()
  const vms = useVmStore((s) => s.vms)

  const runCommand = (cb: () => void) => {
    setOpen(false)
    cb()
  }

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [setOpen])

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setOpen(false)}
            className="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm"
          />
          {/* Dialog */}
          <motion.div
            initial={{ opacity: 0, scale: 0.96, y: -10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: -10 }}
            transition={{ duration: 0.15 }}
            className="fixed left-1/2 top-[20%] -translate-x-1/2 z-50 w-full max-w-lg"
          >
            <Command
              className={cn(
                'overflow-hidden rounded-xl border border-border',
                'bg-popover shadow-2xl',
              )}
              shouldFilter={true}
            >
              <div className="flex items-center border-b border-border px-4">
                <Search size={16} className="text-muted-foreground mr-3 flex-shrink-0" />
                <Command.Input
                  placeholder="Search commands, VMs, pages…"
                  className={cn(
                    'flex-1 py-4 text-sm bg-transparent outline-none',
                    'placeholder:text-muted-foreground',
                  )}
                />
                <kbd className="text-xs text-muted-foreground border border-border px-1.5 py-0.5 rounded">
                  ESC
                </kbd>
              </div>

              <Command.List className="max-h-80 overflow-y-auto p-2">
                <Command.Empty className="py-8 text-center text-sm text-muted-foreground">
                  No results found.
                </Command.Empty>

                {/* Navigation */}
                <Command.Group heading="Navigation">
                  {[
                    { label: 'Dashboard', path: '/dashboard', icon: <LayoutDashboard size={14} /> },
                    { label: 'Virtual Machines', path: '/vms', icon: <Server size={14} /> },
                    { label: 'Storage', path: '/storage', icon: <HardDrive size={14} /> },
                    { label: 'Network', path: '/network', icon: <Network size={14} /> },
                    { label: 'Settings', path: '/settings', icon: <Settings size={14} /> },
                  ].map((item) => (
                    <Command.Item
                      key={item.path}
                      value={item.label}
                      onSelect={() => runCommand(() => navigate(item.path))}
                      className={cn(
                        'flex items-center gap-3 px-3 py-2 rounded-lg text-sm cursor-pointer',
                        'aria-selected:bg-accent transition-colors',
                      )}
                    >
                      <span className="text-muted-foreground">{item.icon}</span>
                      {item.label}
                    </Command.Item>
                  ))}
                </Command.Group>

                {/* Actions */}
                <Command.Group heading="Actions">
                  <Command.Item
                    value="Create new VM"
                    onSelect={() => runCommand(() => navigate('/vms/create'))}
                    className={cn(
                      'flex items-center gap-3 px-3 py-2 rounded-lg text-sm cursor-pointer',
                      'aria-selected:bg-accent transition-colors',
                    )}
                  >
                    <Plus size={14} className="text-muted-foreground" />
                    Create new VM
                  </Command.Item>
                </Command.Group>

                {/* VMs */}
                {vms.length > 0 && (
                  <Command.Group heading="Virtual Machines">
                    {vms.slice(0, 8).map((vm) => (
                      <Command.Item
                        key={vm.id}
                        value={vm.name}
                        onSelect={() => runCommand(() => navigate(`/vms/${vm.id}`))}
                        className={cn(
                          'flex items-center gap-3 px-3 py-2 rounded-lg text-sm cursor-pointer',
                          'aria-selected:bg-accent transition-colors',
                        )}
                      >
                        <Server size={14} className="text-muted-foreground" />
                        <span className="flex-1">{vm.name}</span>
                        <span className="text-xs text-muted-foreground">{vm.state}</span>
                      </Command.Item>
                    ))}
                  </Command.Group>
                )}
              </Command.List>
            </Command>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  )
}
