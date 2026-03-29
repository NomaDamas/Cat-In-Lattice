use super::Game;
use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// Maze dimensions — fits comfortably in ~22x15 terminal cells.
const MW: usize = 21;
const MH: usize = 15;

/// Cell types encoded in the map string.
const WALL: u8 = b'#';
const DOT: u8 = b'.';
const POWER: u8 = b'O';
const EMPTY: u8 = b' ';

const MOVE_INTERVAL: f64 = 0.18;
const GHOST_INTERVAL: f64 = 0.25;
const POWER_DURATION: f64 = 5.0;

/// A hard-coded small maze.  `#` = wall, `.` = dot, `O` = power pellet, ` ` = empty.
/// P = pacman start, G = ghost start (both become EMPTY at runtime).
const MAP: &[&[u8; MW]; MH] = &[
    b"#####################",
    b"#O...#.....#.......O#",
    b"#.##.#.###.#.##.###.#",
    b"#.................#.#",
    b"#.##.#.#####.#.##.#.#",
    b"#....#...#...#......#",
    b"####.###.#.###.######",
    b"#........G..........#",
    b"####.###.#.###.######",
    b"#....#...#...#......#",
    b"#.##.#.#####.#.##.#.#",
    b"#.......P...........#",
    b"#.##.#.###.#.##.###.#",
    b"#O...#.....#.......O#",
    b"#####################",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
    None,
}

impl Dir {
    fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
            Dir::None => (0, 0),
        }
    }
}

struct Ghost {
    x: i32,
    y: i32,
    dir: Dir,
    frightened: bool,
    eaten: bool,
}

pub struct PacmanGame {
    grid: [[u8; MW]; MH],
    px: i32,
    py: i32,
    pdir: Dir,
    next_dir: Dir,
    ghosts: Vec<Ghost>,
    score: u32,
    dots_left: u32,
    game_over: bool,
    won: bool,
    move_timer: f64,
    ghost_timer: f64,
    power_timer: f64,
    lives: u32,
}

impl PacmanGame {
    pub fn new() -> Self {
        let mut grid = [[EMPTY; MW]; MH];
        let mut px = 10i32;
        let mut py = 11i32;
        let mut ghost_starts = Vec::new();
        let mut dots_left = 0u32;

        for (r, row) in MAP.iter().enumerate() {
            for (c, &cell) in row.iter().enumerate() {
                match cell {
                    b'P' => {
                        grid[r][c] = EMPTY;
                        px = c as i32;
                        py = r as i32;
                    }
                    b'G' => {
                        grid[r][c] = EMPTY;
                        ghost_starts.push((c as i32, r as i32));
                    }
                    DOT | POWER => {
                        grid[r][c] = cell;
                        dots_left += 1;
                    }
                    _ => {
                        grid[r][c] = cell;
                    }
                }
            }
        }

        // If there's only one ghost start position, add a second offset by 2
        while ghost_starts.len() < 2 {
            let base = ghost_starts[0];
            ghost_starts.push((base.0 + 2, base.1));
        }

        let ghosts = ghost_starts
            .into_iter()
            .map(|(gx, gy)| Ghost {
                x: gx,
                y: gy,
                dir: Dir::Up,
                frightened: false,
                eaten: false,
            })
            .collect();

        Self {
            grid,
            px,
            py,
            pdir: Dir::None,
            next_dir: Dir::None,
            ghosts,
            score: 0,
            dots_left,
            game_over: false,
            won: false,
            move_timer: 0.0,
            ghost_timer: 0.0,
            power_timer: 0.0,
            lives: 3,
        }
    }

    fn is_walkable(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= MW as i32 || y < 0 || y >= MH as i32 {
            return false;
        }
        self.grid[y as usize][x as usize] != WALL
    }

    fn move_pacman(&mut self) {
        // Try the queued direction first
        let (ndx, ndy) = self.next_dir.delta();
        if self.next_dir != Dir::None && self.is_walkable(self.px + ndx, self.py + ndy) {
            self.pdir = self.next_dir;
        }
        let (dx, dy) = self.pdir.delta();
        let nx = self.px + dx;
        let ny = self.py + dy;
        if self.is_walkable(nx, ny) {
            self.px = nx;
            self.py = ny;
        }

        // Eat dot / power pellet
        let cell = self.grid[self.py as usize][self.px as usize];
        if cell == DOT {
            self.grid[self.py as usize][self.px as usize] = EMPTY;
            self.score += 10;
            self.dots_left -= 1;
        } else if cell == POWER {
            self.grid[self.py as usize][self.px as usize] = EMPTY;
            self.score += 50;
            self.dots_left -= 1;
            self.power_timer = POWER_DURATION;
            for g in &mut self.ghosts {
                g.frightened = true;
                g.eaten = false;
            }
        }

        if self.dots_left == 0 {
            self.won = true;
            self.game_over = true;
        }
    }

