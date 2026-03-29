pub mod breakout;
pub mod pacman;
pub mod snake;
pub mod tetris;

use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

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
