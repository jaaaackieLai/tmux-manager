# tmux-tool

[![Version](https://img.shields.io/badge/version-1.0.0-green)](https://github.com/jaaaackielai/tmux-tool/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS-lightgrey)]()
[![Pure Bash](https://img.shields.io/badge/pure-bash-orange)]()

> 一個快速、鍵盤導向、可選 AI 摘要的 tmux 工作階段管理工具。

`tmux-manager` 讓你在多個 tmux session 之間切換與管理時，不再丟失上下文。
工具以純 Bash 實作，啟動輕快；即使不開啟 AI 功能，也能完整使用。

[English README](../../README.md)

## 為什麼用 tmux-manager

- 專注不分心：所有 session 一頁總覽，並即時預覽輸出。
- 操作更快：`attach`、`rename`、`kill`、`new` 全部在同一個 TUI 完成。
- 上下文更清楚：可選 AI 一行摘要，快速看懂每個 session 正在做什麼。
- 輕量零負擔：必需依賴只有 `tmux`，AI 相關只需額外 `curl` + `jq`。

## 介面預覽

### 清單視圖（List View）

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

### 詳細視圖（Detail View）

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

## 主要功能

- 雙層 TUI：清單視圖 + 詳細視圖。
- 即時預覽目前選取 session 的終端輸出。
- 每個 session 的 AI 一行摘要。
- 重新命名時可用 AI 建議名稱。
- AI 任務在背景執行，不阻塞互動介面。
- 內建自我更新：`tmux-manager --update`。

## 快速開始

### 1) 安裝

從本地 clone 安裝：

```bash
git clone <repo-url> && cd tmux-tool
./install.sh
```

從 GitHub 直接安裝：

```bash
curl -fsSL https://jaaaackielai.github.io/tmux-manager/install.sh | bash
```

自訂安裝路徑（預設 `~/.local`）：

```bash
INSTALL_PREFIX=/usr/local ./install.sh
```

程式安裝到 `${INSTALL_PREFIX}/share/tmux-manager/`，並在 `${INSTALL_PREFIX}/bin/tmux-manager` 建立 symlink。

### 2) 驗證

```bash
hash -r
command -v tmux-manager
tmux-manager -h
```

### 3) 啟動

```bash
tmux-manager
```

## 相依套件

| 套件 | 必要 | 用途 | macOS | Debian/Ubuntu |
|---|---|---|---|---|
| `tmux` | 是 | Session 管理 | `brew install tmux` | `sudo apt install tmux` |
| `curl` | 否 | 呼叫 AI API | 內建 | `sudo apt install curl` |
| `jq` | 否 | 解析 AI 回應 JSON | `brew install jq` | `sudo apt install jq` |

## 快捷鍵

### 清單視圖（List View）

| 按鍵 | 動作 |
|---|---|
| `Up/Down` | 移動選取 |
| `Enter` | 進入詳細視圖 |
| `n` | 建立新 session |
| `f` | 重新整理清單 + AI 摘要 |
| `q` | 離開 |

### 詳細視圖（Detail View）

| 按鍵 | 動作 |
|---|---|
| `Up/Down` | 移動選單 |
| `Enter` | 執行選取動作 |
| `a` | Attach session |
| `r` | 重新命名（含 AI 建議） |
| `k` | 結束 session |
| `ESC` / `q` | 回到清單 |

## 設定

### 透過 CLI 快速設定

```bash
tmux-manager --config --list              # 列出所有設定值
tmux-manager --config NEW_DEFAULT_DIR     # 讀取單一設定
tmux-manager --config NEW_DEFAULT_DIR ~/projects  # 寫入設定值
```

### 設定檔

預設路徑：`~/.config/tmux-manager/config.sh`

覆寫路徑：`TMUX_MANAGER_CONFIG_FILE=/path/to/config.sh tmux-manager`

### 設定項目

| 設定 Key | 型別 | 預設值 | 說明 |
|----------|------|--------|------|
| `NEW_DEFAULT_DIR` | 路徑 | (空) | 新 session 的工作目錄。設定後，按 `n` 建立的 session 會自動 `cd` 到此目錄。 |
| `NEW_DEFAULT_CMD` | 字串 | (空) | 新 session 建立後自動執行的指令（例如 `source .venv/bin/activate`）。透過 `tmux send-keys` 送入。 |
| `NEW_ASK_DIR` | 布林 | `0` | 設為 `1` 時，按 `n` 建立新 session 會先詢問工作目錄。`NEW_DEFAULT_DIR` 的值作為預設提示。 |
| `NEW_ASK_CMD` | 布林 | `0` | 設為 `1` 時，按 `n` 建立新 session 會先詢問啟動指令。`NEW_DEFAULT_CMD` 的值作為預設提示；輸入 `-` 可跳過。 |
| `POLL_INTERVAL` | 數字 | `0.2` | 鍵盤輪詢與畫面重繪間隔（秒）。數值越低 UI 回應越快，但 CPU 使用率越高。 |

布林值接受 `1`/`true`/`yes`/`on`（真），其餘皆為假。

設定檔範例：

```bash
TMUX_MANAGER_NEW_DEFAULT_DIR="$HOME/work/my-project"
TMUX_MANAGER_NEW_DEFAULT_CMD="source .venv/bin/activate"
TMUX_MANAGER_NEW_ASK_DIR=1
TMUX_MANAGER_NEW_ASK_CMD=1
TMUX_MANAGER_POLL_INTERVAL="0.5"
```

## AI 功能

先設定 API Key：

```bash
export ANTHROPIC_API_KEY='sk-ant-...'
```

啟用後，`tmux-manager` 會：

- 擷取近期 pane 輸出，
- 產生每個 session 的繁中一行摘要，
- 提供可直接採用的短名稱建議。

若未設定 API Key，非 AI 功能仍可完整運作。

## Shell 啟動時自動開啟

加到 `~/.bashrc` 或 `~/.zshrc`：

```bash
if [[ -z "${TMUX:-}" ]] && command -v tmux-manager >/dev/null 2>&1; then
    tmux-manager
fi
```

## CLI 指令

```text
tmux-manager --help
tmux-manager --version
tmux-manager --update
tmux-manager --uninstall
tmux-manager --config [--list | KEY | KEY VALUE]
```

## 授權

MIT
