interface StatCardProps {
  label: string
  value: string
  sub?: string
  color?: 'blue' | 'green' | 'yellow' | 'red' | 'purple'
  icon?: React.ReactNode
}

const BORDER_COLORS: Record<string, string> = {
  blue:   'border-l-blue-500',
  green:  'border-l-emerald-500',
  yellow: 'border-l-amber-500',
  red:    'border-l-rose-500',
  purple: 'border-l-violet-500',
}

export function StatCard({ label, value, sub, color = 'blue', icon }: StatCardProps) {
  return (
    <div className={`rounded-xl border border-zinc-800/50 bg-zinc-900/50 p-4 transition-all duration-150 hover:border-zinc-700/50 border-l-2 ${BORDER_COLORS[color]}`}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs text-zinc-400 uppercase tracking-wide">{label}</span>
        {icon && <span className="text-zinc-500">{icon}</span>}
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
      {sub && <div className="text-xs text-zinc-500 mt-1">{sub}</div>}
    </div>
  )
}
