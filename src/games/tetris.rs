use super::Game;
use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};

const BOARD_W: usize = 10;
const BOARD_H: usize = 15;
const DROP_INTERVAL: f64 = 0.5;
const SOFT_DROP_INTERVAL: f64 = 0.05;

/// The 7 standard tetrominos — each stored as 4 (col, row) offsets.
/// Rotation is done via matrix transform.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceKind {
    fn cells(self) -> [(i32, i32); 4] {
        match self {
            PieceKind::I => [(0, 0), (1, 0), (2, 0), (3, 0)],
            PieceKind::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            PieceKind::T => [(0, 0), (1, 0), (2, 0), (1, 1)],
            PieceKind::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            PieceKind::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            PieceKind::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            PieceKind::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
        }
    }

    fn color(self) -> Color {
        match self {
            PieceKind::I => Color::Cyan,
            PieceKind::O => Color::Yellow,
            PieceKind::T => Color::Magenta,
            PieceKind::S => Color::Green,
            PieceKind::Z => Color::Red,
            PieceKind::J => Color::Blue,
            PieceKind::L => Color::LightRed,
        }
    }

    fn random() -> Self {
        match rand::thread_rng().gen_range(0..7) {
            0 => PieceKind::I,
            1 => PieceKind::O,
            2 => PieceKind::T,
            3 => PieceKind::S,
            4 => PieceKind::Z,
            5 => PieceKind::J,
            _ => PieceKind::L,
        }
    }
}

#[derive(Clone)]
struct Piece {
    kind: PieceKind,
    cells: [(i32, i32); 4],
    x: i32,
    y: i32,
}

impl Piece {
    fn new(kind: PieceKind) -> Self {
        Self {
            kind,
            cells: kind.cells(),
            x: (BOARD_W as i32) / 2 - 2,
            y: 0,
        }
    }

    fn absolute_cells(&self) -> [(i32, i32); 4] {
        let mut out = [(0i32, 0i32); 4];
        for (i, &(cx, cy)) in self.cells.iter().enumerate() {
            out[i] = (self.x + cx, self.y + cy);
        }
        out
    }

    fn rotated_cells(&self) -> [(i32, i32); 4] {
        // Rotate 90 degrees clockwise around the bounding-box center.
        // For O piece, rotation is a no-op.
        if self.kind == PieceKind::O {
            return self.cells;
        }
        let mut out = [(0i32, 0i32); 4];
        // Find bounding box
        let max_x = self.cells.iter().map(|c| c.0).max().unwrap();
        let max_y = self.cells.iter().map(|c| c.1).max().unwrap();
        for (i, &(cx, cy)) in self.cells.iter().enumerate() {
            // 90-deg CW: (x, y) -> (max_y - y, x)
            out[i] = (max_y - cy, cx);
        }
        // Normalize so min coords are 0
        let min_x = out.iter().map(|c| c.0).min().unwrap();
        let min_y = out.iter().map(|c| c.1).min().unwrap();
        for c in &mut out {
            c.0 -= min_x;
            c.1 -= min_y;
        }
        let _ = max_x; // suppress warning
        out
    }
}

pub struct TetrisGame {
    board: [[Option<Color>; BOARD_W]; BOARD_H],
    current: Piece,
    score: u32,
    lines_cleared: u32,
    game_over: bool,
    timer: f64,
    soft_dropping: bool,
}

impl TetrisGame {
    pub fn new() -> Self {
        Self {
            board: [[None; BOARD_W]; BOARD_H],
            current: Piece::new(PieceKind::random()),
            score: 0,
            lines_cleared: 0,
            game_over: false,
            timer: 0.0,
            soft_dropping: false,
        }
    }

    fn fits(&self, piece: &Piece, cells: &[(i32, i32); 4]) -> bool {
        for &(cx, cy) in cells {
            let ax = piece.x + cx;
            let ay = piece.y + cy;
            if ax < 0 || ax >= BOARD_W as i32 || ay >= BOARD_H as i32 {
                return false;
            }
            if ay >= 0 {
                if self.board[ay as usize][ax as usize].is_some() {
                    return false;
                }
            }
        }
        true
    }

    fn lock_piece(&mut self) {
        let color = self.current.kind.color();
        for (ax, ay) in self.current.absolute_cells() {
            if ay >= 0 && (ay as usize) < BOARD_H && (ax as usize) < BOARD_W {
                self.board[ay as usize][ax as usize] = Some(color);
            }
        }
        self.clear_lines();
        self.current = Piece::new(PieceKind::random());
        if !self.fits(&self.current, &self.current.cells) {
            self.game_over = true;
        }
    }

    fn clear_lines(&mut self) {
        let mut cleared = 0u32;
        let mut y = BOARD_H as i32 - 1;
        while y >= 0 {
            let row = y as usize;
            if self.board[row].iter().all(|c| c.is_some()) {
                cleared += 1;
                // Shift everything above down
                for r in (1..=row).rev() {
                    self.board[r] = self.board[r - 1];
                }
                self.board[0] = [None; BOARD_W];
                // Don't decrement y; check this row again (now contains the row above).
            } else {
                y -= 1;
            }
        }
        self.lines_cleared += cleared;
        self.score += match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let mut test = self.current.clone();
        test.x += dx;
        test.y += dy;
        if self.fits(&test, &test.cells) {
            self.current = test;
            true
        } else {
            false
        }
    }

