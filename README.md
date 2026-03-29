# Cat-In-Lattice

A Ghostty companion pane with a pixel art cat, banners, and mini-games.

```
        ╱╲     ╱╲
       ╱██╲   ╱██╲           "Code is poetry."
      ╱████╲_╱████╲                -- unknown
     ┌──────────────┐
     │  ◕        ◕  │         [COMPLETE] build finished
     │      ▼▼      │
     │  ╱══╗██╔══╲  │         Events left: 18
     │ │ ═══╝╚═══ │ │         Rate: 85%
     └──┬────────┬──┘
      ┌─┘ ░░░░░░ └─┐
     ┌┘  ░░░░░░░░  └┐
     │  ░░░░░░░░░░  │
     │ ░░░░░░░░░░░░ │
     └─┬──┘    └──┬─┘
       └──┘    └──┘  ~
 [P]et  [F]eed  [1]Pacman  [2]Snake  [3]Tetris  [4]Breakout  [H]elp  [Q]uit
```

## Features

**Companion Cat (Priority 1)**
- Pixel art cat with idle breathing + tail animation (4 frames)
- Mood-based expressions: Happy, Neutral, Sad, Angry, Sleeping
- Tamagotchi-style persistence: affinity, hunger saved between sessions
- ~20 random events/day (pet, feed) with 10s response window
- Unlockable accessories (hat, bow, glasses, scarf) at high affinity

**Banner System (Priority 1)**
- Rotating inspirational quotes (56 bundled)
- Slack channel integration for team announcements
- Agent status alerts via file watching (regex pattern matching)

**Mini-Games (Priority 2)**
- Pacman, Snake, Tetris, Breakout
- Play while waiting for agents to finish
- Shared in-game HUD with restart button and lives display
- 2-panel default layout, 3-panel when gaming

**Adaptive Layout**
- Fixed-width cat panel (pixel art never breaks)
- Responsive banner/game panels
- Recent game records in the banner while you play
- Bottom key hints bar always visible

## Install

### From source

```bash
git clone https://github.com/NomaDamas/Cat-In-Lattice.git
cd Cat-In-Lattice
cargo build --release
cp target/release/cat-in-lattice /usr/local/bin/
```

### Homebrew (coming soon)

```bash
brew install NomaDamas/tap/cat-in-lattice
```

## Usage

```bash
# Start the companion pane
cat-in-lattice

# With custom config directory
cat-in-lattice --config ~/.my-cat-config/

# Load Slack secrets from a local env file
cat-in-lattice --env-file .env

# Or keep ~/.cat-in-lattice/.env (or ./.env) and use the short notice command
cat-in-lattice notice "Deploy starts at 6pm"

# Verify Slack history access
cat-in-lattice --env-file .env --check-slack

# Send a Slack webhook test message
cat-in-lattice --env-file .env --slack-test-message "Cat-In-Lattice test message"

# Send an announcement without a bot token:
# queues a local banner notice, and also posts to Slack if webhook_url is set
cat-in-lattice --env-file .env --announce "Deploy starts at 6pm"
cat-in-lattice -a "Deploy starts at 6pm"

# Open or reuse the bottom tmux sidecar pane
cat-in-lattice-sidecar start
```

### In Ghostty

Split your Ghostty terminal and run `cat-in-lattice` in the bottom pane:

1. Open Ghostty
2. Split pane: `Cmd+D` (horizontal split)
3. In the bottom pane: `cat-in-lattice`
4. Start your agent (Claude, Codex, etc.) in the top pane

### Controls

| Key | Action |
|-----|--------|
| `P` | Pet the cat |
| `F` | Feed the cat |
| `1` | Play Pacman |
| `2` | Play Snake |
| `3` | Play Tetris |
| `4` | Play Breakout |
| `R` | Restart current game |
| `H` | Toggle help |
| `Esc` | Exit game / Quit |
| `Q` | Quit |

### In-Game Controls

| Key | Action |
|-----|--------|
| Arrow keys | Move / Rotate |
| Space | Action (drop, launch, etc.) |
| R | Restart current game |
| Esc | Exit game |

## Configuration

Config is stored at `~/.cat-in-lattice/config.json`:

