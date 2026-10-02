# Rust 遷移與 Prompt Slots 設計草案

日期：2026-10-02。狀態：供審閱，尚未開始實作。

## 目標與已確認需求

- 將目前 Bash 實作的 tmux-manager 改為 Rust，保留既有 session 管理與 AI 摘要功能。
- 增加可自行新增、編輯、儲存的 prompt slots。
- 使用者已確認主要情境：在 tmux 中使用 AI CLI，點選 prompt 後貼到指定 pane。
- 成功標準：在正在使用的 AI CLI 上開啟 prompt 選單，點選後回到原 pane，內容可編輯，由使用者送出。

以下為計畫建議，尚未確認為使用者要求：維持 TUI；支援 Linux、macOS、WSL；本機全域 prompt 集；不加入 GUI、系統剪貼簿、雲端同步或模板變數。

## 現況

主程式 `tmux-manager` 載入 `lib/*.sh`；版本 1.2.1，沒有 Cargo 專案。
已有 list/detail TUI、new/attach/rename/kill、目前 pane 預覽、多 pane AI 擷取、背景 Anthropic 摘要與命名建議、設定 CLI、安裝、自我更新及解除安裝。
設定檔是會被 Bash 執行的 `config.sh`；測試為 Bats，`tests/bats` 是 submodule。
attach 後管理器等待 tmux client 結束，不能直接在管理器畫面點 prompt；因此必須補上 session 內的入口。
目前 Windows 工具環境可找到 WSL，找不到 PATH 上的 cargo/rustc；本次不安裝工具、不假設 WSL 已具備 Rust 或 tmux。

## 架構選擇

| 方案 | 優點 | 代價 | 判斷 |
|---|---|---|---|
| Rust TUI，透過 tmux CLI 管理 session | 延續既有工作流、適合 SSH、單一執行檔 | 需實作滑鼠與 popup | 建議 |
| Rust 原生桌面 GUI | 滑鼠操作直觀 | 重做桌面視窗、遠端與終端整合 | 第一版不採用 |
| Rust 核心搭配 Web/Tauri 介面 | Web UI 彈性大 | 額外前端與部署方式，產品程式碼不全是 Rust | 不符合這次方向 |

一個 Cargo package，避免先拆成多個 crates。使用 Ratatui/Crossterm、Clap、Tokio、Reqwest（Rustls）、Serde、TOML，以及文字編輯、Unicode、檔案鎖、暫存檔與 checksum 所需的少量函式庫。
實作起點選擇互相相容的穩定版本並鎖定 Cargo.lock，不在計畫中猜最新版本。
所有產品執行邏輯、HTTP、設定、更新與安裝操作用 Rust；tmux 仍是外部必要工具，AI 不再依賴 curl/jq。可保留薄的 shell 下載入口以相容既有安裝網址；它只取得 Rust binary，沒有產品邏輯。嚴格零 shell 的使用者直接下載 binary 或 cargo install。

## 平台、設定與既有功能

- 支援 Linux、macOS、Windows 的 WSL；Windows 原生 tmux 整合不在第一版範圍。
- 先以 tmux 3.3+ 為測試基線，啟動時檢查 popup 與貼上相關能力；多行直貼另需可判定 bracketed paste 的能力。
- 保留既有按鍵與 `--help`、`--version`、`--config`、`--update`、`--uninstall`；help/version 不需要 tmux。
- 正式設定：`${XDG_CONFIG_HOME:-$HOME/.config}/tmux-manager/config.toml`。
- 保留 `TMUX_MANAGER_CONFIG_FILE` 和既有 `TMUX_MANAGER_*` 環境變數。優先序：CLI > 環境變數 > TOML > 預設值；這是 Rust 版明確的新規則。
- 現有五個設定鍵與 `POLL_INTERVAL=0.2` 預設保持；空字串可以透過 config CLI 寫入。
- 提供明確的 `migrate-config` 指令：只解析允許清單上的簡單 assignment，支援引號及已知 HOME 展開；不執行 config.sh。遇到命令替換、shell 指令或其他展開，報告行號並停止寫入。保留原檔，目的檔已存在則不覆寫。
- AI 保留 Anthropic provider 與目前預設模型，模型改為可設定；無 key 時照常使用 session 與 prompt 功能。摘要仍以最多 80 行、多 pane 標記內容為輸入。
- AI 背景任務上限 4，同一輪 request timeout 15 秒；刷新取消上一輪任務，結果依 session ID 和 generation 對應，避免改名／刷新後套錯資料。

## Prompt Slots 第一版

全域儲存檔 `prompts.toml` 放在設定檔所在目錄；以 schema_version=1 管理格式。
每個 slot 有穩定 UUID `id`、`title`、`body`、`tags`、`order`；slot 數量不預設上限，列表必須可捲動。
title 去除首尾空白後需非空；body 需有非空白內容，上限 64 KiB UTF-8；保留中文字、emoji、引號、反斜線與換行。
CRLF 在匯入／輸入時正規化為 LF；拒絕 ESC、NUL、裸 CR 及其他控制字元，允許 LF 和 Tab。
支援新增、編輯、刪除確認、移動順序、依標題／標籤搜尋與內容預覽；編輯器支援多行和終端貼上。
新增／編輯以 Ctrl-S 儲存，Esc 取消；有未儲存修改時顯示確認。
使用 sibling 暫存檔 + 原子替換，Unix 檔案權限 0600；使用獨立 lockfile 鎖定 read-modify-write，並比對 revision，避免 manager 與 popup 同時編輯時覆寫。
格式損毀時顯示錯誤並保留原檔，不把損毀資料當作空清單寫回。