    fn move_ghosts(&mut self) {
        let mut rng = rand::thread_rng();
        for gi in 0..self.ghosts.len() {
            if self.ghosts[gi].eaten {
                continue;
            }
            // Simple AI: try to move toward pacman ~50% of the time, otherwise random.
            // When frightened, always move randomly.
            let dirs = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];
            let mut candidates: Vec<Dir> = Vec::new();
            let opposite = match self.ghosts[gi].dir {
                Dir::Up => Dir::Down,
                Dir::Down => Dir::Up,
                Dir::Left => Dir::Right,
                Dir::Right => Dir::Left,
                Dir::None => Dir::None,
            };
            for &d in &dirs {
                if d == opposite {
                    continue; // Don't reverse
                }
                let (dx, dy) = d.delta();
                if self.is_walkable(self.ghosts[gi].x + dx, self.ghosts[gi].y + dy) {
                    candidates.push(d);
                }
            }
            if candidates.is_empty() {
                // Dead end — allow reversing
                let (dx, dy) = opposite.delta();
                if self.is_walkable(self.ghosts[gi].x + dx, self.ghosts[gi].y + dy) {
                    candidates.push(opposite);
                }
            }
            if candidates.is_empty() {
                continue;
            }

            let chosen = if self.ghosts[gi].frightened || rng.gen_bool(0.4) {
                // Random
                candidates[rng.gen_range(0..candidates.len())]
            } else {
                // Chase: pick direction that minimizes manhattan distance to pacman
                let mut best = candidates[0];
                let mut best_dist = i32::MAX;
                for &d in &candidates {
                    let (dx, dy) = d.delta();
                    let nx = self.ghosts[gi].x + dx;
                    let ny = self.ghosts[gi].y + dy;
                    let dist = (nx - self.px).abs() + (ny - self.py).abs();
                    if dist < best_dist {
                        best_dist = dist;
                        best = d;
                    }
                }
                best
            };

            let (dx, dy) = chosen.delta();
            self.ghosts[gi].x += dx;
            self.ghosts[gi].y += dy;
            self.ghosts[gi].dir = chosen;
        }
    }

    fn check_collision(&mut self) {
        for g in &mut self.ghosts {
            if g.eaten {
                continue;
            }
            if g.x == self.px && g.y == self.py {
                if g.frightened {
                    g.eaten = true;
                    self.score += 200;
                } else {
                    self.lives -= 1;
                    if self.lives == 0 {
                        self.game_over = true;
                    } else {
                        // Reset positions
                        self.px = 10;
                        self.py = 11;
                        self.pdir = Dir::None;
                        self.next_dir = Dir::None;
                    }
                    return;
                }
            }
        }
    }
}

impl Game for PacmanGame {
    fn update(&mut self, dt: f64) {
        if self.game_over {
            return;
        }

        // Power pellet timer
        if self.power_timer > 0.0 {
            self.power_timer -= dt;
            if self.power_timer <= 0.0 {
                self.power_timer = 0.0;
                for g in &mut self.ghosts {
                    g.frightened = false;
                }
            }
        }

        // Pacman movement
        self.move_timer += dt;
        if self.move_timer >= MOVE_INTERVAL {
            self.move_timer -= MOVE_INTERVAL;
            self.move_pacman();
            self.check_collision();
        }

        // Ghost movement
        self.ghost_timer += dt;
        if self.ghost_timer >= GHOST_INTERVAL {
            self.ghost_timer -= GHOST_INTERVAL;
            self.move_ghosts();
            self.check_collision();
        }
    }

    fn handle_input(&mut self, key: KeyCode) {
        if self.game_over {
            return;
        }
        let d = match key {
            KeyCode::Up => Dir::Up,
            KeyCode::Down => Dir::Down,
            KeyCode::Left => Dir::Left,
            KeyCode::Right => Dir::Right,
            _ => return,
        };
        self.next_dir = d;
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let ox = area.x;
        let oy = area.y;

        let wall_style = Style::default().fg(Color::Blue);
        let dot_style = Style::default().fg(Color::White);
        let power_style = Style::default().fg(Color::LightYellow);
        let pac_style = Style::default().fg(Color::Yellow);

        for row in 0..MH {
            for col in 0..MW {
                let px = ox + col as u16;
                let py = oy + row as u16;
                if px >= area.right() || py >= area.bottom() {
                    continue;
                }
                let cell = self.grid[row][col];
                match cell {
                    WALL => {
                        buf.get_mut(px, py).set_char('█').set_style(wall_style);
                    }
                    DOT => {
                        buf.get_mut(px, py).set_char('·').set_style(dot_style);
                    }
                    POWER => {
                        buf.get_mut(px, py).set_char('◉').set_style(power_style);
                    }
                    _ => {}
                }
            }
        }

        // Pacman
        {
            let px = ox + self.px as u16;
            let py = oy + self.py as u16;
            if px < area.right() && py < area.bottom() {
                let ch = match self.pdir {
                    Dir::Right => '▶',
                    Dir::Left => '◀',
                    Dir::Up => '▲',
                    Dir::Down => '▼',
                    Dir::None => '◉',
                };
                buf.get_mut(px, py).set_char(ch).set_style(pac_style);
            }
        }

        // Ghosts
        for g in &self.ghosts {
            if g.eaten {
                continue;
            }
            let px = ox + g.x as u16;
            let py = oy + g.y as u16;
            if px < area.right() && py < area.bottom() {
                let (ch, color) = if g.frightened {
                    ('ᗣ', Color::LightBlue)
                } else {
                    ('ᗣ', Color::Red)
                };
                buf.get_mut(px, py)
                    .set_char(ch)
                    .set_style(Style::default().fg(color));
            }
        }

        // Status
        let status_y = oy + MH as u16;
        if status_y < area.bottom() {
            let msg = if self.game_over {
                if self.won {
                    format!("YOU WIN!  Score: {}", self.score)
                } else {
                    format!("GAME OVER  Score: {}", self.score)
                }
            } else {
                format!(
                    "Lives: {}  Score: {}{}",
                    self.lives,
                    self.score,
                    if self.power_timer > 0.0 {
                        "  POWER!"
                    } else {
                        ""
                    }
                )
            };
            let style = if self.game_over {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Yellow)
            };
            for (i, ch) in msg.chars().enumerate() {
                let px = ox + i as u16;
                if px < area.right() {
                    buf.get_mut(px, status_y).set_char(ch).set_style(style);
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
        "Pacman"
    }
}
