import axios from 'axios'

// Auto-detect l'URL du backend Pylos
const BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000'

export const api = axios.create({
  baseURL: BASE_URL,
  timeout: 30000,
})

// ─── Types ────────────────────────────────────────────────────────────────────

export interface LogEntry {
  id: string
  timestamp: number
  provider: string
  model: string
  object: string
  status: 'success' | 'error'
  latency_ms: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cost_usd: number
  finish_reason: string | null
  error_message: string | null
  virtual_key: string | null
  is_stream: boolean
  input_preview: string | null
  output_preview: string | null
}

export interface LogStats {
  total_requests: number
  success_rate: number
  average_latency_ms: number
  total_tokens: number
  total_cost_usd: number
  total_prompt_tokens: number
  total_completion_tokens: number
}

export interface HistogramBucket {
  timestamp: number
  count: number
  success: number
  error: number
}

export interface TokenBucket {
  timestamp: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
}

export interface LogsResponse {
  logs: LogEntry[]
  pagination: { limit: number; offset: number; total_count: number }
  stats: LogStats
  has_logs: boolean
}

export interface Provider {
  name: string
  keys_count: number
  keys: Array<{ name: string; value: string; models: string[]; weight: number }>
  network: { base_url: string | null; timeout_secs: number; max_retries: number }
}

export interface VirtualKey {
  id: string
  name: string
  description: string | null
  is_active: boolean
  value: string
  provider_configs: Array<{ provider: string; allowed_models: string[]; weight: number }>
}

// ─── API calls ────────────────────────────────────────────────────────────────

export const logsApi = {
  getLogs: (params: Record<string, string | number>) =>
    api.get<LogsResponse>('/api/logs', { params }).then(r => r.data),

  getStats: (params: Record<string, string>) =>
    api.get<LogStats>('/api/logs/stats', { params }).then(r => r.data),

  getHistogram: (params: Record<string, string>) =>
    api.get<{ buckets: HistogramBucket[]; bucket_size_seconds: number }>(
      '/api/logs/histogram',
      { params }
    ).then(r => r.data),

  getTokenHistogram: (params: Record<string, string>) =>
    api.get<{ buckets: TokenBucket[]; bucket_size_seconds: number }>(
      '/api/logs/histogram/tokens',
      { params }
    ).then(r => r.data),

  getFilterData: () =>
    api.get('/api/logs/filterdata').then(r => r.data),
}

export const providersApi = {
  getAll: () =>
    api.get<{ providers: Provider[]; total: number }>('/providers').then(r => r.data),
}

export const virtualKeysApi = {
  getAll: () =>
    api.get<{ virtual_keys: VirtualKey[]; total: number }>('/virtual-keys').then(r => r.data),
}

export const healthApi = {
  check: () => api.get('/health').then(r => r.data),
  getRoot: () => api.get('/').then(r => r.data),
}
