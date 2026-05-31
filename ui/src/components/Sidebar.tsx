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
  BarChart2,
} from 'lucide-react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { healthApi } from '../lib/api'

const NAV = [
  { to: '/dashboard',  icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/analytics',  icon: BarChart2,       label: 'RUM Analytics' },
  { to: '/playground', icon: FlaskConical,    label: 'Playground' },
  { to: '/logs',       icon: ScrollText,      label: 'Logs' },
  { to: '/providers',  icon: Server,          label: 'Providers' },
  { to: '/keys',       icon: KeyRound,        label: 'Virtual Keys' },
  { to: '/models',     icon: BookOpen,        label: 'Models' },
]

export function Sidebar({ isOpen, onClose }: { isOpen?: boolean; onClose?: () => void }) {
  const [collapsed, setCollapsed] = useState(false)

  const { data: health, isError } = useQuery({
    queryKey: ['health'],
    queryFn: healthApi.check,
    refetchInterval: 30_000,
    retry: 1,
  })

  const isHealthy = !isError && health !== undefined

  return (
    <>
      {/* Backdrop overlay for mobile drawer */}
      {isOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm md:hidden transition-opacity duration-300"
          onClick={onClose}
        />
      )}

      <aside
        className={`fixed inset-y-0 left-0 z-50 flex flex-col h-screen bg-zinc-950 border-r border-zinc-800/50 transition-all duration-300 md:duration-200
          md:static md:translate-x-0
          ${isOpen ? 'translate-x-0 shadow-2xl' : '-translate-x-full'}
          ${collapsed ? 'md:w-16' : 'md:w-56'} w-64`}
      >
        {/* Logo */}
        <div className="flex items-center justify-between px-4 py-5 border-b border-zinc-800/50">
          <div className="flex items-center gap-3">
            <div className="flex-shrink-0 w-8 h-8 overflow-hidden rounded-lg bg-zinc-800">
              <img src="/logo.png" alt="Pylos" className="w-full h-full object-contain" />
            </div>
            {(!collapsed || isOpen) && (
              <span className="font-bold text-lg text-white">Pylos</span>
            )}
          </div>
          {isOpen && (
            <button
              onClick={onClose}
              className="md:hidden text-zinc-400 hover:text-white p-1 rounded-lg hover:bg-zinc-800/50"
            >
              <ChevronLeft size={20} />
            </button>
          )}
        </div>

        {/* Nav */}
        <nav className="flex-1 px-2 py-4 space-y-1 overflow-y-auto">
          {NAV.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              onClick={onClose}
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
                  {(!collapsed || isOpen) && <span>{label}</span>}
                </>
              )}
            </NavLink>
          ))}
        </nav>

        {/* Status indicator */}
        <div className="px-4 py-3 border-t border-zinc-800/50">
          {(!collapsed || isOpen) && (
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
          {collapsed && !isOpen && (
            <div className="flex justify-center">
              {isHealthy
                ? <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                : <AlertCircle size={12} className="text-red-400" />
              }
            </div>
          )}
        </div>

        {/* Admin Key Configuration */}
        <button
          onClick={() => {
            if (onClose) onClose();
            const currentKey = localStorage.getItem('pylos_admin_key') || '';
            const newKey = window.prompt("Configure Pylos Admin Key (PYLOS_ADMIN_KEY):", currentKey);
            if (newKey !== null) {
              localStorage.setItem('pylos_admin_key', newKey);
              window.location.reload();
            }
          }}
          className="flex items-center gap-3 px-4 py-3 border-t border-zinc-800/50
            text-zinc-500 hover:text-white hover:bg-zinc-800/50 transition-colors text-sm w-full text-left"
          title="Configure Admin Key"
        >
          <KeyRound size={16} className="flex-shrink-0" />
          {(!collapsed || isOpen) && <span>Admin Key</span>}
        </button>

        {/* Collapse button */}
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="hidden md:flex items-center justify-center py-3 border-t border-zinc-800/50
            text-zinc-500 hover:text-white hover:bg-zinc-800/50 transition-colors"
        >
          {collapsed
            ? <ChevronRight size={16} />
            : <ChevronLeft size={16} />
          }
        </button>
      </aside>
    </>
  )
}
