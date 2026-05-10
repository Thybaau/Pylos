import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { virtualKeysApi, type VkBudgetResponse } from '../lib/api'
import { KeyRound, CheckCircle, XCircle, Shield, TrendingUp, ChevronDown, ChevronUp } from 'lucide-react'

function BudgetBar({ used, max }: { used: number; max: number }) {
  const pct = max > 0 ? Math.min((used / max) * 100, 100) : 0
  const color = pct > 90 ? 'bg-red-500' : pct > 70 ? 'bg-yellow-500' : 'bg-green-500'
  return (
    <div className="w-full">
      <div className="flex justify-between text-xs text-gray-400 mb-1">
        <span>${used.toFixed(4)}</span>
        <span>${max.toFixed(2)}</span>
      </div>
      <div className="h-1.5 bg-gray-800 rounded-full overflow-hidden">
        <div className={`h-full rounded-full transition-all ${color}`} style={{ width: `${pct}%` }} />
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

  if (isLoading) return <div className="text-xs text-gray-500">Loading...</div>
  if (!data || (data.budget.length === 0 && data.rate_limits.length === 0)) {
    return <div className="text-xs text-gray-500">No governance configured</div>
  }

  return (
    <div className="grid grid-cols-2 gap-4 text-xs">
      {data.budget.length > 0 && (
        <div>
          <div className="text-gray-400 font-medium mb-2">Budget</div>
          {data.budget.map(b => (
            <div key={b.period} className="mb-3">
              <div className="text-gray-500 mb-1 capitalize">{b.period}</div>
              <BudgetBar used={b.current_usd} max={b.max_usd} />
            </div>
          ))}
        </div>
      )}
      {data.rate_limits.length > 0 && (
        <div>
          <div className="text-gray-400 font-medium mb-2">Rate Limits</div>
          {data.rate_limits.map(rl => (
            <div key={rl.window_type} className="mb-3">
              <div className="text-gray-500 mb-1 capitalize">{rl.window_type}</div>
              <div className="flex justify-between text-gray-300">
                <span>{rl.current_value}</span>
                <span>/ {rl.max_value}</span>
              </div>
              <div className="h-1.5 bg-gray-800 rounded-full overflow-hidden mt-1">
                <div
                  className={`h-full rounded-full transition-all ${
                    rl.max_value > 0 && rl.current_value / rl.max_value > 0.9
                      ? 'bg-red-500' : 'bg-blue-500'
                  }`}
                  style={{
                    width: rl.max_value > 0
                      ? `${Math.min((rl.current_value / rl.max_value) * 100, 100)}%`
                      : '0%'
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default function VirtualKeys() {
  const [expandedId, setExpandedId] = useState<string | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['virtual-keys'],
    queryFn: virtualKeysApi.getAll,
    refetchInterval: 30_000,
  })

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div>
        <h1 className="text-2xl font-bold text-white">Virtual Keys</h1>
        <p className="text-sm text-gray-400 mt-1">
          {data?.total ?? '—'} configured
        </p>
      </div>

      <div className="rounded-xl border border-gray-800 bg-gray-900 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="border-b border-gray-800">
            <tr>
              {['Name', 'Value', 'Status', 'Providers', 'Models', ''].map(h => (
                <th key={h} className="text-left px-5 py-3.5 text-xs text-gray-500 uppercase tracking-wide font-medium">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {isLoading
              ? Array.from({ length: 3 }).map((_, i) => (
                  <tr key={i} className="border-b border-gray-800/50">
                    {Array.from({ length: 6 }).map((_, j) => (
                      <td key={j} className="px-5 py-3.5">
                        <div className="h-3 bg-gray-800 rounded animate-pulse w-24" />
                      </td>
                    ))}
                  </tr>
                ))
              : data?.virtual_keys.map(vk => (
                  <>
                    <tr
                      key={vk.id}
                      className={`border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors cursor-pointer
                        ${expandedId === vk.id ? 'bg-gray-800/20' : ''}`}
                      onClick={() => setExpandedId(expandedId === vk.id ? null : vk.id)}
                    >
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-2">
                          <KeyRound size={14} className="text-blue-400" />
                          <div>
                            <div className="font-medium text-white">{vk.name}</div>
                            {vk.description && (
                              <div className="text-xs text-gray-500">{vk.description}</div>
                            )}
                          </div>
                        </div>
                      </td>
                      <td className="px-5 py-3.5 font-mono text-xs text-gray-400">
                        {vk.value}
                      </td>
                      <td className="px-5 py-3.5">
                        {vk.is_active ? (
                          <span className="flex items-center gap-1.5 text-green-400 text-xs">
                            <CheckCircle size={12} /> Active
                          </span>
                        ) : (
                          <span className="flex items-center gap-1.5 text-gray-500 text-xs">
                            <XCircle size={12} /> Inactive
                          </span>
                        )}
                      </td>
                      <td className="px-5 py-3.5">
                        <div className="flex flex-wrap gap-1">
                          {vk.provider_configs.map(pc => (
                            <span key={pc.provider}
                              className="px-2 py-0.5 rounded-full text-xs bg-gray-800 text-gray-300 border border-gray-700">
                              {pc.provider}
                            </span>
                          ))}
                        </div>
                      </td>
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-1.5 text-gray-400 text-xs">
                          <Shield size={11} />
                          {vk.provider_configs.some(pc => pc.allowed_models.includes('*'))
                            ? 'All models'
                            : vk.provider_configs.flatMap(pc => pc.allowed_models).slice(0, 2).join(', ')
                          }
                        </div>
                      </td>
                      <td className="px-5 py-3.5 text-gray-500">
                        {expandedId === vk.id
                          ? <ChevronUp size={14} />
                          : <ChevronDown size={14} />
                        }
                      </td>
                    </tr>
                    {expandedId === vk.id && (
                      <tr key={`${vk.id}-budget`} className="bg-gray-800/10">
                        <td colSpan={6} className="px-8 py-4">
                          <div className="flex items-center gap-2 text-xs text-gray-400 mb-3">
                            <TrendingUp size={12} />
                            <span>Governance</span>
                          </div>
                          <VkBudgetPanel vkId={vk.id} />
                        </td>
                      </tr>
                    )}
                  </>
                ))
            }
          </tbody>
        </table>

        {!isLoading && !data?.virtual_keys.length && (
          <div className="text-center py-16 text-gray-600">
            No virtual keys configured
          </div>
        )}
      </div>
    </div>
  )
}
