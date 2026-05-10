import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { modelsApi } from '../lib/api'
import { ChevronDown, ChevronUp } from 'lucide-react'

const PROVIDERS = ['all', 'openai', 'anthropic', 'gemini', 'cohere', 'groq', 'mistral', 'xai', 'bedrock', 'ollama']

function formatPrice(price: number): string {
  if (price === 0) return 'Free'
  if (price < 1) return `$${price.toFixed(3)}`
  return `$${price.toFixed(2)}`
}

function formatContext(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`
  return String(n)
}

function CapBadge({ ok, label }: { ok: boolean; label: string }) {
  return ok ? (
    <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs bg-blue-900/50 text-blue-300 border border-blue-800/50">
      {label}
    </span>
  ) : null
}

export default function ModelCatalog() {
  const [provider, setProvider] = useState('all')
  const [search, setSearch] = useState('')
  const [expandedId, setExpandedId] = useState<string | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['models', provider],
    queryFn: () => modelsApi.getAll(provider === 'all' ? undefined : provider),
    refetchInterval: 60_000,
  })

  const models = data?.data ?? []
  const filtered = models.filter(m => {
    if (!search) return true
    const q = search.toLowerCase()
    return m.id.toLowerCase().includes(q) ||
      (m.pylos.display_name?.toLowerCase().includes(q) ?? false)
  })

  // Groupe par provider
  const grouped = filtered.reduce((acc, m) => {
    const p = m.pylos.provider
    if (!acc[p]) acc[p] = []
    acc[p].push(m)
    return acc
  }, {} as Record<string, typeof filtered>)

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Model Catalog</h1>
          <p className="text-sm text-gray-400 mt-1">
            {filtered.length} models — pricing & capabilities
          </p>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-3">
        <input
          type="text"
          placeholder="Search models..."
          value={search}
          onChange={e => setSearch(e.target.value)}
          className="flex-1 px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
        />
        <div className="flex gap-1.5 flex-wrap">
          {PROVIDERS.map(p => (
            <button
              key={p}
              onClick={() => setProvider(p)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium capitalize transition-colors
                ${provider === p
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-800 text-gray-400 hover:text-white border border-gray-700'}`}
            >
              {p}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="h-16 bg-gray-800 rounded-xl animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-6">
          {Object.entries(grouped).sort(([a], [b]) => a.localeCompare(b)).map(([prov, pModels]) => (
            <div key={prov}>
              <h2 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2 capitalize">
                {prov} <span className="normal-case font-normal">({pModels.length})</span>
              </h2>
              <div className="rounded-xl border border-gray-800 bg-gray-900 overflow-hidden">
                <table className="w-full text-sm">
                  <thead className="border-b border-gray-800">
                    <tr>
                      {['Model', 'Context', 'Input /1M', 'Output /1M', 'Capabilities', ''].map(h => (
                        <th key={h} className="text-left px-4 py-3 text-xs text-gray-500 uppercase tracking-wide font-medium">
                          {h}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {pModels.map(({ id, pylos }) => (
                      <>
                        <tr
                          key={id}
                          className={`border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors cursor-pointer
                            ${expandedId === id ? 'bg-gray-800/20' : ''}`}
                          onClick={() => setExpandedId(expandedId === id ? null : id)}
                        >
                          <td className="px-4 py-3">
                            <div className="font-medium text-white">{pylos.display_name || pylos.model_id}</div>
                            <div className="text-xs text-gray-500 font-mono">{pylos.model_id}</div>
                          </td>
                          <td className="px-4 py-3 text-gray-300">
                            {formatContext(pylos.context_window)}
                          </td>
                          <td className="px-4 py-3 text-green-400 font-mono text-xs">
                            {formatPrice(pylos.input_price_per_1m_usd)}
                          </td>
                          <td className="px-4 py-3 text-orange-400 font-mono text-xs">
                            {formatPrice(pylos.output_price_per_1m_usd)}
                          </td>
                          <td className="px-4 py-3">
                            <div className="flex gap-1 flex-wrap">
                              <CapBadge ok={pylos.supports_vision} label="Vision" />
                              <CapBadge ok={pylos.supports_tools} label="Tools" />
                              <CapBadge ok={pylos.supports_embeddings} label="Embed" />
                              <CapBadge ok={pylos.supports_streaming} label="Stream" />
                            </div>
                          </td>
                          <td className="px-4 py-3 text-gray-500">
                            {expandedId === id ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                          </td>
                        </tr>
                        {expandedId === id && (
                          <tr key={`${id}-detail`} className="bg-gray-800/10">
                            <td colSpan={6} className="px-6 py-4">
                              <div className="grid grid-cols-3 gap-4 text-xs">
                                <div>
                                  <div className="text-gray-500 mb-1">Max output</div>
                                  <div className="text-white">{formatContext(pylos.max_output_tokens)} tokens</div>
                                </div>
                                <div>
                                  <div className="text-gray-500 mb-1">Provider</div>
                                  <div className="text-white capitalize">{pylos.provider}</div>
                                </div>
                                <div>
                                  <div className="text-gray-500 mb-1">Cost estimate (1K in + 500 out)</div>
                                  <div className="text-white">
                                    ${((pylos.input_price_per_1m_usd / 1000) + (pylos.output_price_per_1m_usd / 2000)).toFixed(4)}
                                  </div>
                                </div>
                              </div>
                            </td>
                          </tr>
                        )}
                      </>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ))}

          {filtered.length === 0 && (
            <div className="text-center py-16 text-gray-600">
              No models found
            </div>
          )}
        </div>
      )}
    </div>
  )
}
