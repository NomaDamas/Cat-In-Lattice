use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlackConfig {
    pub webhook_url: Option<String>,
    pub channel: Option<String>,
    pub token: Option<String>,
}

impl SlackConfig {
    pub fn is_configured(&self) -> bool {
        self.token.is_some() && self.channel.is_some()
    }

    pub fn has_webhook(&self) -> bool {
        self.webhook_url.is_some()
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

#[derive(Debug, Clone)]
pub struct SlackCheckReport {
    pub recent_notice_count: usize,
    pub latest_notice: Option<SlackNotice>,
}

pub struct SlackNotifier {
    config: SlackConfig,
    cache: Vec<SlackNotice>,
    last_fetch: Option<Instant>,
    last_seen_timestamp: Option<DateTime<Utc>>,
}

impl SlackNotifier {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            cache: Vec::new(),
            last_fetch: None,
            last_seen_timestamp: None,
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

    /// Fetch only notices that are newer than the last successfully seen one.
    /// The first successful poll establishes a baseline and returns no alerts.
    pub fn fetch_new_notices(&mut self) -> Vec<SlackNotice> {
        let notices = self.fetch_notices();
        self.new_notices_from_fetched(notices)
    }

    /// Verify Slack history access using the configured token + channel.
    pub fn check_connection(&self) -> Result<SlackCheckReport, SlackError> {
        let notices = self.fetch_from_api()?;

        Ok(SlackCheckReport {
            recent_notice_count: notices.len(),
            latest_notice: notices.into_iter().next(),
        })
    }

    /// Send a test message via Slack incoming webhook.
    pub fn send_test_message(&self, text: &str) -> Result<(), SlackError> {
        self.send_webhook_message(text)
    }

    /// Send a generic message via Slack incoming webhook.
    pub fn send_webhook_message(&self, text: &str) -> Result<(), SlackError> {
        let webhook_url = self
            .config
            .webhook_url
            .as_deref()
            .ok_or(SlackError::WebhookNotConfigured)?;

        #[derive(Serialize)]
        struct SlackWebhookPayload<'a> {
            text: &'a str,
        }

        ureq::post(webhook_url)
            .send_json(SlackWebhookPayload { text })
            .map_err(|e| SlackError::Request(e.to_string()))?;

        Ok(())
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

        Ok(Self::notices_from_messages(
            resp.messages.unwrap_or_default(),
        ))
    }

    /// Invalidate the cache so the next call hits the API.
    pub fn invalidate_cache(&mut self) {
        self.last_fetch = None;
    }

    /// Update configuration at runtime.
    pub fn update_config(&mut self, config: SlackConfig) {
        self.config = config;
        self.invalidate_cache();
        self.last_seen_timestamp = None;
    }
}

#[derive(Debug)]
pub enum SlackError {
    NotConfigured,
    WebhookNotConfigured,
    Request(String),
    Api(String),
}

impl std::fmt::Display for SlackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Slack is not configured"),
            Self::WebhookNotConfigured => write!(f, "Slack webhook is not configured"),
            Self::Request(e) => write!(f, "HTTP error: {e}"),
            Self::Api(e) => write!(f, "Slack API error: {e}"),
        }
    }
}

impl std::error::Error for SlackError {}

impl SlackNotifier {
    fn new_notices_from_fetched(&mut self, notices: Vec<SlackNotice>) -> Vec<SlackNotice> {
        let newest_timestamp = notices.iter().map(|notice| notice.timestamp).max();
        let Some(current_newest) = newest_timestamp else {
            return Vec::new();
        };

        let mut new_notices = match self.last_seen_timestamp {
            Some(last_seen) => notices
                .into_iter()
                .filter(|notice| notice.timestamp > last_seen)
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };

        new_notices.sort_by_key(|notice| notice.timestamp);
        self.last_seen_timestamp = Some(current_newest);
        new_notices
    }

    fn notices_from_messages(messages: Vec<SlackMessage>) -> Vec<SlackNotice> {
        messages
            .into_iter()
            .filter_map(|msg| {
                let text = msg.text?;
                let user = msg.user.unwrap_or_else(|| "unknown".into());
                let ts = msg
                    .ts
                    .as_deref()
                    .and_then(parse_slack_timestamp)
                    .unwrap_or_else(Utc::now);
                Some(SlackNotice {
                    text,
                    user,
                    timestamp: ts,
                })
            })
            .collect()
    }
}

fn parse_slack_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    let secs: f64 = raw.parse().ok()?;
    DateTime::from_timestamp(secs as i64, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_configured_requires_token_and_channel() {
        let mut config = SlackConfig::default();
        assert!(!config.is_configured());

        config.token = Some("xoxb-test".into());
        assert!(!config.is_configured());

        config.channel = Some("C123".into());
        assert!(config.is_configured());
    }

    #[test]
    fn test_has_webhook_detects_webhook_url() {
        let mut config = SlackConfig::default();
        assert!(!config.has_webhook());

        config.webhook_url = Some("https://hooks.slack.com/services/test".into());
        assert!(config.has_webhook());
    }

    #[test]
    fn test_notices_from_messages_filters_missing_text() {
        let notices = SlackNotifier::notices_from_messages(vec![
            SlackMessage {
                text: Some("hello".into()),
                user: Some("U123".into()),
                ts: Some("1711700000.123".into()),
            },
            SlackMessage {
                text: None,
                user: Some("U456".into()),
                ts: Some("1711700001.123".into()),
            },
        ]);

        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].text, "hello");
        assert_eq!(notices[0].user, "U123");
    }

    #[test]
    fn test_check_connection_requires_history_config() {
        let notifier = SlackNotifier::new(SlackConfig::default());
        let err = notifier.check_connection().unwrap_err();

        assert!(matches!(err, SlackError::NotConfigured));
    }

    #[test]
    fn test_send_test_message_requires_webhook() {
        let notifier = SlackNotifier::new(SlackConfig::default());
        let err = notifier.send_test_message("hello").unwrap_err();

        assert!(matches!(err, SlackError::WebhookNotConfigured));
    }

    #[test]
    fn test_fetch_new_notices_primes_then_returns_only_newer_messages() {
        let mut notifier = SlackNotifier::new(SlackConfig::default());
        let first = notifier.new_notices_from_fetched(vec![
            SlackNotice {
                text: "older".into(),
                user: "U1".into(),
                timestamp: DateTime::from_timestamp(100, 0).unwrap(),
            },
            SlackNotice {
                text: "newer".into(),
                user: "U2".into(),
                timestamp: DateTime::from_timestamp(200, 0).unwrap(),
            },
        ]);
        assert!(first.is_empty());

        let second = notifier.new_notices_from_fetched(vec![
            SlackNotice {
                text: "brand new".into(),
                user: "U3".into(),
                timestamp: DateTime::from_timestamp(300, 0).unwrap(),
            },
            SlackNotice {
                text: "still old".into(),
                user: "U2".into(),
                timestamp: DateTime::from_timestamp(200, 0).unwrap(),
            },
        ]);

        assert_eq!(second.len(), 1);
        assert_eq!(second[0].text, "brand new");
    }
}
