use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::cat::state::Mood;
use crate::layout::AppLayout;

/// Render the entire UI for one frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let layout = AppLayout::compute(area, app.layout_mode);

    render_cat(frame, app, layout.cat);

    if let Some(game_area) = layout.game {
        render_game(frame, app, game_area);
    }

    render_banner(frame, app, layout.banner);
    render_help_bar(frame, app, layout.help_bar);

    // Overlay: event popup
    if app.event_scheduler.active_event().is_some() {
        render_event_popup(frame, app, area);
    }

    // Overlay: help screen
    if app.show_help {
        render_help_overlay(frame, area);
    }
}

/// Color tint for the cat based on mood.
fn mood_color(mood: Mood) -> Color {
    match mood {
        Mood::Happy => Color::Green,
        Mood::Neutral => Color::Yellow,
        Mood::Sad => Color::Blue,
        Mood::Angry => Color::Red,
        Mood::Sleeping => Color::Magenta,
    }
}

/// Secondary color for text based on mood.
fn mood_fg(mood: Mood) -> Color {
    match mood {
        Mood::Happy => Color::LightGreen,
        Mood::Neutral => Color::White,
        Mood::Sad => Color::LightBlue,
        Mood::Angry => Color::LightRed,
        Mood::Sleeping => Color::LightMagenta,
    }
}

/// Render the cat pixel art panel.
fn render_cat(frame: &mut Frame, app: &App, area: Rect) {
    let color = mood_color(app.cat_state.mood);
    let frame_lines = app
        .animation
        .current_frame_with_accessories(&app.cat_state.accessories);

    let lines: Vec<Line<'_>> = frame_lines
        .iter()
        .map(|s| Line::from(Span::styled(s.clone(), Style::default().fg(color))))
        .collect();

    // Stats below the cat art
    let stats = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(
                " Mood: {:?}  Aff: {:.0}  Hun: {:.0}",
                app.cat_state.mood, app.cat_state.affinity, app.cat_state.hunger
            ),
            Style::default().fg(mood_fg(app.cat_state.mood)),
        )),
        Line::from(Span::styled(
            format!(
                " Pets: {}  Feeds: {}",
                app.cat_state.total_pets, app.cat_state.total_feeds
            ),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let mut all_lines = lines;
    all_lines.extend(stats);

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));

    let para = Paragraph::new(all_lines).block(block);
    frame.render_widget(para, area);
}

/// Render the banner panel (alerts take priority, then quotes).
fn render_banner(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::NONE)
        .title(" Banner ")
        .title_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show the highest-priority alert if one exists, otherwise show a quote
    if let Some(alert) = app.alert_queue.top() {
        let style = if alert.is_high_priority() {
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };
        let lines = vec![
            Line::from(Span::styled(
                format!("[{}]", alert.alert_type),
                style,
            )),
            Line::from(""),
            Line::from(Span::styled(alert.message.clone(), Style::default().fg(Color::White))),
        ];
        let para = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(para, inner);
    } else {
        let quote = app.quote_rotator.current();
        let lines = vec![
            Line::from(Span::styled(
                format!("\"{}\"", quote.text),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  -- {}", quote.author),
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let para = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(para, inner);
    }

    // Event schedule info at the bottom of the banner
    let remaining = app.event_scheduler.remaining_count();
    let rate = app.event_scheduler.success_rate();
    let info_y = area.bottom().saturating_sub(2);
    if info_y > area.y && area.width > 4 {
        let info = format!(" Events left: {}  Rate: {:.0}%", remaining, rate);
        let info_rect = Rect::new(area.x + 1, info_y, area.width.saturating_sub(2), 1);
        let para = Paragraph::new(Line::from(Span::styled(
            info,
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(para, info_rect);
    }
}

/// Render the game panel (delegates to the Game trait's render).
fn render_game(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(game) = &app.active_game {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" {} ", game.name()))
            .title_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // The Game trait renders directly to the buffer
        let buf = frame.buffer_mut();
        game.render(inner, buf);
    }
}

/// Render the bottom help bar.
fn render_help_bar(frame: &mut Frame, app: &App, area: Rect) {
    let hints = if app.active_game.is_some() {
        " [Esc]Back  [Arrows]Move  [Space]Action  [Q]uit "
    } else if app.event_scheduler.active_event().is_some() {
        " [Space/Enter]Respond  [P]et  [F]eed  [1-4]Games  [H]elp  [Q]uit "
    } else {
        " [P]et  [F]eed  [1]Pacman  [2]Snake  [3]Tetris  [4]Breakout  [H]elp  [Q]uit "
    };

    let para = Paragraph::new(Line::from(Span::styled(
        hints,
        Style::default()
            .fg(Color::Black)
            .bg(Color::DarkGray),
    )));
    frame.render_widget(para, area);
}

/// Render the event popup overlay when a cat event is active.
fn render_event_popup(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(event) = app.event_scheduler.active_event() {
        let msg = match &event.event_type {
            crate::cat::events::EventType::Pet => "Your cat wants to be petted! [Space/Enter]",
            crate::cat::events::EventType::Feed => "Your cat is hungry! [Space/Enter]",
            crate::cat::events::EventType::Special(acc) => {
                // We can't easily format with the accessory in a static str,
                // so we'll handle it below.
                match acc {
                    crate::cat::state::Accessory::Hat => "Special: Unlock a Hat! [Space/Enter]",
                    crate::cat::state::Accessory::Bow => "Special: Unlock a Bow! [Space/Enter]",
                    crate::cat::state::Accessory::Glasses => {
                        "Special: Unlock Glasses! [Space/Enter]"
                    }
                    crate::cat::state::Accessory::Scarf => {
                        "Special: Unlock a Scarf! [Space/Enter]"
                    }
                }
            }
        };

        let popup_w = (msg.len() as u16 + 4).min(area.width);
        let popup_h: u16 = 3;
        let x = area.x + area.width.saturating_sub(popup_w) / 2;
        let y = area.y + area.height.saturating_sub(popup_h) / 2;
        let popup_area = Rect::new(x, y, popup_w, popup_h);

        // Clear the popup area
        let clear = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));
        frame.render_widget(clear, popup_area);

        let inner = Rect::new(x + 1, y + 1, popup_w.saturating_sub(2), 1);
        let para = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(para, inner);
    }
}

/// Render a full-screen help overlay.
fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled(
            " Cat-In-Lattice Help ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("  Interactions:", Style::default().fg(Color::Yellow))),
        Line::from("    P         Pet the cat (raises affinity)"),
        Line::from("    F         Feed the cat (reduces hunger)"),
        Line::from(""),
        Line::from(Span::styled("  Mini-Games:", Style::default().fg(Color::Yellow))),
        Line::from("    1         Pacman"),
        Line::from("    2         Snake"),
        Line::from("    3         Tetris"),
        Line::from("    4         Breakout"),
        Line::from("    Esc       Exit game"),
        Line::from(""),
        Line::from(Span::styled("  Events:", Style::default().fg(Color::Yellow))),
        Line::from("    Space/Enter   Respond to a cat event"),
        Line::from(""),
        Line::from(Span::styled("  General:", Style::default().fg(Color::Yellow))),
        Line::from("    H         Toggle this help"),
        Line::from("    Q / Esc   Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press H or Esc to close ",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup_h = (help_text.len() as u16 + 2).min(area.height);
    let popup_w = 50u16.min(area.width);
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(help_text).block(block);
    frame.render_widget(para, popup_area);
}
