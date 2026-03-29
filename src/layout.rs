use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Fixed width allocated to the cat art panel (characters).
pub const CAT_PANEL_WIDTH: u16 = 28;

/// Height of the bottom help bar.
const HELP_BAR_HEIGHT: u16 = 1;

/// Which layout mode the app is currently in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Two-panel: [Cat | Banner]
    Default,
    /// Three-panel: [Cat | Game | Banner]
    Gaming,
}

/// The computed rectangles for each UI region.
pub struct AppLayout {
    pub cat: Rect,
    pub banner: Rect,
    pub game: Option<Rect>,
    pub help_bar: Rect,
}

impl AppLayout {
    /// Compute the layout for the given terminal area and mode.
    pub fn compute(area: Rect, mode: LayoutMode) -> Self {
        // Split vertically: main content area on top, help bar at bottom.
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(HELP_BAR_HEIGHT),
            ])
            .split(area);

        let main_area = vert[0];
        let help_bar = vert[1];

        match mode {
            LayoutMode::Default => {
                // Two columns: cat (fixed) | banner (remaining)
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(CAT_PANEL_WIDTH),
                        Constraint::Min(10),
                    ])
                    .split(main_area);

                AppLayout {
                    cat: cols[0],
                    banner: cols[1],
                    game: None,
                    help_bar,
                }
            }
            LayoutMode::Gaming => {
                // Three columns: cat (fixed) | game (flexible) | banner (1/3 of remaining)
                let remaining = main_area.width.saturating_sub(CAT_PANEL_WIDTH);
                let banner_w = (remaining / 3).max(15);
                let game_w = remaining.saturating_sub(banner_w).max(10);

                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(CAT_PANEL_WIDTH),
                        Constraint::Length(game_w),
                        Constraint::Min(banner_w),
                    ])
                    .split(main_area);

                AppLayout {
                    cat: cols[0],
                    game: Some(cols[1]),
                    banner: cols[2],
                    help_bar,
                }
            }
        }
    }
}
