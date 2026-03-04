# tmux-manager

[![Version](https://img.shields.io/badge/version-1.2.1-green)](https://github.com/jaaaackielai/tmux-manager/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS-lightgrey)]()
[![Pure Bash](https://img.shields.io/badge/pure-bash-orange)]()

> A fast, keyboard-first tmux session manager with AI-powered context summaries.

`tmux-manager` helps you manage many tmux sessions without losing context.
It is built in pure Bash, starts quickly, and works even without AI enabled.

[繁體中文說明](./docs/zh-tw/README.md)

## Why tmux-manager

- Stay focused: see active sessions and recent output in one place.
- Work faster: attach, rename, kill, and create sessions from a single TUI.
- Keep context: optional AI summaries describe what each session is doing.
- Lightweight: only `tmux` is required; `curl` and `jq` are optional for AI.

## Preview

### List View

```text
 tmux-manager  v1.0.0
 ───────────────────────────────────────
 > my-project   [AI: 正在重構登入模組...]
   api-server   [AI: 跑測試中，3 個失敗]
   deploy       [AI: SSH 連線到 prod]
 ───────────────────────────────────────
 Preview (my-project):
   $ npm run test
   PASS src/auth.test.ts
   Tests: 12 passed, 12 total
 ───────────────────────────────────────
 [Enter] open  [n] new  [f] refresh  [q] quit
```

### Detail View

```text
 my-project                    v1.0.0
 ───────────────────────────────────────
 Info: 2 windows (created Thu Jan 1 00:00:00 2025)
 AI:   正在重構登入模組...
 ───────────────────────────────────────
 > attach
   rename
   kill
   back
 ───────────────────────────────────────
 [Up/Down] select  [Enter] confirm  [a]ttach [r]ename [k]ill  [ESC] back
```

## Features

- Two-level TUI: list view + detail view.
- Real-time pane preview for the selected session.
- AI one-line summaries for each session.
- AI-assisted session rename suggestions.
- Background AI jobs so UI stays responsive.
- Self-update support: `tmux-manager --update`.

## Quick Start

### 1) Install

From local clone:

```bash
git clone <repo-url> && cd tmux-manager
./install.sh
```

Direct install from GitHub:

```bash
curl -fsSL https://jaaaackielai.github.io/tmux-manager/install.sh | bash
```

Custom install prefix (default is `~/.local`):

```bash
INSTALL_PREFIX=/usr/local ./install.sh
```

Files are installed to `${INSTALL_PREFIX}/share/tmux-manager/` with a symlink at `${INSTALL_PREFIX}/bin/tmux-manager`.

### 2) Verify

```bash
hash -r
command -v tmux-manager
tmux-manager -h
```

### 3) Launch

```bash
tmux-manager
```

## Dependencies

| Dependency | Required | Purpose | macOS | Debian/Ubuntu |
|---|---|---|---|---|
| `tmux` | Yes | Session management | `brew install tmux` | `sudo apt install tmux` |
| `curl` | No | AI API requests | Built-in | `sudo apt install curl` |
| `jq` | No | AI JSON parsing | `brew install jq` | `sudo apt install jq` |

## Keybindings

### List View

| Key | Action |
|---|---|
| `Up/Down` | Move selection |
| `Enter` | Open detail view |
| `n` | New session |
| `f` | Refresh sessions + AI summaries |
| `q` | Quit |

### Detail View

| Key | Action |
|---|---|
| `Up/Down` | Move menu selection |
| `Enter` | Run selected action |
| `a` | Attach session |
| `r` | Rename (with AI suggestion) |
| `k` | Kill session |
| `ESC` / `q` | Back to list |

### tmux tip: synchronize input

`C-b s` can toggle synchronized input across all panes in a window.
Add this line to `~/.tmux.conf`:

```bash
bind s setw synchronize-panes
```

Then reload: `tmux source-file ~/.tmux.conf`

## Configuration

### Quick config via CLI

```bash
tmux-manager --config --list              # List all settings and values
tmux-manager --config NEW_DEFAULT_DIR     # Read one setting
tmux-manager --config NEW_DEFAULT_DIR ~/projects  # Set a value
```

### Config file

Default location: `~/.config/tmux-manager/config.sh`

Override with: `TMUX_MANAGER_CONFIG_FILE=/path/to/config.sh tmux-manager`

### Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `NEW_DEFAULT_DIR` | path | (empty) | Working directory for new sessions. When set, every session created with `n` starts in this directory. |
| `NEW_DEFAULT_CMD` | string | (empty) | Command to run automatically in new sessions (e.g. `source .venv/bin/activate`). Sent via `tmux send-keys` after session creation. |
| `NEW_ASK_DIR` | bool | `0` | When `1`, the `n` (new session) prompt asks for a working directory each time. The value of `NEW_DEFAULT_DIR` is used as the default hint. |
| `NEW_ASK_CMD` | bool | `0` | When `1`, the `n` (new session) prompt asks for an init command each time. The value of `NEW_DEFAULT_CMD` is used as the default hint; enter `-` to skip. |
| `POLL_INTERVAL` | number | `0.2` | Keyboard poll and screen refresh interval in seconds. Lower values make the UI more responsive but use more CPU. |

Bool values accept `1`/`true`/`yes`/`on` (truthy) or anything else (falsy).

Example config file:

```bash
TMUX_MANAGER_NEW_DEFAULT_DIR="$HOME/work/my-project"
TMUX_MANAGER_NEW_DEFAULT_CMD="source .venv/bin/activate"
TMUX_MANAGER_NEW_ASK_DIR=1
TMUX_MANAGER_NEW_ASK_CMD=1
TMUX_MANAGER_POLL_INTERVAL="0.5"
```

## AI Features

Set API key:

```bash
export ANTHROPIC_API_KEY='sk-ant-...'
```

When enabled, `tmux-manager`:

- captures recent pane output,
- generates a one-line Traditional Chinese status summary,
- suggests a short rename candidate.

Without API key, all non-AI features still work.

## Auto-launch on shell start

Add to `~/.bashrc` or `~/.zshrc`:

```bash
if [[ -z "${TMUX:-}" ]] && command -v tmux-manager >/dev/null 2>&1; then
    tmux-manager
fi
```

## CLI

```text
tmux-manager --help
tmux-manager --version
tmux-manager --update
tmux-manager --uninstall
tmux-manager --config [--list | KEY | KEY VALUE]
```

## License

MIT
