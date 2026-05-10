import { useQuery } from '@tanstack/react-query'
import { virtualKeysApi } from '../lib/api'
import { KeyRound, CheckCircle, XCircle, Shield } from 'lucide-react'

export default function VirtualKeys() {
  const { data, isLoading } = useQuery({
    queryKey: ['virtual-keys'],
    queryFn: virtualKeysApi.getAll,
    refetchInterval: 30_000,
  })

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-white">Virtual Keys</h1>
        <p className="text-sm text-gray-400 mt-1">
          {data?.total ?? '—'} configured
        </p>
      </div>

      <div className="rounded-xl border border-gray-800 bg-gray-900 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="border-b border-gray-800">
            <tr>
              {['Name', 'Value', 'Status', 'Providers', 'Models'].map(h => (
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
                    {Array.from({ length: 5 }).map((_, j) => (
                      <td key={j} className="px-5 py-3.5">
                        <div className="h-3 bg-gray-800 rounded animate-pulse w-24" />
                      </td>
                    ))}
                  </tr>
                ))
              : data?.virtual_keys.map(vk => (
                  <tr key={vk.id} className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors">
                    <td className="px-5 py-3.5">
                      <div className="flex items-center gap-2">
                        <KeyRound size={14} className="text-blue-400" />
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
                          <CheckCircle size={12} />
                          Active
                        </span>
                      ) : (
                        <span className="flex items-center gap-1.5 text-gray-500 text-xs">
                          <XCircle size={12} />
                          Inactive
                        </span>
                      )}
                    </td>
                    <td className="px-5 py-3.5">
                      <div className="flex flex-wrap gap-1">
                        {vk.provider_configs.map(pc => (
                          <span key={pc.provider}
                            className="px-2 py-0.5 rounded-full text-xs bg-gray-800 text-gray-300 border border-gray-700">
                            {pc.provider}
                          </span>
                        ))}
                      </div>
                    </td>
                    <td className="px-5 py-3.5">
                      <div className="flex items-center gap-1.5 text-gray-400 text-xs">
                        <Shield size={11} />
                        {vk.provider_configs.some(pc => pc.allowed_models.includes('*'))
                          ? 'All models'
                          : vk.provider_configs.flatMap(pc => pc.allowed_models).slice(0, 2).join(', ')
                        }
                      </div>
                    </td>
                  </tr>
                ))
            }
          </tbody>
        </table>

        {!isLoading && !data?.virtual_keys.length && (
          <div className="text-center py-16 text-gray-600">
            No virtual keys configured
          </div>
        )}
      </div>
    </div>
  )
}
