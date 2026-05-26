import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { virtualKeysApi, providersApi, modelsApi, type VirtualKey, type VkBudgetResponse } from '../lib/api'
import { providerColor } from '../lib/utils'
import {
  KeyRound, CheckCircle, XCircle, Shield, TrendingUp,
  ChevronDown, ChevronUp, Plus, Pencil, Trash2, X, Check,
  AlertTriangle, RotateCcw, Copy, RotateCw,
} from 'lucide-react'
import { ProviderIcon } from '../components/ProviderIcon'

// ─── Budget / Rate panel (unchanged) ─────────────────────────────────────────

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

  if (isLoading) return <div className="text-xs text-gray-500">Loading…</div>
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
                      : '0%',
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

// ─── Types ────────────────────────────────────────────────────────────────────

interface ProviderCfgEntry { provider: string; models: string; weight: number }

interface VkFormState {
  name: string
  description: string
  is_active: boolean
  provider_configs: ProviderCfgEntry[]
}

const DEFAULT_FORM: VkFormState = {
  name: '',
  description: '',
  is_active: true,
  provider_configs: [],
}

function vkToForm(vk: VirtualKey): VkFormState {
  return {
    name: vk.name,
    description: vk.description ?? '',
    is_active: vk.is_active,
    provider_configs: vk.provider_configs.map(pc => ({
      provider: pc.provider,
      models: pc.allowed_models.join(', '),
      weight: pc.weight,
    })),
  }
}

function formToPayload(form: VkFormState) {
  return {
    name: form.name,
    description: form.description || null,
    is_active: form.is_active,
    provider_configs: form.provider_configs.map(pc => ({
      provider: pc.provider,
      allowed_models: pc.models.split(',').map(s => s.trim()).filter(Boolean),
      weight: pc.weight,
    })),
  }
}

// ─── Governance Sub-components ──────────────────────────────────────────────

