pub mod breakout;
pub mod pacman;
pub mod snake;
pub mod tetris;

use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameLives {
    pub current: u32,
    pub max: u32,
}

impl GameLives {
    pub const fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameHud {
    pub score: u32,
    pub lives: Option<GameLives>,
    pub status: Option<String>,
    pub details: Vec<String>,
}

impl GameHud {
    pub fn simple(score: u32) -> Self {
        Self {
            score,
            lives: None,
            status: None,
            details: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRecord {
    pub game: String,
    pub score: u32,
    pub outcome: String,
}

impl GameRecord {
    pub fn new(game: impl Into<String>, score: u32, outcome: impl Into<String>) -> Self {
        Self {
            game: game.into(),
            score,
            outcome: outcome.into(),
        }
    }

    pub fn summary(&self) -> String {
        format!("{} · {} · {}", self.game, self.outcome, self.score)
    }
}

/// Shared trait every mini-game must implement.
pub trait Game {
    /// Advance the simulation by `dt` seconds.
    fn update(&mut self, dt: f64);
    /// React to a single key press.
    fn handle_input(&mut self, key: KeyCode);
    /// Draw the current state into the given area.
    fn render(&self, area: Rect, buf: &mut Buffer);
    /// Has the player lost (or won)?
    fn is_game_over(&self) -> bool;
    /// Current score.
    fn score(&self) -> u32;
    /// Human-readable name shown in the UI chrome.
    fn name(&self) -> &str;
    /// Shared footer/HUD data rendered outside the playfield.
    fn hud(&self) -> GameHud {
        GameHud::simple(self.score())
    }
    /// Summary stored in the recent-games list.
    fn record(&self) -> GameRecord {
        let outcome = if self.is_game_over() {
            "Finished"
        } else {
            "Quit"
        };
        GameRecord::new(self.name(), self.score(), outcome)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameType {
    Pacman,
    Snake,
    Tetris,
    Breakout,
}

impl GameType {
    /// Instantiate the selected game as a boxed trait object.
    pub fn create(&self) -> Box<dyn Game> {
        match self {
            GameType::Pacman => Box::new(pacman::PacmanGame::new()),
            GameType::Snake => Box::new(snake::SnakeGame::new()),
            GameType::Tetris => Box::new(tetris::TetrisGame::new()),
            GameType::Breakout => Box::new(breakout::BreakoutGame::new()),
        }
    }
}
