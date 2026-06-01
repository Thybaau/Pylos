import { BrowserRouter, Routes, Route, Navigate, NavLink } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Component, useState, type ReactNode } from 'react'
import { Sidebar } from './components/Sidebar'
import Dashboard from './pages/Dashboard'
import Playground from './pages/Playground'
import Logs from './pages/Logs'
import Providers from './pages/Providers'
import VirtualKeys from './pages/VirtualKeys'
import ModelCatalog from './pages/ModelCatalog'
import Guardrails from './pages/Guardrails'
import Analytics from './pages/Analytics'
import {
  Menu,
  LayoutDashboard,
  BarChart2,
  FlaskConical,
  ScrollText,
  Server,
  KeyRound,
  BookOpen,
  Shield,
} from 'lucide-react'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10_000,
      retry: 2,
    },
  },
})

// ── ErrorBoundary ─────────────────────────────────────────────────────────────

interface EBState { hasError: boolean; message: string }

class ErrorBoundary extends Component<{ children: ReactNode }, EBState> {
  constructor(props: { children: ReactNode }) {
    super(props)
    this.state = { hasError: false, message: '' }
  }

  static getDerivedStateFromError(error: Error): EBState {
    return { hasError: true, message: error.message }
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-full gap-4 text-gray-400 p-8">
          <div className="text-4xl">⚠️</div>
          <div className="text-lg font-semibold text-white">Something went wrong</div>
          <div className="text-sm text-gray-500 font-mono max-w-md text-center break-all">
            {this.state.message}
          </div>
          <button
            onClick={() => this.setState({ hasError: false, message: '' })}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded-lg transition-colors"
          >
            Try again
          </button>
        </div>
      )
    }
    return this.props.children
  }
}

const MOBILE_NAV = [
  { to: '/dashboard',  icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/analytics',  icon: BarChart2,       label: 'Analytics' },
  { to: '/playground', icon: FlaskConical,    label: 'Playground' },
  { to: '/logs',       icon: ScrollText,      label: 'Logs' },
  { to: '/providers',  icon: Server,          label: 'Providers' },
  { to: '/keys',       icon: KeyRound,        label: 'Keys' },
  { to: '/models',     icon: BookOpen,        label: 'Models' },
  { to: '/guardrails', icon: Shield,          label: 'Guardrails' },
]

// ── App ───────────────────────────────────────────────────────────────────────

export default function App() {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false)

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div className="flex flex-col md:flex-row h-screen overflow-hidden bg-zinc-950 text-zinc-100">
          {/* Top Bar for Mobile */}
          <header className="flex md:hidden items-center justify-between px-4 py-3 bg-zinc-900 border-b border-zinc-800/50 z-30 h-14 shrink-0">
            <button
              onClick={() => setIsMobileMenuOpen(true)}
              className="text-zinc-400 hover:text-white p-1 rounded-lg hover:bg-zinc-800/50 transition-colors"
              aria-label="Open menu"
            >
              <Menu size={24} />
            </button>
            <div className="flex items-center gap-2">
              <div className="w-7 h-7 overflow-hidden rounded-lg bg-zinc-850 border border-zinc-800 flex items-center justify-center p-0.5">
                <img src="/logo.png" alt="Pylos" className="w-full h-full object-contain" />
              </div>
              <span className="font-bold text-base text-white tracking-wide">Pylos</span>
            </div>
            <div className="w-8" /> {/* Spacer */}
          </header>

          <Sidebar isOpen={isMobileMenuOpen} onClose={() => setIsMobileMenuOpen(false)} />

          <main className="flex-1 overflow-y-auto md:overflow-hidden md:h-screen pb-16 md:pb-0">
            <ErrorBoundary>
              <Routes>
                <Route path="/" element={<Navigate to="/dashboard" replace />} />
                <Route path="/dashboard"  element={<Dashboard />} />
                <Route path="/playground" element={<Playground />} />
                <Route path="/logs"       element={<Logs />} />
                <Route path="/providers"  element={<Providers />} />
                <Route path="/keys"       element={<VirtualKeys />} />
                <Route path="/models"     element={<ModelCatalog />} />
                <Route path="/guardrails" element={<Guardrails />} />
                <Route path="/analytics"  element={<Analytics />} />
              </Routes>
            </ErrorBoundary>
          </main>

          {/* Bottom Bar for Mobile */}
          <nav className="flex md:hidden fixed bottom-0 left-0 right-0 z-30 bg-zinc-950/80 backdrop-blur-lg border-t border-zinc-900/80 h-16 px-1 justify-around items-center">
            {MOBILE_NAV.map(({ to, icon: Icon, label }) => (
              <NavLink
                key={to}
                to={to}
                className={({ isActive }) =>
                  `flex flex-col items-center justify-center flex-1 py-1 text-[9px] min-[375px]:text-[10px] transition-all duration-200
                  ${isActive ? 'text-emerald-400 scale-105 font-medium' : 'text-zinc-500 hover:text-zinc-300'}`
                }
              >
                <Icon size={18} className="mb-0.5" />
                <span className="truncate max-w-[50px] min-[375px]:max-w-[60px]">{label}</span>
              </NavLink>
            ))}
          </nav>
        </div>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
