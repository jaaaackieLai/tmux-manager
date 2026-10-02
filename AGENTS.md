# tmux-manager: Interactive tmux session manager

PTT 風格的 tmux session 管理工具，專為同時跑多個 Claude Code session 設計。

## 專案結構

```
tmux-tool/
  tmux-manager          # 主入口 ~74 行（bootstrap: source libs + cleanup + main）
  install.sh            # 安裝腳本（安裝到 share/tmux-manager/ + bin/ symlink）
  CLAUDE.md             # 本檔案
  lib/
    constants.sh        # 常數、顏色、全域狀態宣告（~38 行，含 VIEW_MODE）
    update.sh           # 自我更新：檢查遠端版本 + 執行 installer
    utils.sh            # cursor/terminal/die/check_deps（~25 行）
    config.sh           # 使用者設定載入（新 session 預設路徑/指令）
    sessions.sh         # tmux 操作：refresh/get_info/capture（~45 行）
    ai.sh               # AI 摘要：enabled/start/load/cleanup（~80 行）
    render.sh           # TUI 繪製：render_list/render_detail/draw_*（~200 行）
    actions.sh          # 使用者操作：attach/rename/kill/new（~115 行）
    input.sh            # 鍵盤輸入：read_key/handle_input/handle_detail_input（~80 行）
  tests/
    bats/               # BATS 1.13.0 (git submodule)
    test_helper.bash    # 共用 helper（定義 load_lib）
    test_utils.bats     # cursor_to 輸出格式、die exit code
    test_sessions.bats  # refresh_sessions SELECTED 夾緊邏輯
    test_ai.bats        # ai_enabled 判斷、load_ai_results 解析
    test_input.bats     # read_key escape 序列、SELECTED 邊界、VIEW_MODE 切換
```

### Source 載入順序（依賴由低到高）

```
constants.sh → update.sh → utils.sh → config.sh → sessions.sh → ai.sh → render.sh → actions.sh → input.sh
```

主入口統一 source 全部，lib 檔案之間不互相 source。

### 執行測試

```bash
./tests/bats/bin/bats tests/
```

## 技術架構

- **語言**: 純 bash，零外部依賴（tmux 必要，curl/jq 為 AI 功能選用）
- **TUI**: ANSI escape codes 手刻互動選單，raw terminal mode (stty)
- **AI 摘要**: Anthropic API (Haiku)，背景 subshell 非同步執行，結果寫到 `/tmp/tmux-manager-ai-*`
- **API**: 需要 `ANTHROPIC_API_KEY` 環境變數（沒設就跳過 AI 功能）

## 功能

兩層式 UI：列表頁選擇 session，Enter 進入詳細頁操作。

### 列表頁按鍵

| 按鍵 | 功能 |
|------|------|
| Up/Down | 上下選擇 session |
| Enter | 進入 session 詳細頁 |
| n | 建立新 session |
| f | 重新整理 + 重跑 AI 摘要 |
| q | 離開 |

### 詳細頁按鍵

| 按鍵 | 功能 |
|------|------|
| Up/Down | 上下選擇操作（attach/rename/kill/back） |
| Enter | 執行選中的操作 |
| a | 快捷 attach |
| r | 快捷 rename |
| k | 快捷 kill |
| ESC/q | 回到列表頁 |

## TUI 佈局

### 列表頁

```
 tmux-manager  v1.0.0
 ─────────────────────────────
 > session-0  [AI: 正在開發登入功能...]
   session-1  [AI: ...]
   session-2  [AI: 跑測試中]
 ─────────────────────────────
 Preview (session-0):
   最後 N 行終端輸出...
 ─────────────────────────────
 [Enter] open  [n] new  [f] refresh  [q] quit
```

### 詳細頁

```
 session-0                    v1.0.0
 ─────────────────────────────
 Info: 2 windows (created Thu Jan 1 00:00:00 2025)
 AI:   正在開發登入功能...
 ─────────────────────────────
 > attach
   rename
   kill
   back
 ─────────────────────────────
 [Up/Down] select  [Enter] confirm  [a]ttach [r]ename [k]ill  [ESC] back
```

## 安裝

安裝後的檔案結構：
```
${INSTALL_PREFIX}/share/tmux-manager/   # 所有程式檔案
  tmux-manager                          # 主程式
  lib/                                  # 函式庫
${INSTALL_PREFIX}/bin/tmux-manager      # symlink -> ../share/tmux-manager/tmux-manager
```

```bash
./install.sh                          # 安裝到 ~/.local（預設）
INSTALL_PREFIX=/usr/local ./install.sh  # 自訂安裝路徑（需 sudo）
curl -fsSL https://jaaaackielai.github.io/tmux-tool/install.sh | bash
```

更新：

```bash
tmux-manager --update
```

移除：

```bash
tmux-manager --uninstall
# 或
./install.sh --uninstall
```

## 自動啟動（加到 .bashrc / .zshrc）

```bash
if [[ -z "${TMUX:-}" ]] && command -v tmux-manager >/dev/null 2>&1; then
    tmux-manager
fi
```

## 版本管理

使用 [Semantic Versioning](https://semver.org)：`MAJOR.MINOR.PATCH`

| 位置 | 何時 bump | 範例 |
|------|-----------|------|
| MAJOR | 破壞性變更（不向下相容） | 改 CLI 介面、砍舊功能 |
| MINOR | 新增功能（向下相容） | 加按鍵、新操作 |
| PATCH | 修 bug | 修顯示錯誤 |

版本號定義在 `lib/constants.sh` 的 `VERSION` 變數。`--update` 機制靠比對遠端版本號決定是否更新，所以推新功能時必須 bump 版本。

## 開發備註

- AI model: `claude-haiku-4-5-20251001`
- Preview 抓最後 15 行顯示，AI 摘要抓最後 150 行（取 tail 80 行送 API）
- Temp files 用 PID 隔離: `/tmp/tmux-manager-ai-$$`
- attach 前會還原 terminal state，detach 後重新設定 raw mode
- 每次commit前先更改版本，需要在 lib/constants.sh:7 and README.md:3 做更新