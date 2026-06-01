import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { virtualKeysApi, type VkBudgetResponse } from '../lib/api'
import {
  CreditCard, TrendingUp, RotateCw, RotateCcw,
} from 'lucide-react'

function BudgetBar({ used, max }: { used: number; max: number }) {
  const pct = max > 0 ? Math.min((used / max) * 100, 100) : 0
  const color = pct > 90 ? '#f43f5e' : pct > 70 ? '#f59e0b' : '#10b981'
  return (
    <div className="w-full">
      <div className="flex justify-between text-xs text-zinc-400 mb-1">
        <span>${used.toFixed(4)}</span>
        <span>${max.toFixed(2)}</span>
      </div>
      <div className="h-1.5 bg-zinc-800/50 rounded-full overflow-hidden">
        <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, background: color }} />
      </div>
    </div>
  )
}

function VkBudgetPanel({ vkId }: { vkId: string }) {
  const { data, isLoading } = useQuery<VkBudgetResponse>({
    queryKey: ['vk-budget', vkId],
    queryFn: () => virtualKeysApi.getBudget(vkId),
    refetchInterval: 30_000,
  })
  if (isLoading) return <div className="text-xs text-zinc-500">Loading…</div>
  if (!data || (data.budget.length === 0 && data.rate_limits.length === 0)) {
    return <div className="text-xs text-zinc-500">No governance configured</div>
  }
  return (
    <div className="grid grid-cols-2 gap-4 text-xs">
      {data.budget.length > 0 && (
        <div>
          <div className="text-zinc-400 font-medium mb-2">Budget</div>
          {data.budget.map(b => (
            <div key={b.period} className="mb-3">
              <div className="text-zinc-500 mb-1 capitalize">{b.period}</div>
              <BudgetBar used={b.current_usd} max={b.max_usd} />
            </div>
          ))}
        </div>
      )}
      {data.rate_limits.length > 0 && (
        <div>
          <div className="text-zinc-400 font-medium mb-2">Rate Limits</div>
          {data.rate_limits.map(rl => (
            <div key={rl.window_type} className="mb-3">
              <div className="text-zinc-500 mb-1 capitalize">{rl.window_type}</div>
              <div className="flex justify-between text-zinc-300">
                <span>{rl.current_value}</span>
                <span>/ {rl.max_value}</span>
              </div>
              <div className="h-1.5 bg-zinc-800/50 rounded-full overflow-hidden mt-1">
                <div className="h-full rounded-full transition-all"
                  style={{ width: rl.max_value > 0 ? `${Math.min((rl.current_value / rl.max_value) * 100, 100)}%` : '0%', background: rl.max_value > 0 && rl.current_value / rl.max_value > 0.9 ? '#f43f5e' : '#3b82f6' }} />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default function Budgets() {
  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ['virtual-keys'],
    queryFn: virtualKeysApi.getAll,
    refetchInterval: 30_000,
  })
  const [expandedId, setExpandedId] = useState<string | null>(null)

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between">
        <div><h1 className="text-2xl font-bold text-white">Budgets & Billing</h1><p className="text-sm text-zinc-400 mt-1">Track spending limits and usage across virtual keys</p></div>
        <button onClick={() => refetch()} disabled={isFetching}
          className="flex items-center justify-center p-2 text-zinc-400 hover:text-white bg-zinc-950 hover:bg-zinc-900 border border-zinc-800 disabled:opacity-50 rounded-lg transition-colors">
          <RotateCw size={15} className={isFetching ? 'animate-spin' : ''} />
        </button>
      </div>
      <div className="rounded-xl border border-zinc-800/50 bg-zinc-900/30 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="border-b border-zinc-800/50">
            <tr>
              {['Virtual Key', 'Budget', 'Rate Limits', ''].map(h => (
                <th key={h} className="text-left px-5 py-3.5 text-xs text-zinc-500 uppercase tracking-wide font-medium">{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {isLoading ? Array.from({ length: 3 }).map((_, i) => (
              <tr key={i} className="border-b border-zinc-800/30">
                {Array.from({ length: 4 }).map((_, j) => <td key={j} className="px-5 py-3.5"><div className="h-3 bg-zinc-800 rounded animate-pulse w-24" /></td>)}
              </tr>
            )) : data?.virtual_keys.map(vk => (
              <>
                <tr key={vk.id}
                  className={`border-b border-zinc-800/30 transition-colors cursor-pointer group ${expandedId === vk.id ? 'bg-emerald-500/5 border-l-2 border-l-emerald-500' : 'hover:bg-zinc-800/30'}`}
                  onClick={() => setExpandedId(expandedId === vk.id ? null : vk.id)}>
                  <td className="px-5 py-3.5">
                    <div className="flex items-center gap-2">
                      <CreditCard size={14} className="text-emerald-400 shrink-0" />
                      <div>
                        <div className="font-medium text-white">{vk.name}</div>
                        {vk.description && <div className="text-xs text-zinc-500">{vk.description}</div>}
                      </div>
                    </div>
                  </td>
                  <td className="px-5 py-3.5 text-xs text-zinc-400">—</td>
                  <td className="px-5 py-3.5 text-xs text-zinc-400">—</td>
                  <td className="px-5 py-3.5 text-right text-zinc-600">{expandedId === vk.id ? <RotateCcw size={14} /> : <RotateCw size={14} />}</td>
                </tr>
                {expandedId === vk.id && (
                  <tr key={`${vk.id}-gov`} className="bg-zinc-800/20">
                    <td colSpan={4} className="px-8 py-4">
                      <div className="flex items-center gap-2 text-xs text-zinc-400 mb-3">
                        <TrendingUp size={12} />
                        <span>Governance Details</span>
                      </div>
                      <VkBudgetPanel vkId={vk.id} />
                    </td>
                  </tr>
                )}
              </>
            ))}
          </tbody>
        </table>
        {!isLoading && !data?.virtual_keys.length && (
          <div className="text-center py-16 text-zinc-600">No virtual keys — create one to set budgets</div>
        )}
      </div>
    </div>
  )
}
