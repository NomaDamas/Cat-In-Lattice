use crossterm::event::KeyCode;
use notify::RecommendedWatcher;

use crate::banner::{AlertQueue, QuoteRotator, SlackNotifier};
use crate::cat::animation::AnimationController;
use crate::cat::events::{EventScheduler, EventType};
use crate::cat::state::CatState;
use crate::config::Config;
use crate::games::{Game, GameRecord, GameType};
use crate::layout::LayoutMode;
use crate::persistence::Persistence;
use crate::watcher;

use std::collections::VecDeque;
use std::process::Command;
use std::sync::mpsc;
use std::time::Instant;

use crate::banner::alerts::Alert;

/// Auto-save interval in seconds.
const AUTOSAVE_INTERVAL_SECS: u64 = 60;

/// Slack poll interval in seconds.
const SLACK_POLL_INTERVAL_SECS: u64 = 300;
const PANE_FOCUS_POLL_INTERVAL_MS: u64 = 250;

/// Main application state that ties every subsystem together.
pub struct App {
    pub cat_state: CatState,
    pub animation: AnimationController,
    pub event_scheduler: EventScheduler,
    pub quote_rotator: QuoteRotator,
    pub alert_queue: AlertQueue,
    pub active_game: Option<Box<dyn Game>>,
    pub recent_games: VecDeque<GameRecord>,
    pub layout_mode: LayoutMode,
    pub config: Config,
    pub should_quit: bool,
    pub show_help: bool,
    persistence: Persistence,
    last_save: Instant,
    last_tick: Instant,
    last_slack_poll: Instant,
    slack_notifier: SlackNotifier,
    active_game_type: Option<GameType>,
    active_game_recorded: bool,
    pane_is_active: bool,
    tmux_pane_id: Option<String>,
    last_pane_focus_check: Instant,
    // Watcher channel and handle (kept alive for the lifetime of the app)
    watcher_rx: Option<mpsc::Receiver<Alert>>,
    #[allow(dead_code)]
    _watcher_handle: Option<RecommendedWatcher>,
}

impl App {
    /// Build a new App from the given config. Loads persisted state or creates
    /// a fresh cat.
    pub fn new(config: Config) -> Self {
        let persistence = Persistence::new(&config.data_dir);
        let cat_state = persistence.load();
        let recent_games = persistence.load_game_records().into_iter().collect();

        let mut event_scheduler = EventScheduler::new();
        event_scheduler.active_hour_start = config.active_hours.0 as u8;
        event_scheduler.active_hour_end = config.active_hours.1 as u8;
        event_scheduler.events_per_day = config.events_per_day;
        event_scheduler.ensure_today_events(&cat_state);

        let mut animation = AnimationController::new();
        animation.set_from_mood(cat_state.mood);

        let slack_notifier = SlackNotifier::new(config.slack.clone());

        // Start file watcher
        let (watcher_rx, watcher_handle) = match watcher::spawn_file_watcher(&config.watcher) {
            Some((rx, handle)) => (Some(rx), Some(handle)),
            None => (None, None),
        };

        Self {
            cat_state,
            animation,
            event_scheduler,
            quote_rotator: QuoteRotator::new(),
            alert_queue: AlertQueue::default(),
            active_game: None,
            recent_games,
            layout_mode: LayoutMode::Default,
            config,
            should_quit: false,
            show_help: false,
            persistence,
            last_save: Instant::now(),
            last_tick: Instant::now(),
            last_slack_poll: Instant::now(),
            slack_notifier,
            active_game_type: None,
            active_game_recorded: false,
            pane_is_active: true,
            tmux_pane_id: std::env::var("TMUX_PANE").ok(),
            last_pane_focus_check: Instant::now(),
            watcher_rx,
            _watcher_handle: watcher_handle,
        }
    }

    /// Called every frame. Advances time-based systems.
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        let mut needs_render = false;

