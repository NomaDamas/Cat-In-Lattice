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

use anyhow::Context;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use banner::SlackNotifier;
use config::Config;
use watcher::append_announcement_line;

/// Cat-In-Lattice: a Ghostty companion pane with pixel art cat, banners, and
/// mini-games.
#[derive(Parser)]
#[command(name = "cat-in-lattice", version, about)]
struct Cli {
    /// Path to config directory (default: ~/.cat-in-lattice/)
    #[arg(long = "config")]
    config_dir: Option<PathBuf>,
    /// Path to dotenv-style env file with runtime-only secret overrides.
    #[arg(long = "env-file")]
    env_file: Option<PathBuf>,
    /// Verify Slack history access and print a short report, then exit.
    #[arg(long = "check-slack")]
    check_slack: bool,
    /// Send a Slack webhook test message, then exit.
    #[arg(long = "slack-test-message")]
    slack_test_message: Option<String>,
    /// Post an announcement via webhook when configured and always queue a local banner notice.
    #[arg(short = 'a', long = "announce")]
    announce: Option<String>,
    /// Short-form command surface.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Send a notice to the local banner and Slack webhook.
    Notice {
        /// Announcement text to broadcast.
        #[arg(required = true)]
        message: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let announce = cli.announce.clone().or_else(|| match &cli.command {
        Some(Command::Notice { message }) => Some(message.join(" ")),
        None => None,
    });

    // Load configuration
    let mut config = Config::load(cli.config_dir.as_deref());
    if let Err(e) = config.ensure_data_dir() {
        eprintln!("Warning: could not create data directory: {e}");
    }
    // Save config so the file exists for the user to edit
    let _ = config.save();
    if let Some(env_file) = cli.env_file.as_deref() {
        config
            .apply_env_file(env_file)
            .with_context(|| format!("failed to load env file: {}", env_file.display()))?;
    } else {
        for candidate in [config.data_dir.join(".env"), PathBuf::from(".env")] {
            if candidate.exists() {
                config
                    .apply_env_file(&candidate)
                    .with_context(|| format!("failed to load env file: {}", candidate.display()))?;
                break;
            }
        }
    }

    if cli.check_slack || cli.slack_test_message.is_some() || announce.is_some() {
        return run_cli_actions(
            &config,
            cli.check_slack,
            cli.slack_test_message.as_deref(),
            announce.as_deref(),
        );
    }

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

fn run_cli_actions(
    config: &Config,
    check_slack: bool,
    slack_test_message: Option<&str>,
    announce: Option<&str>,
) -> anyhow::Result<()> {
    let notifier = SlackNotifier::new(config.slack.clone());

    if check_slack {
        let report = notifier
            .check_connection()
            .context("failed to verify Slack history access")?;
        println!(
            "Slack history check: OK ({} recent message(s))",
            report.recent_notice_count
        );
        if let Some(notice) = report.latest_notice {
            println!("Latest notice: {}", notice);
        } else {
            println!("Latest notice: none found in the channel history window");
        }
    }

    if let Some(message) = slack_test_message {
        notifier
            .send_test_message(message)
            .context("failed to send Slack webhook test message")?;
        println!("Slack webhook test: OK");
    }

    if let Some(message) = announce {
        let local_path =
            append_announcement_line(message).context("failed to queue local announcement")?;
        println!("Local announcement queued: {}", local_path.display());

        if config.slack.has_webhook() {
            notifier
                .send_webhook_message(message)
                .context("failed to send Slack announcement via webhook")?;
            println!("Slack webhook announcement: OK");
        } else {
            println!("Slack webhook announcement: skipped (webhook not configured)");
        }
    }

    if !check_slack && slack_test_message.is_none() && announce.is_none() {
        println!("Nothing to do. Use --check-slack, --slack-test-message, and/or --announce.");
    }

    Ok(())
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
