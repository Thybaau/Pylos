import { NavLink } from 'react-router-dom'
import {
  LayoutDashboard,
  ScrollText,
  Server,
  KeyRound,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  ChevronUp,
  FlaskConical,
  BookOpen,
  AlertCircle,
  BarChart2,
  Shield,
  Users,
  User,
  Landmark,
  Layers,
  CreditCard,
  FileBadge,
  Wrench,
  Search,
  Database,
  ShieldCheck,
  LogOut,
} from 'lucide-react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { healthApi, authApi } from '../lib/api'

const NAV_MAIN = [
  { to: '/dashboard',  icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/analytics',  icon: BarChart2,       label: 'RUM Analytics' },
  { to: '/playground', icon: FlaskConical,    label: 'Playground' },
  { to: '/logs',       icon: ScrollText,      label: 'Logs' },
  { to: '/providers',  icon: Server,          label: 'Providers' },
  { to: '/keys',       icon: KeyRound,        label: 'Virtual Keys' },
  { to: '/models',     icon: BookOpen,        label: 'Models' },
  { to: '/guardrails', icon: Shield,          label: 'Guardrails' },
]

const NAV_ACCESS = [
  { to: '/teams',          icon: Users,          label: 'Teams' },
  { to: '/users',          icon: User,           label: 'Internal Users' },
  { to: '/organizations',  icon: Landmark,       label: 'Organizations' },
  { to: '/access-groups',  icon: Layers,         label: 'Access Groups' },
  { to: '/budgets',        icon: CreditCard,     label: 'Budgets' },
]

const NAV_POLICIES = [
  { to: '/policies', icon: FileBadge, label: 'Policies' },
]

const NAV_TOOLS = [
  { to: '/tools/search', icon: Search, label: 'Search Tools' },
  { to: '/tools/vector-stores', icon: Database, label: 'Vector Stores' },
  { to: '/tools/policies', icon: ShieldCheck, label: 'Tool Policies' },
]

export function Sidebar({ isOpen, onClose }: { isOpen?: boolean; onClose?: () => void }) {
  const [collapsed, setCollapsed] = useState(false)
  const [toolsExpanded, setToolsExpanded] = useState(true)

  const userJson = typeof window !== 'undefined' ? sessionStorage.getItem('pylos_user') : null;
  const user = userJson ? JSON.parse(userJson) : null;

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
          {NAV_MAIN.map(({ to, icon: Icon, label }) => (
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

          {NAV_POLICIES.map(({ to, icon: Icon, label }) => (
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

          <div className="flex flex-col mt-1">
            <button
              onClick={() => {
                if (collapsed) setCollapsed(false);
                setToolsExpanded(!toolsExpanded);
              }}
              className="flex items-center justify-between px-3 py-2 rounded-lg text-sm transition-colors text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-100 w-full"
            >
              <div className="flex items-center gap-3">
                <Wrench size={18} className="flex-shrink-0" />
                {(!collapsed || isOpen) && <span>Tools</span>}
              </div>
              {(!collapsed || isOpen) && (
                toolsExpanded ? <ChevronUp size={16} className="opacity-70" /> : <ChevronDown size={16} className="opacity-70" />
              )}
            </button>
            {toolsExpanded && (!collapsed || isOpen) && (
              <div className="flex flex-col mt-1 ml-4 space-y-1 border-l border-zinc-800/50 pl-2">
                {NAV_TOOLS.map(({ to, icon: Icon, label }) => (
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
                        {isActive && <div className="w-1 h-1 rounded-full bg-emerald-500 flex-shrink-0 absolute -ml-4" />}
                        <Icon size={16} className="flex-shrink-0" />
                        <span>{label}</span>
                      </>
                    )}
                  </NavLink>
                ))}
              </div>
            )}
          </div>

          {(!collapsed || isOpen) ? (
            <div className="px-3 pt-6 pb-2 text-xs font-semibold text-zinc-500 uppercase tracking-wider">
              Access Control
            </div>
          ) : (
            <div className="mx-3 mt-6 mb-2 border-t border-zinc-800/50" />
          )}

          {NAV_ACCESS.map(({ to, icon: Icon, label }) => (
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
            const currentKey = sessionStorage.getItem('pylos_admin_key') || '';
            const newKey = window.prompt("Configure Pylos Admin Key (PYLOS_ADMIN_KEY):", currentKey);
            if (newKey !== null) {
              sessionStorage.setItem('pylos_admin_key', newKey);
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

        {/* User profile card */}
        {user ? (
          <div className="px-4 py-3 border-t border-zinc-800/50 flex flex-col gap-2">
            {(!collapsed || isOpen) ? (
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center font-bold text-indigo-400 text-sm">
                  {user.name ? user.name.substring(0, 2).toUpperCase() : 'U'}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-semibold text-white truncate">{user.name}</p>
                  <p className="text-xs text-zinc-500 truncate">{user.email}</p>
                </div>
              </div>
            ) : (
              <div className="flex justify-center">
                <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center font-bold text-indigo-400 text-sm" title={user.name}>
                  {user.name ? user.name.substring(0, 2).toUpperCase() : 'U'}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="px-4 py-3 border-t border-zinc-800/50 flex flex-col gap-2">
            {(!collapsed || isOpen) ? (
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-full bg-zinc-850 border border-zinc-800 flex items-center justify-center font-bold text-amber-500 text-sm">
                  AD
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-semibold text-white truncate">Administrator</p>
                  <p className="text-xs text-zinc-500 truncate">Static Admin Key</p>
                </div>
              </div>
            ) : (
              <div className="flex justify-center">
                <div className="w-8 h-8 rounded-full bg-zinc-850 border border-zinc-800 flex items-center justify-center font-bold text-amber-500 text-sm" title="Administrator">
                  AD
                </div>
              </div>
            )}
          </div>
        )}

        <button
          onClick={async () => {
            if (onClose) onClose();
            try {
              await authApi.logout();
            } catch (e) {
              console.error("Logout request failed:", e);
            } finally {
              sessionStorage.removeItem('pylos_admin_key');
              sessionStorage.removeItem('pylos_user');
              window.location.href = '/login';
            }
          }}
          className="flex items-center gap-3 px-4 py-3 border-t border-zinc-800/50
            text-zinc-500 hover:text-red-400 hover:bg-red-500/10 transition-colors text-sm w-full text-left"
          title="Logout"
        >
          <LogOut size={16} className="flex-shrink-0" />
          {(!collapsed || isOpen) && <span>Logout</span>}
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
