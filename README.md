# tmux-manager

[![Version](https://img.shields.io/badge/version-2.0.1-green)](https://github.com/jaaaackieLai/tmux-manager/releases)

Rust 實作的 tmux 工作階段管理器，提供 AI 摘要與可編輯、持久儲存的 Prompt Slots。Bash 版（v1.x）執行 `tmux-manager --update` 即改裝為 Rust 版，舊版檔案會一併清除，設定自動轉換。

## 安裝與升級

新安裝（Linux x86_64／aarch64、macOS x86_64／Apple Silicon、WSL）：

```bash
curl -fsSL https://jaaaackielai.github.io/tmux-manager/install.sh | bash
```

預設裝到 `~/.local/bin/tmux-manager`。可用 `INSTALL_PREFIX=/usr/local` 指定其他位置，沒有寫入權限時會自動改用 sudo。執行時只需要 tmux，不再需要 curl 或 jq；Linux 版為靜態連結，不受系統 glibc 版本限制。

從 Bash 版（v1.x）升級：

```bash
tmux-manager --update
tmux-manager --version   # 應顯示 tmux-manager 2.x
```

升級時會：

- 移除 v1 的主程式與 `lib/*.sh`，在原位置換成單一 Rust binary（不保留 v1 備份）
- 第一次啟動時把 `~/.config/tmux-manager/config.sh` 自動轉成 `config.toml`；原 `config.sh` 保留，確認設定無誤後可自行刪除，或以 `tmux-manager uninstall --purge` 連同其他資料一併清除
- `.bashrc`／`.zshrc` 的自動啟動設定不需修改

之後的更新一樣執行 `tmux-manager --update`。裝在 `/usr/local` 等需要權限的位置時，請改用 `sudo tmux-manager --update`。

## 本機試用

```bash
cargo build --release --locked
./target/release/tmux-manager             # session 管理器；列表按 p 管理 prompts
./target/release/tmux-manager prompts     # 也可單獨開 prompt 管理模式
```

在管理器列表按 `p` 或單擊上方 `[p] Prompt Slots` 可直接管理 prompts，即使沒有 session 也能使用。按 Esc／q 返回原本的管理器與 session 選取，不需退出再執行另一個指令。這個入口是管理模式；要貼上請先開啟 session 操作選單，再按 `p` 選擇明確 pane。

從 manager **attach 進 tmux 後，window 底部會常駐顯示已儲存的 prompt 標題**。先點選上方工作 pane，再單擊底部 prompt，內容會填入最近使用的工作 pane，焦點也會回到工作區，**不補送 Enter**。方向鍵／Enter 不會選取或執行 prompt；底部列只接受滑鼠，prompt 多時用滾輪或 `[‹]`／`[›]` 翻頁。

底部列會跟著 session 的各個 window 顯示，包含之後新增的 window；保存 prompt 變更會自動更新。底部保留四行，視窗太小時安全退化。透過 manager attach 時會暫時啟用該 session 的 mouse，不需改 tmux.conf；最後一個 client detach 後移除本工具的底部列並恢復原 mouse 設定。多行若顯示「多行尚未貼上」，內容僅存為畫面所示的具名 buffer，能力限制見下方「多行能力」。

在 prompt 管理模式按 `n` 新增，Tab 切換標題、標籤與多行內容，Ctrl-S 儲存。`e` 編輯、`d` 刪除確認、`J/K` 調整順序、`/` 搜尋。編輯時 Esc 先返回 prompt 列表，未儲存內容需確認放棄；Ctrl-R 可重新載入外部資料並保留草稿，有 revision conflict 時不會覆寫其他視窗的修改。

管理器列表的 ↑↓/Tab 選 session、Enter 開浮動操作選單、`n` 新增後 attach、`f` 更新摘要、`q` 離開。選單保留背景列表與 Preview，選項間留一行空白，底部按鍵提示為單行；小高度使用緊湊排列，窄視窗縮短提示。↑↓/Tab 選 attach／rename／kill／back，Enter 執行；`a/r/k` 分別 attach／改名／結束 session，`p` 先選 window/pane 再開 prompt 選單。Esc／q／點選單外關閉，保留 session 選取與面板比例；Preview 與摘要在選單開啟時仍更新。

滑鼠左鍵單擊 session 開浮動選單，單擊 attach／rename／kill／back 執行對應操作，kill 仍需確認。選單外的點擊只關閉選單，同一次點擊不會操作背景 session、prompt 入口或分隔線；選單開啟時滾輪只選操作項目。pane 選擇器也可單擊進入 prompt 選單。點擊以實際繪製位置與 session ID 對應，支援列表捲動及背景刷新；滑鼠抬起、拖曳與右鍵不會觸發 session 操作。

Sessions／Preview 預設依內容自動分配高度，session 少時讓出更多預覽空間；列表有欄位對齊、列距與選取底色，窄畫面將 AI 摘要移到下一行。預覽空間不足時優先顯示最新輸出。

可用滑鼠左鍵拖曳兩個面板之間的分隔線，或按 `[` 縮小 Sessions、`]` 放大 Sessions；每次約 5 百分點，至少移動一行，直到達到面板最小高度。按 `\` 恢復自動配置。手動比例只保留於本次執行，視窗縮放會重新夾緊高度。

## 在 AI CLI 裡開 prompt popup

先輸出 binding，檢查後加入自己的 tmux.conf；不會自動修改設定：

```bash
./target/release/tmux-manager bindings --print
```

片段使用目前 binary 的絕對路徑，prefix + **大寫 P** 開啟 popup。按 Enter 或滑鼠左鍵單擊 slot 會貼到開啟前的 pane，保持原 socket 與 pane ID，不會補送 Enter；成功後 popup 關閉，內容由你編輯並送出。需要 tmux 的 `mouse on` 才能在 popup 點擊。

```tmux
set -g mouse on
```

也可直接啟動選單，pane ID 必須明確；非 tmux 環境需同時指定 socket：

```bash
./target/release/tmux-manager prompts --target-pane %12 --socket /path/to/tmux.socket
```

不帶 target 時只有管理功能。目標消失、結束、input-off 或 copy mode 時停止貼上，沒有其他 pane 回退。失敗保留本次唯一具名 buffer，畫面會顯示名稱；不修改其他 buffers。

### 多行能力

| tmux | 單行 | 多行 | 本分支實測 |
|---|---|---|---|
| 3.3–3.6 | 指定 pane 直貼 | 無法查詢 mode 2004 時僅存 buffer | Linux 3.4 通過 |
| 3.7c | 指定 pane 直貼 | `bracket_paste_flag=1` 才直貼 | Linux 3.7c 通過 |
| 有 `pane_private_modes` 的版本 | 指定 pane 直貼 | 偵測 mode 2004，作為查詢備援 | 尚未實測 |

多行包含尾端 LF；能力未知或關閉時顯示「尚未貼上」，不合併換行、不假裝完成。`paste-buffer -p -r` 在測試 PTY 保留中文、emoji、換行與尾端 LF；開啟 synchronize-panes 時沒有廣播至其他 pane。tmux 成功不等同所有前景 CLI 都不會送出。

## 設定與舊版遷移

預設檔案 `${XDG_CONFIG_HOME:-$HOME/.config}/tmux-manager/config.toml`，prompts 存在同目錄的 `prompts.toml`。路徑可用 `--config-file` 或 `TMUX_MANAGER_CONFIG_FILE` 指定。優先序為 CLI > `TMUX_MANAGER_*` 環境變數 > TOML > 預設。

```toml
NEW_DEFAULT_DIR = ""
NEW_DEFAULT_CMD = ""
NEW_ASK_DIR = false
NEW_ASK_CMD = false
POLL_INTERVAL = 0.2
AI_MODEL = "claude-haiku-4-5-20251001"
```

```bash
./target/release/tmux-manager --config --list
./target/release/tmux-manager --config NEW_DEFAULT_CMD ""
./target/release/tmux-manager --config POLL_INTERVAL 0.2
./target/release/tmux-manager migrate-config ~/.config/tmux-manager/config.sh
```

從 Bash 版（v1.x）升級後第一次啟動時，若沒有 `config.toml` 但同資料夾有舊 `config.sh`，會自動轉換成 `config.toml`，原檔保留。`config.sh` 不會被執行：遷移只接受允許鍵的簡單 assignment、引號與 `$HOME` 展開；雙引號內的 `&&`、`;`、`|` 等視為字面值，命令替換、反引號、跳脫、source、其他展開會報行號並停止。自動轉換失敗時以預設值啟動並在狀態列說明原因，修正後可手動執行 `migrate-config`。目的檔已存在不覆寫。`TMUX_MANAGER_*` 環境變數格式錯誤（例如空字串或非數字）時忽略該值，不阻止啟動。複雜啟動命令可透過新 `--config KEY VALUE` 原樣儲存。

設定與 prompts 原子寫入，新增資料檔 mode 0600。Prompt body 上限 64 KiB UTF-8，CRLF 轉 LF，允許 LF/Tab，拒絕其他控制字元。schema_version=1，UUID 是身份，標題／標籤可重複。

AI 使用 `ANTHROPIC_API_KEY`，未設定也能完整使用管理器與 prompts。HTTP 不再需要 curl/jq；摘要最多使用 80 行多 pane 內容，同時最多 4 requests，每項上限 15 秒，刷新會取消舊任務。

## 安裝、更新與移除

試用請直接執行 `target/release/tmux-manager`。確認後才安裝：

```bash
./target/release/tmux-manager install --prefix ~/.local
# 或
cargo install --path . --locked
```

Rust installer 安裝到 prefix/bin，原子切換，不寫入舊 symlink 的 target。manifest 在 prefix/share/tmux-manager；備份在 prefix/bin/tmux-manager.backup-*，只保留最新一份。從 Bash 版（v1.x）升級時，切換成功後移除舊版 share/tmux-manager 的主程式與 lib/*.sh，不留備份（只刪已知檔名）。卸載移除 binary、備份、manifest 與舊版檔案，預設保留設定與 prompts；加 `--purge` 一併刪除。`cargo install` 沒有本工具 manifest，請使用 Cargo 管理卸載；要使用內建更新，先執行 Rust `install`。

```bash
tmux-manager --update      # 需已發布對應平台 artifact 與 SHA-256
tmux-manager --uninstall   # 只移除 manifest 管理且 checksum 相符的 binary，保留設定與 prompts
tmux-manager uninstall --purge  # 同上，並刪除設定、prompts 與舊 config.sh
```

`install.sh` 是下載入口，也可在本機建置後呼叫。prefix 不可寫入（例如 `INSTALL_PREFIX=/usr/local`）時會改用 sudo 執行 installer。下載失敗時會說明原因並保留現有安裝。SHA-256 檢查完整性，不等同發布者簽章。

推送 `v*` tag 時 release workflow 會建置各平台 binary 並建立 GitHub Release（tag 必須與 Cargo 版本一致）。Bash 版（v1.x）執行 `tmux-manager --update` 會讀取 main 上的 `lib/constants.sh` 版本號，版本不同就改跑 `install.sh` 安裝 Rust binary 並清除舊版檔案，因此**合併到 main 前必須先推送對應 tag 完成發布**。

支援 Linux（x86_64、aarch64）、macOS（x86_64、aarch64）與 WSL；Windows 原生不在範圍。

## 開發與驗證

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test tmux_pty -- --ignored --nocapture
./tests/bats/bin/bats tests/   # install.sh
```

真實 PTY 測試需要 tmux、python3 以及建立 socket/PTY 權限，CI 另列必跑步驟。Rust 固定 1.88.0 與 Cargo.lock。binding 使用 `run-shell -C` 在 popup 前展開原 pane/socket，再以 direct argv 啟動 binary。Bash 版（v1.x）程式碼已移除，需要時可從 `v1.2.1` tag 取得。`lib/constants.sh` 只保留 `VERSION`，必須與 Cargo 版本同步，讓舊版 `--update` 偵測到新版並改裝 Rust binary。
