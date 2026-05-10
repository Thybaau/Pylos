use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use pylos_core::domain::openai::Usage;

// ─────────────────────────────────────────────────────────────────────────────
// Types de log — compatibles avec l'API bifrost /api/logs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: i64, // Unix ms
    pub provider: String,
    pub model: String,
    pub object: String, // "chat.completion", "chat.completion.stream"...
    pub status: LogStatus,
    pub latency_ms: f64,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: f64,
    pub finish_reason: Option<String>,
    pub error_message: Option<String>,
    pub virtual_key: Option<String>,
    pub is_stream: bool,
    /// Preview du message user (tronqué à 200 chars)
    pub input_preview: Option<String>,
    /// Preview de la réponse assistant (tronqué à 200 chars)
    pub output_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogStatus {
    Success,
    Error,
}

// ─────────────────────────────────────────────────────────────────────────────
// Stats agrégées — équivalent GET /api/logs/stats
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStats {
    pub total_requests: u64,
    pub success_rate: f64,
    pub average_latency_ms: f64,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Bucket histogramme — équivalent GET /api/logs/histogram
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub timestamp: i64, // Unix ms — début du bucket
    pub count: u64,
    pub success: u64,
    pub error: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    pub timestamp: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// LogStore — ring buffer en mémoire, thread-safe
// Taille max configurable (défaut: 10 000 entrées)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LogStore {
    inner: Arc<RwLock<LogStoreInner>>,
}

struct LogStoreInner {
    entries: VecDeque<LogEntry>,
    max_size: usize,
}

impl LogStore {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LogStoreInner {
                entries: VecDeque::with_capacity(max_size),
                max_size,
            })),
        }
    }

    /// Ajoute une entrée (supprime la plus ancienne si ring buffer plein)
    pub async fn push(&self, entry: LogEntry) {
        let mut inner = self.inner.write().await;
        if inner.entries.len() >= inner.max_size {
            inner.entries.pop_front();
        }
        inner.entries.push_back(entry);
    }

    /// Retourne les N derniers logs (les plus récents en premier)
    pub async fn list(
        &self,
        limit: usize,
        offset: usize,
        filter: &LogFilter,
    ) -> (Vec<LogEntry>, u64) {
        let inner = self.inner.read().await;

        let filtered: Vec<&LogEntry> = inner
            .entries
            .iter()
            .rev() // Plus récents en premier
            .filter(|e| filter.matches(e))
            .collect();

        let total = filtered.len() as u64;
        let page: Vec<LogEntry> = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        (page, total)
    }

    /// Calcule les stats agrégées sur la fenêtre filtrée
    pub async fn stats(&self, filter: &LogFilter) -> LogStats {
        let inner = self.inner.read().await;

        let entries: Vec<&LogEntry> = inner.entries.iter().filter(|e| filter.matches(e)).collect();

        let total = entries.len() as u64;
        if total == 0 {
            return LogStats {
                total_requests: 0,
                success_rate: 0.0,
                average_latency_ms: 0.0,
                total_tokens: 0,
                total_cost_usd: 0.0,
                total_prompt_tokens: 0,
                total_completion_tokens: 0,
            };
        }

        let success = entries
            .iter()
            .filter(|e| e.status == LogStatus::Success)
            .count() as u64;

        let total_latency: f64 = entries.iter().map(|e| e.latency_ms).sum();
        let total_tokens: i64 = entries.iter().map(|e| e.total_tokens as i64).sum();
        let total_prompt: i64 = entries.iter().map(|e| e.prompt_tokens as i64).sum();
        let total_completion: i64 = entries.iter().map(|e| e.completion_tokens as i64).sum();
        let total_cost: f64 = entries.iter().map(|e| e.cost_usd).sum();

        LogStats {
            total_requests: total,
            success_rate: (success as f64 / total as f64) * 100.0,
            average_latency_ms: total_latency / total as f64,
            total_tokens,
            total_cost_usd: total_cost,
            total_prompt_tokens: total_prompt,
            total_completion_tokens: total_completion,
        }
    }

    /// Histogramme temporel — buckets de N secondes
    pub async fn histogram(&self, filter: &LogFilter, bucket_secs: i64) -> Vec<HistogramBucket> {
        let inner = self.inner.read().await;

        let entries: Vec<&LogEntry> = inner.entries.iter().filter(|e| filter.matches(e)).collect();

        if entries.is_empty() {
            return vec![];
        }

        // Groupe par bucket
        let mut buckets: std::collections::BTreeMap<i64, HistogramBucket> =
            std::collections::BTreeMap::new();

        for entry in &entries {
            let bucket_ts = (entry.timestamp / (bucket_secs * 1000)) * (bucket_secs * 1000);
            let b = buckets.entry(bucket_ts).or_insert(HistogramBucket {
                timestamp: bucket_ts,
                count: 0,
                success: 0,
                error: 0,
            });
            b.count += 1;
            match entry.status {
                LogStatus::Success => b.success += 1,
                LogStatus::Error => b.error += 1,
            }
        }

        buckets.into_values().collect()
    }

    /// Histogramme tokens
    pub async fn token_histogram(&self, filter: &LogFilter, bucket_secs: i64) -> Vec<TokenBucket> {
        let inner = self.inner.read().await;

        let entries: Vec<&LogEntry> = inner.entries.iter().filter(|e| filter.matches(e)).collect();

        let mut buckets: std::collections::BTreeMap<i64, TokenBucket> =
            std::collections::BTreeMap::new();

        for entry in &entries {
            let bucket_ts = (entry.timestamp / (bucket_secs * 1000)) * (bucket_secs * 1000);
            let b = buckets.entry(bucket_ts).or_insert(TokenBucket {
                timestamp: bucket_ts,
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            });
            b.prompt_tokens += entry.prompt_tokens as i64;
            b.completion_tokens += entry.completion_tokens as i64;
            b.total_tokens += entry.total_tokens as i64;
        }

        buckets.into_values().collect()
    }

    /// Nombre total d'entrées dans le store
    pub async fn total_count(&self) -> usize {
        self.inner.read().await.entries.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Filtre de logs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: Option<LogStatus>,
    pub since_ms: Option<i64>, // timestamp Unix ms
    pub until_ms: Option<i64>,
    pub virtual_key: Option<String>,
}

