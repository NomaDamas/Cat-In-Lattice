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

/// Minimum terminal width we can reasonably render in.
pub const MIN_WIDTH: u16 = 10;
/// Minimum terminal height we can reasonably render in.
pub const MIN_HEIGHT: u16 = 4;

impl AppLayout {
    /// Compute the layout for the given terminal area and mode.
    ///
    /// For extremely small terminals (below `MIN_WIDTH` x `MIN_HEIGHT`),
    /// all regions collapse to the full area so rendering is a no-op
    /// rather than a panic.
    pub fn compute(area: Rect, mode: LayoutMode) -> Self {
        // Guard: if the terminal is impossibly small, give everything the
        // full area so callers can decide to skip rendering.
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            return AppLayout {
                cat: area,
                banner: area,
                game: if mode == LayoutMode::Gaming {
                    Some(area)
                } else {
                    None
                },
                help_bar: Rect::new(area.x, area.y, area.width, 0),
            };
        }

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
                // Clamp cat panel to available width so it never exceeds
                // the terminal width.
                let cat_w = CAT_PANEL_WIDTH.min(main_area.width.saturating_sub(1));

                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(cat_w),
                        Constraint::Min(1),
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
                let cat_w = CAT_PANEL_WIDTH.min(main_area.width.saturating_sub(2));
                let remaining = main_area.width.saturating_sub(cat_w);
                let banner_w = (remaining / 3).max(1);
                let game_w = remaining.saturating_sub(banner_w).max(1);

                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(cat_w),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layout_two_panel() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = AppLayout::compute(area, LayoutMode::Default);
        assert!(layout.game.is_none());
        assert!(layout.cat.width > 0);
        assert!(layout.banner.width > 0);
        assert_eq!(layout.help_bar.height, HELP_BAR_HEIGHT);
        assert_eq!(
            layout.cat.width + layout.banner.width,
            area.width
        );
    }

    #[test]
    fn test_gaming_layout_three_panel() {
        let area = Rect::new(0, 0, 100, 30);
        let layout = AppLayout::compute(area, LayoutMode::Gaming);
        assert!(layout.game.is_some());
        let game = layout.game.unwrap();
        assert!(game.width > 0);
        assert!(layout.cat.width > 0);
        assert!(layout.banner.width > 0);
    }

    #[test]
    fn test_tiny_terminal_does_not_panic() {
        // Extremely small: 5x3
        let area = Rect::new(0, 0, 5, 3);
        let layout = AppLayout::compute(area, LayoutMode::Default);
        // Should not panic; all rects should be valid
        assert!(layout.cat.width <= area.width);
        assert!(layout.banner.width <= area.width);
    }

    #[test]
    fn test_tiny_terminal_gaming_does_not_panic() {
        let area = Rect::new(0, 0, 8, 3);
        let layout = AppLayout::compute(area, LayoutMode::Gaming);
        assert!(layout.game.is_some());
    }

    #[test]
    fn test_zero_area_does_not_panic() {
        let area = Rect::new(0, 0, 0, 0);
        let layout = AppLayout::compute(area, LayoutMode::Default);
        assert_eq!(layout.help_bar.height, 0);
    }

    #[test]
    fn test_narrow_terminal_default() {
        // Width is exactly CAT_PANEL_WIDTH -- banner should still get some space
        let area = Rect::new(0, 0, 20, 10);
        let layout = AppLayout::compute(area, LayoutMode::Default);
        assert!(layout.cat.width <= 20);
        assert!(layout.banner.width >= 1);
    }

    #[test]
    fn test_help_bar_at_bottom() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = AppLayout::compute(area, LayoutMode::Default);
        assert_eq!(layout.help_bar.y + layout.help_bar.height, area.height);
    }
}
