use super::Game;
use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};

const WIDTH: usize = 30;
const HEIGHT: usize = 15;
const MOVE_INTERVAL: f64 = 0.15; // seconds between moves

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

pub struct SnakeGame {
    body: Vec<(i32, i32)>,
    dir: Dir,
    next_dir: Dir,
    food: (i32, i32),
    score: u32,
    game_over: bool,
    timer: f64,
}

impl SnakeGame {
    pub fn new() -> Self {
        let cx = WIDTH as i32 / 2;
        let cy = HEIGHT as i32 / 2;
        let body = vec![(cx, cy), (cx - 1, cy), (cx - 2, cy)];
        let mut g = Self {
            body,
            dir: Dir::Right,
            next_dir: Dir::Right,
            food: (0, 0),
            score: 0,
            game_over: false,
            timer: 0.0,
        };
        g.spawn_food();
        g
    }

    fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let pos = (
                rng.gen_range(0..WIDTH as i32),
                rng.gen_range(0..HEIGHT as i32),
            );
            if !self.body.contains(&pos) {
                self.food = pos;
                return;
            }
        }
    }

    fn step(&mut self) {
        if self.game_over {
            return;
        }
        self.dir = self.next_dir;
        let (dx, dy) = self.dir.delta();
        let head = self.body[0];
        let new_head = (head.0 + dx, head.1 + dy);

        // wall collision
        if new_head.0 < 0
            || new_head.0 >= WIDTH as i32
            || new_head.1 < 0
            || new_head.1 >= HEIGHT as i32
        {
            self.game_over = true;
            return;
        }
        // self collision
        if self.body.contains(&new_head) {
            self.game_over = true;
            return;
        }

        self.body.insert(0, new_head);

        if new_head == self.food {
            self.score += 1;
            self.spawn_food();
        } else {
            self.body.pop();
        }
    }
}

impl Game for SnakeGame {
    fn update(&mut self, dt: f64) {
        if self.game_over {
            return;
        }
        self.timer += dt;
        while self.timer >= MOVE_INTERVAL {
            self.timer -= MOVE_INTERVAL;
            self.step();
        }
    }

