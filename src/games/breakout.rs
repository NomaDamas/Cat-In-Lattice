use super::Game;
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};

const WIDTH: usize = 32;
const HEIGHT: usize = 15;
const PADDLE_W: i32 = 6;
const BRICK_ROWS: usize = 4;
const BRICKS_PER_ROW: usize = 8;
const BALL_INTERVAL: f64 = 0.07; // seconds between ball steps
const PADDLE_SPEED: i32 = 2;

pub struct BreakoutGame {
    paddle_x: i32,
    ball_x: f64,
    ball_y: f64,
    ball_dx: f64,
    ball_dy: f64,
    bricks: [[bool; BRICKS_PER_ROW]; BRICK_ROWS],
    score: u32,
    lives: u32,
    game_over: bool,
    timer: f64,
    ball_launched: bool,
}

impl BreakoutGame {
    pub fn new() -> Self {
        let mut g = Self {
            paddle_x: (WIDTH as i32) / 2 - PADDLE_W / 2,
            ball_x: 0.0,
            ball_y: 0.0,
            ball_dx: 1.0,
            ball_dy: -1.0,
            bricks: [[true; BRICKS_PER_ROW]; BRICK_ROWS],
            score: 0,
            lives: 3,
            game_over: false,
            timer: 0.0,
            ball_launched: false,
        };
        g.reset_ball();
        g
    }

    fn reset_ball(&mut self) {
        self.ball_x = self.paddle_x as f64 + PADDLE_W as f64 / 2.0;
        self.ball_y = (HEIGHT as f64) - 2.0;
        self.ball_dx = 1.0;
        self.ball_dy = -1.0;
        self.ball_launched = false;
    }

    fn brick_rect(col: usize, row: usize) -> (i32, i32, i32, i32) {
        // Each brick is 3 chars wide, 1 char tall, starting at row 1
        let bw = (WIDTH / BRICKS_PER_ROW) as i32;
        let x = col as i32 * bw;
        let y = row as i32 + 1; // offset from top
        (x, y, bw, 1)
    }

    fn step_ball(&mut self) {
        if self.game_over || !self.ball_launched {
            return;
        }

        let nx = self.ball_x + self.ball_dx;
        let ny = self.ball_y + self.ball_dy;

        // Wall bounces (left/right)
        if nx < 0.0 || nx >= WIDTH as f64 {
            self.ball_dx = -self.ball_dx;
        }
        // Ceiling bounce
        if ny < 0.0 {
            self.ball_dy = -self.ball_dy;
        }
        // Floor — lose a life
        if ny >= HEIGHT as f64 {
            self.lives -= 1;
            if self.lives == 0 {
                self.game_over = true;
            } else {
                self.reset_ball();
            }
            return;
        }

        self.ball_x += self.ball_dx;
        self.ball_y += self.ball_dy;

        let bx = self.ball_x as i32;
        let by = self.ball_y as i32;

        // Paddle collision
        let paddle_y = HEIGHT as i32 - 1;
        if by == paddle_y && bx >= self.paddle_x && bx < self.paddle_x + PADDLE_W {
            self.ball_dy = -self.ball_dy.abs();
            // Angle based on where on paddle the ball hit
            let relative = (self.ball_x - self.paddle_x as f64) / PADDLE_W as f64;
            self.ball_dx = (relative - 0.5) * 2.5;
            if self.ball_dx.abs() < 0.3 {
                self.ball_dx = if self.ball_dx >= 0.0 { 0.3 } else { -0.3 };
            }
        }

        // Brick collision
        for row in 0..BRICK_ROWS {
            for col in 0..BRICKS_PER_ROW {
                if !self.bricks[row][col] {
                    continue;
                }
                let (rx, ry, rw, rh) = Self::brick_rect(col, row);
                if bx >= rx && bx < rx + rw && by >= ry && by < ry + rh {
                    self.bricks[row][col] = false;
                    self.score += 10;
                    self.ball_dy = -self.ball_dy;
                    // Check win
                    if self.bricks.iter().all(|r| r.iter().all(|&b| !b)) {
                        self.game_over = true;
                    }
                    return;
                }
            }
        }
    }
}

impl Game for BreakoutGame {
    fn update(&mut self, dt: f64) {
        if self.game_over {
            return;
        }
        self.timer += dt;
        while self.timer >= BALL_INTERVAL {
            self.timer -= BALL_INTERVAL;
            self.step_ball();
        }
        // Keep ball on paddle if not launched
        if !self.ball_launched {
            self.ball_x = self.paddle_x as f64 + PADDLE_W as f64 / 2.0;
            self.ball_y = (HEIGHT as f64) - 2.0;
        }
    }

