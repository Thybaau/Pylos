import { NavLink } from 'react-router-dom'
import {
  LayoutDashboard,
  ScrollText,
  Server,
  KeyRound,
  ChevronLeft,
  ChevronRight,
  FlaskConical,
  BookOpen,
  AlertCircle,
} from 'lucide-react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { healthApi } from '../lib/api'

const NAV = [
  { to: '/dashboard',  icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/playground', icon: FlaskConical,    label: 'Playground' },
  { to: '/logs',       icon: ScrollText,      label: 'Logs' },
  { to: '/providers',  icon: Server,          label: 'Providers' },
  { to: '/keys',       icon: KeyRound,        label: 'Virtual Keys' },
  { to: '/models',     icon: BookOpen,        label: 'Models' },
]

export function Sidebar() {
  const [collapsed, setCollapsed] = useState(false)

  const { data: health, isError } = useQuery({
    queryKey: ['health'],
    queryFn: healthApi.check,
    refetchInterval: 30_000,
    retry: 1,
  })

  const isHealthy = !isError && health !== undefined

  return (
    <aside
      className={`flex flex-col h-screen bg-zinc-900/50 border-r border-zinc-800/50 transition-all duration-200
        ${collapsed ? 'w-16' : 'w-56'}`}
    >
      {/* Logo */}
      <div className="flex items-center gap-3 px-4 py-5 border-b border-zinc-800/50">
        <div className="flex-shrink-0 w-8 h-8 overflow-hidden rounded-lg bg-zinc-800">
          <img src="/logo.png" alt="Pylos" className="w-full h-full object-contain" />
        </div>
        {!collapsed && (
          <span className="font-bold text-lg text-white">Pylos</span>
        )}
      </div>

      {/* Nav */}
      <nav className="flex-1 px-2 py-4 space-y-1">
        {NAV.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors
              ${isActive
                ? 'bg-zinc-800 text-white'
                : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-100'
              }`
            }
          >
            {({ isActive }) => (
              <>
                {isActive && <div className="w-1 h-1 rounded-full bg-emerald-500 flex-shrink-0" />}
                <Icon size={18} className="flex-shrink-0" />
                {!collapsed && <span>{label}</span>}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      {/* Status indicator */}
      <div className="px-4 py-3 border-t border-zinc-800/50">
        {!collapsed && (
          <div className="flex items-center gap-2 text-xs">
            {isHealthy ? (
              <>
                <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                <span className="text-zinc-500">Gateway active</span>
              </>
            ) : (
              <>
                <AlertCircle size={12} className="text-red-400" />
                <span className="text-red-400">Gateway unreachable</span>
              </>
            )}
          </div>
        )}
        {collapsed && (
          <div className="flex justify-center">
            {isHealthy
              ? <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
              : <AlertCircle size={12} className="text-red-400" />
            }
          </div>
        )}
      </div>

      {/* Collapse button */}
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="flex items-center justify-center py-3 border-t border-zinc-800/50
          text-zinc-500 hover:text-white hover:bg-zinc-800/50 transition-colors"
      >
        {collapsed
          ? <ChevronRight size={16} />
          : <ChevronLeft size={16} />
        }
      </button>
    </aside>
  )
}
