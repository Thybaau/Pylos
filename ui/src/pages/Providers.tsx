import { useQuery } from '@tanstack/react-query'
import { providersApi } from '../lib/api'
import { providerColor } from '../lib/utils'
import { Server, Key, Globe, RotateCcw } from 'lucide-react'

export default function Providers() {
  const { data, isLoading } = useQuery({
    queryKey: ['providers'],
    queryFn: providersApi.getAll,
    refetchInterval: 30_000,
  })

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-white">Providers</h1>
        <p className="text-sm text-gray-400 mt-1">
          {data?.total ?? '—'} configured
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
        {isLoading
          ? Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="rounded-xl border border-gray-800 bg-gray-900 p-5 animate-pulse h-40" />
            ))
          : data?.providers.map(p => (
              <ProviderCard key={p.name} provider={p} />
            ))
        }

        {!isLoading && !data?.providers.length && (
          <div className="col-span-full text-center py-16 text-gray-600">
            No providers configured
          </div>
        )}
      </div>
    </div>
  )
}

function ProviderCard({ provider }: { provider: import('../lib/api').Provider }) {
  const color = providerColor(provider.name)

  return (
    <div className="rounded-xl border border-gray-800 bg-gray-900 p-5 hover:border-gray-700 transition-colors">
      {/* Header */}
      <div className="flex items-center gap-3 mb-4">
        <div
          className="w-9 h-9 rounded-lg flex items-center justify-center text-white font-bold text-sm"
          style={{ background: color + '20', color }}
        >
          <Server size={16} />
        </div>
        <div>
          <div className="font-semibold text-white capitalize">{provider.name}</div>
          <div className="text-xs text-gray-500">
            {provider.keys_count} key{provider.keys_count !== 1 ? 's' : ''}
          </div>
        </div>
        <div className="ml-auto w-2 h-2 rounded-full bg-green-400" title="Active" />
      </div>

      {/* Network info */}
      <div className="space-y-2 text-xs">
        {provider.network.base_url && (
          <div className="flex items-center gap-2 text-gray-400">
            <Globe size={12} />
            <span className="truncate font-mono">{provider.network.base_url}</span>
          </div>
        )}
        <div className="flex items-center gap-4 text-gray-500">
          <span className="flex items-center gap-1">
            <RotateCcw size={11} />
            {provider.network.max_retries} retries
          </span>
          <span>{provider.network.timeout_secs}s timeout</span>
        </div>
      </div>

      {/* Keys preview */}
      {provider.keys.length > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-800 space-y-1.5">
          {provider.keys.slice(0, 3).map((k, i) => (
            <div key={i} className="flex items-center gap-2 text-xs">
              <Key size={11} className="text-gray-600" />
              <span className="text-gray-300">{k.name}</span>
              <span className="ml-auto text-gray-600 font-mono">{k.value}</span>
            </div>
          ))}
          {provider.keys.length > 3 && (
            <div className="text-xs text-gray-600 pl-5">
              +{provider.keys.length - 3} more
            </div>
          )}
        </div>
      )}
    </div>
  )
}
