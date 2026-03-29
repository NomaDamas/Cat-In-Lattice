use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::time::Duration;

const DEFAULT_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertType {
    AgentComplete,
    AgentError,
    AgentProgress,
    Custom(String),
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentComplete => write!(f, "COMPLETE"),
            Self::AgentError => write!(f, "ERROR"),
            Self::AgentProgress => write!(f, "PROGRESS"),
            Self::Custom(label) => write!(f, "{label}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub alert_type: AlertType,
    pub message: String,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub ttl: Duration,
}

impl Alert {
    pub fn new(alert_type: AlertType, message: impl Into<String>) -> Self {
        Self {
            alert_type,
            message: message.into(),
            priority: Priority::Normal,
            created_at: Utc::now(),
            ttl: DEFAULT_TTL,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .to_std()
            .unwrap_or(Duration::ZERO);
        elapsed >= self.ttl
    }

    /// Returns true if this is a high-priority alert that should override quotes.
    pub fn is_high_priority(&self) -> bool {
        self.priority >= Priority::High
    }
}

impl std::fmt::Display for Alert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.alert_type, self.message)
    }
}

/// A bounded alert queue that automatically prunes expired entries.
pub struct AlertQueue {
    alerts: VecDeque<Alert>,
    max_size: usize,
}

impl AlertQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            alerts: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push an alert, pruning expired entries and evicting oldest if at capacity.
    pub fn push(&mut self, alert: Alert) {
        self.prune_expired();
        if self.alerts.len() >= self.max_size {
            self.alerts.pop_front();
        }
        self.alerts.push_back(alert);
    }

    /// Remove all expired alerts.
    pub fn prune_expired(&mut self) {
        self.alerts.retain(|a| !a.is_expired());
    }

    /// Get the highest-priority active alert (most recent among ties).
    pub fn top(&mut self) -> Option<&Alert> {
        self.prune_expired();
        self.alerts.iter().max_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then(a.created_at.cmp(&b.created_at))
        })
    }

    /// Returns true if any active high-priority alert exists.
    pub fn has_high_priority(&mut self) -> bool {
        self.prune_expired();
        self.alerts.iter().any(|a| a.is_high_priority())
    }

    /// All active (non-expired) alerts, ordered by priority descending.
    pub fn active(&mut self) -> Vec<&Alert> {
        self.prune_expired();
        let mut refs: Vec<&Alert> = self.alerts.iter().collect();
        refs.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(b.created_at.cmp(&a.created_at))
        });
        refs
    }

    /// Number of active alerts.
    pub fn len(&mut self) -> usize {
        self.prune_expired();
        self.alerts.len()
    }

    /// True if no active alerts remain.
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// Drain all alerts.
    pub fn clear(&mut self) {
        self.alerts.clear();
    }
}

impl Default for AlertQueue {
    fn default() -> Self {
        Self::new(64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_display() {
        let a = Alert::new(AlertType::AgentComplete, "build finished");
        assert_eq!(format!("{a}"), "[COMPLETE] build finished");
    }

    #[test]
    fn queue_respects_max_size() {
        let mut q = AlertQueue::new(2);
        q.push(Alert::new(AlertType::AgentProgress, "a"));
        q.push(Alert::new(AlertType::AgentProgress, "b"));
        q.push(Alert::new(AlertType::AgentProgress, "c"));
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn high_priority_overrides() {
        let mut q = AlertQueue::default();
        q.push(Alert::new(AlertType::AgentProgress, "low"));
        q.push(Alert::new(AlertType::AgentError, "critical").with_priority(Priority::Critical));
        let top = q.top().unwrap();
        assert_eq!(top.priority, Priority::Critical);
    }

    #[test]
    fn expired_alerts_are_pruned() {
        let mut q = AlertQueue::default();
        q.push(Alert::new(AlertType::AgentProgress, "short").with_ttl(Duration::ZERO));
        assert!(q.is_empty());
    }
}