    fn handle_input(&mut self, key: KeyCode) {
        let new = match key {
            KeyCode::Up => Dir::Up,
            KeyCode::Down => Dir::Down,
            KeyCode::Left => Dir::Left,
            KeyCode::Right => Dir::Right,
            _ => return,
        };
        // Prevent reversing into yourself
        if new != self.dir.opposite() {
            self.next_dir = new;
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Guard: if the area is too small to draw anything, skip rendering.
        if area.width < 3 || area.height < 3 {
            return;
        }
        // Border
        let border_style = Style::default().fg(Color::DarkGray);
        // top/bottom border
        for x in 0..=(WIDTH as u16 + 1).min(area.width.saturating_sub(1)) {
            if area.y < area.bottom() {
                buf[Position::new(area.x + x, area.y)].set_char('─').set_style(border_style);
            }
            let bot = area.y + HEIGHT as u16 + 1;
            if bot < area.bottom() {
                buf[Position::new(area.x + x, bot)].set_char('─').set_style(border_style);
            }
        }
        for y in 0..=(HEIGHT as u16 + 1).min(area.height.saturating_sub(1)) {
            buf[Position::new(area.x, area.y + y)].set_char('│').set_style(border_style);
            let right = area.x + WIDTH as u16 + 1;
            if right < area.right() {
                buf[Position::new(right, area.y + y)].set_char('│').set_style(border_style);
            }
        }
        // corners
        buf[Position::new(area.x, area.y)].set_char('┌').set_style(border_style);
        if area.x + WIDTH as u16 + 1 < area.right() {
            buf[Position::new(area.x + WIDTH as u16 + 1, area.y)].set_char('┐').set_style(border_style);
        }
        let bot = area.y + HEIGHT as u16 + 1;
        if bot < area.bottom() {
            buf[Position::new(area.x, bot)].set_char('└').set_style(border_style);
            if area.x + WIDTH as u16 + 1 < area.right() {
                buf[Position::new(area.x + WIDTH as u16 + 1, bot)].set_char('┘').set_style(border_style);
            }
        }

        let ox = area.x + 1;
        let oy = area.y + 1;

        // Food
        let (fx, fy) = self.food;
        let px = ox + fx as u16;
        let py = oy + fy as u16;
        if px < area.right() && py < area.bottom() {
            buf[Position::new(px, py)]
                .set_char('●')
                .set_style(Style::default().fg(Color::Red));
        }

        // Snake body
        for (i, &(sx, sy)) in self.body.iter().enumerate() {
            let px = ox + sx as u16;
            let py = oy + sy as u16;
            if px < area.right() && py < area.bottom() {
                let ch = if i == 0 { '█' } else { '▓' };
                let color = if i == 0 { Color::Green } else { Color::LightGreen };
                buf[Position::new(px, py)]
                    .set_char(ch)
                    .set_style(Style::default().fg(color));
            }
        }

        // Score line
        let score_y = area.y + HEIGHT as u16 + 2;
        if score_y < area.bottom() {
            let msg = if self.game_over {
                format!("GAME OVER  Score: {}", self.score)
            } else {
                format!("Score: {}", self.score)
            };
            let style = if self.game_over {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Yellow)
            };
            for (i, ch) in msg.chars().enumerate() {
                let px = area.x + i as u16;
                if px < area.right() {
                    buf[Position::new(px, score_y)].set_char(ch).set_style(style);
                }
            }
        }
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn name(&self) -> &str {
        "Snake"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let game = SnakeGame::new();
        assert!(!game.game_over);
        assert_eq!(game.score, 0);
        assert_eq!(game.body.len(), 3);
        assert_eq!(game.dir, Dir::Right);
    }

    #[test]
    fn test_initial_food_not_on_body() {
        let game = SnakeGame::new();
        assert!(!game.body.contains(&game.food));
    }

    #[test]
    fn test_movement_right() {
        let mut game = SnakeGame::new();
        let head_before = game.body[0];
        game.step();
        if !game.game_over {
            let head_after = game.body[0];
            assert_eq!(head_after.0, head_before.0 + 1);
            assert_eq!(head_after.1, head_before.1);
        }
    }

    #[test]
    fn test_direction_change() {
        let mut game = SnakeGame::new();
        game.handle_input(KeyCode::Down);
        assert_eq!(game.next_dir, Dir::Down);
    }

    #[test]
    fn test_cannot_reverse() {
        let mut game = SnakeGame::new();
        // Initial direction is Right, so Left should be blocked
        game.handle_input(KeyCode::Left);
        assert_eq!(game.next_dir, Dir::Right);
    }

    #[test]
    fn test_wall_collision() {
        let mut game = SnakeGame::new();
        // Place snake at right edge heading right
        game.body = vec![(WIDTH as i32 - 1, 5), (WIDTH as i32 - 2, 5), (WIDTH as i32 - 3, 5)];
        game.dir = Dir::Right;
        game.next_dir = Dir::Right;
        game.step();
        assert!(game.game_over);
    }

    #[test]
    fn test_self_collision() {
        let mut game = SnakeGame::new();
        // Create a body that will collide with itself
        game.body = vec![(5, 5), (6, 5), (6, 6), (5, 6), (4, 6), (4, 5)];
        game.dir = Dir::Up;
        game.next_dir = Dir::Up;
        // Head at (5,5), moving up to (5,4), no collision yet
        // But let's make it collide: head at (5,5) moving left
        game.dir = Dir::Left;
        game.next_dir = Dir::Left;
        game.step();
        // (4,5) is in the body, so this should be game over
        assert!(game.game_over);
    }

    #[test]
    fn test_update_does_nothing_when_game_over() {
        let mut game = SnakeGame::new();
        game.game_over = true;
        let score_before = game.score;
        game.update(10.0);
        assert_eq!(game.score, score_before);
    }

    #[test]
    fn test_score_starts_zero() {
        let game = SnakeGame::new();
        assert_eq!(game.score(), 0);
        assert!(!game.is_game_over());
        assert_eq!(game.name(), "Snake");
    }

    #[test]
    fn test_render_small_area_no_panic() {
        let game = SnakeGame::new();
        let area = Rect::new(0, 0, 2, 2);
        let mut buf = Buffer::empty(area);
        game.render(area, &mut buf);
        // Should not panic
    }

    #[test]
    fn test_dir_opposites() {
        assert_eq!(Dir::Up.opposite(), Dir::Down);
        assert_eq!(Dir::Down.opposite(), Dir::Up);
        assert_eq!(Dir::Left.opposite(), Dir::Right);
        assert_eq!(Dir::Right.opposite(), Dir::Left);
    }

    #[test]
    fn test_dir_deltas() {
        assert_eq!(Dir::Up.delta(), (0, -1));
        assert_eq!(Dir::Down.delta(), (0, 1));
        assert_eq!(Dir::Left.delta(), (-1, 0));
        assert_eq!(Dir::Right.delta(), (1, 0));
    }
}
