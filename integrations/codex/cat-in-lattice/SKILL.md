---
name: cat-in-lattice
description: Start or reuse the Cat-In-Lattice tmux sidecar in a bottom pane. Use when the user explicitly asks for $cat-in-lattice, wants the cat companion opened below the current pane, or wants to send a notice through the installed local launcher.
metadata:
  short-description: Launch the bottom cat sidecar or send a notice
---

# cat-in-lattice Skill

Use this only when the user explicitly invokes `$cat-in-lattice` or clearly asks to open the Cat-In-Lattice sidecar.

## Goal
- Open or reuse `cat-in-lattice` in a **bottom tmux pane** without stealing focus.
- Reuse the existing pane instead of spawning duplicates in the same window.
- Send notices with the same local launcher when the user asks for a banner/Slack announcement.

## Workflow
1. If the request is only to open the cat sidecar, run:
   - `/Users/kwon/vscode/Cat-In-Lattice/scripts/cat-in-lattice-sidecar.sh start`
2. If the user asks for a status check, run:
   - `/Users/kwon/vscode/Cat-In-Lattice/scripts/cat-in-lattice-sidecar.sh status`
3. If the user asks to close it, run:
   - `/Users/kwon/vscode/Cat-In-Lattice/scripts/cat-in-lattice-sidecar.sh stop`
4. If the user asks to broadcast a notice, run:
   - `/Users/kwon/vscode/Cat-In-Lattice/scripts/cat-in-lattice-sidecar.sh notice "<message>"`

## Rules
- Prefer the bottom split launcher over running `cat-in-lattice` directly.
- Do not create duplicate cat panes in the same tmux window.
- Keep the current pane focused after launching the sidecar.
- The sidecar script already knows the repo path and local install location; reuse it.
