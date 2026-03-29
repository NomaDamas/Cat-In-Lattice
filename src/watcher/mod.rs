use crate::banner::alerts::{Alert, AlertType, Priority};
use notify::RecommendedWatcher;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const DEFAULT_PATTERNS: &[&str] = &["done", "error", "complete", "failed"];
const DEFAULT_NOTICE_PATTERNS: &[&str] = &["announcement", "announce", "notice", "공지"];
const DEFAULT_PATTERN_SYMBOLS: &[&str] = &["\u{2713}", "\u{2717}"];
pub const DEFAULT_STATUS_PATH: &str = "/tmp/agent-status.json";
pub const DEFAULT_ANNOUNCEMENT_PATH: &str = "/tmp/cat-in-lattice-announcements.log";

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
        patterns.extend(DEFAULT_NOTICE_PATTERNS.iter().map(|s| s.to_string()));
        patterns.extend(DEFAULT_PATTERN_SYMBOLS.iter().map(|s| s.to_string()));
        Self {
            patterns,
            watch_paths: vec![
                PathBuf::from(DEFAULT_STATUS_PATH),
                PathBuf::from(DEFAULT_ANNOUNCEMENT_PATH),
            ],
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
    } else if lower.contains("announce") || lower.contains("notice") || lower.contains("공지") {
        AlertType::Custom("NOTICE".to_string())
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
        let message = match result.alert_type {
            AlertType::Custom(_) => line.trim().to_string(),
            _ => format!("{}: {}", result.matched_text, line),
        };

        Alert::new(result.alert_type, message).with_priority(result.priority)
    })
}

/// Append a human announcement to the watched notice log so the app can show it
/// without needing Slack read permissions.
pub fn append_announcement_line(message: &str) -> std::io::Result<PathBuf> {
    let path = PathBuf::from(DEFAULT_ANNOUNCEMENT_PATH);
    write_line(&path, &format!("announcement: {message}"))?;
    Ok(path)
}

fn write_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
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

    let watcher = match notify::recommended_watcher(|_| {}) {
        Ok(w) => w,
        Err(_) => return None,
    };

    // Spawn a polling thread so alerts keep working even when platform file
    // notifications are unreliable for symlinked temp paths or tmux workflows.
    std::thread::spawn(move || {
        let matcher = PatternMatcher::compile(&patterns);
        let mut last_seen_contents: HashMap<PathBuf, String> = HashMap::new();

        for path in &watch_paths {
            last_seen_contents.insert(
                path.clone(),
                std::fs::read_to_string(path).unwrap_or_default(),
            );
        }

        loop {
            for path in &watch_paths {
                let content = match std::fs::read_to_string(path) {
                    Ok(content) => content,
                    Err(_) => {
                        last_seen_contents.insert(path.clone(), String::new());
                        continue;
                    }
                };

                let previous = last_seen_contents.entry(path.clone()).or_default();
                if previous == &content {
                    continue;
                }

                *previous = content.clone();
                for line in content.lines() {
                    if let Some(alert) = scan_line_to_alert(line, &matcher) {
                        if alert_tx.send(alert).is_err() {
                            return; // main thread dropped the receiver
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(500));
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
        assert!(cfg.patterns.len() >= 10);
        assert!(cfg
            .watch_paths
            .contains(&PathBuf::from(DEFAULT_STATUS_PATH)));
        assert!(cfg
            .watch_paths
            .contains(&PathBuf::from(DEFAULT_ANNOUNCEMENT_PATH)));
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

    #[test]
    fn announcement_line_becomes_notice_alert() {
        let cfg = WatcherConfig::default();
        let matcher = PatternMatcher::compile(&cfg.patterns);
        let alert = scan_line_to_alert("announcement: deploy in 10 minutes", &matcher).unwrap();

        assert_eq!(alert.alert_type, AlertType::Custom("NOTICE".into()));
        assert_eq!(alert.priority, Priority::Normal);
        assert_eq!(alert.message, "announcement: deploy in 10 minutes");
    }

    #[test]
    fn append_announcement_writes_expected_line() {
        let tmp = std::env::temp_dir().join("cil-watch-append-announcement.log");
        let _ = std::fs::remove_file(&tmp);

        write_line(&tmp, "announcement: hello").unwrap();

        let contents = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(contents, "announcement: hello\n");

        write_line(&tmp, "announcement: updated").unwrap();
        let updated = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(updated, "announcement: updated\n");

        let _ = std::fs::remove_file(&tmp);
    }
}
