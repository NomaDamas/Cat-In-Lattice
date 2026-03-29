pub mod app;
pub mod banner;
pub mod cat;
pub mod config;
pub mod games;
pub mod layout;
pub mod persistence;
pub mod ui;
pub mod watcher;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use config::Config;

/// Cat-In-Lattice: a Ghostty companion pane with pixel art cat, banners, and
/// mini-games.
#[derive(Parser)]
#[command(name = "cat-in-lattice", version, about)]
struct Cli {
    /// Path to config directory (default: ~/.cat-in-lattice/)
    #[arg(long = "config")]
    config_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = Config::load(cli.config_dir.as_deref());
    if let Err(e) = config.ensure_data_dir() {
        eprintln!("Warning: could not create data directory: {e}");
    }
    // Save config so the file exists for the user to edit
    let _ = config.save();

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config);

    // Main event loop
    let result = run_loop(&mut terminal, &mut app);

    // Cleanup: always restore terminal, even on error
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Save state on exit
    app.save();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    let tick_rate = Duration::from_millis(16); // ~60 fps

    loop {
        // Render
        terminal.draw(|frame| {
            ui::draw(frame, app);
        })?;

        // Poll for events with the tick-rate timeout
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                // crossterm 0.28 fires Press and Release on some platforms;
                // only handle Press.
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
            // Resize is handled automatically by ratatui on the next draw().
        }

        // Advance time-based state
        app.tick();

        if app.should_quit {
            return Ok(());
        }
    }
}