## 互動流程

管理器內：session detail 按 `p` → 選擇 window/pane → prompt 選單 → 點選 slot → 貼到先前選定的 pane。
AI CLI 使用中：tmux prefix + 大寫 P → popup prompt 選單 → 點選 slot → popup 關閉 → 原 AI CLI 顯示貼上內容。
popup 左側顯示 slot 清單，右側預覽內容，底部顯示目標 session/window/pane。窄畫面改為上下布局。
點擊 slot 列即執行貼上；鍵盤上下／Tab 選取、Enter 貼上；Esc 取消。編輯／刪除是獨立操作，不混用貼上按鍵。
以實際 render 的 Rect 計算 hit testing；只處理一次左鍵按下，忽略放開與拖曳，避免一次點擊貼兩次。
無有效 target 時只有管理功能；必須選定 pane 後才能貼上，不能猜最近活動 pane。
popup 在開啟前取得原 pane ID 與 tmux socket；不可把 popup 自己的 TMUX_PANE 當作目標。
新增 `tmux-manager prompts --target-pane %12` 及 `tmux-manager bindings --print`；後者輸出適合使用者加入 tmux.conf 的綁定，不自動覆寫設定。
popup 以獨立 Rust 程式執行，不需要 daemon；開啟時重新讀取 prompt 檔，儲存立即對其他程式可見。

## 貼上契約與可行性驗證

只對明確的 pane ID 和同一 tmux socket 操作；呼叫 tmux 時使用 Rust Command 的參數陣列，不把 prompt 拼進 shell 指令。
內容由 stdin 傳給具唯一名稱的 tmux buffer，再使用 `paste-buffer -p -r` 貼上；不額外送 Enter/C-m。
在貼上前檢查 pane 仍存在、未結束、未關閉輸入、未處於 copy mode；pane 消失時停止，不重新定位其他 pane。
多行內容需要 bracketed paste。以 tmux 可用的 private-mode 查詢確認 mode 2004；未知或關閉時不直接注入多行內容，提供「存到 tmux buffer」及說明。
第一個實作任務必須在目標 tmux 版本及實際 AI CLI 驗證此能力。若基線版本無法查詢，保留單行直貼並清楚要求升級可支援版本，不能假造查詢欄位或假定 -p 一定有效。
同步輸入開啟時需證明只貼到指定 pane；若測試基線會廣播貼上，先阻止該情境並說明，不能暗中改全域設定。
tmux 命令成功只代表資料已交付 pane；「由使用者送出」需以實際支援的 AI CLI 驗收。前景程式切換存在競態，不能宣稱可通用保證所有 CLI 都不會送出。
貼上成功刪除本次命名 buffer；失敗保留一份具名 buffer 供手動取回並顯示名稱。不碰使用者的既有 buffers。

## 發布與遷移

Rust 遷移版建議作為 v2.0.0，清楚記錄設定格式、最低 tmux 能力及安裝方式變更。
先發布 Linux x86_64、macOS x86_64/aarch64 binary，Windows/WSL 使用 Linux binary。
Rust installer 支援 INSTALL_PREFIX，安裝到 prefix/bin；舊 symlink 必須先辨識、備份，再原子切換，不能透過 symlink 覆寫舊程式。
更新下載 GitHub Releases 對應平台的 artifact，驗證同版 checksum，成功才切換；失敗保留舊 binary。Checksum 用於完整性檢查，不宣稱等同發布者簽章。
卸載預設保留 config/prompts；只處理自己安裝的精確路徑，不遞迴清除使用者設定。
遷移期間保留 Bash 程式與 Bats 作為行為參考；功能對等後停止將它們納入正式安裝。本次規劃不刪除檔案、不操作 submodule。

## 驗收與範圍邊界

新 Rust 版應覆蓋既有功能的使用者行為，而非逐項複製 Bash 內部實作。
核心驗收：儲存中文多行 prompt → 關閉重開仍存在 → 在 AI CLI 開 popup → 滑鼠單擊貼回指定 pane → 內容完整且尚未送出。
另驗收未開 AI、空 session、pane 消失、多個 pane、快速重點、損毀設定、同時開兩個編輯視窗、終端縮放與正常／異常退出後終端狀態。
第一版不含網站／桌面版自動貼上、系統剪貼簿、prompt 雲端同步、模板變數或自動送出。

## 技術依據

- [Ratatui terminal/event 範例](https://ratatui.rs/recipes/apps/terminal-and-event-handler/)：terminal lifecycle 與滑鼠事件。
- [tmux 官方手冊](https://man.openbsd.org/tmux.1)：popup、穩定 pane ID、buffer 與 bracketed paste。
- [tmux format 實作](https://github.com/tmux/tmux/blob/master/format.c)：新版本 private-mode 查詢；須在支援基線驗證可用性。
- [tmux paste-buffer 實作](https://github.com/tmux/tmux/blob/master/cmd-paste-buffer.c)：貼上行為；不同支援版本仍需整合測試。
