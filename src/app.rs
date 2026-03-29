use crossterm::event::KeyCode;
use notify::RecommendedWatcher;

use crate::banner::{AlertQueue, QuoteRotator, SlackNotifier};
use crate::cat::animation::AnimationController;
use crate::cat::events::{EventScheduler, EventType};
use crate::cat::state::CatState;
use crate::config::Config;
use crate::games::{Game, GameType};
use crate::layout::LayoutMode;
use crate::persistence::Persistence;
use crate::watcher;

use std::sync::mpsc;
use std::time::Instant;

use crate::banner::alerts::Alert;

/// Auto-save interval in seconds.
const AUTOSAVE_INTERVAL_SECS: u64 = 60;

/// Slack poll interval in seconds.
const SLACK_POLL_INTERVAL_SECS: u64 = 300;

/// Main application state that ties every subsystem together.
pub struct App {
    pub cat_state: CatState,
    pub animation: AnimationController,
    pub event_scheduler: EventScheduler,
    pub quote_rotator: QuoteRotator,
    pub alert_queue: AlertQueue,
    pub active_game: Option<Box<dyn Game>>,
    pub layout_mode: LayoutMode,
    pub config: Config,
    pub should_quit: bool,
    pub show_help: bool,
    persistence: Persistence,
    last_save: Instant,
    last_tick: Instant,
    last_slack_poll: Instant,
    slack_notifier: SlackNotifier,
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

        let mut event_scheduler = EventScheduler::new();
        event_scheduler.active_hour_start = config.active_hours.0 as u8;
        event_scheduler.active_hour_end = config.active_hours.1 as u8;
        event_scheduler.events_per_day = config.events_per_day;
        event_scheduler.ensure_today_events(&cat_state);

        let mut animation = AnimationController::new();
        animation.set_from_mood(cat_state.mood);

        let slack_notifier = SlackNotifier::new(config.slack.clone());

        // Start file watcher
        let (watcher_rx, watcher_handle) =
            match watcher::spawn_file_watcher(&config.watcher) {
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
            layout_mode: LayoutMode::Default,
            config,
            should_quit: false,
            show_help: false,
            persistence,
            last_save: Instant::now(),
            last_tick: Instant::now(),
            last_slack_poll: Instant::now(),
            slack_notifier,
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

        // Sync animation to mood
        self.animation.set_from_mood(self.cat_state.mood);

        // Advance animation
        if self.animation.tick() {
            needs_render = true;
        }

        // Advance active game
        if let Some(game) = &mut self.active_game {
            game.update(dt);
            needs_render = true;
            if game.is_game_over() {
                self.cat_state.affinity = (self.cat_state.affinity + 0.5).min(100.0);
            }
        }

        // Drain watcher alerts
        if let Some(rx) = &self.watcher_rx {
            while let Ok(alert) = rx.try_recv() {
                self.alert_queue.push(alert);
                needs_render = true;
            }
        }

        // Periodic Slack poll (in background thread to avoid blocking)
        if now.duration_since(self.last_slack_poll).as_secs() >= SLACK_POLL_INTERVAL_SECS {
            self.last_slack_poll = now;
            let notices = self.slack_notifier.fetch_notices();
            for notice in notices.into_iter().take(3) {
                use crate::banner::alerts::{AlertType, Priority};
                let alert = Alert::new(
                    AlertType::Custom("Slack".to_string()),
                    notice.to_string(),
                )
                .with_priority(Priority::Normal);
                self.alert_queue.push(alert);
            }
        }

        // Auto-save
        if now.duration_since(self.last_save).as_secs() >= AUTOSAVE_INTERVAL_SECS {
            let _ = self.persistence.save(&self.cat_state);
            self.last_save = now;
        }

        needs_render
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
        self.layout_mode = LayoutMode::Gaming;
    }

    /// Exit the current mini-game.
    fn exit_game(&mut self) {
        self.active_game = None;
        self.layout_mode = LayoutMode::Default;
    }

    /// Save state to disk (call on exit).
    pub fn save(&self) {
        if let Err(e) = self.persistence.save(&self.cat_state) {
            eprintln!("Warning: failed to save cat state: {e}");
        }
    }
}
