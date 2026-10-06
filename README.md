# tmux-manager

[![Version](https://img.shields.io/badge/version-2.2.0-green)](https://github.com/jaaaackieLai/tmux-manager/releases)

互動式 tmux session 管理工具，適合同時跑多個 Claude Code 等 AI CLI session。列出所有 session 與即時預覽，可用 AI 摘要每個 session 正在做什麼，並提供可儲存、一鍵貼上的 Prompt Slots。

## 安裝

支援 Linux（x86_64、aarch64）、macOS（Intel、Apple Silicon）與 WSL，執行時只需要 tmux。

```bash
curl -fsSL https://jaaaackielai.github.io/tmux-manager/install.sh | bash
```

預設安裝到 `~/.local/bin/tmux-manager`。要裝到其他位置可指定 `INSTALL_PREFIX=/usr/local`，沒有寫入權限時會自動使用 sudo。

AI 摘要需要設定 `ANTHROPIC_API_KEY`；沒設定時其他功能照常使用。

## 使用

```bash
tmux-manager            # 開啟 session 管理器
tmux-manager prompts    # 直接管理 Prompt Slots
```

**Session 管理器**

| 按鍵 | 動作 |
|---|---|
| ↑↓ / Tab | 選擇 session |
| Enter / 滑鼠單擊 | 開啟操作選單（attach／rename／kill） |
| `n` | 新增 session 並 attach |
| `p` | 管理 Prompt Slots |
| `f` | 更新 AI 摘要 |
| `[` `]` `\` | 調整／重設 Sessions 與 Preview 面板比例（也可拖曳分隔線） |
| `q` | 離開 |

**Prompt Slots**

在 prompt 管理模式按 `n` 新增、`e` 編輯、`d` 刪除、`J/K` 調整順序、`/` 搜尋，編輯時 Ctrl-S 儲存。

從管理器 attach 進 session 後，window 底部會顯示已儲存的 prompt。先點選要輸入的 pane，再單擊底部 prompt，內容就會貼到該 pane（不會自動送出 Enter）。

也可以在 tmux 裡用快捷鍵開 prompt popup：

```bash
tmux-manager bindings --print   # 輸出 tmux.conf 設定片段，確認後自行加入
```

加入後按 prefix + `P` 開啟 popup，需要 tmux 的 `set -g mouse on` 才能用滑鼠點選。

**設定**

設定檔位於 `~/.config/tmux-manager/config.toml`，也可用指令修改：

```bash
tmux-manager --config --list
tmux-manager --config NEW_DEFAULT_CMD "claude"
```

## 升級

```bash
tmux-manager --update
```

裝在 `/usr/local` 等需要權限的位置時改用 `sudo tmux-manager --update`。

從 Bash 版（v1.x）升級也是同一個指令：舊版檔案會被移除並換成新版，原本的 `config.sh` 會在第一次啟動時自動轉成 `config.toml`。

## 移除

```bash
tmux-manager --uninstall           # 保留設定與 prompts
tmux-manager uninstall --purge     # 一併刪除設定與 prompts
```
