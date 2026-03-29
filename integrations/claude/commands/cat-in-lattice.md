---
allowed-tools: Bash(/Users/kwon/vscode/Cat-In-Lattice/scripts/cat-in-lattice-sidecar.sh:*), Bash(cat-in-lattice:*), Bash(tmux:*)
description: Open the Cat-In-Lattice bottom sidecar or send a notice
---

## Usage
- `/cat-in-lattice` → open or reuse the bottom cat pane
- `/cat-in-lattice notice 공지사항` → send a notice through the installed launcher
- `/cat-in-lattice status` → print the current pane status
- `/cat-in-lattice stop` → stop the running cat pane

## Your task
1. If no arguments are provided, run:
   - `/Users/kwon/vscode/Cat-In-Lattice/scripts/cat-in-lattice-sidecar.sh start`
2. If arguments are provided, pass them directly to the same script.
3. Report the resulting pane/session status or notice result concisely.
