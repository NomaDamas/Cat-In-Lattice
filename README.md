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
- 2-panel default layout, 3-panel when gaming

**Adaptive Layout**
- Fixed-width cat panel (pixel art never breaks)
- Responsive banner/game panels
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
| `H` | Toggle help |
| `Esc` | Exit game / Quit |
| `Q` | Quit |

### In-Game Controls

| Key | Action |
|-----|--------|
| Arrow keys | Move / Rotate |
| Space | Action (drop, launch, etc.) |
| Esc | Exit game |

## Configuration

Config is stored at `~/.cat-in-lattice/config.json`:

```json
{
  "slack": {
    "token": "xoxb-your-slack-token",
    "channel": "C1234567890"
  },
  "watcher": {
    "patterns": ["done", "error", "complete", "failed"],
    "watch_paths": ["/tmp/agent-status.json"]
  },
  "active_hours": [8, 23],
  "events_per_day": 20
}
```

### Slack Setup

1. Create a Slack app at https://api.slack.com/apps
2. Add `channels:history` scope
3. Install to workspace and copy the Bot Token
4. Set `token` and `channel` in config

### Agent Status Watching

The watcher monitors files for pattern matches and shows alerts in the banner. Write agent status to `/tmp/agent-status.json` or configure custom watch paths.

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
