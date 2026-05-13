import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { virtualKeysApi, type VirtualKey, type VkBudgetResponse } from '../lib/api'
import {
  KeyRound, CheckCircle, XCircle, Shield, TrendingUp,
  ChevronDown, ChevronUp, Plus, Pencil, Trash2, X, Check,
  AlertTriangle, RotateCcw, Copy,
} from 'lucide-react'

// ─── Budget / Rate panel ──────────────────────────────────────────────────────

function BudgetBar({ used, max }: { used: number; max: number }) {
  const pct = max > 0 ? Math.min((used / max) * 100, 100) : 0
  const color = pct > 90
    ? '#f43f5e'
    : pct > 70
      ? '#f59e0b'
      : '#10b981'
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
                <div
                  className="h-full rounded-full transition-all"
                  style={{
                    width: rl.max_value > 0
                      ? `${Math.min((rl.current_value / rl.max_value) * 100, 100)}%`
                      : '0%',
                    background: rl.max_value > 0 && rl.current_value / rl.max_value > 0.9
                      ? '#f43f5e' : '#3b82f6',
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
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
        <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-sm mx-4 p-6">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-9 h-9 rounded-full bg-emerald-500/15 flex items-center justify-center">
              <Check size={16} className="text-emerald-400" />
            </div>
            <div>
              <div className="font-semibold text-white">Virtual key created</div>
              <div className="text-xs text-zinc-500">Copy it now — it won't be shown again</div>
            </div>
          </div>
          <div className="bg-zinc-950/50 border border-zinc-800/50 rounded-lg p-3 flex items-center gap-2 mb-5">
            <span className="font-mono text-sm text-emerald-300 flex-1 break-all">{createdKey}</span>
            <button onClick={copyKey} className="shrink-0 text-zinc-400 hover:text-white transition-colors">
              {copied ? <Check size={14} className="text-emerald-400" /> : <Copy size={14} />}
            </button>
          </div>
          <button
            onClick={onClose}
            className="w-full py-2 bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white text-sm rounded-lg transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-zinc-800/50">
          <h2 className="text-lg font-semibold text-white">
            {isEdit ? 'Edit virtual key' : 'Create virtual key'}
          </h2>
          <button onClick={onClose} className="text-zinc-500 hover:text-white transition-colors">
            <X size={18} />
          </button>
        </div>

        <div className="p-5 space-y-5">
          {/* Name */}
          <div>
            <label className="block text-xs text-zinc-400 mb-1.5">Name *</label>
            <input
              type="text"
              value={form.name}
              onChange={e => setField('name', e.target.value)}
              placeholder="My Project"
              className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-xs text-zinc-400 mb-1.5">Description (optional)</label>
            <input
              type="text"
              value={form.description}
              onChange={e => setField('description', e.target.value)}
              placeholder="Production key for service X"
              className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200
                focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
            />
          </div>

          {/* Active */}
          <div className="flex items-center gap-3">
            <button
              onClick={() => setField('is_active', !form.is_active)}
              className={`relative w-10 h-5 rounded-full transition-colors ${form.is_active ? 'bg-emerald-600' : 'bg-zinc-700'}`}
            >
              <span
                className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all
                  ${form.is_active ? 'left-5' : 'left-0.5'}`}
              />
            </button>
            <span className="text-sm text-zinc-300">Active</span>
          </div>

          {/* Provider configs */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-xs text-zinc-400">Allowed providers</label>
              <button
                onClick={addPc}
                className="text-xs text-emerald-400 hover:text-emerald-300 flex items-center gap-1"
              >
                <Plus size={12} /> Add
              </button>
            </div>
            {form.provider_configs.length === 0 ? (
              <p className="text-xs text-zinc-600 italic">No restrictions — all configured providers allowed</p>
            ) : (
              <div className="space-y-2">
                {form.provider_configs.map((pc, i) => (
                  <div key={i} className="bg-zinc-950/50 border border-zinc-800/50 rounded-lg p-3 grid grid-cols-3 gap-2">
                    <div>
                      <label className="block text-xs text-zinc-500 mb-1">Provider</label>
                      <input
                        value={pc.provider}
                        onChange={e => setPc(i, 'provider', e.target.value)}
                        placeholder="openai"
                        className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-xs text-zinc-200
                          focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
                      />
                    </div>
                    <div>
                      <label className="block text-xs text-zinc-500 mb-1">Models</label>
                      <input
                        value={pc.models}
                        onChange={e => setPc(i, 'models', e.target.value)}
                        placeholder="*, gpt-4o"
                        className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-xs text-zinc-200
                          font-mono focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
                      />
                    </div>
                    <div className="flex items-end gap-2">
                      <div className="flex-1">
                        <label className="block text-xs text-zinc-500 mb-1">Weight</label>
                        <input
                          type="number"
                          value={pc.weight}
                          onChange={e => setPc(i, 'weight', Number(e.target.value))}
                          min={0.1} max={10} step={0.1}
                          className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-xs text-zinc-200
                            focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20"
                        />
                      </div>
                      <button
                        onClick={() => removePc(i)}
                        className="mb-0.5 p-1.5 text-zinc-600 hover:text-red-400 transition-colors"
                      >
                        <X size={13} />
                      </button>
                    </div>
                  </div>
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
        <div className="flex justify-end gap-3 px-5 py-4 border-t border-zinc-800/50">
          <button onClick={onClose} className="px-4 py-2 text-sm text-zinc-400 hover:text-white transition-colors">
            Cancel
          </button>
          <button
            onClick={() => onSave(form)}
            disabled={isSaving || !form.name.trim()}
            className="px-4 py-2 text-sm bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] disabled:opacity-50
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-sm mx-4 p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-9 h-9 rounded-full bg-red-500/15 flex items-center justify-center">
            <AlertTriangle size={16} className="text-red-400" />
          </div>
          <div>
            <div className="font-semibold text-white">Delete virtual key</div>
            <div className="text-xs text-zinc-500">This action cannot be undone</div>
          </div>
        </div>
        <p className="text-sm text-zinc-400 mb-5">
          Delete <span className="text-white font-medium">{name}</span>?
          All requests using this key will be rejected immediately.
        </p>
        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="px-4 py-2 text-sm text-zinc-400 hover:text-white">Cancel</button>
          <button
            onClick={onConfirm}
            disabled={isDeleting}
            className="px-4 py-2 text-sm bg-red-600 hover:bg-red-500 disabled:opacity-50 text-white rounded-lg
              flex items-center gap-2 transition-colors active:scale-[0.98]"
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

  const { data, isLoading } = useQuery({
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
          <p className="text-sm text-zinc-400 mt-1">
            {data?.total ?? '—'} configured
          </p>
        </div>
        <button
          onClick={() => { setMutationError(null); setShowCreate(true) }}
          className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white text-sm
            rounded-lg transition-colors"
        >
          <Plus size={15} />
          Create key
        </button>
      </div>

      {/* Table */}
      <div className="rounded-xl border border-zinc-800/50 bg-zinc-900/30 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="border-b border-zinc-800/50">
            <tr>
              {['Name', 'Value', 'Status', 'Providers', 'Models', ''].map(h => (
                <th key={h} className="text-left px-5 py-3.5 text-xs text-zinc-500 uppercase tracking-wide font-medium">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {isLoading
              ? Array.from({ length: 3 }).map((_, i) => (
                  <tr key={i} className="border-b border-zinc-800/30">
                    {Array.from({ length: 6 }).map((_, j) => (
                      <td key={j} className="px-5 py-3.5">
                        <div className="h-3 bg-zinc-800 rounded animate-pulse w-24" />
                      </td>
                    ))}
                  </tr>
                ))
              : data?.virtual_keys.map(vk => (
                  <>
                    <tr
                      key={vk.id}
                      className={`border-b border-zinc-800/30 transition-colors cursor-pointer group
                        ${expandedId === vk.id ? 'bg-emerald-500/5 border-l-2 border-l-emerald-500' : 'hover:bg-zinc-800/30'}`}
                      onClick={() => setExpandedId(expandedId === vk.id ? null : vk.id)}
                    >
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-2">
                          <KeyRound size={14} className="text-emerald-400 shrink-0" />
                          <div>
                            <div className="font-medium text-white">{vk.name}</div>
                            {vk.description && (
                              <div className="text-xs text-zinc-500">{vk.description}</div>
                            )}
                          </div>
                        </div>
                      </td>
                      <td className="px-5 py-3.5 font-mono text-xs text-zinc-400">
                        {vk.value}
                      </td>
                      <td className="px-5 py-3.5">
                        {vk.is_active ? (
                          <span className="flex items-center gap-1.5 text-emerald-400 text-xs">
                            <CheckCircle size={12} /> Active
                          </span>
                        ) : (
                          <span className="flex items-center gap-1.5 text-zinc-500 text-xs">
                            <XCircle size={12} /> Inactive
                          </span>
                        )}
                      </td>
                      <td className="px-5 py-3.5">
                        <div className="flex flex-wrap gap-1">
                          {vk.provider_configs.map(pc => (
                            <span key={pc.provider}
                              className="px-2 py-0.5 rounded-full text-xs bg-zinc-800 text-zinc-300 border border-zinc-700/50">
                              {pc.provider}
                            </span>
                          ))}
                          {vk.provider_configs.length === 0 && (
                            <span className="text-xs text-zinc-600 italic">all</span>
                          )}
                        </div>
                      </td>
                      <td className="px-5 py-3.5">
                        <div className="flex items-center gap-1.5 text-zinc-400 text-xs">
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
                            className="opacity-0 group-hover:opacity-100 p-1.5 text-zinc-500 hover:text-emerald-400
                              hover:bg-emerald-400/10 rounded-lg transition-all"
                            title="Edit"
                          >
                            <Pencil size={13} />
                          </button>
                          <button
                            onClick={() => setDeletingVk(vk)}
                            className="opacity-0 group-hover:opacity-100 p-1.5 text-zinc-500 hover:text-red-400
                              hover:bg-red-400/10 rounded-lg transition-all"
                            title="Delete"
                          >
                            <Trash2 size={13} />
                          </button>
                          <span className="text-zinc-600 ml-1">
                            {expandedId === vk.id ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                          </span>
                        </div>
                      </td>
                    </tr>
                    {expandedId === vk.id && (
                      <tr key={`${vk.id}-budget`} className="bg-zinc-800/20">
                        <td colSpan={6} className="px-8 py-4">
                          <div className="flex items-center gap-2 text-xs text-zinc-400 mb-3">
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
          <div className="text-center py-16 text-zinc-600">
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
