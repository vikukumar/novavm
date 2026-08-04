import { NavLink } from 'react-router-dom'
import { motion, AnimatePresence } from 'framer-motion'
import {
  LayoutDashboard,
  Server,
  HardDrive,
  Network,
  Camera,
  ScrollText,
  Settings,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react'

import { cn } from '@/lib/utils'
import { useUiStore } from '@/stores/uiStore'
import { useVmStore } from '@/stores/vmStore'

interface NavItem {
  path: string
  label: string
  icon: React.ReactNode
  badge?: number
}

const navItems: NavItem[] = [
  { path: '/dashboard', label: 'Dashboard', icon: <LayoutDashboard size={18} /> },
  { path: '/vms', label: 'Virtual Machines', icon: <Server size={18} /> },
  { path: '/storage', label: 'Storage', icon: <HardDrive size={18} /> },
  { path: '/network', label: 'Network', icon: <Network size={18} /> },
  { path: '/snapshots', label: 'Snapshots', icon: <Camera size={18} /> },
  { path: '/logs', label: 'Logs', icon: <ScrollText size={18} /> },
]

export function Sidebar() {
  const collapsed = useUiStore((s) => s.sidebarCollapsed)
  const toggleSidebar = useUiStore((s) => s.toggleSidebar)
  const vms = useVmStore((s) => s.vms)
  const runningCount = vms.filter((v) => v.state === 'running').length

  return (
    <motion.aside
      initial={false}
      animate={{ width: collapsed ? 60 : 220 }}
      transition={{ duration: 0.25, ease: 'easeInOut' }}
      className="relative flex flex-col h-full bg-card border-r border-border overflow-hidden"
    >
      {/* Logo */}
      <div className="flex items-center gap-3 px-4 py-5 border-b border-border">
        <img
          src="/novavm_icon.png"
          alt="NovaVM Logo"
          className="flex-shrink-0 w-8 h-8 rounded-lg shadow-lg object-cover border border-white/10"
        />
        <AnimatePresence>
          {!collapsed && (
            <motion.div
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -10 }}
              transition={{ duration: 0.2 }}
              className="overflow-hidden"
            >
              <p className="font-bold text-sm tracking-tight">NovaVM</p>
              <p className="text-xs text-muted-foreground">
                {runningCount} running
              </p>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Navigation */}
      <nav className="flex-1 py-3 space-y-0.5 overflow-y-auto">
        {navItems.map((item) => (
          <SidebarItem key={item.path} item={item} collapsed={collapsed} />
        ))}
      </nav>

      {/* Settings at bottom */}
      <div className="border-t border-border py-3">
        <SidebarItem
          item={{ path: '/settings', label: 'Settings', icon: <Settings size={18} /> }}
          collapsed={collapsed}
        />
      </div>

      {/* Collapse button */}
      <button
        onClick={toggleSidebar}
        className={cn(
          'absolute -right-3 top-1/2 -translate-y-1/2',
          'w-6 h-6 rounded-full bg-card border border-border',
          'flex items-center justify-center',
          'hover:bg-accent transition-colors',
          'z-10 shadow-sm',
        )}
        aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      >
        {collapsed ? <ChevronRight size={12} /> : <ChevronLeft size={12} />}
      </button>
    </motion.aside>
  )
}

function SidebarItem({
  item,
  collapsed,
}: {
  item: NavItem
  collapsed: boolean
}) {
  return (
    <NavLink
      to={item.path}
      className={({ isActive }) =>
        cn(
          'flex items-center gap-3 mx-2 px-3 py-2 rounded-lg text-sm font-medium',
          'transition-all duration-150',
          'hover:bg-accent hover:text-accent-foreground',
          isActive
            ? 'bg-primary/10 text-primary font-semibold'
            : 'text-muted-foreground',
        )
      }
      title={collapsed ? item.label : undefined}
    >
      <span className="flex-shrink-0">{item.icon}</span>
      <AnimatePresence>
        {!collapsed && (
          <motion.span
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="truncate"
          >
            {item.label}
          </motion.span>
        )}
      </AnimatePresence>
      {!collapsed && item.badge !== undefined && item.badge > 0 && (
        <span className="ml-auto text-xs bg-primary text-primary-foreground rounded-full px-1.5 py-0.5 min-w-[20px] text-center">
          {item.badge}
        </span>
      )}
    </NavLink>
  )
}
