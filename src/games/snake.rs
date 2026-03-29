use super::Game;
use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

const WIDTH: usize = 30;
const HEIGHT: usize = 15;
const MOVE_INTERVAL: f64 = 0.15; // seconds between moves

#[derive(Clone, Copy, PartialEq, Eq)]
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
        // Border
        let border_style = Style::default().fg(Color::DarkGray);
        // top/bottom border
        for x in 0..=(WIDTH as u16 + 1).min(area.width.saturating_sub(1)) {
            if area.y < area.bottom() {
                buf.get_mut(area.x + x, area.y).set_char('─').set_style(border_style);
            }
            let bot = area.y + HEIGHT as u16 + 1;
            if bot < area.bottom() {
                buf.get_mut(area.x + x, bot).set_char('─').set_style(border_style);
            }
        }
        for y in 0..=(HEIGHT as u16 + 1).min(area.height.saturating_sub(1)) {
            buf.get_mut(area.x, area.y + y).set_char('│').set_style(border_style);
            let right = area.x + WIDTH as u16 + 1;
            if right < area.right() {
                buf.get_mut(right, area.y + y).set_char('│').set_style(border_style);
            }
        }
        // corners
        buf.get_mut(area.x, area.y).set_char('┌').set_style(border_style);
        if area.x + WIDTH as u16 + 1 < area.right() {
            buf.get_mut(area.x + WIDTH as u16 + 1, area.y).set_char('┐').set_style(border_style);
        }
        let bot = area.y + HEIGHT as u16 + 1;
        if bot < area.bottom() {
            buf.get_mut(area.x, bot).set_char('└').set_style(border_style);
            if area.x + WIDTH as u16 + 1 < area.right() {
                buf.get_mut(area.x + WIDTH as u16 + 1, bot).set_char('┘').set_style(border_style);
            }
        }

        let ox = area.x + 1;
        let oy = area.y + 1;

        // Food
        let (fx, fy) = self.food;
        let px = ox + fx as u16;
        let py = oy + fy as u16;
        if px < area.right() && py < area.bottom() {
            buf.get_mut(px, py)
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
                buf.get_mut(px, py)
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
                    buf.get_mut(px, score_y).set_char(ch).set_style(style);
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