function ProviderSelector({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const { data } = useQuery({ queryKey: ['providers'], queryFn: providersApi.getAll })
  const providers = data?.providers ?? []

  return (
    <div className="relative group">
      <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 group-focus-within:text-blue-400 flex items-center">
        <ProviderIcon name={value} size={14} />
      </div>
      <select
        value={value}
        onChange={e => onChange(e.target.value)}
        className="w-full bg-gray-900 border border-gray-700 rounded-lg pl-9 pr-3 py-2 text-sm text-gray-200
          appearance-none focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20"
      >
        <option value="" disabled>Select provider…</option>
        {providers.map(p => (
          <option key={p.name} value={p.name}>{p.name.charAt(0).toUpperCase() + p.name.slice(1)}</option>
        ))}
      </select>
      <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-600">
        <ChevronDown size={14} />
      </div>
    </div>
  )
}

function ModelSelector({ provider, value, onChange }: { provider: string; value: string; onChange: (v: string) => void }) {
  const { data: modelsData } = useQuery({
    queryKey: ['models', provider],
    queryFn: () => modelsApi.getAll(provider),
    enabled: !!provider && provider !== '*',
  })

  const models = modelsData?.data ?? []
  const selectedModels = value.split(',').map(s => s.trim()).filter(Boolean)

  const toggleModel = (m: string) => {
    let next: string[]
    if (m === '*') {
      next = ['*']
    } else {
      const currentWithoutWildcard = selectedModels.filter(s => s !== '*')
      if (currentWithoutWildcard.includes(m)) {
        next = currentWithoutWildcard.filter(s => s !== m)
      } else {
        next = [...currentWithoutWildcard, m]
      }
    }
    onChange(next.join(', '))
  }

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-1.5 p-2 bg-gray-900 border border-gray-700 rounded-lg min-h-[42px]">
        {selectedModels.map(m => (
          <span key={m} className="flex items-center gap-1 px-2 py-0.5 bg-blue-500/10 text-blue-300 border border-blue-500/20 rounded text-[10px] font-medium">
            {m}
            <button onClick={() => toggleModel(m)} className="hover:text-white"><X size={10} /></button>
          </span>
        ))}
        {selectedModels.length === 0 && <span className="text-xs text-gray-600 px-1 py-0.5">Pick models…</span>}
      </div>

      {provider && (
        <div className="max-h-32 overflow-y-auto p-1 bg-gray-900/50 border border-gray-800 rounded-lg grid grid-cols-2 gap-1 custom-scrollbar">
          <button
            onClick={() => toggleModel('*')}
            className={`text-left px-2 py-1.5 rounded text-[10px] transition-colors ${
              selectedModels.includes('*') ? 'bg-blue-600/20 text-blue-400' : 'text-gray-500 hover:bg-gray-800'
            }`}
          >
            All Models (*)
          </button>
          {models.map(m => (
            <button
              key={m.id}
              onClick={() => toggleModel(m.pylos.model_id)}
              className={`text-left px-2 py-1.5 rounded text-[10px] truncate transition-colors ${
                selectedModels.includes(m.pylos.model_id) ? 'bg-blue-600/20 text-blue-400' : 'text-gray-400 hover:bg-gray-800'
              }`}
              title={m.pylos.model_id}
            >
              {m.pylos.model_id}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function ProviderConfigItem({
  pc,
  index,
  onUpdate,
  onRemove,
}: {
  pc: ProviderCfgEntry
  index: number
  onUpdate: (i: number, field: keyof ProviderCfgEntry, value: string | number) => void
  onRemove: (i: number) => void
}) {
  const color = providerColor(pc.provider)

  return (
    <div className="bg-gray-900/40 border border-gray-800 rounded-xl overflow-hidden transition-all hover:border-gray-700/50">
      <div className="px-4 py-2 bg-gray-800/30 border-b border-gray-800 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: color }} />
          <span className="text-[10px] font-bold text-gray-500 uppercase tracking-widest">
            Rule #{index + 1}
          </span>
        </div>
        <button
          onClick={() => onRemove(index)}
          className="p-1 text-gray-600 hover:text-red-400 transition-colors"
        >
          <Trash2 size={12} />
        </button>
      </div>

      <div className="p-4 space-y-4">
        <div className="grid grid-cols-12 gap-4">
          <div className="col-span-7 space-y-3">
            <div>
              <label className="block text-[10px] font-medium text-gray-500 uppercase mb-1.5 ml-1">Provider</label>
              <ProviderSelector value={pc.provider} onChange={v => onUpdate(index, 'provider', v)} />
            </div>
            <div>
              <label className="block text-[10px] font-medium text-gray-500 uppercase mb-1.5 ml-1">Allowed Models</label>
              <ModelSelector
                provider={pc.provider}
                value={pc.models}
                onChange={v => onUpdate(index, 'models', v)}
              />
            </div>
          </div>

          <div className="col-span-5">
            <label className="block text-[10px] font-medium text-gray-500 uppercase mb-1.5 ml-1">Routing Weight</label>
            <div className="bg-gray-900 border border-gray-700 rounded-lg p-3 space-y-3">
              <input
                type="range"
                min="0"
                max="10"
                step="0.1"
                value={pc.weight}
                onChange={e => onUpdate(index, 'weight', parseFloat(e.target.value))}
                className="w-full h-1 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-blue-500"
              />
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-gray-500">Low</span>
                <div className="bg-blue-500/10 text-blue-400 px-2 py-0.5 rounded text-xs font-mono font-bold">
                  {pc.weight.toFixed(1)}
                </div>
                <span className="text-[10px] text-gray-500">High</span>
              </div>
              <p className="text-[9px] text-gray-600 leading-tight">
                Higher weight increases the probability of choosing this provider when multiple match.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// ─── VkModal ──────────────────────────────────────────────────────────────────

function VkModal({
  initial,
  isEdit,
  onClose,
  onSave,
  isSaving,
  error,
  createdKey,
}: {
  initial: VkFormState
  isEdit: boolean
  onClose: () => void
  onSave: (form: VkFormState) => void
  isSaving: boolean
  error: string | null
  createdKey?: string
}) {
  const [form, setForm] = useState<VkFormState>(initial)
  const [copied, setCopied] = useState(false)

  const setField = <K extends keyof VkFormState>(k: K, v: VkFormState[K]) =>
    setForm(f => ({ ...f, [k]: v }))

  const setPc = (i: number, field: keyof ProviderCfgEntry, value: string | number) =>
    setForm(f => {
      const pcs = [...f.provider_configs]
      pcs[i] = { ...pcs[i], [field]: value }
      return { ...f, provider_configs: pcs }
    })

  const addPc = () =>
    setForm(f => ({
      ...f,
      provider_configs: [...f.provider_configs, { provider: '', models: '*', weight: 1.0 }],
    }))

  const removePc = (i: number) =>
    setForm(f => ({ ...f, provider_configs: f.provider_configs.filter((_, idx) => idx !== i) }))

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  // If key was just created — show it first
  if (createdKey) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
        <div className="bg-gray-900 border border-gray-700 rounded-2xl shadow-2xl w-full max-w-sm mx-4 p-6">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-9 h-9 rounded-full bg-green-500/15 flex items-center justify-center">
              <Check size={16} className="text-green-400" />
            </div>
            <div>
              <div className="font-semibold text-white">Virtual key created</div>
              <div className="text-xs text-gray-500">Copy it now — it won't be shown again</div>
            </div>
          </div>
          <div className="bg-gray-800 rounded-lg p-3 flex items-center gap-2 mb-5">
            <span className="font-mono text-sm text-green-300 flex-1 break-all">{createdKey}</span>
            <button onClick={copyKey} className="shrink-0 text-gray-400 hover:text-white transition-colors">
              {copied ? <Check size={14} className="text-green-400" /> : <Copy size={14} />}
            </button>
          </div>
          <button
            onClick={onClose}
            className="w-full py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded-lg transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="bg-gray-900 border border-gray-700 rounded-2xl shadow-2xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-gray-800">
          <h2 className="text-lg font-semibold text-white">
            {isEdit ? 'Edit virtual key' : 'Create virtual key'}
          </h2>
          <button onClick={onClose} className="text-gray-500 hover:text-white transition-colors">
            <X size={18} />
          </button>
        </div>

        <div className="p-5 space-y-5">
          {/* Name */}
          <div>
            <label className="block text-xs text-gray-400 mb-1.5">Name *</label>
            <input
              type="text"
              value={form.name}
              onChange={e => setField('name', e.target.value)}
              placeholder="My Project"
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm text-gray-200
                focus:outline-none focus:border-blue-500"
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-xs text-gray-400 mb-1.5">Description (optional)</label>
            <input
              type="text"
              value={form.description}
              onChange={e => setField('description', e.target.value)}
              placeholder="Production key for service X"
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm text-gray-200
                focus:outline-none focus:border-blue-500"
            />
          </div>

          {/* Active */}
          <div className="flex items-center gap-3">
            <button
              onClick={() => setField('is_active', !form.is_active)}
              className={`relative w-10 h-5 rounded-full transition-colors ${form.is_active ? 'bg-blue-600' : 'bg-gray-700'}`}
            >
              <span
                className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all
                  ${form.is_active ? 'left-5' : 'left-0.5'}`}
              />
            </button>
            <span className="text-sm text-gray-300">Active</span>
          </div>

          {/* Provider configs */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <label className="text-xs font-semibold text-gray-300 uppercase tracking-wider">Governance Settings</label>
              <button
                onClick={addPc}
                className="text-xs bg-blue-600/10 hover:bg-blue-600/20 text-blue-400 border border-blue-500/30 px-2 py-1 rounded flex items-center gap-1 transition-all"
              >
                <Plus size={12} /> Add Rule
              </button>
            </div>
            {form.provider_configs.length === 0 ? (
              <div className="bg-gray-800/40 border border-dashed border-gray-700 rounded-xl p-8 text-center">
                <Shield size={24} className="text-gray-700 mx-auto mb-2" />
                <p className="text-xs text-gray-500">No restrictions — all configured providers allowed</p>
                <button onClick={addPc} className="mt-4 text-xs text-blue-400 hover:underline">Add your first governance rule</button>
              </div>
            ) : (
              <div className="space-y-4">
                {form.provider_configs.map((pc, i) => (
                  <ProviderConfigItem
                    key={i}
                    pc={pc}
                    index={i}
                    onUpdate={setPc}
                    onRemove={removePc}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Error */}
          {error && (
            <div className="flex items-center gap-2 text-red-400 text-xs bg-red-900/20 border border-red-800/50 rounded-lg px-3 py-2">
              <AlertTriangle size={13} />
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 px-5 py-4 border-t border-gray-800">
          <button onClick={onClose} className="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">
            Cancel
          </button>
          <button
            onClick={() => onSave(form)}
            disabled={isSaving || !form.name.trim()}
            className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50
              disabled:cursor-not-allowed text-white rounded-lg flex items-center gap-2 transition-colors"
          >
            {isSaving ? <RotateCcw size={14} className="animate-spin" /> : <Check size={14} />}
            {isEdit ? 'Update' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  )
}

// ─── DeleteConfirmModal ───────────────────────────────────────────────────────

function DeleteConfirmModal({
  name,
  onClose,
  onConfirm,
  isDeleting,
}: {
  name: string
  onClose: () => void
  onConfirm: () => void
  isDeleting: boolean
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="bg-gray-900 border border-gray-700 rounded-2xl shadow-2xl w-full max-w-sm mx-4 p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-9 h-9 rounded-full bg-red-500/15 flex items-center justify-center">
            <AlertTriangle size={16} className="text-red-400" />
          </div>
          <div>
            <div className="font-semibold text-white">Delete virtual key</div>
            <div className="text-xs text-gray-500">This action cannot be undone</div>
          </div>
        </div>
        <p className="text-sm text-gray-400 mb-5">
          Delete <span className="text-white font-medium">{name}</span>?
          All requests using this key will be rejected immediately.
        </p>
        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="px-4 py-2 text-sm text-gray-400 hover:text-white">Cancel</button>
          <button
            onClick={onConfirm}
            disabled={isDeleting}
            className="px-4 py-2 text-sm bg-red-600 hover:bg-red-500 disabled:opacity-50 text-white rounded-lg
              flex items-center gap-2 transition-colors"
          >
            {isDeleting ? <RotateCcw size={13} className="animate-spin" /> : <Trash2 size={13} />}
            Delete
          </button>
        </div>
      </div>
    </div>
  )
}

// ─── Main page ────────────────────────────────────────────────────────────────

export default function VirtualKeys() {
  const qc = useQueryClient()
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [editingVk, setEditingVk] = useState<VirtualKey | null>(null)
  const [deletingVk, setDeletingVk] = useState<VirtualKey | null>(null)
  const [mutationError, setMutationError] = useState<string | null>(null)
  const [newKeyValue, setNewKeyValue] = useState<string | null>(null)

  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ['virtual-keys'],
    queryFn: virtualKeysApi.getAll,
    refetchInterval: 30_000,
  })

  const invalidate = () => qc.invalidateQueries({ queryKey: ['virtual-keys'] })

  const createMutation = useMutation({
    mutationFn: (form: VkFormState) => virtualKeysApi.create(formToPayload(form)),
    onSuccess: (result) => {
      invalidate()
      setShowCreate(false)
      setMutationError(null)
      if (result?.value) setNewKeyValue(result.value)
    },
    onError: (e: Error) => setMutationError(e.message),
  })

  const updateMutation = useMutation({
    mutationFn: ({ id, form }: { id: string; form: VkFormState }) =>
      virtualKeysApi.update(id, formToPayload(form)),
    onSuccess: () => { invalidate(); setEditingVk(null); setMutationError(null) },
    onError: (e: Error) => setMutationError(e.message),
  })

  const deleteMutation = useMutation({
    mutationFn: (id: string) => virtualKeysApi.remove(id),
    onSuccess: () => { invalidate(); setDeletingVk(null) },
  })

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Virtual Keys</h1>
          <p className="text-sm text-gray-400 mt-1">
            {data?.total ?? '—'} configured
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => refetch()}
            disabled={isFetching}
            className="flex items-center justify-center p-2 text-gray-400 hover:text-white bg-gray-900 hover:bg-gray-800 border border-gray-800 disabled:opacity-50 rounded-lg transition-colors"
            title="Refresh keys"
          >
            <RotateCw size={15} className={isFetching ? 'animate-spin' : ''} />
          </button>
          <button
            onClick={() => { setMutationError(null); setShowCreate(true) }}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm
              rounded-lg transition-colors"
          >
            <Plus size={15} />
            Create key
          </button>
        </div>
      </div>

      {/* Table */}
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
                      className={`border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors cursor-pointer group
                        ${expandedId === vk.id ? 'bg-gray-800/20' : ''}`}
                      onClick={() => setExpandedId(expandedId === vk.id ? null : vk.id)}
                    >
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-2">
                          <KeyRound size={14} className="text-blue-400 shrink-0" />
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
                              className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs bg-gray-800 text-gray-300 border border-gray-700">
                              <ProviderIcon name={pc.provider} size={10} />
                              <span className="capitalize">{pc.provider}</span>
                            </span>
                          ))}
                          {vk.provider_configs.length === 0 && (
                            <span className="text-xs text-gray-600 italic">all</span>
                          )}
                        </div>
                      </td>
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-1.5 text-gray-400 text-xs">
                          <Shield size={11} />
                          {vk.provider_configs.length === 0
                            ? 'All models'
                            : vk.provider_configs.some(pc => pc.allowed_models.includes('*'))
                              ? 'All models'
                              : vk.provider_configs.flatMap(pc => pc.allowed_models).slice(0, 2).join(', ')
                          }
                        </div>
                      </td>
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-1.5" onClick={e => e.stopPropagation()}>
                          <button
                            onClick={() => { setMutationError(null); setEditingVk(vk) }}
                            className="opacity-0 group-hover:opacity-100 p-1.5 text-gray-500 hover:text-blue-400
                              hover:bg-blue-400/10 rounded-lg transition-all"
                            title="Edit"
                          >
                            <Pencil size={13} />
                          </button>
                          <button
                            onClick={() => setDeletingVk(vk)}
                            className="opacity-0 group-hover:opacity-100 p-1.5 text-gray-500 hover:text-red-400
                              hover:bg-red-400/10 rounded-lg transition-all"
                            title="Delete"
                          >
                            <Trash2 size={13} />
                          </button>
                          <span className="text-gray-600 ml-1">
                            {expandedId === vk.id ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                          </span>
                        </div>
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
            No virtual keys configured — create one to enable governance
          </div>
        )}
      </div>

      {/* Modals */}
      {showCreate && (
        <VkModal
          initial={DEFAULT_FORM}
          isEdit={false}
          onClose={() => { setShowCreate(false); setMutationError(null) }}
          onSave={form => createMutation.mutate(form)}
          isSaving={createMutation.isPending}
          error={mutationError}
        />
      )}

      {editingVk && (
        <VkModal
          initial={vkToForm(editingVk)}
          isEdit={true}
          onClose={() => { setEditingVk(null); setMutationError(null) }}
          onSave={form => updateMutation.mutate({ id: editingVk.id, form })}
          isSaving={updateMutation.isPending}
          error={mutationError}
        />
      )}

      {deletingVk && (
        <DeleteConfirmModal
          name={deletingVk.name}
          onClose={() => setDeletingVk(null)}
          onConfirm={() => deleteMutation.mutate(deletingVk.id)}
          isDeleting={deleteMutation.isPending}
        />
      )}

      {/* Created key display */}
      {newKeyValue && (
        <VkModal
          initial={DEFAULT_FORM}
          isEdit={false}
          onClose={() => setNewKeyValue(null)}
          onSave={() => {}}
          isSaving={false}
          error={null}
          createdKey={newKeyValue}
        />
      )}
    </div>
  )
}
