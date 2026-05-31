import { useState, Fragment, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { modelsApi, type ModelInfo } from '../lib/api'
import {
  ChevronDown, Plus, Pencil, Trash2, X, Check,
  AlertTriangle, RotateCcw, AlertCircle, Search,
} from 'lucide-react'

const PROVIDERS = ['all', 'openai', 'anthropic', 'gemini', 'cohere', 'groq', 'mistral', 'xai', 'deepseek', 'bedrock', 'ollama-jo3', 'lemonade-jo3', 'lemonade-optimus']

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
    <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs bg-zinc-800 text-zinc-300 border border-zinc-700/50">
      {label}
    </span>
  ) : null
}

// ─── Types form ───────────────────────────────────────────────────────────────

interface ModelFormState {
  provider: string
  model_id: string
  display_name: string
  context_window: string
  max_output_tokens: string
  input_price_per_1m_usd: string
  output_price_per_1m_usd: string
  supports_vision: boolean
  supports_tools: boolean
  supports_streaming: boolean
  supports_embeddings: boolean
  is_deprecated: boolean
  enabled: boolean
}

const DEFAULT_FORM: ModelFormState = {
  provider: '',
  model_id: '',
  display_name: '',
  context_window: '0',
  max_output_tokens: '0',
  input_price_per_1m_usd: '0',
  output_price_per_1m_usd: '0',
  supports_vision: false,
  supports_tools: true,
  supports_streaming: true,
  supports_embeddings: false,
  is_deprecated: false,
  enabled: true,
}



function formToPayload(f: ModelFormState) {
  return {
    provider: f.provider,
    model_id: f.model_id,
    display_name: f.display_name || null,
    context_window: parseInt(f.context_window) || 0,
    max_output_tokens: parseInt(f.max_output_tokens) || 0,
    input_price_per_1m_usd: parseFloat(f.input_price_per_1m_usd) || 0,
    output_price_per_1m_usd: parseFloat(f.output_price_per_1m_usd) || 0,
    supports_vision: f.supports_vision,
    supports_tools: f.supports_tools,
    supports_streaming: f.supports_streaming,
    supports_embeddings: f.supports_embeddings,
    is_deprecated: f.is_deprecated,
    enabled: f.enabled,
  }
}

// Convert API ModelInfo to form state
function pylosToForm(p: ModelInfo): ModelFormState {
  return {
    provider: p.provider,
    model_id: p.model_id,
    display_name: p.display_name ?? '',
    context_window: String(p.context_window),
    max_output_tokens: String(p.max_output_tokens),
    input_price_per_1m_usd: String(p.input_price_per_1m_usd),
    output_price_per_1m_usd: String(p.output_price_per_1m_usd),
    supports_vision: p.supports_vision,
    supports_tools: p.supports_tools,
    supports_streaming: p.supports_streaming,
    supports_embeddings: p.supports_embeddings,
    is_deprecated: p.is_deprecated,
    enabled: p.enabled,
  }
}

// ─── ModelModal ───────────────────────────────────────────────────────────────

