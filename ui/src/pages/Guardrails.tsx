import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Shield, Save, ShieldAlert, KeyRound, AlertTriangle, FileText } from 'lucide-react'
import { configApi } from '../lib/api'

export default function Guardrails() {
  const queryClient = useQueryClient()
  
  const { data: configData, isLoading } = useQuery({
    queryKey: ['config'],
    queryFn: configApi.get,
  })

  const [enabled, setEnabled] = useState(false)
  const [maskPii, setMaskPii] = useState(true)
  const [maskSecrets, setMaskSecrets] = useState(false)
  const [preventPromptInjection, setPreventPromptInjection] = useState(false)
  const [blockedKeywords, setBlockedKeywords] = useState<string>('')

  // Sync state when config loads
  useEffect(() => {
    if (configData && configData.plugins) {
      const guardrailsPlugin = configData.plugins.find((p: any) => p.name === 'guardrails')
      if (guardrailsPlugin) {
        setEnabled(guardrailsPlugin.enabled)
        setMaskPii(guardrailsPlugin.config?.mask_pii ?? true)
        setMaskSecrets(guardrailsPlugin.config?.mask_secrets ?? false)
        setPreventPromptInjection(guardrailsPlugin.config?.prevent_prompt_injection ?? false)
        
        const keywords = guardrailsPlugin.config?.blocked_keywords || []
        setBlockedKeywords(keywords.join(', '))
      }
    }
  }, [configData])

  const mutation = useMutation({
    mutationFn: configApi.updateGuardrails,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['config'] })
      alert('Guardrails configuration updated successfully')
    },
    onError: (error: any) => {
      alert(`Failed to update guardrails: ${error.message}`)
    }
  })

  const handleSave = () => {
    const keywordsArray = blockedKeywords
      .split(',')
      .map(k => k.trim())
      .filter(k => k.length > 0)

    mutation.mutate({
      enabled,
      mask_pii: maskPii,
      mask_secrets: maskSecrets,
      prevent_prompt_injection: preventPromptInjection,
      blocked_keywords: keywordsArray
    })
  }

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-zinc-500 animate-pulse">Loading configuration...</div>
      </div>
    )
  }

  return (
    <div className="flex-1 flex flex-col h-full bg-zinc-950 overflow-y-auto">
      <header className="px-6 py-8 md:py-10 max-w-4xl mx-auto w-full">
        <div className="flex items-center gap-3 mb-2">
          <div className="p-2 bg-emerald-500/10 rounded-lg">
            <Shield className="text-emerald-400" size={28} />
          </div>
          <h1 className="text-3xl font-bold text-white tracking-tight">Guardrails</h1>
        </div>
        <p className="text-zinc-400 text-lg">
          Configure security, masking, and content filtering for LLM interactions.
        </p>
      </header>

      <div className="px-6 pb-12 max-w-4xl mx-auto w-full space-y-6">
        
        {/* Main Toggle */}
        <div className="bg-zinc-900 border border-zinc-800/50 rounded-2xl p-6 shadow-xl relative overflow-hidden">
          <div className="absolute top-0 right-0 p-4 opacity-5 pointer-events-none">
            <Shield size={120} />
          </div>
          <div className="relative flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold text-white mb-1">Enable Guardrails</h2>
              <p className="text-zinc-400 text-sm">
                Globally enable or disable all guardrail features for the gateway.
              </p>
            </div>
            <button
              onClick={() => setEnabled(!enabled)}
              className={`relative inline-flex h-7 w-12 items-center rounded-full transition-colors duration-300 ease-in-out focus:outline-none ${
                enabled ? 'bg-emerald-500' : 'bg-zinc-700'
              }`}
            >
              <span
                className={`inline-block h-5 w-5 transform rounded-full bg-white transition duration-300 ease-in-out ${
                  enabled ? 'translate-x-6' : 'translate-x-1'
                }`}
              />
            </button>
          </div>
        </div>

        {/* Configuration sections - disabled if guardrails are not enabled */}
        <div className={`space-y-6 transition-opacity duration-300 ${enabled ? 'opacity-100' : 'opacity-50 pointer-events-none'}`}>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* PII Masking */}
            <div className="bg-zinc-900 border border-zinc-800/50 rounded-2xl p-6">
              <div className="flex items-center gap-3 mb-4">
                <FileText className="text-blue-400" size={20} />
                <h3 className="text-lg font-medium text-white">PII Masking</h3>
              </div>
              <p className="text-zinc-400 text-sm mb-6">
                Automatically mask Personally Identifiable Information (emails, phone numbers, credit cards) before sending to providers.
              </p>
              <label className="flex items-center cursor-pointer gap-3">
                <input
                  type="checkbox"
                  checked={maskPii}
                  onChange={(e) => setMaskPii(e.target.checked)}
                  className="w-5 h-5 rounded border-zinc-700 bg-zinc-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-zinc-900"
                />
                <span className="text-zinc-300 font-medium">Mask PII</span>
              </label>
            </div>

            {/* Secrets Masking */}
            <div className="bg-zinc-900 border border-zinc-800/50 rounded-2xl p-6">
              <div className="flex items-center gap-3 mb-4">
                <KeyRound className="text-purple-400" size={20} />
                <h3 className="text-lg font-medium text-white">Secrets Masking</h3>
              </div>
              <p className="text-zinc-400 text-sm mb-6">
                Detect and mask API keys, JWT tokens, and private keys in the prompts.
              </p>
              <label className="flex items-center cursor-pointer gap-3">
                <input
                  type="checkbox"
                  checked={maskSecrets}
                  onChange={(e) => setMaskSecrets(e.target.checked)}
                  className="w-5 h-5 rounded border-zinc-700 bg-zinc-800 text-purple-500 focus:ring-purple-500 focus:ring-offset-zinc-900"
                />
                <span className="text-zinc-300 font-medium">Mask Secrets</span>
              </label>
            </div>
          </div>

          {/* Prompt Injection */}
          <div className="bg-zinc-900 border border-zinc-800/50 rounded-2xl p-6">
            <div className="flex items-center gap-3 mb-4">
              <AlertTriangle className="text-amber-400" size={20} />
              <h3 className="text-lg font-medium text-white">Prompt Injection Prevention</h3>
            </div>
            <p className="text-zinc-400 text-sm mb-6">
              Block requests that contain common prompt injection patterns (e.g. "ignore previous instructions", "reveal your system prompt").
            </p>
            <label className="flex items-center cursor-pointer gap-3">
              <input
                type="checkbox"
                checked={preventPromptInjection}
                onChange={(e) => setPreventPromptInjection(e.target.checked)}
                className="w-5 h-5 rounded border-zinc-700 bg-zinc-800 text-amber-500 focus:ring-amber-500 focus:ring-offset-zinc-900"
              />
              <span className="text-zinc-300 font-medium">Prevent Prompt Injections</span>
            </label>
          </div>

          {/* Blocked Keywords */}
          <div className="bg-zinc-900 border border-zinc-800/50 rounded-2xl p-6">
            <div className="flex items-center gap-3 mb-4">
              <ShieldAlert className="text-rose-400" size={20} />
              <h3 className="text-lg font-medium text-white">Blocked Keywords</h3>
            </div>
            <p className="text-zinc-400 text-sm mb-4">
              Comma-separated list of keywords. If any of these are found in a user's prompt, the request will be blocked.
            </p>
            <textarea
              value={blockedKeywords}
              onChange={(e) => setBlockedKeywords(e.target.value)}
              placeholder="e.g. hate, violence, confidential_project_x"
              className="w-full bg-zinc-950 border border-zinc-800 rounded-xl p-4 text-zinc-200 placeholder-zinc-600 focus:outline-none focus:ring-2 focus:ring-rose-500/50 focus:border-transparent min-h-[120px] resize-y"
            />
          </div>

        </div>

        {/* Save Button */}
        <div className="flex justify-end pt-4">
          <button
            onClick={handleSave}
            disabled={mutation.isPending}
            className="flex items-center gap-2 px-6 py-3 bg-white text-zinc-950 font-medium rounded-xl hover:bg-zinc-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {mutation.isPending ? (
              <div className="w-5 h-5 border-2 border-zinc-950 border-t-transparent rounded-full animate-spin" />
            ) : (
              <Save size={20} />
            )}
            Save Configuration
          </button>
        </div>

      </div>
    </div>
  )
}
