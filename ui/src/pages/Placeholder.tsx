import { AlertCircle, ArrowLeft } from 'lucide-react'
import { useNavigate } from 'react-router-dom'

interface PlaceholderProps {
  title: string
  description?: string
}

export default function Placeholder({ title, description }: PlaceholderProps) {
  const navigate = useNavigate()

  return (
    <div className="p-6 h-full flex flex-col justify-center items-center text-center">
      <div className="max-w-md bg-zinc-900/50 border border-zinc-800/80 rounded-2xl p-8 shadow-xl backdrop-blur-md">
        <div className="w-12 h-12 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center text-indigo-400 mx-auto mb-4">
          <AlertCircle size={24} />
        </div>
        <h1 className="text-xl font-bold text-white mb-2">{title}</h1>
        <p className="text-sm text-zinc-400 mb-6">
          {description || "This page or feature is currently under active development. Check back soon for updates!"}
        </p>
        <button
          onClick={() => navigate(-1)}
          className="inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-zinc-850 bg-zinc-800 hover:bg-zinc-750 text-white text-xs font-semibold transition-all"
        >
          <ArrowLeft size={14} />
          Go Back
        </button>
      </div>
    </div>
  )
}