function ModelModal({
  initial,
  isEdit,
  onClose,
  onSave,
  isSaving,
  error,
}: {
  initial: ModelFormState
  isEdit: boolean
  onClose: () => void
  onSave: (form: ModelFormState) => void
  isSaving: boolean
  error: string | null
}) {
  const [form, setForm] = useState<ModelFormState>(initial)
  const set = <K extends keyof ModelFormState>(k: K, v: ModelFormState[K]) =>
    setForm(f => ({ ...f, [k]: v }))

  const Toggle = ({ k }: { k: keyof ModelFormState }) => (
    <button
      type="button"
      onClick={() => set(k, !form[k] as ModelFormState[typeof k])}
      className={`relative w-9 h-5 rounded-full transition-colors ${form[k] ? 'bg-emerald-600' : 'bg-zinc-700'}`}
    >
      <span className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all ${form[k] ? 'left-4' : 'left-0.5'}`} />
    </button>
  )

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between p-5 border-b border-zinc-800/50">
          <h2 className="text-lg font-semibold text-white">
            {isEdit ? 'Edit model' : 'Add custom model'}
          </h2>
          <button onClick={onClose} className="text-zinc-500 hover:text-white"><X size={18} /></button>
        </div>

        <div className="p-5 space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs text-zinc-400 mb-1.5">Provider *</label>
              <input
                value={form.provider}
                onChange={e => set('provider', e.target.value)}
                disabled={isEdit}
                placeholder="openai, ollama…"
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                  disabled:opacity-50 focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
              />
            </div>
            <div>
              <label className="block text-xs text-zinc-400 mb-1.5">Model ID *</label>
              <input
                value={form.model_id}
                onChange={e => set('model_id', e.target.value)}
                disabled={isEdit}
                placeholder="gpt-4o, llama3.2:3b…"
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                  disabled:opacity-50 font-mono focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
              />
            </div>
          </div>

          <div>
            <label className="block text-xs text-zinc-400 mb-1.5">Display name</label>
            <input
              value={form.display_name}
              onChange={e => set('display_name', e.target.value)}
              placeholder="GPT-4o"
              className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs text-zinc-400 mb-1.5">Context window (tokens)</label>
              <input
                type="number"
                value={form.context_window}
                onChange={e => set('context_window', e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                  focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
              />
            </div>
            <div>
              <label className="block text-xs text-zinc-400 mb-1.5">Max output tokens</label>
              <input
                type="number"
                value={form.max_output_tokens}
                onChange={e => set('max_output_tokens', e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                  focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs text-zinc-400 mb-1.5">Input price / 1M tokens (USD)</label>
              <input
                type="number"
                step="0.001"
                value={form.input_price_per_1m_usd}
                onChange={e => set('input_price_per_1m_usd', e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                  focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
              />
            </div>
            <div>
              <label className="block text-xs text-zinc-400 mb-1.5">Output price / 1M tokens (USD)</label>
              <input
                type="number"
                step="0.001"
                value={form.output_price_per_1m_usd}
                onChange={e => set('output_price_per_1m_usd', e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                  focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
              />
            </div>
          </div>

          <div className="space-y-2.5">
            <label className="block text-xs text-zinc-400">Capabilities</label>
            {(
              [
                { k: 'supports_vision', label: 'Vision' },
                { k: 'supports_tools', label: 'Tool calling' },
                { k: 'supports_streaming', label: 'Streaming' },
                { k: 'supports_embeddings', label: 'Embeddings' },
                { k: 'is_deprecated', label: 'Deprecated' },
              ] as const
            ).map(({ k, label }) => (
              <div key={k} className="flex items-center justify-between">
                <span className="text-sm text-zinc-300">{label}</span>
                <Toggle k={k} />
              </div>
            ))}
            <div className="flex items-center justify-between mt-2">
              <span className="text-sm text-zinc-300">Enabled</span>
              <Toggle k="enabled" />
            </div>
          </div>

          {error && (
            <div className="flex items-center gap-2 text-red-400 text-xs bg-red-900/20 border border-red-800/50 rounded-lg px-3 py-2">
              <AlertTriangle size={13} /> {error}
            </div>
          )}
        </div>

        <div className="flex justify-end gap-3 px-5 py-4 border-t border-zinc-800/50">
          <button onClick={onClose} className="px-4 py-2 text-sm text-zinc-400 hover:text-white">Cancel</button>
          <button
            onClick={() => onSave(form)}
            disabled={isSaving || !form.provider.trim() || !form.model_id.trim()}
            className="px-4 py-2 text-sm bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] disabled:opacity-50
              text-white rounded-lg flex items-center gap-2 transition-colors"
          >
            {isSaving ? <RotateCcw size={14} className="animate-spin" /> : <Check size={14} />}
            {isEdit ? 'Update' : 'Add'}
          </button>
        </div>
      </div>
    </div>
  )
}

// ─── DeleteConfirm ────────────────────────────────────────────────────────────

function DeleteConfirm({
  modelId,
  onClose,
  onConfirm,
  isDeleting,
}: {
  modelId: string
  onClose: () => void
  onConfirm: () => void
  isDeleting: boolean
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-sm mx-4 p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-9 h-9 rounded-full bg-red-500/15 flex items-center justify-center">
            <AlertTriangle size={16} className="text-red-400" />
          </div>
          <div>
            <div className="font-semibold text-white">Remove from catalog</div>
            <div className="text-xs text-zinc-500">This removes only the catalog entry</div>
          </div>
        </div>
        <p className="text-sm text-zinc-400 mb-5">
          Remove <span className="text-white font-mono text-xs">{modelId}</span> from the model catalog?
        </p>
        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="px-4 py-2 text-sm text-zinc-400 hover:text-white">Cancel</button>
          <button
            onClick={onConfirm}
            disabled={isDeleting}
            className="px-4 py-2 text-sm bg-red-600 hover:bg-red-500 disabled:opacity-50 text-white rounded-lg
              flex items-center gap-2 active:scale-[0.98]"
          >
            {isDeleting ? <RotateCcw size={13} className="animate-spin" /> : <Trash2 size={13} />}
            Remove
          </button>
        </div>
      </div>
    </div>
  )
}

// ─── Main page ────────────────────────────────────────────────────────────────

export default function ModelCatalog() {
  const qc = useQueryClient()
  const [provider, setProvider] = useState('all')
  const [search, setSearch] = useState('')
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [editingModel, setEditingModel] = useState<ModelInfo | null>(null)
  const [deletingModel, setDeletingModel] = useState<{ provider: string; model_id: string } | null>(null)
  const [collapsedProviders, setCollapsedProviders] = useState<Set<string>>(new Set())
  const [mutationError, setMutationError] = useState<string | null>(null)

  const toggleProvider = (p: string) => {
    setCollapsedProviders(prev => {
      const next = new Set(prev)
      if (next.has(p)) next.delete(p)
      else next.add(p)
      return next
    })
  }

  const { data, isLoading, isError, refetch } = useQuery({
    queryKey: ['models', provider],
    queryFn: () => modelsApi.getAll(provider === 'all' ? undefined : provider),
    refetchInterval: 60_000,
  })

  const invalidate = () => qc.invalidateQueries({ queryKey: ['models'] })

  const upsertMutation = useMutation({
    mutationFn: (form: ModelFormState) => modelsApi.upsert(formToPayload(form)),
    onSuccess: () => {
      invalidate()
      setShowCreate(false)
      setEditingModel(null)
      setMutationError(null)
    },
    onError: (e: Error) => setMutationError(e.message),
  })

  const deleteMutation = useMutation({
    mutationFn: ({ provider, model_id }: { provider: string; model_id: string }) =>
      modelsApi.remove(provider, model_id),
    onSuccess: () => { invalidate(); setDeletingModel(null) },
  })

  const models = (data?.data ?? []).filter(Boolean)
  const filtered = models.filter(m => {
    if (!search) return true
    const q = search.toLowerCase()
    return m.id.toLowerCase().includes(q) ||
      (m.pylos?.display_name?.toLowerCase().includes(q) ?? false)
  })

  const grouped = filtered.reduce((acc, m) => {
    if (!m) return acc
    const p = m.pylos?.provider ?? m.provider ?? 'unknown'
    if (!p) return acc
    if (!acc[p]) acc[p] = []
    acc[p].push(m)
    return acc
  }, {} as Record<string, typeof filtered>)

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between flex-wrap gap-3">
        <div>
          <h1 className="text-2xl font-bold text-white">Model Catalog</h1>
          <p className="text-sm text-zinc-400 mt-1">
            {filtered.length} models — pricing &amp; capabilities
          </p>
        </div>
        <button
          onClick={() => { setMutationError(null); setShowCreate(true) }}
          className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white text-sm rounded-lg transition-colors"
        >
          <Plus size={15} /> Add model
        </button>
      </div>

      {/* Filters */}
      <div className="flex gap-3 flex-wrap">
        <input
          type="text"
          placeholder="Search models..."
          value={search}
          onChange={e => setSearch(e.target.value)}
          className="flex-1 min-w-[200px] px-3 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
        />
        <div className="flex gap-1.5 flex-wrap">
          {PROVIDERS.map(p => (
            <button
              key={p}
              onClick={() => setProvider(p)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium capitalize transition-colors
                ${provider === p
                  ? 'bg-zinc-800 text-white'
                  : 'bg-zinc-900 text-zinc-500 border border-zinc-800 hover:text-zinc-300'}`}
            >
              {p}
            </button>
          ))}
        </div>
      </div>

      {/* Error state */}
      {isError && (
        <div className="flex items-center gap-3 bg-red-900/20 border border-red-800/50 rounded-xl p-4 text-red-300">
          <AlertCircle size={16} className="shrink-0" />
          <span className="text-sm">Failed to load models. </span>
          <button onClick={() => refetch()} className="text-sm underline hover:no-underline">Retry</button>
        </div>
      )}

      {isLoading ? (
        <div className="space-y-4">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="h-20 rounded-xl border border-zinc-800/50 bg-zinc-900/30 animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-10">
          {Object.entries(grouped).sort(([a], [b]) => a.localeCompare(b)).map(([prov, pModels]) => {
            const isCollapsed = collapsedProviders.has(prov)
            return (
              <div key={prov} className="space-y-3">
                <div 
                  className="flex items-center justify-between px-1 cursor-pointer group select-none"
                  onClick={() => toggleProvider(prov)}
                >
                  <div className="flex items-center gap-3">
                    <h2 className="text-sm font-bold text-zinc-200 capitalize flex items-center gap-2">
                      <span className={`w-1.5 h-4 rounded-full ${isCollapsed ? 'bg-zinc-600' : 'bg-emerald-500'}`} />
                      {prov}
                    </h2>
                    <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-500 border border-zinc-700/50">
                      {pModels.length} models
                    </span>
                  </div>
                  <div className={`text-zinc-500 group-hover:text-zinc-300 transition-transform duration-200 ${isCollapsed ? '-rotate-90' : ''}`}>
                    <ChevronDown size={16} />
                  </div>
                </div>

                {!isCollapsed && (
                  <div className="rounded-xl border border-zinc-800/50 bg-zinc-900/30 overflow-hidden">
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="bg-zinc-900/50">
                      <th className="px-6 py-4 text-[10px] font-bold text-zinc-500 uppercase tracking-widest w-[30%]">Model</th>
                      <th className="px-6 py-4 text-[10px] font-bold text-zinc-500 uppercase tracking-widest text-center">Context</th>
                      <th className="px-6 py-4 text-[10px] font-bold text-zinc-500 uppercase tracking-widest text-right">Input /1M</th>
                      <th className="px-6 py-4 text-[10px] font-bold text-zinc-500 uppercase tracking-widest text-right">Output /1M</th>
                      <th className="px-6 py-4 text-[10px] font-bold text-zinc-500 uppercase tracking-widest text-center">Capabilities</th>
                      <th className="px-6 py-4 w-20">Test</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-zinc-800/40">
                    {pModels.map(({ id, pylos }) => (
                      <Fragment key={id}>
                        <tr
                          className={`transition-colors cursor-pointer group
                            ${expandedId === id ? 'bg-zinc-800/20' : 'hover:bg-zinc-800/30'}
                            ${pylos?.enabled === false ? 'opacity-50 grayscale' : ''}`}
                          onClick={() => setExpandedId(expandedId === id ? null : id)}
                        >
                          <td className="px-6 py-4 flex items-center justify-between">
            <Link to={`/playground?model=${pylos?.provider}::${pylos?.model_id}`}>Test</Link>
                            <div className="flex flex-col">
                              <span className="font-semibold text-zinc-100 group-hover:text-white transition-colors">
                                {pylos?.display_name || pylos?.model_id || id}
                              </span>
                              <span className="text-[10px] text-zinc-500 font-mono mt-0.5">
                                {pylos?.model_id ?? id}
                              </span>
                            </div>
                          </td>
                          <td className="px-6 py-4 text-center">
                            <span className="px-2 py-1 rounded-md bg-zinc-800/50 text-zinc-300 text-xs font-medium border border-zinc-700/30">
                              {formatContext(pylos?.context_window ?? 0)}
                            </span>
                          </td>
                          <td className="px-6 py-4 text-right">
                            <span className={`text-xs font-mono font-semibold ${pylos?.input_price_per_1m_usd === 0 ? 'text-emerald-400' : 'text-blue-400'}`}>
                              {formatPrice(pylos?.input_price_per_1m_usd ?? 0)}
                            </span>
                          </td>
                          <td className="px-6 py-4 text-right">
                            <span className={`text-xs font-mono font-semibold ${pylos?.output_price_per_1m_usd === 0 ? 'text-emerald-400' : 'text-amber-400'}`}>
                              {formatPrice(pylos?.output_price_per_1m_usd ?? 0)}
                            </span>
                          </td>
                          <td className="px-6 py-4">
                            <div className="flex gap-1.5 justify-center flex-wrap">
                              <CapBadge ok={pylos?.supports_vision ?? false} label="Vision" />
                              <CapBadge ok={pylos?.supports_tools ?? true} label="Tools" />
                              <CapBadge ok={pylos?.supports_embeddings ?? false} label="Embed" />
                              <CapBadge ok={pylos?.supports_streaming ?? true} label="Stream" />
                            </div>
                          </td>
                          <td className="px-6 py-4">
                            <div className="flex items-center justify-end gap-2" onClick={e => e.stopPropagation()}>
                              {pylos && (
                                <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                  <button
                                    onClick={() => { setMutationError(null); setEditingModel(pylos) }}
                                    className="p-1.5 text-zinc-500 hover:text-emerald-400 hover:bg-emerald-400/10 rounded-lg transition-all"
                                  >
                                    <Pencil size={14} />
                                  </button>
                                  <button
                                    onClick={() => setDeletingModel({ provider: pylos.provider, model_id: pylos.model_id })}
                                    className="p-1.5 text-zinc-500 hover:text-red-400 hover:bg-red-400/10 rounded-lg transition-all"
                                  >
                                    <Trash2 size={14} />
                                  </button>
                                </div>
                              )}
                              <div className={`text-zinc-600 transition-transform duration-200 ${expandedId === id ? 'rotate-180' : ''}`}>
                                <ChevronDown size={16} />
                              </div>
                            </div>
                          </td>
                        </tr>
                        {expandedId === id && pylos && (
                          <tr className="bg-zinc-800/20 border-l-2 border-l-emerald-500">
                            <td colSpan={6} className="px-10 py-6">
                              <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
                                <div>
                                  <div className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest mb-2">Technical Details</div>
                                  <div className="space-y-1.5">
                                    <div className="flex justify-between text-xs">
                                      <span className="text-zinc-400">Max Output</span>
                                      <span className="text-zinc-200 font-medium">{formatContext(pylos.max_output_tokens)} tokens</span>
                                    </div>
                                    <div className="flex justify-between text-xs">
                                      <span className="text-zinc-400">Provider</span>
                                      <span className="text-zinc-200 font-medium capitalize">{pylos.provider}</span>
                                    </div>
                                  </div>
                                </div>
                                <div>
                                  <div className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest mb-2">Cost Estimation</div>
                                  <div className="space-y-1 text-xs">
                                    <div className="text-zinc-400">Avg. request cost</div>
                                    <div className="text-lg font-mono text-white">
                                      ${((pylos.input_price_per_1m_usd / 1000) + (pylos.output_price_per_1m_usd / 2000)).toFixed(4)}
                                    </div>
                                    <div className="text-[10px] text-zinc-500">(1K prompt + 500 completion)</div>
                                  </div>
                                </div>
                                <div className="md:col-span-2">
                                  <div className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest mb-2">Integration Snippet</div>
                                  <div className="bg-zinc-950/50 rounded-xl p-3 border border-zinc-800/50 font-mono text-[10px] text-emerald-300 overflow-x-auto">
                                    {`model: "${pylos.model_id}", // Managed by Pylos`}
                                  </div>
                                </div>
                              </div>
                            </td>
                          </tr>
                        )}
                      </Fragment>
                    ))}
                  </tbody>
                </table>
                  </div>
                )}
              </div>
            )
          })}

          {filtered.length === 0 && !isError && (
            <div className="flex flex-col items-center justify-center py-24 text-zinc-500 space-y-4">
              <div className="w-16 h-16 rounded-full bg-zinc-800/30 flex items-center justify-center">
                <Search size={24} className="text-zinc-600" />
              </div>
              <div className="text-sm">No models found matching your criteria</div>
              <button 
                onClick={() => {setSearch(''); setProvider('all')}}
                className="text-xs text-emerald-500 hover:underline"
              >
                Clear all filters
              </button>
            </div>
          )}
        </div>
      )}

      {/* Modals */}
      {showCreate && (
        <ModelModal
          initial={DEFAULT_FORM}
          isEdit={false}
          onClose={() => { setShowCreate(false); setMutationError(null) }}
          onSave={form => upsertMutation.mutate(form)}
          isSaving={upsertMutation.isPending}
          error={mutationError}
        />
      )}

      {/* Add Test link column header */}
      {editingModel && (
        <ModelModal
          initial={pylosToForm(editingModel)}
          isEdit={true}
          onClose={() => { setEditingModel(null); setMutationError(null) }}
          onSave={form => upsertMutation.mutate(form)}
          isSaving={upsertMutation.isPending}
          error={mutationError}
        />
      )}


      {editingModel && (
        <ModelModal
          initial={pylosToForm(editingModel)}
          isEdit={true}
          onClose={() => { setEditingModel(null); setMutationError(null) }}
          onSave={form => upsertMutation.mutate(form)}
          isSaving={upsertMutation.isPending}
          error={mutationError}
        />
      )}

      {deletingModel && (
        <DeleteConfirm
          modelId={`${deletingModel.provider}/${deletingModel.model_id}`}
          onClose={() => setDeletingModel(null)}
          onConfirm={() => deleteMutation.mutate(deletingModel)}
          isDeleting={deleteMutation.isPending}
        />
      )}
    </div>
  )
}
