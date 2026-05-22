import axios from 'axios'

const getBaseUrl = (): string => {
  const envUrl = import.meta.env.VITE_API_URL;
  if (envUrl) {
    // Si l'API pointe vers localhost mais que le frontend est chargé sur un autre domaine,
    // on utilise des chemins relatifs pour passer par Nginx/reverse proxy.
    if (
      typeof window !== 'undefined' &&
      envUrl.includes('localhost') &&
      window.location.hostname !== 'localhost' &&
      window.location.hostname !== '127.0.0.1'
    ) {
      return '';
    }
    return envUrl;
  }
  return '';
};

const BASE_URL = getBaseUrl();

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
  rate_limit_id: string | null
  provider_configs: Array<{ provider: string; allowed_models: string[]; weight: number }>
}

export interface BudgetUsage {
  period: string
  max_usd: number
  current_usd: number
  reset_at_ms: number
}

export interface RateLimitStatus {
  window_type: string
  max_value: number
  current_value: number
  reset_at_ms: number
}

export interface VkBudgetResponse {
  virtual_key_id: string
  budget: BudgetUsage[]
  rate_limits: RateLimitStatus[]
}

export interface ModelInfo {
  id: string
  provider: string
  model_id: string
  display_name: string | null
  context_window: number
  max_output_tokens: number
  input_price_per_1m_usd: number
  output_price_per_1m_usd: number
  supports_vision: boolean
  supports_tools: boolean
  supports_streaming: boolean
  supports_embeddings: boolean
  is_deprecated: boolean
}

export interface ModelListResponse {
  object: string
  data: Array<{
    id: string
    object: string
    owned_by: string
    provider: string        // champ direct sur chaque entrée
    pylos: ModelInfo
  }>
}

// ─── API calls ────────────────────────────────────────────────────────────────

export const logsApi = {
  getLogs: (params: Record<string, string | number>) =>
    api.get<LogsResponse>('/api/logs', { params }).then(r => r.data),

  getStats: (params: Record<string, string>) =>
    api.get<LogStats>('/api/logs/stats', { params }).then(r => r.data),

  getHistogram: (params: Record<string, string>) =>
    api.get<{ buckets: HistogramBucket[]; bucket_size_seconds: number }>(
      '/api/logs/histogram', { params }
    ).then(r => r.data),

  getTokenHistogram: (params: Record<string, string>) =>
    api.get<{ buckets: TokenBucket[]; bucket_size_seconds: number }>(
      '/api/logs/histogram/tokens', { params }
    ).then(r => r.data),

  getFilterData: () => api.get('/api/logs/filterdata').then(r => r.data),
}

export const providersApi = {
  getAll: () =>
    api.get<{ providers: Provider[]; total: number }>('/providers').then(r => r.data),

  create: (data: { name: string } & Record<string, unknown>) =>
    api.post('/providers', data).then(r => r.data),

  update: (name: string, data: Record<string, unknown>) =>
    api.put(`/providers/${name}`, data).then(r => r.data),

  remove: (name: string) =>
    api.delete(`/providers/${name}`).then(r => r.data),
}

export const virtualKeysApi = {
  getAll: () =>
    api.get<{ virtual_keys: VirtualKey[]; total: number }>('/virtual-keys').then(r => r.data),

  create: (data: Record<string, unknown>) =>
    api.post<{ id: string; name: string; value: string }>('/virtual-keys', data).then(r => r.data),

  update: (id: string, data: Record<string, unknown>) =>
    api.put(`/virtual-keys/${id}`, data).then(r => r.data),

  remove: (id: string) =>
    api.delete(`/virtual-keys/${id}`).then(r => r.data),

  getBudget: (id: string) =>
    api.get<VkBudgetResponse>(`/virtual-keys/${id}/budget`).then(r => r.data),
}

export const modelsApi = {
  getAll: (provider?: string) =>
    api.get<ModelListResponse>('/v1/models', {
      params: provider ? { provider } : {}
    }).then(r => r.data),

  upsert: (data: Record<string, unknown>) =>
    api.post('/v1/models/catalog', data).then(r => r.data),

  remove: (provider: string, model_id: string) =>
    api.delete(`/v1/models/catalog/${provider}/${model_id}`).then(r => r.data),
}

export const healthApi = {
  check: () => api.get('/health').then(r => r.data),
  getRoot: () => api.get('/').then(r => r.data),
}

export const configApi = {
  get: () => api.get('/config').then(r => r.data),
  reload: () => api.post('/config/reload').then(r => r.data),
}
