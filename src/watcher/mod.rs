use crate::banner::alerts::{Alert, AlertQueue, AlertType, Priority};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const DEFAULT_PATTERNS: &[&str] = &["done", "error", "complete", "failed"];
const DEFAULT_PATTERN_SYMBOLS: &[&str] = &["\u{2713}", "\u{2717}"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WatcherConfig {
    /// Regex patterns to match against stdout lines or file content.
    pub patterns: Vec<String>,
    /// File paths to watch for changes.
    pub watch_paths: Vec<PathBuf>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        let mut patterns: Vec<String> = DEFAULT_PATTERNS.iter().map(|s| s.to_string()).collect();
        patterns.extend(DEFAULT_PATTERN_SYMBOLS.iter().map(|s| s.to_string()));
        Self {
            patterns,
            watch_paths: vec![PathBuf::from("/tmp/agent-status.json")],
        }
    }
}

/// Compiled set of regex patterns for matching.
pub struct PatternMatcher {
    patterns: Vec<regex::Regex>,
}

impl PatternMatcher {
    pub fn compile(raw: &[String]) -> Self {
        let patterns = raw
            .iter()
            .filter_map(|p| {
                regex::Regex::new(&format!("(?i){p}")).ok()
            })
            .collect();
        Self { patterns }
    }

    fn find_match(&self, line: &str) -> Option<MatchResult> {
        for pat in &self.patterns {
            if let Some(m) = pat.find(line) {
                let matched = m.as_str().to_lowercase();
                let alert_type = classify_match(&matched);
                let priority = priority_for(&alert_type);
                return Some(MatchResult {
                    alert_type,
                    priority,
                    matched_text: m.as_str().to_string(),
                });
            }
        }
        None
    }
}

struct MatchResult {
    alert_type: AlertType,
    priority: Priority,
    matched_text: String,
}

fn classify_match(s: &str) -> AlertType {
    let lower = s.to_lowercase();
    if lower.contains("error") || lower.contains("failed") || lower.contains("\u{2717}") {
        AlertType::AgentError
    } else if lower.contains("done") || lower.contains("complete") || lower.contains("\u{2713}") {
        AlertType::AgentComplete
    } else {
        AlertType::AgentProgress
    }
}

fn priority_for(alert_type: &AlertType) -> Priority {
    match alert_type {
        AlertType::AgentError => Priority::High,
        AlertType::AgentComplete => Priority::Normal,
        AlertType::AgentProgress => Priority::Low,
        AlertType::Custom(_) => Priority::Normal,
    }
}

/// Scans a single line of text and pushes an alert if a pattern matches.
pub fn scan_line(line: &str, matcher: &PatternMatcher, queue: &Arc<Mutex<AlertQueue>>) {
    if let Some(result) = matcher.find_match(line) {
        let alert = Alert::new(result.alert_type, format!("{}: {}", result.matched_text, line))
            .with_priority(result.priority);
        if let Ok(mut q) = queue.lock() {
            q.push(alert);
        }
    }
}

/// A channel-based alert sender so watchers can emit alerts without holding the queue lock.
#[derive(Clone)]
pub struct AlertSender {
    tx: mpsc::UnboundedSender<Alert>,
}

impl AlertSender {
    pub fn send(&self, alert: Alert) {
        let _ = self.tx.send(alert);
    }
}

/// Spawn a background task that drains alerts from the channel into the queue.
pub fn spawn_alert_drain(
    queue: Arc<Mutex<AlertQueue>>,
) -> (AlertSender, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Alert>();
    let handle = tokio::spawn(async move {
        while let Some(alert) = rx.recv().await {
            if let Ok(mut q) = queue.lock() {
                q.push(alert);
            }
        }
    });
    (AlertSender { tx }, handle)
}

/// Spawn the stdout scanner task. Reads lines from the provided reader and scans them.
pub fn spawn_stdin_watcher(
    config: WatcherConfig,
    queue: Arc<Mutex<AlertQueue>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let matcher = PatternMatcher::compile(&config.patterns);
        let stdin = tokio::io::stdin();
        let reader = tokio::io::BufReader::new(stdin);
        use tokio::io::AsyncBufReadExt;
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            scan_line(&line, &matcher, &queue);
        }
    })
}

/// Spawn the file watcher task. Uses `notify` to observe file changes and read new content.
pub fn spawn_file_watcher(
    config: WatcherConfig,
    queue: Arc<Mutex<AlertQueue>>,
) -> Option<tokio::task::JoinHandle<()>> {
    if config.watch_paths.is_empty() {
        return None;
    }

    let paths = config.watch_paths.clone();
    let patterns = config.patterns.clone();

    Some(tokio::spawn(async move {
        let matcher = PatternMatcher::compile(&patterns);
        let queue_inner = queue.clone();

        let (tx, mut rx) = mpsc::unbounded_channel::<PathBuf>();

        let _watcher = {
            let tx = tx.clone();
            let mut watcher: RecommendedWatcher =
                match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                for path in event.paths {
                                    let _ = tx.send(path);
                                }
                            }
                            _ => {}
                        }
                    }
                }) {
                    Ok(w) => w,
                    Err(_) => return,
                };

            for path in &paths {
                // Watch the parent directory if the file doesn't exist yet
                let watch_target = if path.exists() {
                    path.clone()
                } else if let Some(parent) = path.parent() {
                    if parent.exists() {
                        parent.to_path_buf()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                let _ = watcher.watch(&watch_target, RecursiveMode::NonRecursive);
            }
            watcher
        };

        while let Some(changed_path) = rx.recv().await {
            // Only process paths we care about
            if !paths.iter().any(|p| *p == changed_path) {
                continue;
            }
            if let Ok(content) = tokio::fs::read_to_string(&changed_path).await {
                for line in content.lines() {
                    scan_line(line, &matcher, &queue_inner);
                }
            }
        }
    }))
}

/// Convenience: spawn both stdin and file watchers with the given config.
pub fn spawn_all_watchers(
    config: WatcherConfig,
    queue: Arc<Mutex<AlertQueue>>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();
    handles.push(spawn_stdin_watcher(config.clone(), queue.clone()));
    if let Some(h) = spawn_file_watcher(config, queue) {
        handles.push(h);
    }
    handles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_patterns() {
        let cfg = WatcherConfig::default();
        assert!(cfg.patterns.len() >= 6);
        assert!(!cfg.watch_paths.is_empty());
    }

    #[test]
    fn pattern_matcher_detects_error() {
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        let result = matcher.find_match("build failed with exit code 1");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.alert_type, AlertType::AgentError);
        assert_eq!(r.priority, Priority::High);
    }

    #[test]
    fn pattern_matcher_detects_completion() {
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        let result = matcher.find_match("task complete!");
        assert!(result.is_some());
        assert_eq!(result.unwrap().alert_type, AlertType::AgentComplete);
    }

    #[test]
    fn scan_line_pushes_to_queue() {
        let queue = Arc::new(Mutex::new(AlertQueue::default()));
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        scan_line("agent done processing", &matcher, &queue);
        let mut q = queue.lock().unwrap();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn scan_line_ignores_non_matching() {
        let queue = Arc::new(Mutex::new(AlertQueue::default()));
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        scan_line("just a regular log line", &matcher, &queue);
        let mut q = queue.lock().unwrap();
        assert!(q.is_empty());
    }
}