    fn handle_input(&mut self, key: KeyCode) {
        if self.game_over {
            return;
        }
        match key {
            KeyCode::Left => {
                self.paddle_x = (self.paddle_x - PADDLE_SPEED).max(0);
            }
            KeyCode::Right => {
                self.paddle_x =
                    (self.paddle_x + PADDLE_SPEED).min(WIDTH as i32 - PADDLE_W);
            }
            KeyCode::Char(' ') | KeyCode::Up => {
                if !self.ball_launched {
                    self.ball_launched = true;
                }
            }
            _ => {}
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let border = Style::default().fg(Color::DarkGray);
        let w = WIDTH as u16;
        let h = HEIGHT as u16;

        // Border
        for x in 0..=(w + 1).min(area.width.saturating_sub(1)) {
            buf[Position::new(area.x + x, area.y)].set_char('─').set_style(border);
            if area.y + h + 1 < area.bottom() {
                buf[Position::new(area.x + x, area.y + h + 1)].set_char('─').set_style(border);
            }
        }
        for y in 0..=(h + 1).min(area.height.saturating_sub(1)) {
            buf[Position::new(area.x, area.y + y)].set_char('│').set_style(border);
            if area.x + w + 1 < area.right() {
                buf[Position::new(area.x + w + 1, area.y + y)].set_char('│').set_style(border);
            }
        }
        buf[Position::new(area.x, area.y)].set_char('┌').set_style(border);
        if area.x + w + 1 < area.right() {
            buf[Position::new(area.x + w + 1, area.y)].set_char('┐').set_style(border);
        }
        if area.y + h + 1 < area.bottom() {
            buf[Position::new(area.x, area.y + h + 1)].set_char('└').set_style(border);
            if area.x + w + 1 < area.right() {
                buf[Position::new(area.x + w + 1, area.y + h + 1)].set_char('┘').set_style(border);
            }
        }

        let ox = area.x + 1;
        let oy = area.y + 1;

        // Bricks
        let brick_colors = [Color::Red, Color::LightRed, Color::Yellow, Color::Green];
        for row in 0..BRICK_ROWS {
            for col in 0..BRICKS_PER_ROW {
                if !self.bricks[row][col] {
                    continue;
                }
                let (rx, ry, rw, _) = Self::brick_rect(col, row);
                let style = Style::default().fg(brick_colors[row % brick_colors.len()]);
                for dx in 0..rw {
                    let px = ox + (rx + dx) as u16;
                    let py = oy + ry as u16;
                    if px < area.right() && py < area.bottom() {
                        buf[Position::new(px, py)].set_char('█').set_style(style);
                    }
                }
            }
        }

        // Paddle
        let paddle_style = Style::default().fg(Color::Cyan);
        let py = oy + HEIGHT as u16 - 1;
        if py < area.bottom() {
            for dx in 0..PADDLE_W {
                let px = ox + (self.paddle_x + dx) as u16;
                if px < area.right() {
                    buf[Position::new(px, py)].set_char('▀').set_style(paddle_style);
                }
            }
        }

        // Ball
        let bx = ox + self.ball_x as u16;
        let by = oy + self.ball_y as u16;
        if bx < area.right() && by < area.bottom() {
            buf[Position::new(bx, by)]
                .set_char('●')
                .set_style(Style::default().fg(Color::White));
        }

        // Status line
        let status_y = area.y + h + 2;
        if status_y < area.bottom() {
            let msg = if self.game_over {
                if self.bricks.iter().all(|r| r.iter().all(|&b| !b)) {
                    format!("YOU WIN!  Score: {}", self.score)
                } else {
                    format!("GAME OVER  Score: {}", self.score)
                }
            } else if !self.ball_launched {
                format!("Lives: {}  Score: {}  [SPACE] launch", self.lives, self.score)
            } else {
                format!("Lives: {}  Score: {}", self.lives, self.score)
            };
            let style = if self.game_over {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Yellow)
            };
            for (i, ch) in msg.chars().enumerate() {
                let px = area.x + i as u16;
                if px < area.right() {
                    buf[Position::new(px, status_y)].set_char(ch).set_style(style);
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
        "Breakout"
    }
}
