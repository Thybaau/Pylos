import { useState, useRef, useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useSearchParams } from 'react-router-dom'
import { api } from '../lib/api'
import { formatLatency, formatCost, providerColor } from '../lib/utils'
import {
  Send, StopCircle, Trash2, ChevronDown,
  Zap, Clock, Hash, Coins, CheckCircle, XCircle,
  SlidersHorizontal, X,
} from 'lucide-react'

// ─── Types ────────────────────────────────────────────────────────────────────

interface Model {
  id: string
  provider: string
  details?: { family?: string; parameter_size?: string }
  supports_streaming?: boolean
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
        className="w-full appearance-none bg-zinc-900 border border-zinc-800 rounded-lg
          px-3 py-2 text-sm text-zinc-100 pr-8
          focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20
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
      <ChevronDown size={14} className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 pointer-events-none" />
    </div>
  )
}

function ChatMessage({ msg, isStreaming }: { msg: Message; isStreaming?: boolean }) {
  const isUser = msg.role === 'user'
  const isSystem = msg.role === 'system'

  if (isSystem) {
    return (
      <div className="rounded-lg border border-dashed border-zinc-700 bg-zinc-800/40 px-4 py-2 text-xs text-zinc-400">
        <span className="font-semibold text-zinc-500 mr-2">SYSTEM</span>
        {msg.content}
      </div>
    )
  }

  return (
    <div className={`flex gap-3 ${isUser ? 'flex-row-reverse' : 'flex-row'}`}>
      <div className={`w-7 h-7 rounded-full flex items-center justify-center flex-shrink-0 text-xs font-bold
        ${isUser ? 'bg-emerald-600 text-white' : 'bg-zinc-700 text-zinc-300'}`}>
        {isUser ? 'U' : 'AI'}
      </div>
      <div className={`max-w-[85%] sm:max-w-[80%] rounded-2xl px-4 py-3 text-sm leading-relaxed
        ${isUser
          ? 'bg-emerald-600 text-white rounded-tr-sm'
          : 'bg-zinc-800/50 border border-zinc-700/50 text-zinc-100 rounded-tl-sm'
        }`}>
        <pre className="whitespace-pre-wrap font-sans break-words text-xs sm:text-sm">
          {msg.content}
          {isStreaming && <span className="inline-block w-1.5 h-4 bg-zinc-400 ml-0.5 animate-pulse" />}
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
      ${hasError ? 'border-red-800/50 bg-red-900/10' : 'border-zinc-800/50 bg-zinc-900/30'}`}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full" style={{ background: color }} />
          <span className="text-xs font-semibold text-zinc-300">{result.provider}</span>
          <span className="text-xs text-zinc-500 font-mono">{result.model}</span>
        </div>
        {hasError
          ? <XCircle size={14} className="text-red-400" />
          : <CheckCircle size={14} className="text-emerald-400" />
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
    <div className="bg-zinc-800/50 rounded-lg px-2 py-1.5 text-center">
      <div className="flex items-center justify-center gap-1 text-zinc-500 mb-0.5">
        {icon}
        <span>{label}</span>
      </div>
      <div className="font-semibold text-zinc-200">{value}</div>
    </div>
  )
}

// ─── Page principale ──────────────────────────────────────────────────────────

export default function Playground() {
  const [searchParams] = useSearchParams()
  const modelParam = searchParams.get('model')
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
  const [showConfig, setShowConfig] = useState(false)
  const abortRef = useRef<AbortController | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)
  const { data: modelsData, isLoading: modelsLoading } = useQuery({
    queryKey: ['models'],
    queryFn: fetchModels,
    staleTime: 60_000,
  })

  const models = modelsData ?? []

  // Sync streaming flag with selected model capabilities
  useEffect(() => {
    if (!models || !selectedModel) return
    const [provider, ...rest] = selectedModel.split('::')
    const modelId = rest.join('::')
    const mdl = models.find(m => m.provider === provider && m.id === modelId)
    if (mdl && typeof mdl.supports_streaming === 'boolean') {
      if (streaming !== mdl.supports_streaming) {
        setStreaming(mdl.supports_streaming)
      }
    }
  }, [selectedModel, models])

  useEffect(() => {
    if (!models.length) return

    if (modelParam) {
      const matched = models.find(
        m => `${m.provider}::${m.id}` === modelParam || `${m.provider}/${m.id}` === modelParam || m.id === modelParam
      )
      if (matched) {
        setSelectedModel(`${matched.provider}::${matched.id}`)
        return
      }
    }

    if (!selectedModel) {
      const ollama = models.find(m => m.provider === 'ollama-jo3' && m.id === 'llama3.1:8b')
        ?? models.find(m => m.provider === 'ollama-jo3')
        ?? models[0]
      if (ollama) setSelectedModel(`${ollama.provider}::${ollama.id}`)
    }
  }, [models, modelParam])

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
            let streamError: string | null = null
            try {
              const parsed = JSON.parse(data)
              if (parsed.error) {
                streamError = typeof parsed.error === 'object'
                  ? (parsed.error.message ?? JSON.stringify(parsed.error))
                  : parsed.error
              } else {
                const delta = parsed.choices?.[0]?.delta?.content ?? ''
                finishReason = parsed.choices?.[0]?.finish_reason ?? finishReason
                if (delta) {
                  fullContent += delta
                  setStreamingContent(fullContent)
                }
              }
            } catch { /* ignore parse errors */ }
            if (streamError) {
              throw new Error(streamError)
            }
          }
        }

        // Validate that we received some content
        if (!fullContent.trim()) {
          throw new Error('Empty response from model')
        }

        const latency = performance.now() - start
        const assistantMsg: Message = { role: 'assistant', content: fullContent }
        setMessages(prev => [...prev, assistantMsg])
        setStreamingContent('')

        const completionTokens = Math.max(1, Math.floor(fullContent.length / 4))
        const promptText = newMessages.map(m => m.content).join(' ')
        const promptTokens = Math.max(1, Math.floor(promptText.length / 4))

        setLastResult({
          model,
          provider,
          content: fullContent,
          latency_ms: latency,
          tokens: { prompt: promptTokens, completion: completionTokens, total: promptTokens + completionTokens },
          cost_usd: 0,
          finish_reason: finishReason,
          streaming: true,
        })
      } else {
        const resp = await api.post('/v1/chat/completions', payload, {
          signal: abortRef.current.signal,
        })
        const latency = performance.now() - start
        const choice = resp.data.choices?.[0]
        const usage = resp.data.usage ?? {}
        const content = choice?.message?.content ?? ''

        // Validate that we received some content
        if (!content.trim()) {
          throw new Error('Empty response from model')
        }

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
        { role: 'assistant', content: `Error: ${msg}` },
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
    <div className="flex h-full overflow-hidden relative">

      {/* Backdrop overlay for config drawer on mobile */}
      {showConfig && (
        <div
          className="fixed inset-0 z-30 bg-black/60 backdrop-blur-sm md:hidden"
          onClick={() => setShowConfig(false)}
        />
      )}

      {/* ── Config sidebar ─────────────────────────────────────── */}
      <aside
        className={`fixed inset-y-0 right-0 z-45 w-64 bg-zinc-950 border-l border-zinc-800/80 flex flex-col overflow-y-auto transition-transform duration-300 md:duration-200
          md:static md:translate-x-0
          ${showConfig ? 'translate-x-0' : 'translate-x-full'}
        `}
      >
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold text-zinc-500 uppercase tracking-wide">Configuration</h2>
            <button
              onClick={() => setShowConfig(false)}
              className="md:hidden text-zinc-400 hover:text-white p-1 rounded-lg hover:bg-zinc-800/50"
            >
              <X size={16} />
            </button>
          </div>

          {/* Model selector */}
          <div className="space-y-1.5">
            <label className="text-xs text-zinc-400">Model</label>
            {modelsLoading
              ? <div className="h-9 bg-zinc-800 rounded-lg animate-pulse" />
              : <ModelSelect
                  models={models}
                  value={selectedModel}
                  onChange={setSelectedModel}
                  disabled={isRunning}
                />
            }
            {currentProvider && (
              <div className="flex items-center gap-1.5 text-xs text-zinc-500">
                <div className="w-1.5 h-1.5 rounded-full" style={{ background: providerColor(currentProvider) }} />
                {currentProvider}
              </div>
            )}
          </div>

          <div className="border-t border-zinc-800/50" />

          {/* System prompt */}
          <div className="space-y-1.5">
            <label className="text-xs text-zinc-400">System Prompt</label>
            <textarea
              value={systemPrompt}
              onChange={e => setSystemPrompt(e.target.value)}
              rows={3}
              disabled={isRunning}
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm
                text-zinc-200 resize-none focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20
                disabled:opacity-50"
            />
          </div>

          <div className="border-t border-zinc-800/50" />

          {/* Temperature */}
          <div className="space-y-1.5">
            <div className="flex justify-between">
              <label className="text-xs text-zinc-400">Temperature</label>
              <span className="text-xs text-zinc-300 font-mono">{temperature.toFixed(1)}</span>
            </div>
            <input
              type="range" min="0" max="2" step="0.1"
              value={temperature}
              onChange={e => setTemperature(parseFloat(e.target.value))}
              disabled={isRunning}
              className="w-full accent-emerald-500"
            />
          </div>

          {/* Max tokens */}
          <div className="space-y-1.5">
            <div className="flex justify-between">
              <label className="text-xs text-zinc-400">Max Tokens</label>
              <span className="text-xs text-zinc-300 font-mono">{maxTokens}</span>
            </div>
            <input
              type="range" min="64" max="4096" step="64"
              value={maxTokens}
              onChange={e => setMaxTokens(parseInt(e.target.value))}
              disabled={isRunning}
              className="w-full accent-emerald-500"
            />
          </div>

          {/* Streaming toggle */}
          <div className="flex items-center justify-between">
            <label className="text-xs text-zinc-400">Streaming</label>
            <button
              onClick={() => setStreaming(!streaming)}
              disabled={isRunning}
              className={`relative w-10 h-5 rounded-full transition-colors
                ${streaming ? 'bg-emerald-600' : 'bg-zinc-700'}`}
            >
              <div className={`absolute top-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform
                ${streaming ? 'translate-x-5' : 'translate-x-0.5'}`} />
            </button>
          </div>

          <div className="border-t border-zinc-800/50" />

          {/* Clear button */}
          <button
            onClick={clear}
            disabled={isRunning || messages.length === 0}
            className="w-full flex items-center justify-center gap-2 py-2 rounded-lg
              border border-zinc-800 text-zinc-400 text-sm hover:border-red-700 hover:text-red-400
              transition-colors disabled:opacity-40"
          >
            <Trash2 size={13} />
            Clear conversation
          </button>
        </div>

        {/* Result badge */}
        {lastResult && (
          <div className="mt-auto p-4 border-t border-zinc-800/50">
            <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wide mb-2">Last run</h3>
            <ResultBadge result={lastResult} />
          </div>
        )}
      </aside>

      {/* ── Conversation panel ────────────────────────────────── */}
      <div className="flex-1 flex flex-col min-h-0 bg-zinc-950">
        {/* Mobile Header Toolbar */}
        <div className="flex md:hidden items-center justify-between border-b border-zinc-800/50 px-4 py-2 bg-zinc-900/50">
          <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">Playground</span>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowConfig(true)}
              className="p-1.5 rounded-lg text-zinc-400 hover:text-white hover:bg-zinc-800 transition-colors"
              title="Configuration"
            >
              <SlidersHorizontal size={18} />
            </button>
            <button
              onClick={clear}
              disabled={isRunning || messages.length === 0}
              className="p-1.5 rounded-lg text-zinc-400 hover:text-red-400 hover:bg-zinc-800 transition-colors disabled:opacity-40"
              title="Clear conversation"
            >
              <Trash2 size={18} />
            </button>
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-4 sm:p-5 space-y-4">
          {messages.length === 0 && !isRunning && (
            <div className="h-full flex flex-col items-center justify-center text-zinc-600 gap-3">
              <div className="w-12 h-12 rounded-2xl bg-zinc-900 border border-zinc-800/50 flex items-center justify-center">
                <Zap size={20} className="text-emerald-500" />
              </div>
                <div className="text-center">
                <div className="font-medium text-zinc-400">Pylos Playground</div>
                <div className="text-sm mt-1">
                  Select a model and send a message
                </div>
              </div>
              {models.length > 0 && (
                <div className="text-xs text-zinc-700 mt-2">
                  {models.filter(m => m.provider === 'ollama-jo3').length} local Ollama models available
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
              <div className="w-7 h-7 rounded-full bg-zinc-700 flex items-center justify-center text-xs font-bold text-zinc-300">
                AI
              </div>
              <div className="bg-zinc-800/50 border border-zinc-700/50 rounded-2xl rounded-tl-sm px-4 py-3">
                <div className="flex gap-1.5">
                  {[0, 1, 2].map(i => (
                    <div key={i}
                      className="w-2 h-2 rounded-full bg-zinc-500 animate-bounce"
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
        <div className="border-t border-zinc-800/50 p-4 bg-zinc-950">
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
              className="flex-1 bg-zinc-900 border border-zinc-800 rounded-xl px-4 py-3 text-sm
                text-zinc-100 placeholder-zinc-600 resize-none
                focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20
                disabled:opacity-50 disabled:cursor-not-allowed"
            />
            {isRunning ? (
              <button
                onClick={stop}
                className="p-3 rounded-xl bg-red-600 hover:bg-red-500 text-white transition-colors flex-shrink-0 active:scale-[0.98]"
              >
                <StopCircle size={18} />
              </button>
            ) : (
              <button
                onClick={run}
                disabled={!input.trim() || !selectedModel}
                className="p-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white transition-colors flex-shrink-0
                  disabled:opacity-40 disabled:cursor-not-allowed active:scale-[0.98]"
              >
                <Send size={18} />
              </button>
            )}
          </div>
          {selectedModel && (
            <div className="text-xs text-zinc-600 mt-2 ml-1">
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