```json
{
  "slack": {
    "webhook_url": "https://hooks.slack.com/services/...",
    "token": "xoxb-your-slack-token",
    "channel": "C1234567890"
  },
  "watcher": {
    "patterns": ["done", "error", "complete", "failed", "announcement", "announce", "notice", "공지"],
    "watch_paths": [
      "/tmp/agent-status.json",
      "/tmp/cat-in-lattice-announcements.log"
    ]
  },
  "active_hours": [8, 23],
  "events_per_day": 20
}
```

### Slack Setup

1. Create a Slack app at https://api.slack.com/apps
2. Add `channels:history` scope
3. Install to workspace and copy the Bot Token
4. (Optional) Create an Incoming Webhook if you want a built-in test sender
5. Set `token`, `channel`, and optional `webhook_url` in config

### Recommended secret handling

Keep Slack secrets out of `config.json`.

1. Copy `.env.example` to `~/.cat-in-lattice/.env` (global) or `.env` (project-local)
2. Fill in:
   - `CAT_IN_LATTICE_SLACK_TOKEN`
   - `CAT_IN_LATTICE_SLACK_CHANNEL`
   - optional `CAT_IN_LATTICE_SLACK_WEBHOOK_URL`
3. Use either:
   - `cat-in-lattice notice "공지사항"`
   - `cat-in-lattice -a "공지사항"`
   - or `--env-file .env` explicitly when you want a non-default file

`config.json` remains safe to commit/share, while `.env` stays local and is ignored by git.

### Slack Verification

- `cat-in-lattice --env-file .env --check-slack` validates `token` + `channel` by reading recent channel history
- `cat-in-lattice --env-file .env --slack-test-message "..."` validates `webhook_url` by sending a test post

### Webhook vs announcement channel

- Incoming webhook = **send message to Slack**
- Announcement channel feed = **read a specific channel with Bot Token + channel ID**

If you want Cat-In-Lattice to show new notices from a team announcement channel, configure `token` + `channel`. The existing banner poller reads that channel and displays only newly seen notices in the app.

If you **do not have a bot token**, use webhook-only announcements instead:

```bash
cat-in-lattice notice "Standup starts in 10 minutes"
```

That command:

- writes `announcement: ...` into `/tmp/cat-in-lattice-announcements.log` so the running app shows it in the banner
- posts the same text to Slack when `CAT_IN_LATTICE_SLACK_WEBHOOK_URL` / `webhook_url` is configured
- still works locally even if no webhook is configured

### Bottom sidecar launcher

Use the bundled launcher when you want the cat to appear below the current tmux pane:

```bash
cat-in-lattice-sidecar start
cat-in-lattice-sidecar status
cat-in-lattice-sidecar notice "작업 완료"
cat-in-lattice-sidecar stop
```

The launcher reuses an existing cat pane in the current tmux window, keeps focus on your current pane, and opens the UI in a **bottom split** by default. Notices render near the bottom of the cat UI and hide automatically while another tmux pane is focused.

### Codex / Claude integrations

Local integrations are bundled in this repo and can be installed as symlinks:

```bash
ln -sfn /Users/kwon/vscode/Cat-In-Lattice/integrations/codex/cat-in-lattice ~/.codex/skills/cat-in-lattice
mkdir -p ~/.claude/commands
ln -sfn /Users/kwon/vscode/Cat-In-Lattice/integrations/claude/commands/cat-in-lattice.md ~/.claude/commands/cat-in-lattice.md
```

After that:

- Codex: use `$cat-in-lattice` to open/reuse the bottom cat pane
- Claude: use `/cat-in-lattice` to open/reuse the bottom cat pane
- Both can send notices via the bundled sidecar script

### Agent Status Watching

The watcher monitors files for pattern matches and shows alerts in the banner. Write agent status to `/tmp/agent-status.json`, send human notices with `--announce`, or configure custom watch paths.

## Data

Cat state is persisted at `~/.cat-in-lattice/cat_state.json`. Your cat's affinity, hunger, accessories, and stats survive across sessions.

## Tech Stack

- **Rust** with ratatui for TUI rendering
- **crossterm** for terminal control
- **notify** for file system watching
- **ureq** for Slack API (sync HTTP)
- Unicode half-block characters for pixel art

## License

MIT