        if now.duration_since(self.last_pane_focus_check).as_millis()
            >= PANE_FOCUS_POLL_INTERVAL_MS as u128
        {
            self.last_pane_focus_check = now;
            let pane_is_active = self.query_pane_active();
            if pane_is_active != self.pane_is_active {
                self.pane_is_active = pane_is_active;
                needs_render = true;
            }
        }

        if !self.pane_is_active && !self.alert_queue.is_empty() {
            self.alert_queue.clear();
            needs_render = true;
        }

        // Cat vital ticks
        self.cat_state.tick_hunger();
        self.cat_state.tick_affinity_decay();

        // Ensure today's events exist
        self.event_scheduler.ensure_today_events(&self.cat_state);

        // Process expired events
        let missed = self.event_scheduler.process_expired(&mut self.cat_state);
        if missed > 0 {
            self.animation.play_angry();
            needs_render = true;
        }

        // Advance animation (tick first so transient animations can expire)
        if self.animation.tick() {
            needs_render = true;
        }

        // Sync animation to mood only when idle (don't override transient animations)
        use crate::cat::animation::AnimationState;
        if self.animation.state() == AnimationState::Idle {
            self.animation.set_from_mood(self.cat_state.mood);
        }

        // Advance active game
        let mut completed_record = None;
        if let Some(game) = &mut self.active_game {
            game.update(dt);
            needs_render = true;
            if game.is_game_over() && !self.active_game_recorded {
                self.cat_state.affinity = (self.cat_state.affinity + 0.5).min(100.0);
                completed_record = Some(game.record());
                self.active_game_recorded = true;
            }
        }
        if let Some(record) = completed_record {
            self.push_recent_game(record);
            needs_render = true;
        }

        // Drain watcher alerts
        if let Some(rx) = &self.watcher_rx {
            let mut pending_alerts = Vec::new();
            while let Ok(alert) = rx.try_recv() {
                pending_alerts.push(alert);
            }
            for alert in pending_alerts {
                needs_render |= self.queue_alert(alert);
            }
        }

        // Periodic Slack poll (in background thread to avoid blocking)
        if now.duration_since(self.last_slack_poll).as_secs() >= SLACK_POLL_INTERVAL_SECS {
            self.last_slack_poll = now;
            let notices = self.slack_notifier.fetch_new_notices();
            for notice in notices {
                use crate::banner::alerts::{AlertType, Priority};
                let alert = Alert::new(AlertType::Custom("Slack".to_string()), notice.to_string())
                    .with_priority(Priority::Normal);
                needs_render |= self.queue_alert(alert);
            }
        }

        // Auto-save
        if now.duration_since(self.last_save).as_secs() >= AUTOSAVE_INTERVAL_SECS {
            let _ = self.persistence.save(&self.cat_state);
            self.last_save = now;
        }

