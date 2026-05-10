import { useState, useRef, useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api } from '../lib/api'
import { formatLatency, formatCost, providerColor } from '../lib/utils'
import {
  Send, StopCircle, Trash2, ChevronDown,
  Zap, Clock, Hash, Coins, CheckCircle, XCircle,
} from 'lucide-react'

// ─── Types ────────────────────────────────────────────────────────────────────

interface Model {
  id: string
  provider: string
  details?: { family?: string; parameter_size?: string }
}

interface Message {
  role: 'system' | 'user' | 'assistant'
  content: string
}

interface RunResult {
  model: string
  provider: string
  content: string
  latency_ms: number
  tokens: { prompt: number; completion: number; total: number }
  cost_usd: number
  finish_reason: string
  error?: string
  streaming?: boolean
}

// ─── API helpers ──────────────────────────────────────────────────────────────

async function fetchModels(): Promise<Model[]> {
  const r = await api.get('/v1/models')
  return r.data.data ?? []
}

// ─── Composants ──────────────────────────────────────────────────────────────

function ModelSelect({
  models, value, onChange, disabled,
}: {
  models: Model[]
  value: string
  onChange: (v: string) => void
  disabled?: boolean
}) {
  const grouped: Record<string, Model[]> = {}
  for (const m of models) {
    if (!grouped[m.provider]) grouped[m.provider] = []
    grouped[m.provider].push(m)
  }

  return (
    <div className="relative">
      <select
        value={value}
        onChange={e => onChange(e.target.value)}
        disabled={disabled}
        className="w-full appearance-none bg-gray-800 border border-gray-700 rounded-lg
          px-3 py-2 text-sm text-gray-100 pr-8
          focus:outline-none focus:ring-2 focus:ring-blue-500
          disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <option value="">— Select a model —</option>
        {Object.entries(grouped).map(([provider, ms]) => (
          <optgroup key={provider} label={`▸ ${provider.toUpperCase()}`}>
            {ms.map(m => (
              <option key={`${provider}/${m.id}`} value={`${provider}::${m.id}`}>
                {m.id}
                {m.details?.parameter_size ? ` (${m.details.parameter_size})` : ''}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
      <ChevronDown size={14} className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none" />
    </div>
  )
}

function ChatMessage({ msg, isStreaming }: { msg: Message; isStreaming?: boolean }) {
  const isUser = msg.role === 'user'
  const isSystem = msg.role === 'system'

  if (isSystem) {
    return (
      <div className="rounded-lg border border-dashed border-gray-700 bg-gray-800/40 px-4 py-2 text-xs text-gray-400">
        <span className="font-semibold text-gray-500 mr-2">SYSTEM</span>
        {msg.content}
      </div>
    )
  }

  return (
    <div className={`flex gap-3 ${isUser ? 'flex-row-reverse' : 'flex-row'}`}>
      <div className={`w-7 h-7 rounded-full flex items-center justify-center flex-shrink-0 text-xs font-bold
        ${isUser ? 'bg-blue-600 text-white' : 'bg-gray-700 text-gray-300'}`}>
        {isUser ? 'U' : 'AI'}
      </div>
      <div className={`max-w-[80%] rounded-2xl px-4 py-3 text-sm leading-relaxed
        ${isUser
          ? 'bg-blue-600 text-white rounded-tr-sm'
          : 'bg-gray-800 text-gray-100 rounded-tl-sm border border-gray-700'
        }`}>
        <pre className="whitespace-pre-wrap font-sans break-words">
          {msg.content}
          {isStreaming && <span className="inline-block w-1.5 h-4 bg-blue-400 ml-0.5 animate-pulse" />}
        </pre>
      </div>
    </div>
  )
}

function ResultBadge({ result }: { result: RunResult }) {
  const color = providerColor(result.provider)
  const hasError = !!result.error

  return (
    <div className={`rounded-xl border p-4 space-y-3
      ${hasError ? 'border-red-800/50 bg-red-900/10' : 'border-gray-800 bg-gray-900'}`}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full" style={{ background: color }} />
          <span className="text-xs font-semibold text-gray-300">{result.provider}</span>
          <span className="text-xs text-gray-500 font-mono">{result.model}</span>
        </div>
        {hasError
          ? <XCircle size={14} className="text-red-400" />
          : <CheckCircle size={14} className="text-green-400" />
        }
      </div>

      {/* Metrics row */}
      {!hasError && (
        <div className="grid grid-cols-4 gap-2 text-xs">
          <Metric icon={<Clock size={10} />} label="Latency" value={formatLatency(result.latency_ms)} />
          <Metric icon={<Hash size={10} />} label="Tokens" value={result.tokens.total.toString()} />
          <Metric icon={<Coins size={10} />} label="Cost" value={formatCost(result.cost_usd)} />
          <Metric icon={<Zap size={10} />} label="Finish" value={result.finish_reason} />
        </div>
      )}

      {/* Error */}
      {hasError && (
        <div className="text-xs text-red-300 font-mono bg-red-950/30 rounded p-2 break-all">
          {result.error}
        </div>
      )}
    </div>
  )
}

function Metric({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="bg-gray-800/60 rounded-lg px-2 py-1.5 text-center">
      <div className="flex items-center justify-center gap-1 text-gray-500 mb-0.5">
        {icon}
        <span>{label}</span>
      </div>
      <div className="font-semibold text-gray-200">{value}</div>
    </div>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function Playground() {
  const [selectedModel, setSelectedModel] = useState('')
  const [systemPrompt, setSystemPrompt] = useState('You are a helpful assistant.')
  const [input, setInput] = useState('')
  const [messages, setMessages] = useState<Message[]>([])
  const [streaming, setStreaming] = useState(true)
  const [temperature, setTemperature] = useState(0.7)
  const [maxTokens, setMaxTokens] = useState(512)
  const [isRunning, setIsRunning] = useState(false)
  const [lastResult, setLastResult] = useState<RunResult | null>(null)
  const [streamingContent, setStreamingContent] = useState('')
  const abortRef = useRef<AbortController | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)

  const { data: modelsData, isLoading: modelsLoading } = useQuery({
    queryKey: ['models'],
    queryFn: fetchModels,
    staleTime: 60_000,
  })

  const models = modelsData ?? []

  // Auto-sélectionne llama3.1:8b si disponible
  useEffect(() => {
    if (models.length && !selectedModel) {
      const ollama = models.find(m => m.provider === 'ollama' && m.id === 'llama3.1:8b')
        ?? models.find(m => m.provider === 'ollama')
        ?? models[0]
      if (ollama) setSelectedModel(`${ollama.provider}::${ollama.id}`)
    }
  }, [models, selectedModel])

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, streamingContent])

  function parseSelected() {
    const [provider, ...rest] = selectedModel.split('::')
    return { provider, model: rest.join('::') }
  }

  async function run() {
    if (!input.trim() || !selectedModel || isRunning) return

    const { provider, model } = parseSelected()
    const userMsg: Message = { role: 'user', content: input.trim() }
    const newMessages = [...messages, userMsg]
    setMessages(newMessages)
    setInput('')
    setIsRunning(true)
    setStreamingContent('')
    setLastResult(null)

    const start = performance.now()
    abortRef.current = new AbortController()

    const payload = {
      model,
      messages: [
        ...(systemPrompt ? [{ role: 'system', content: systemPrompt }] : []),
        ...newMessages,
      ],
      temperature,
      max_tokens: maxTokens,
      stream: streaming,
    }

    try {
      if (streaming) {
        // ── Mode streaming SSE ───────────────────────────────────────────────
        const resp = await fetch('/v1/chat/completions', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload),
          signal: abortRef.current.signal,
        })

        if (!resp.ok) {
          const err = await resp.json()
          throw new Error(err.error?.message ?? `HTTP ${resp.status}`)
        }

        const reader = resp.body!.getReader()
        const decoder = new TextDecoder()
        let fullContent = ''
        let finishReason = 'stop'

        while (true) {
          const { done, value } = await reader.read()
          if (done) break

          const chunk = decoder.decode(value, { stream: true })
          for (const line of chunk.split('\n')) {
            const data = line.replace(/^data: /, '').trim()
            if (!data || data === '[DONE]') continue
            try {
              const parsed = JSON.parse(data)
              const delta = parsed.choices?.[0]?.delta?.content ?? ''
              finishReason = parsed.choices?.[0]?.finish_reason ?? finishReason
              if (delta) {
                fullContent += delta
                setStreamingContent(fullContent)
              }
            } catch { /* ignore parse errors */ }
          }
        }

        const latency = performance.now() - start
        const assistantMsg: Message = { role: 'assistant', content: fullContent }
        setMessages(prev => [...prev, assistantMsg])
        setStreamingContent('')
        setLastResult({
          model,
          provider,
          content: fullContent,
          latency_ms: latency,
          tokens: { prompt: 0, completion: 0, total: 0 },
          cost_usd: 0,
          finish_reason: finishReason,
          streaming: true,
        })
      } else {
        // ── Mode non-streaming ───────────────────────────────────────────────
        const resp = await api.post('/v1/chat/completions', payload, {
          signal: abortRef.current.signal,
        })
        const latency = performance.now() - start
        const choice = resp.data.choices?.[0]
        const usage = resp.data.usage ?? {}
        const content = choice?.message?.content ?? ''

        const assistantMsg: Message = { role: 'assistant', content }
        setMessages(prev => [...prev, assistantMsg])
        setLastResult({
          model,
          provider,
          content,
          latency_ms: latency,
          tokens: {
            prompt: usage.prompt_tokens ?? 0,
            completion: usage.completion_tokens ?? 0,
            total: usage.total_tokens ?? 0,
          },
          cost_usd: 0,
          finish_reason: choice?.finish_reason ?? 'stop',
        })
      }
    } catch (err: unknown) {
      if ((err as Error)?.name === 'AbortError') return
      const msg = err instanceof Error ? err.message : String(err)
      setMessages(prev => [
        ...prev,
        { role: 'assistant', content: `❌ Error: ${msg}` },
      ])
      setLastResult({
        model,
        provider,
        content: '',
        latency_ms: performance.now() - start,
        tokens: { prompt: 0, completion: 0, total: 0 },
        cost_usd: 0,
        finish_reason: 'error',
        error: msg,
      })
    } finally {
      setIsRunning(false)
      setStreamingContent('')
    }
  }

  function stop() {
    abortRef.current?.abort()
    setIsRunning(false)
    setStreamingContent('')
  }

  function clear() {
    setMessages([])
    setLastResult(null)
    setStreamingContent('')
  }

  const { provider: currentProvider } = selectedModel ? parseSelected() : { provider: '' }

  return (
    <div className="flex h-full overflow-hidden">

      {/* ── Panneau gauche : config ─────────────────────────────────────── */}
      <aside className="w-64 flex-shrink-0 border-r border-gray-800 bg-gray-900 flex flex-col overflow-y-auto">
        <div className="p-4 space-y-4">
          <h2 className="text-xs font-semibold text-gray-500 uppercase tracking-wide">Configuration</h2>

          {/* Model selector */}
          <div className="space-y-1.5">
            <label className="text-xs text-gray-400">Model</label>
            {modelsLoading
              ? <div className="h-9 bg-gray-800 rounded-lg animate-pulse" />
              : <ModelSelect
                  models={models}
                  value={selectedModel}
                  onChange={setSelectedModel}
                  disabled={isRunning}
                />
            }
            {currentProvider && (
              <div className="flex items-center gap-1.5 text-xs text-gray-500">
                <div className="w-1.5 h-1.5 rounded-full" style={{ background: providerColor(currentProvider) }} />
                {currentProvider}
              </div>
            )}
          </div>

          {/* System prompt */}
          <div className="space-y-1.5">
            <label className="text-xs text-gray-400">System Prompt</label>
            <textarea
              value={systemPrompt}
              onChange={e => setSystemPrompt(e.target.value)}
              rows={3}
              disabled={isRunning}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm
                text-gray-200 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500
                disabled:opacity-50"
            />
          </div>

          {/* Temperature */}
          <div className="space-y-1.5">
            <div className="flex justify-between">
              <label className="text-xs text-gray-400">Temperature</label>
              <span className="text-xs text-gray-300 font-mono">{temperature.toFixed(1)}</span>
            </div>
            <input
              type="range" min="0" max="2" step="0.1"
              value={temperature}
              onChange={e => setTemperature(parseFloat(e.target.value))}
              disabled={isRunning}
              className="w-full accent-blue-500"
            />
          </div>

          {/* Max tokens */}
          <div className="space-y-1.5">
            <div className="flex justify-between">
              <label className="text-xs text-gray-400">Max Tokens</label>
              <span className="text-xs text-gray-300 font-mono">{maxTokens}</span>
            </div>
            <input
              type="range" min="64" max="4096" step="64"
              value={maxTokens}
              onChange={e => setMaxTokens(parseInt(e.target.value))}
              disabled={isRunning}
              className="w-full accent-blue-500"
            />
          </div>

          {/* Streaming toggle */}
          <div className="flex items-center justify-between">
            <label className="text-xs text-gray-400">Streaming</label>
            <button
              onClick={() => setStreaming(!streaming)}
              disabled={isRunning}
              className={`relative w-10 h-5 rounded-full transition-colors
                ${streaming ? 'bg-blue-600' : 'bg-gray-700'}`}
            >
              <div className={`absolute top-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform
                ${streaming ? 'translate-x-5' : 'translate-x-0.5'}`} />
            </button>
          </div>

          {/* Clear button */}
          <button
            onClick={clear}
            disabled={isRunning || messages.length === 0}
            className="w-full flex items-center justify-center gap-2 py-2 rounded-lg
              border border-gray-700 text-gray-400 text-sm hover:border-red-700 hover:text-red-400
              transition-colors disabled:opacity-40"
          >
            <Trash2 size={13} />
            Clear conversation
          </button>
        </div>

        {/* Result badge */}
        {lastResult && (
          <div className="mt-auto p-4 border-t border-gray-800">
            <h3 className="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-2">Last run</h3>
            <ResultBadge result={lastResult} />
          </div>
        )}
      </aside>

      {/* ── Panneau droit : conversation ────────────────────────────────── */}
      <div className="flex-1 flex flex-col min-h-0">
        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {messages.length === 0 && !isRunning && (
            <div className="h-full flex flex-col items-center justify-center text-gray-600 gap-3">
              <div className="w-12 h-12 rounded-2xl bg-gray-800 flex items-center justify-center">
                <Zap size={20} className="text-blue-500" />
              </div>
              <div className="text-center">
                <div className="font-medium text-gray-400">Playground Pylos</div>
                <div className="text-sm mt-1">
                  Sélectionne un modèle et envoie un message
                </div>
              </div>
              {models.length > 0 && (
                <div className="text-xs text-gray-700 mt-2">
                  {models.filter(m => m.provider === 'ollama').length} modèles Ollama locaux disponibles
                </div>
              )}
            </div>
          )}

          {messages.map((msg, i) => (
            <ChatMessage
              key={i}
              msg={msg}
              isStreaming={isRunning && i === messages.length - 1 && msg.role === 'assistant'}
            />
          ))}

          {/* Streaming assistant bubble */}
          {isRunning && streamingContent && (
            <ChatMessage
              msg={{ role: 'assistant', content: streamingContent }}
              isStreaming
            />
          )}

          {/* Thinking indicator */}
          {isRunning && !streamingContent && (
            <div className="flex gap-3">
              <div className="w-7 h-7 rounded-full bg-gray-700 flex items-center justify-center text-xs font-bold text-gray-300">
                AI
              </div>
              <div className="bg-gray-800 border border-gray-700 rounded-2xl rounded-tl-sm px-4 py-3">
                <div className="flex gap-1.5">
                  {[0, 1, 2].map(i => (
                    <div key={i}
                      className="w-2 h-2 rounded-full bg-gray-500 animate-bounce"
                      style={{ animationDelay: `${i * 0.15}s` }}
                    />
                  ))}
                </div>
              </div>
            </div>
          )}

          <div ref={bottomRef} />
        </div>

        {/* Input */}
        <div className="border-t border-gray-800 p-4">
          <div className="flex gap-3 items-end">
            <textarea
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault()
                  run()
                }
              }}
              placeholder="Type a message… (Enter to send, Shift+Enter for newline)"
              rows={2}
              disabled={isRunning || !selectedModel}
              className="flex-1 bg-gray-800 border border-gray-700 rounded-xl px-4 py-3 text-sm
                text-gray-100 placeholder-gray-600 resize-none
                focus:outline-none focus:ring-2 focus:ring-blue-500
                disabled:opacity-50 disabled:cursor-not-allowed"
            />
            {isRunning ? (
              <button
                onClick={stop}
                className="p-3 rounded-xl bg-red-600 hover:bg-red-500 text-white transition-colors flex-shrink-0"
              >
                <StopCircle size={18} />
              </button>
            ) : (
              <button
                onClick={run}
                disabled={!input.trim() || !selectedModel}
                className="p-3 rounded-xl bg-blue-600 hover:bg-blue-500 text-white transition-colors flex-shrink-0
                  disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <Send size={18} />
              </button>
            )}
          </div>
          {selectedModel && (
            <div className="text-xs text-gray-600 mt-2 ml-1">
              {parseSelected().model} via {parseSelected().provider}
              {streaming ? ' · streaming' : ' · blocking'}
              · temp {temperature} · max {maxTokens} tokens
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
