use crate::banner::alerts::{Alert, AlertType, Priority};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;

const DEFAULT_PATTERNS: &[&str] = &["done", "error", "complete", "failed"];
const DEFAULT_PATTERN_SYMBOLS: &[&str] = &["\u{2713}", "\u{2717}"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WatcherConfig {
    /// Regex patterns to match against file content.
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
            .filter_map(|p| regex::Regex::new(&format!("(?i){p}")).ok())
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

/// Scans a single line of text and returns an alert if a pattern matches.
pub fn scan_line_to_alert(line: &str, matcher: &PatternMatcher) -> Option<Alert> {
    matcher.find_match(line).map(|result| {
        Alert::new(result.alert_type, format!("{}: {}", result.matched_text, line))
            .with_priority(result.priority)
    })
}

/// Spawn a background file watcher thread.
/// Returns a receiver that yields alerts when watched files change and match patterns.
/// The watcher handle is returned so the caller can keep it alive.
pub fn spawn_file_watcher(
    config: &WatcherConfig,
) -> Option<(mpsc::Receiver<Alert>, RecommendedWatcher)> {
    if config.watch_paths.is_empty() {
        return None;
    }

    let (alert_tx, alert_rx) = mpsc::channel::<Alert>();
    let patterns = config.patterns.clone();
    let watch_paths = config.watch_paths.clone();

    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<PathBuf>();

    let mut watcher: RecommendedWatcher =
        match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            let _ = notify_tx.send(path);
                        }
                    }
                    _ => {}
                }
            }
        }) {
            Ok(w) => w,
            Err(_) => return None,
        };

    for path in &watch_paths {
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

    // Spawn a thread to process file change notifications
    std::thread::spawn(move || {
        let matcher = PatternMatcher::compile(&patterns);
        while let Ok(changed_path) = notify_rx.recv() {
            if !watch_paths.contains(&changed_path) {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&changed_path) {
                for line in content.lines() {
                    if let Some(alert) = scan_line_to_alert(line, &matcher) {
                        if alert_tx.send(alert).is_err() {
                            return; // main thread dropped the receiver
                        }
                    }
                }
            }
        }
    });

    Some((alert_rx, watcher))
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
    fn scan_line_returns_alert() {
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        let alert = scan_line_to_alert("agent done processing", &matcher);
        assert!(alert.is_some());
    }

    #[test]
    fn scan_line_ignores_non_matching() {
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        let alert = scan_line_to_alert("just a regular log line", &matcher);
        assert!(alert.is_none());
    }
}
