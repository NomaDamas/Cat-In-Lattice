use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: Option<String>,
    pub channel: Option<String>,
    pub token: Option<String>,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            webhook_url: None,
            channel: None,
            token: None,
        }
    }
}

impl SlackConfig {
    pub fn is_configured(&self) -> bool {
        self.token.is_some() && self.channel.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct SlackNotice {
    pub text: String,
    pub user: String,
    pub timestamp: DateTime<Utc>,
}

impl std::fmt::Display for SlackNotice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.user, self.text)
    }
}

/// Slack API conversations.history response types (subset).
#[derive(Deserialize)]
struct SlackHistoryResponse {
    ok: bool,
    messages: Option<Vec<SlackMessage>>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct SlackMessage {
    text: Option<String>,
    user: Option<String>,
    ts: Option<String>,
}

pub struct SlackNotifier {
    config: SlackConfig,
    cache: Vec<SlackNotice>,
    last_fetch: Option<Instant>,
}

impl SlackNotifier {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            cache: Vec::new(),
            last_fetch: None,
        }
    }

    /// Fetch latest notices (blocking). Returns cached results if fresh.
    /// Gracefully returns empty when not configured or on error.
    pub fn fetch_notices(&mut self) -> Vec<SlackNotice> {
        if !self.config.is_configured() {
            return Vec::new();
        }

        if let Some(last) = self.last_fetch {
            if last.elapsed() < CACHE_TTL {
                return self.cache.clone();
            }
        }

        match self.fetch_from_api() {
            Ok(notices) => {
                self.cache = notices;
                self.last_fetch = Some(Instant::now());
            }
            Err(_) => {
                // On error keep stale cache
            }
        }

        self.cache.clone()
    }

    fn fetch_from_api(&self) -> Result<Vec<SlackNotice>, SlackError> {
        let token = self
            .config
            .token
            .as_deref()
            .ok_or(SlackError::NotConfigured)?;
        let channel = self
            .config
            .channel
            .as_deref()
            .ok_or(SlackError::NotConfigured)?;

        let resp: SlackHistoryResponse = ureq::get("https://slack.com/api/conversations.history")
            .set("Authorization", &format!("Bearer {token}"))
            .query("channel", channel)
            .query("limit", "10")
            .call()
            .map_err(|e| SlackError::Request(e.to_string()))?
            .into_json()
            .map_err(|e| SlackError::Request(e.to_string()))?;

        if !resp.ok {
            return Err(SlackError::Api(
                resp.error.unwrap_or_else(|| "unknown error".into()),
            ));
        }

        let notices = resp
            .messages
            .unwrap_or_default()
            .into_iter()
            .filter_map(|msg| {
                let text = msg.text?;
                let user = msg.user.unwrap_or_else(|| "unknown".into());
                let ts = msg
                    .ts
                    .and_then(|s| {
                        let secs: f64 = s.parse().ok()?;
                        DateTime::from_timestamp(secs as i64, 0)
                    })
                    .unwrap_or_else(Utc::now);
                Some(SlackNotice {
                    text,
                    user,
                    timestamp: ts,
                })
            })
            .collect();

        Ok(notices)
    }

    /// Invalidate the cache so the next call hits the API.
    pub fn invalidate_cache(&mut self) {
        self.last_fetch = None;
    }

    /// Update configuration at runtime.
    pub fn update_config(&mut self, config: SlackConfig) {
        self.config = config;
        self.invalidate_cache();
    }
}

#[derive(Debug)]
enum SlackError {
    NotConfigured,
    Request(String),
    Api(String),
}

impl std::fmt::Display for SlackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Slack is not configured"),
            Self::Request(e) => write!(f, "HTTP error: {e}"),
            Self::Api(e) => write!(f, "Slack API error: {e}"),
        }
    }
}

impl std::error::Error for SlackError {}