impl LogFilter {
    pub fn matches(&self, entry: &LogEntry) -> bool {
        if let Some(ref p) = self.provider {
            if &entry.provider != p {
                return false;
            }
        }
        if let Some(ref m) = self.model {
            if !entry.model.contains(m.as_str()) {
                return false;
            }
        }
        if let Some(ref s) = self.status {
            if &entry.status != s {
                return false;
            }
        }
        if let Some(since) = self.since_ms {
            if entry.timestamp < since {
                return false;
            }
        }
        if let Some(until) = self.until_ms {
            if entry.timestamp > until {
                return false;
            }
        }
        if let Some(ref vk) = self.virtual_key {
            match &entry.virtual_key {
                Some(k) if k == vk => {}
                _ => return false,
            }
        }
        true
    }

    /// Détermine la taille des buckets en fonction de la plage temporelle
    pub fn bucket_size_secs(&self) -> i64 {
        let range_ms = match (self.since_ms, self.until_ms) {
            (Some(s), Some(u)) => u - s,
            (Some(s), None) => now_ms() - s,
            _ => 3600 * 1000, // 1h par défaut
        };
        let range_secs = range_ms / 1000;
        match range_secs {
            0..=7200 => 60,         // < 2h   → 1 min
            7201..=86400 => 600,    // < 24h  → 10 min
            86401..=259200 => 3600, // < 3j   → 1h
            _ => 86400,             // > 3j   → 1j
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn generate_log_id() -> String {
    format!("log_{}", fastrand::u64(..))
}

/// Construit une LogEntry depuis les données d'une requête d'inférence
pub fn build_log_entry(
    provider: &str,
    model: &str,
    is_stream: bool,
    status: LogStatus,
    latency_ms: f64,
    usage: Option<&Usage>,
    finish_reason: Option<String>,
    error_message: Option<String>,
    input_preview: Option<String>,
    output_preview: Option<String>,
    virtual_key: Option<String>,
) -> LogEntry {
    let (prompt_tokens, completion_tokens, total_tokens) = usage
        .map(|u| (u.prompt_tokens, u.completion_tokens, u.total_tokens))
        .unwrap_or((0, 0, 0));

    // Estimation grossière du coût (en attente d'un catalogue de prix)
    let cost_usd = estimate_cost(provider, model, prompt_tokens, completion_tokens);

    let object = if is_stream {
        "chat.completion.stream".to_string()
    } else {
        "chat.completion".to_string()
    };

    LogEntry {
        id: generate_log_id(),
        timestamp: now_ms(),
        provider: provider.to_string(),
        model: model.to_string(),
        object,
        status,
        latency_ms,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        cost_usd,
        finish_reason,
        error_message,
        virtual_key,
        is_stream,
        input_preview: input_preview.map(|s| truncate(&s, 200)),
        output_preview: output_preview.map(|s| truncate(&s, 200)),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_chars).collect::<String>())
    }
}

/// Estimation du coût basée sur les prix publics (ordre de grandeur)
fn estimate_cost(provider: &str, model: &str, prompt: i32, completion: i32) -> f64 {
    let (input_per_1m, output_per_1m): (f64, f64) = match provider {
        "openai" | "openrouter" => {
            if model.contains("gpt-4o-mini") {
                (0.15, 0.60)
            } else if model.contains("gpt-4o") {
                (5.0, 15.0)
            } else {
                (1.0, 3.0)
            }
        }
        "anthropic" => {
            if model.contains("haiku") {
                (0.25, 1.25)
            } else if model.contains("sonnet") {
                (3.0, 15.0)
            } else {
                (15.0, 75.0)
            }
        }
        "bedrock" => {
            if model.contains("nova-lite") {
                (0.06, 0.24)
            } else if model.contains("nova-pro") {
                (0.80, 3.20)
            } else if model.contains("claude") {
                (3.0, 15.0)
            } else {
                (0.50, 1.50)
            }
        }
        _ => (1.0, 3.0),
    };

    let input_cost = (prompt as f64 / 1_000_000.0) * input_per_1m;
    let output_cost = (completion as f64 / 1_000_000.0) * output_per_1m;
    (input_cost + output_cost * 1_000_000.0).round() / 1_000_000.0
}
