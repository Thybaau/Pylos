import { Search } from 'lucide-react'

export default function SearchTools() {
  return (
    <div className="flex flex-col h-full bg-zinc-950 p-8">
      <div className="flex items-center gap-3 mb-6">
        <div className="p-2 bg-zinc-900 rounded-lg border border-zinc-800">
          <Search className="w-5 h-5 text-emerald-400" />
        </div>
        <h1 className="text-2xl font-semibold text-white">Search Tools</h1>
      </div>
      <div className="flex-1 bg-zinc-900/50 border border-zinc-800/50 rounded-xl p-8 flex flex-col items-center justify-center text-zinc-400">
        <Search className="w-12 h-12 mb-4 opacity-50" />
        <h2 className="text-lg font-medium text-zinc-300 mb-2">Search Tools Configuration</h2>
        <p className="text-sm max-w-md text-center">
          Configure search engines and retrieval tools for the AI agents.
        </p>
      </div>
    </div>
  )
}
