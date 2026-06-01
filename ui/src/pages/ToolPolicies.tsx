import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toolPoliciesApi, providersApi, modelsApi, type ToolPolicy } from '../lib/api'
import {
  ShieldCheck, Plus, Pencil, Trash2, X, Check, RotateCcw, AlertTriangle, CheckCircle, XCircle,
} from 'lucide-react'

interface TpFormState { name: string; description: string; tool_type: string; allowed_models: string[]; allowed_providers: string[]; max_tokens_per_call: string; max_calls_per_minute: string; is_active: boolean }
const DEFAULT_FORM: TpFormState = { name: '', description: '', tool_type: 'search', allowed_models: [], allowed_providers: [], max_tokens_per_call: '', max_calls_per_minute: '', is_active: true }

function TpModal({ initial, isEdit, onClose, onSave, isSaving, error }: {
  initial: TpFormState; isEdit: boolean; onClose: () => void; onSave: (f: TpFormState) => void; isSaving: boolean; error: string | null
}) {
  const [form, setForm] = useState<TpFormState>(initial)
  const { data: providers } = useQuery({ queryKey: ['providers'], queryFn: providersApi.getAll })
  const { data: allModels } = useQuery({ queryKey: ['models', 'all'], queryFn: () => modelsApi.getAll() })

  const toggleProvider = (p: string) => setForm(f => ({ ...f, allowed_providers: f.allowed_providers.includes(p) ? f.allowed_providers.filter(x => x !== p) : [...f.allowed_providers, p] }))
  const toggleModel = (m: string) => setForm(f => ({ ...f, allowed_models: f.allowed_models.includes(m) ? f.allowed_models.filter(x => x !== m) : [...f.allowed_models, m] }))

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between p-5 border-b border-zinc-800/50">
          <h2 className="text-lg font-semibold text-white">{isEdit ? 'Edit tool policy' : 'Create tool policy'}</h2>
          <button onClick={onClose} className="text-zinc-500 hover:text-white"><X size={18} /></button>
        </div>
        <div className="p-5 space-y-5">
          <div className="grid grid-cols-2 gap-3">
            <div><label className="block text-xs text-zinc-400 mb-1.5">Name *</label>
              <input type="text" value={form.name} onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
                placeholder="Search Policy" className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50" /></div>
            <div><label className="block text-xs text-zinc-400 mb-1.5">Tool Type *</label>
              <select value={form.tool_type} onChange={e => setForm(f => ({ ...f, tool_type: e.target.value }))}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50">
                <option value="search">Search</option>
                <option value="vector_store">Vector Store</option>
                <option value="code_execution">Code Execution</option>
                <option value="web_browsing">Web Browsing</option>
                <option value="custom">Custom</option>
              </select></div>
          </div>
          <div><label className="block text-xs text-zinc-400 mb-1.5">Description</label>
            <input type="text" value={form.description} onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
              placeholder="Optional" className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50" /></div>
          <div><label className="block text-xs text-zinc-400 mb-1.5">Allowed Providers</label>
            <div className="flex flex-wrap gap-1.5 p-2 bg-zinc-950 border border-zinc-800 rounded-lg min-h-[42px]">
              {providers?.providers.map(p => {
                const sel = form.allowed_providers.includes(p.name)
                return <button key={p.name} type="button" onClick={() => toggleProvider(p.name)}
                  className={`px-2.5 py-1 rounded-md text-xs font-medium border transition-all ${sel ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-zinc-900/40 text-zinc-400 border-zinc-800 hover:text-zinc-200'}`}>{p.name}</button>
              })}
            </div></div>
          <div><label className="block text-xs text-zinc-400 mb-1.5">Allowed Models</label>
            <div className="flex flex-wrap gap-1.5 p-2 bg-zinc-950 border border-zinc-800 rounded-lg min-h-[42px] max-h-32 overflow-y-auto">
              {allModels?.data.map(m => {
                const sel = form.allowed_models.includes(m.pylos.model_id)
                return <button key={m.id} type="button" onClick={() => toggleModel(m.pylos.model_id)}
                  className={`px-2.5 py-1 rounded-md text-xs font-medium border transition-all ${sel ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-zinc-900/40 text-zinc-400 border-zinc-800 hover:text-zinc-200'}`}>{m.pylos.model_id}</button>
              })}
            </div></div>
          <div className="grid grid-cols-2 gap-3">
            <div><label className="block text-xs text-zinc-400 mb-1.5">Max tokens per call</label>
              <input type="number" value={form.max_tokens_per_call} onChange={e => setForm(f => ({ ...f, max_tokens_per_call: e.target.value }))}
                placeholder="Unlimited" className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50" /></div>
            <div><label className="block text-xs text-zinc-400 mb-1.5">Max calls/minute</label>
              <input type="number" value={form.max_calls_per_minute} onChange={e => setForm(f => ({ ...f, max_calls_per_minute: e.target.value }))}
                placeholder="Unlimited" className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50" /></div>
          </div>
          <div className="flex items-center gap-3">
            <button onClick={() => setForm(f => ({ ...f, is_active: !f.is_active }))}
              className={`relative w-10 h-5 rounded-full transition-colors ${form.is_active ? 'bg-emerald-600' : 'bg-zinc-700'}`}>
              <span className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all ${form.is_active ? 'left-5' : 'left-0.5'}`} />
            </button>
            <span className="text-sm text-zinc-300">Active</span>
          </div>
          {error && <div className="flex items-center gap-2 text-red-400 text-xs bg-red-900/20 border border-red-800/50 rounded-lg px-3 py-2"><AlertTriangle size={13} />{error}</div>}
        </div>
        <div className="flex justify-end gap-3 px-5 py-4 border-t border-zinc-800/50">
          <button onClick={onClose} className="px-4 py-2 text-sm text-zinc-400 hover:text-white">Cancel</button>
          <button onClick={() => onSave(form)} disabled={isSaving || !form.name.trim() || !form.tool_type}
            className="px-4 py-2 text-sm bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white rounded-lg flex items-center gap-2">
            {isSaving ? <RotateCcw size={14} className="animate-spin" /> : <Check size={14} />}
            {isEdit ? 'Update' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  )
}

function DeleteConfirmModal({ name, onClose, onConfirm, isDeleting }: {
  name: string; onClose: () => void; onConfirm: () => void; isDeleting: boolean
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-sm mx-4 p-6">
        <div className="flex items-center gap-3 mb-4"><div className="w-9 h-9 rounded-full bg-red-500/15 flex items-center justify-center"><AlertTriangle size={16} className="text-red-400" /></div><div><div className="font-semibold text-white">Delete tool policy</div><div className="text-xs text-zinc-500">This action cannot be undone</div></div></div>
        <p className="text-sm text-zinc-400 mb-5">Delete <span className="text-white font-medium">{name}</span>?</p>
        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="px-4 py-2 text-sm text-zinc-400 hover:text-white">Cancel</button>
          <button onClick={onConfirm} disabled={isDeleting}
            className="px-4 py-2 text-sm bg-red-600 hover:bg-red-500 disabled:opacity-50 text-white rounded-lg flex items-center gap-2">
            {isDeleting ? <RotateCcw size={13} className="animate-spin" /> : <Trash2 size={13} />}Delete
          </button>
        </div>
      </div>
    </div>
  )
}

export default function ToolPolicies() {
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [editing, setEditing] = useState<ToolPolicy | null>(null)
  const [deleting, setDeleting] = useState<ToolPolicy | null>(null)
  const [mutationError, setMutationError] = useState<string | null>(null)

  const { data, isLoading } = useQuery({ queryKey: ['tool-policies'], queryFn: toolPoliciesApi.getAll })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['tool-policies'] })

  function parseForm(f: TpFormState) {
    return {
      name: f.name,
      description: f.description || null,
      tool_type: f.tool_type,
      allowed_models: f.allowed_models,
      allowed_providers: f.allowed_providers,
      max_tokens_per_call: f.max_tokens_per_call ? parseInt(f.max_tokens_per_call) : null,
      max_calls_per_minute: f.max_calls_per_minute ? parseInt(f.max_calls_per_minute) : null,
      is_active: f.is_active,
    }
  }

  const createMut = useMutation({
    mutationFn: (f: TpFormState) => toolPoliciesApi.create(parseForm(f)),
    onSuccess: () => { invalidate(); setShowCreate(false); setMutationError(null) },
    onError: (e: Error) => setMutationError(e.message),
  })
  const updateMut = useMutation({
    mutationFn: ({ id, f }: { id: string; f: TpFormState }) => toolPoliciesApi.update(id, parseForm(f)),
    onSuccess: () => { invalidate(); setEditing(null); setMutationError(null) },
    onError: (e: Error) => setMutationError(e.message),
  })
  const deleteMut = useMutation({ mutationFn: (id: string) => toolPoliciesApi.remove(id), onSuccess: () => { invalidate(); setDeleting(null) } })

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between">
        <div><h1 className="text-2xl font-bold text-white">Tool Policies</h1><p className="text-sm text-zinc-400 mt-1">{data?.total ?? '—'} configured</p></div>
        <button onClick={() => { setMutationError(null); setShowCreate(true) }}
          className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white text-sm rounded-lg"><Plus size={15} />Create policy</button>
      </div>
      <div className="rounded-xl border border-zinc-800/50 bg-zinc-900/30 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="border-b border-zinc-800/50">
            <tr>{['Name', 'Tool Type', 'Providers', 'Models', 'Limits', 'Status', ''].map(h => <th key={h} className="text-left px-5 py-3.5 text-xs text-zinc-500 uppercase tracking-wide font-medium">{h}</th>)}</tr>
          </thead>
          <tbody>
            {isLoading ? Array.from({ length: 3 }).map((_, i) => (
              <tr key={i} className="border-b border-zinc-800/30">{Array.from({ length: 7 }).map((_, j) => <td key={j} className="px-5 py-3.5"><div className="h-3 bg-zinc-800 rounded animate-pulse w-24" /></td>)}</tr>
            )) : data?.tool_policies.map(tp => (
              <tr key={tp.id} className="border-b border-zinc-800/30 transition-colors group hover:bg-zinc-800/30">
                <td className="px-5 py-3.5"><div className="flex items-center gap-2"><ShieldCheck size={14} className="text-emerald-400 shrink-0" /><span className="font-medium text-white">{tp.name}</span></div></td>
                <td className="px-5 py-3.5"><span className="capitalize text-xs px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-300 border border-zinc-700/50">{tp.tool_type}</span></td>
                <td className="px-5 py-3.5 text-xs text-zinc-400">{tp.allowed_providers.join(', ') || 'all'}</td>
                <td className="px-5 py-3.5 text-xs text-zinc-400">{tp.allowed_models.length > 0 ? tp.allowed_models.slice(0, 2).join(', ') + (tp.allowed_models.length > 2 ? '…' : '') : 'all'}</td>
                <td className="px-5 py-3.5 text-xs text-zinc-400">
                  {tp.max_tokens_per_call ? `${tp.max_tokens_per_call} tok` : '—'} / {tp.max_calls_per_minute ? `${tp.max_calls_per_minute}/min` : '—'}
                </td>
                <td className="px-5 py-3.5">{tp.is_active ? <span className="flex items-center gap-1.5 text-emerald-400 text-xs"><CheckCircle size={12} /> Active</span> : <span className="flex items-center gap-1.5 text-zinc-500 text-xs"><XCircle size={12} /> Inactive</span>}</td>
                <td className="px-5 py-3.5"><div className="flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-all">
                  <button onClick={() => { setMutationError(null); setEditing(tp) }} className="p-1.5 text-zinc-500 hover:text-emerald-400 hover:bg-emerald-400/10 rounded-lg"><Pencil size={13} /></button>
                  <button onClick={() => setDeleting(tp)} className="p-1.5 text-zinc-500 hover:text-red-400 hover:bg-red-400/10 rounded-lg"><Trash2 size={13} /></button>
                </div></td>
              </tr>
            ))}
          </tbody>
        </table>
        {!isLoading && !data?.tool_policies.length && <div className="text-center py-16 text-zinc-600">No tool policies configured</div>}
      </div>
      {showCreate && <TpModal initial={DEFAULT_FORM} isEdit={false} onClose={() => { setShowCreate(false); setMutationError(null) }} onSave={f => createMut.mutate(f)} isSaving={createMut.isPending} error={mutationError} />}
      {editing && <TpModal initial={{ name: editing.name, description: editing.description || '', tool_type: editing.tool_type, allowed_models: editing.allowed_models, allowed_providers: editing.allowed_providers, max_tokens_per_call: editing.max_tokens_per_call?.toString() || '', max_calls_per_minute: editing.max_calls_per_minute?.toString() || '', is_active: editing.is_active }} isEdit={true} onClose={() => { setEditing(null); setMutationError(null) }} onSave={f => updateMut.mutate({ id: editing.id, f })} isSaving={updateMut.isPending} error={mutationError} />}
      {deleting && <DeleteConfirmModal name={deleting.name} onClose={() => setDeleting(null)} onConfirm={() => deleteMut.mutate(deleting.id)} isDeleting={deleteMut.isPending} />}
    </div>
  )
}