        needs_render
    }

    pub fn notice_overlay_visible(&self) -> bool {
        self.pane_is_active
    }

    /// Route a key press to the appropriate handler.
    pub fn handle_key(&mut self, key: KeyCode) {
        // Global keys that always work
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.show_help = !self.show_help;
                return;
            }
            KeyCode::Esc => {
                if self.active_game.is_some() {
                    self.exit_game();
                    return;
                }
                if self.show_help {
                    self.show_help = false;
                    return;
                }
                self.should_quit = true;
                return;
            }
            _ => {}
        }

        // If a game is active, route input there
        if matches!(key, KeyCode::Char('r') | KeyCode::Char('R')) && self.active_game.is_some() {
            self.restart_game();
            return;
        }
        if let Some(game) = &mut self.active_game {
            game.handle_input(key);
            return;
        }

        // If there's an active event, respond to it
        if self.event_scheduler.active_event().is_some() {
            match key {
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(event_type) =
                        self.event_scheduler.respond_to_active(&mut self.cat_state)
                    {
                        match event_type {
                            EventType::Pet => self.animation.play_happy(),
                            EventType::Feed => self.animation.play_eating(),
                            EventType::Special(_) => self.animation.play_happy(),
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        // Cat interactions and game launching
        match key {
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.cat_state.pet();
                self.animation.play_happy();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.cat_state.feed();
                self.animation.play_eating();
            }
            KeyCode::Char('1') => self.enter_game(GameType::Pacman),
            KeyCode::Char('2') => self.enter_game(GameType::Snake),
            KeyCode::Char('3') => self.enter_game(GameType::Tetris),
            KeyCode::Char('4') => self.enter_game(GameType::Breakout),
            _ => {}
        }
    }

    /// Start a mini-game.
    fn enter_game(&mut self, game_type: GameType) {
        self.active_game = Some(game_type.create());
        self.active_game_type = Some(game_type);
        self.active_game_recorded = false;
        self.layout_mode = LayoutMode::Gaming;
    }

    /// Exit the current mini-game.
    fn exit_game(&mut self) {
        if let Some(game) = self.active_game.take() {
            if !self.active_game_recorded && (game.score() > 0 || game.is_game_over()) {
                self.push_recent_game(game.record());
            }
        }
        self.active_game = None;
        self.active_game_type = None;
        self.active_game_recorded = false;
        self.layout_mode = LayoutMode::Default;
    }

    fn restart_game(&mut self) {
        let Some(game_type) = self.active_game_type else {
            return;
        };

        if let Some(game) = self.active_game.take() {
            if !self.active_game_recorded && (game.score() > 0 || game.is_game_over()) {
                let mut record = game.record();
                if !game.is_game_over() {
                    record.outcome = "Restarted".to_string();
                }
                self.push_recent_game(record);
            }
        }

        self.active_game = Some(game_type.create());
        self.active_game_recorded = false;
        self.layout_mode = LayoutMode::Gaming;
    }

    fn push_recent_game(&mut self, record: GameRecord) {
        self.recent_games.push_front(record);
        while self.recent_games.len() > 5 {
            self.recent_games.pop_back();
        }
    }

    fn queue_alert(&mut self, alert: Alert) -> bool {
        if !self.pane_is_active {
            return false;
        }

        self.alert_queue.push(alert);
        true
    }

    /// Save state to disk (call on exit).
    pub fn save(&self) {
        if let Err(e) = self.persistence.save(&self.cat_state) {
            eprintln!("Warning: failed to save cat state: {e}");
        }
        let recent_games = self.recent_games.iter().cloned().collect::<Vec<_>>();
        if let Err(e) = self.persistence.save_game_records(&recent_games) {
            eprintln!("Warning: failed to save game records: {e}");
        }
    }

    fn query_pane_active(&self) -> bool {
        let Some(pane_id) = self.tmux_pane_id.as_deref() else {
            return true;
        };

        Command::new("tmux")
            .args(["display-message", "-p", "-t", pane_id, "#{pane_active}"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "1")
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::banner::alerts::AlertType;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_app() -> App {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("cil-app-test-{suffix}"));
        fs::create_dir_all(&data_dir).unwrap();

        let mut config = Config {
            data_dir,
            ..Config::default()
        };
        config.watcher.watch_paths.clear();

        App::new(config)
    }

    #[test]
    fn queue_alert_drops_messages_while_inactive() {
        let mut app = test_app();
        app.pane_is_active = false;

        let queued = app.queue_alert(Alert::new(AlertType::Custom("NOTICE".into()), "hidden"));

        assert!(!queued);
        assert!(app.alert_queue.is_empty());
    }

    #[test]
    fn queue_alert_keeps_messages_while_active() {
        let mut app = test_app();
        app.pane_is_active = true;

        let queued = app.queue_alert(Alert::new(AlertType::Custom("NOTICE".into()), "visible"));

        assert!(queued);
        assert_eq!(app.alert_queue.len(), 1);
    }
}