    fn try_rotate(&mut self) {
        let rotated = self.current.rotated_cells();
        let mut test = self.current.clone();
        test.cells = rotated;
        // Try original position, then wall kicks (-1, +1, -2, +2)
        for kick in [0, -1, 1, -2, 2] {
            let mut kicked = test.clone();
            kicked.x += kick;
            if self.fits(&kicked, &kicked.cells) {
                self.current = kicked;
                return;
            }
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(0, 1) {}
        self.lock_piece();
    }

    fn tick_down(&mut self) {
        if !self.try_move(0, 1) {
            self.lock_piece();
        }
    }
}

impl Game for TetrisGame {
    fn update(&mut self, dt: f64) {
        if self.game_over {
            return;
        }
        self.timer += dt;
        let interval = if self.soft_dropping {
            SOFT_DROP_INTERVAL
        } else {
            DROP_INTERVAL
        };
        while self.timer >= interval {
            self.timer -= interval;
            self.tick_down();
        }
    }

    fn handle_input(&mut self, key: KeyCode) {
        if self.game_over {
            return;
        }
        match key {
            KeyCode::Left => {
                self.try_move(-1, 0);
            }
            KeyCode::Right => {
                self.try_move(1, 0);
            }
            KeyCode::Up => self.try_rotate(),
            KeyCode::Down => {
                self.soft_dropping = true;
            }
            KeyCode::Char(' ') => self.hard_drop(),
            _ => {}
        }
        // Reset soft drop when key is not Down (handled per-frame externally;
        // here we just set it on Down presses — it auto-resets each update
        // cycle since update only checks the flag once).
        if key != KeyCode::Down {
            self.soft_dropping = false;
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let border = Style::default().fg(Color::DarkGray);
        let bw = BOARD_W as u16;
        let bh = BOARD_H as u16;

        // Draw border
        for x in 0..=(bw + 1).min(area.width.saturating_sub(1)) {
            if area.y < area.bottom() {
                buf[Position::new(area.x + x, area.y)].set_char('─').set_style(border);
            }
            let bot = area.y + bh + 1;
            if bot < area.bottom() {
                buf[Position::new(area.x + x, bot)].set_char('─').set_style(border);
            }
        }
        for y in 0..=(bh + 1).min(area.height.saturating_sub(1)) {
            buf[Position::new(area.x, area.y + y)].set_char('│').set_style(border);
            let right = area.x + bw + 1;
            if right < area.right() {
                buf[Position::new(right, area.y + y)].set_char('│').set_style(border);
            }
        }
        buf[Position::new(area.x, area.y)].set_char('┌').set_style(border);
        if area.x + bw + 1 < area.right() {
            buf[Position::new(area.x + bw + 1, area.y)].set_char('┐').set_style(border);
        }
        if area.y + bh + 1 < area.bottom() {
            buf[Position::new(area.x, area.y + bh + 1)].set_char('└').set_style(border);
            if area.x + bw + 1 < area.right() {
                buf[Position::new(area.x + bw + 1, area.y + bh + 1)].set_char('┘').set_style(border);
            }
        }

        let ox = area.x + 1;
        let oy = area.y + 1;

        // Board cells
        for row in 0..BOARD_H {
            for col in 0..BOARD_W {
                let px = ox + col as u16;
                let py = oy + row as u16;
                if px < area.right() && py < area.bottom() {
                    if let Some(color) = self.board[row][col] {
                        buf[Position::new(px, py)]
                            .set_char('█')
                            .set_style(Style::default().fg(color));
                    }
                }
            }
        }

        // Current piece
        let color = self.current.kind.color();
        for (ax, ay) in self.current.absolute_cells() {
            if ay >= 0 {
                let px = ox + ax as u16;
                let py = oy + ay as u16;
                if px < area.right() && py < area.bottom() {
                    buf[Position::new(px, py)]
                        .set_char('█')
                        .set_style(Style::default().fg(color));
                }
            }
        }

        // Ghost piece (drop shadow)
        let mut ghost = self.current.clone();
        loop {
            ghost.y += 1;
            if !self.fits(&ghost, &ghost.cells) {
                ghost.y -= 1;
                break;
            }
        }
        if ghost.y != self.current.y {
            for (ax, ay) in ghost.absolute_cells() {
                if ay >= 0 {
                    let px = ox + ax as u16;
                    let py = oy + ay as u16;
                    if px < area.right() && py < area.bottom() {
                        // Only draw ghost if cell is empty
                        if self.board[ay as usize][ax as usize].is_none() {
                            let already_current = self
                                .current
                                .absolute_cells()
                                .iter()
                                .any(|&(cx, cy)| cx == ax && cy == ay);
                            if !already_current {
                                buf[Position::new(px, py)]
                                    .set_char('▒')
                                    .set_style(Style::default().fg(Color::DarkGray));
                            }
                        }
                    }
                }
            }
        }

        // Score / info (to the right of the board)
        let info_x = area.x + bw + 3;
        if info_x + 10 < area.right() {
            let labels = [
                format!("Score"),
                format!("{}", self.score),
                format!(""),
                format!("Lines"),
                format!("{}", self.lines_cleared),
            ];
            let style = Style::default().fg(Color::White);
            for (i, label) in labels.iter().enumerate() {
                let py = area.y + 1 + i as u16;
                if py < area.bottom() {
                    for (j, ch) in label.chars().enumerate() {
                        let px = info_x + j as u16;
                        if px < area.right() {
                            buf[Position::new(px, py)].set_char(ch).set_style(style);
                        }
                    }
                }
            }
        }

        if self.game_over {
            let msg = "GAME OVER";
            let py = area.y + bh / 2 + 1;
            let start_x = ox + bw / 2 - msg.len() as u16 / 2;
            let style = Style::default().fg(Color::Red);
            if py < area.bottom() {
                for (i, ch) in msg.chars().enumerate() {
                    let px = start_x + i as u16;
                    if px < area.right() {
                        buf[Position::new(px, py)].set_char(ch).set_style(style);
                    }
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
        "Tetris"
    }
}
