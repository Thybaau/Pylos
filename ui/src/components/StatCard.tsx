interface StatCardProps {
  label: string
  value: string
  sub?: string
  color?: 'blue' | 'green' | 'yellow' | 'red' | 'purple'
  icon?: React.ReactNode
}

const COLORS = {
  blue:   'border-blue-500/30 bg-blue-500/5',
  green:  'border-green-500/30 bg-green-500/5',
  yellow: 'border-yellow-500/30 bg-yellow-500/5',
  red:    'border-red-500/30 bg-red-500/5',
  purple: 'border-purple-500/30 bg-purple-500/5',
}

export function StatCard({ label, value, sub, color = 'blue', icon }: StatCardProps) {
  return (
    <div className={`rounded-xl border p-4 ${COLORS[color]}`}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs text-gray-400 uppercase tracking-wide">{label}</span>
        {icon && <span className="text-gray-500">{icon}</span>}
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
      {sub && <div className="text-xs text-gray-500 mt-1">{sub}</div>}
    </div>
  )
}
