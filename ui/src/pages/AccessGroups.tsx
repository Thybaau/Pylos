import { Layers } from 'lucide-react'

export default function AccessGroups() {
  return (
    <div className="flex flex-col h-full bg-zinc-950 p-8">
      <div className="flex items-center gap-3 mb-6">
        <div className="p-2 bg-zinc-900 rounded-lg border border-zinc-800">
          <Layers className="w-5 h-5 text-emerald-400" />
        </div>
        <h1 className="text-2xl font-semibold text-white">Access Groups</h1>
      </div>
      <div className="flex-1 bg-zinc-900/50 border border-zinc-800/50 rounded-xl p-8 flex flex-col items-center justify-center text-zinc-400">
        <Layers className="w-12 h-12 mb-4 opacity-50" />
        <h2 className="text-lg font-medium text-zinc-300 mb-2">Access Groups Configuration</h2>
        <p className="text-sm max-w-md text-center">
          Define access control groups to manage permissions across multiple resources and models.
        </p>
      </div>
    </div>
  )
}
