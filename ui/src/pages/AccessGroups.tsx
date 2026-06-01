import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { accessGroupsApi, providersApi, modelsApi, type AccessGroup } from '../lib/api'
import {
  Plus, Pencil, Trash2, X, Check, RotateCcw, AlertTriangle, CheckCircle, XCircle, Shield,
} from 'lucide-react'

interface AgFormState { name: string; description: string; model_ids: string[]; provider_ids: string[]; is_active: boolean }
const DEFAULT_FORM: AgFormState = { name: '', description: '', model_ids: [], provider_ids: [], is_active: true }

function AgModal({ initial, isEdit, onClose, onSave, isSaving, error }: {
  initial: AgFormState; isEdit: boolean; onClose: () => void; onSave: (f: AgFormState) => void; isSaving: boolean; error: string | null
}) {
  const [form, setForm] = useState<AgFormState>(initial)
  const { data: providers } = useQuery({ queryKey: ['providers'], queryFn: providersApi.getAll })
  const { data: models } = useQuery({ queryKey: ['models', 'all'], queryFn: () => modelsApi.getAll() })

  const toggleProvider = (p: string) => setForm(f => ({ ...f, provider_ids: f.provider_ids.includes(p) ? f.provider_ids.filter(x => x !== p) : [...f.provider_ids, p] }))
  const toggleModel = (m: string) => setForm(f => ({ ...f, model_ids: f.model_ids.includes(m) ? f.model_ids.filter(x => x !== m) : [...f.model_ids, m] }))

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-zinc-900 border border-zinc-800 rounded-2xl shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between p-5 border-b border-zinc-800/50">
          <h2 className="text-lg font-semibold text-white">{isEdit ? 'Edit access group' : 'Create access group'}</h2>
          <button onClick={onClose} className="text-zinc-500 hover:text-white"><X size={18} /></button>
        </div>
        <div className="p-5 space-y-5">
          <div><label className="block text-xs text-zinc-400 mb-1.5">Name *</label>
            <input type="text" value={form.name} onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
              placeholder="My Access Group"
              className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50" /></div>
          <div><label className="block text-xs text-zinc-400 mb-1.5">Description</label>
            <input type="text" value={form.description} onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
              placeholder="Optional description"
              className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-emerald-500/50" /></div>
          <div><label className="block text-xs text-zinc-400 mb-1.5">Allowed Providers</label>
            <div className="flex flex-wrap gap-1.5 p-2 bg-zinc-950 border border-zinc-800 rounded-lg min-h-[42px]">
              {providers?.providers.map(p => {
                const sel = form.provider_ids.includes(p.name)
                return <button key={p.name} type="button" onClick={() => toggleProvider(p.name)}
                  className={`px-2.5 py-1 rounded-md text-xs font-medium border transition-all ${sel ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-zinc-900/40 text-zinc-400 border-zinc-800 hover:text-zinc-200'}`}>{p.name}</button>
              })}
              {!providers?.providers.length && <span className="text-xs text-zinc-500 p-1">No providers</span>}
            </div></div>
          <div><label className="block text-xs text-zinc-400 mb-1.5">Allowed Models</label>
            <div className="flex flex-wrap gap-1.5 p-2 bg-zinc-950 border border-zinc-800 rounded-lg min-h-[42px] max-h-32 overflow-y-auto">
              {models?.data.map(m => {
                const sel = form.model_ids.includes(m.pylos.model_id)
                return <button key={m.id} type="button" onClick={() => toggleModel(m.pylos.model_id)}
                  className={`px-2.5 py-1 rounded-md text-xs font-medium border transition-all ${sel ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-zinc-900/40 text-zinc-400 border-zinc-800 hover:text-zinc-200'}`}>{m.pylos.model_id}</button>
              })}
              {!models?.data.length && <span className="text-xs text-zinc-500 p-1">No models</span>}
            </div></div>
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
          <button onClick={() => onSave(form)} disabled={isSaving || !form.name.trim()}
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
        <div className="flex items-center gap-3 mb-4"><div className="w-9 h-9 rounded-full bg-red-500/15 flex items-center justify-center"><AlertTriangle size={16} className="text-red-400" /></div><div><div className="font-semibold text-white">Delete access group</div><div className="text-xs text-zinc-500">This action cannot be undone</div></div></div>
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

export default function AccessGroups() {
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [editing, setEditing] = useState<AccessGroup | null>(null)
  const [deleting, setDeleting] = useState<AccessGroup | null>(null)
  const [mutationError, setMutationError] = useState<string | null>(null)

  const { data, isLoading } = useQuery({ queryKey: ['access-groups'], queryFn: accessGroupsApi.getAll })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['access-groups'] })

  const createMut = useMutation({ mutationFn: (f: AgFormState) => accessGroupsApi.create(f), onSuccess: () => { invalidate(); setShowCreate(false); setMutationError(null) }, onError: (e: Error) => setMutationError(e.message) })
  const updateMut = useMutation({ mutationFn: ({ id, f }: { id: string; f: AgFormState }) => accessGroupsApi.update(id, f), onSuccess: () => { invalidate(); setEditing(null); setMutationError(null) }, onError: (e: Error) => setMutationError(e.message) })
  const deleteMut = useMutation({ mutationFn: (id: string) => accessGroupsApi.remove(id), onSuccess: () => { invalidate(); setDeleting(null) } })

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      <div className="flex items-center justify-between">
        <div><h1 className="text-2xl font-bold text-white">Access Groups</h1><p className="text-sm text-zinc-400 mt-1">{data?.total ?? '—'} configured</p></div>
        <button onClick={() => { setMutationError(null); setShowCreate(true) }}
          className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white text-sm rounded-lg"><Plus size={15} />Create group</button>
      </div>
      <div className="rounded-xl border border-zinc-800/50 bg-zinc-900/30 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="border-b border-zinc-800/50">
            <tr>{['Name', 'Description', 'Providers', 'Models', 'Status', ''].map(h => <th key={h} className="text-left px-5 py-3.5 text-xs text-zinc-500 uppercase tracking-wide font-medium">{h}</th>)}</tr>
          </thead>
          <tbody>
            {isLoading ? Array.from({ length: 3 }).map((_, i) => (
              <tr key={i} className="border-b border-zinc-800/30">{Array.from({ length: 6 }).map((_, j) => <td key={j} className="px-5 py-3.5"><div className="h-3 bg-zinc-800 rounded animate-pulse w-24" /></td>)}</tr>
            )) : data?.access_groups.map(ag => (
              <tr key={ag.id} className="border-b border-zinc-800/30 transition-colors group hover:bg-zinc-800/30">
                <td className="px-5 py-3.5"><div className="flex items-center gap-2"><Shield size={14} className="text-emerald-400 shrink-0" /><span className="font-medium text-white">{ag.name}</span></div></td>
                <td className="px-5 py-3.5 text-zinc-400 text-xs">{ag.description || '—'}</td>
                <td className="px-5 py-3.5"><div className="flex flex-wrap gap-1">{ag.provider_ids.map(p => <span key={p} className="px-2 py-0.5 rounded-full text-xs bg-zinc-800 text-zinc-300 border border-zinc-700/50">{p}</span>)}</div></td>
                <td className="px-5 py-3.5 text-xs text-zinc-400">{ag.model_ids.length > 0 ? ag.model_ids.slice(0, 3).join(', ') + (ag.model_ids.length > 3 ? '…' : '') : '—'}</td>
                <td className="px-5 py-3.5">{ag.is_active ? <span className="flex items-center gap-1.5 text-emerald-400 text-xs"><CheckCircle size={12} /> Active</span> : <span className="flex items-center gap-1.5 text-zinc-500 text-xs"><XCircle size={12} /> Inactive</span>}</td>
                <td className="px-5 py-3.5"><div className="flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-all">
                  <button onClick={() => { setMutationError(null); setEditing(ag) }} className="p-1.5 text-zinc-500 hover:text-emerald-400 hover:bg-emerald-400/10 rounded-lg"><Pencil size={13} /></button>
                  <button onClick={() => setDeleting(ag)} className="p-1.5 text-zinc-500 hover:text-red-400 hover:bg-red-400/10 rounded-lg"><Trash2 size={13} /></button>
                </div></td>
              </tr>
            ))}
          </tbody>
        </table>
        {!isLoading && !data?.access_groups.length && <div className="text-center py-16 text-zinc-600">No access groups configured</div>}
      </div>
      {showCreate && <AgModal initial={DEFAULT_FORM} isEdit={false} onClose={() => { setShowCreate(false); setMutationError(null) }} onSave={f => createMut.mutate(f)} isSaving={createMut.isPending} error={mutationError} />}
      {editing && <AgModal initial={{ name: editing.name, description: editing.description || '', model_ids: editing.model_ids, provider_ids: editing.provider_ids, is_active: editing.is_active }} isEdit={true} onClose={() => { setEditing(null); setMutationError(null) }} onSave={f => updateMut.mutate({ id: editing.id, f })} isSaving={updateMut.isPending} error={mutationError} />}
      {deleting && <DeleteConfirmModal name={deleting.name} onClose={() => setDeleting(null)} onConfirm={() => deleteMut.mutate(deleting.id)} isDeleting={deleteMut.isPending} />}
    </div>
  )
}
