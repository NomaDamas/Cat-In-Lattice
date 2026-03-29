#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="/Users/kwon/vscode/Cat-In-Lattice"
APP_BIN="${CAT_IN_LATTICE_BIN:-cat-in-lattice}"
APP_NAME="cat-in-lattice"
APP_HEIGHT="${CAT_IN_LATTICE_PANE_HEIGHT:-16}"
DEFAULT_SESSION="cat-in-lattice-sidecar"

usage() {
  cat <<'EOF'
Usage: cat-in-lattice-sidecar.sh [start|status|stop|notice] [message...]

  start   Launch or reuse a bottom tmux pane running cat-in-lattice
  status  Print the currently detected cat pane, if any
  stop    Stop the detected cat pane/session
  notice  Send a notice through cat-in-lattice notice
EOF
}

current_session() {
  if [[ -n "${TMUX_PANE:-}" ]]; then
    tmux display-message -p -t "$TMUX_PANE" '#{session_name}'
  else
    tmux list-sessions -F '#{session_name}' 2>/dev/null | head -n1
  fi
}

current_window() {
  if [[ -n "${TMUX_PANE:-}" ]]; then
    tmux display-message -p -t "$TMUX_PANE" '#{window_id}'
  fi
}

find_existing_pane() {
  local target_window=${1:-}
  if [[ -n "$target_window" ]]; then
    tmux list-panes -t "$target_window" -F '#{pane_id}	#{pane_title}	#{pane_current_command}' 2>/dev/null       | awk -F '	' '$2 == "cat-in-lattice" || $3 == "cat-in-lattice" { print $1; exit }'
  else
    tmux list-panes -a -F '#{pane_id}	#{pane_title}	#{pane_current_command}' 2>/dev/null       | awk -F '	' '$2 == "cat-in-lattice" || $3 == "cat-in-lattice" { print $1; exit }'
  fi
}

print_status() {
  local pane_id=${1:-}
  if [[ -z "$pane_id" ]]; then
    echo 'running=false'
    return 1
  fi

  local line
  line=$(tmux display-message -p -t "$pane_id" 'running=true pane=#{pane_id} title=#{pane_title} active=#{pane_active} session=#{session_name} window=#{window_id}' 2>/dev/null || true)
  if [[ -z "$line" ]]; then
    echo 'running=false'
    return 1
  fi
  echo "$line"
}

start_inside_tmux() {
  local window_id existing pane_id
  window_id=$(current_window)
  existing=$(find_existing_pane "$window_id")
  if [[ -n "$existing" ]]; then
    print_status "$existing"
    return 0
  fi

  pane_id=$(tmux split-window -d -v -l "$APP_HEIGHT" -P -F '#{pane_id}' "cd '$REPO_DIR' && exec $APP_BIN")
  tmux select-pane -t "$pane_id" -T "$APP_NAME"
  print_status "$pane_id"
}

start_detached_session() {
  local existing pane_id session_name
  session_name=${CAT_IN_LATTICE_SESSION_NAME:-$DEFAULT_SESSION}
  existing=$(find_existing_pane)
  if [[ -n "$existing" ]]; then
    print_status "$existing"
    return 0
  fi

  tmux new-session -d -s "$session_name" "cd '$REPO_DIR' && exec $APP_BIN"
  pane_id=$(tmux list-panes -t "$session_name:0" -F '#{pane_id}' | head -n1)
  tmux select-pane -t "$pane_id" -T "$APP_NAME"
  print_status "$pane_id"
}

start() {
  command -v tmux >/dev/null 2>&1 || { echo 'tmux is required' >&2; exit 1; }
  command -v "$APP_BIN" >/dev/null 2>&1 || { echo "missing binary: $APP_BIN" >&2; exit 1; }

  if [[ -n "${TMUX_PANE:-}" ]]; then
    start_inside_tmux
  else
    start_detached_session
  fi
}

status() {
  command -v tmux >/dev/null 2>&1 || { echo 'tmux is required' >&2; exit 1; }
  local existing window_id
  window_id=$(current_window || true)
  if [[ -n "$window_id" ]]; then
    existing=$(find_existing_pane "$window_id")
  else
    existing=$(find_existing_pane)
  fi
  print_status "$existing"
}

stop() {
  command -v tmux >/dev/null 2>&1 || { echo 'tmux is required' >&2; exit 1; }
  local existing session_name
  existing=$(find_existing_pane)
  if [[ -n "$existing" ]]; then
    tmux kill-pane -t "$existing"
    echo "stopped pane=$existing"
    return 0
  fi

  session_name=${CAT_IN_LATTICE_SESSION_NAME:-$DEFAULT_SESSION}
  if tmux has-session -t "$session_name" 2>/dev/null; then
    tmux kill-session -t "$session_name"
    echo "stopped session=$session_name"
    return 0
  fi

  echo 'nothing to stop'
}

notice() {
  if [[ $# -eq 0 ]]; then
    echo 'notice text is required' >&2
    exit 1
  fi
  exec "$APP_BIN" notice "$*"
}

cmd=${1:-start}
case "$cmd" in
  start)
    shift
    start "$@"
    ;;
  status)
    shift
    status "$@"
    ;;
  stop)
    shift
    stop "$@"
    ;;
  notice)
    shift
    notice "$@"
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "unknown command: $cmd" >&2
    usage >&2
    exit 1
    ;;
esac
