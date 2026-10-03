# Prompt Dock 驗證與獨立審查

使用者確認 attach 後底部常駐、只用滑鼠點選、貼入上方 active pane。分支 `feat/rust-prompt-slots`，不 commit／發布／正式安裝。

## 實作

Manager attach 啟動獨立 supervisor，每個 window 一個四行、全寬的底部 pane；保留工作 pane 的 focus。新增 window 會自動建立列，resize 後重新維持四行，空間不足則安全退化。每個 session 的 supervisor 以 fs2 lock 去重；只清除本次 owner UUID 的 panes，最後一個 client detach 後恢復暫時的 session mouse 設定。不修改 tmux.conf 或鍵盤 binding。

Bar 只在實際 button Rect 的左鍵 Down 貼上，release／drag／右鍵／方向鍵／Enter 都不觸發 prompt。滾輪或左右按鈕翻頁，UUID 對應不依賴位置。點擊當下重新查詢同 window 的 active 工作 pane；bar 被選中時使用 last 工作 pane，所有 dock 都被排除，沒有任意 pane fallback。貼上沿用 PasteService，不補送 Enter、不廣播；貼完還原工作 focus。管理器 pane picker 與 AI 擷取的 pane 清單也排除 dock。

PromptStore 定期重新載入，點擊時再讀一次，移除或損壞檔案不會貼上舊快取。多行能力未知／關閉時僅存 buffer，底部用短訊息與具名 buffer 顯示，窄畫面依終端欄寬分行。

## TDD／審查修正

- 第一個真實 PTY 測試先重現 manager attach 沒有底部列，再實作並驗證滑鼠填入與 focus。
- Target active／last、mouse UUID／忽略事件、翻頁／Unicode／小畫面、窄畫面 buffer 名稱都有行為測試。
- 獨立 reviewer 發現 resize 隨 tmux 布局把 dock 撐高到 12 行（IMPORTANT）。80×24 → 80×40 的真實 PTY 先 RED，再修正 supervisor 維持四行，GREEN。
- 獨立 reviewer 發現六個 windows、每條 tmux 指令延遲 0.6 秒時，全程八秒 timeout 強制 SIGKILL 留下三個 dock panes 與 mouse on（IMPORTANT）。同條件先 RED；移除整體 deadline，保留每條 tmux 指令原有兩秒 timeout，等待 supervisor 自己完成／清理，GREEN。
- 啟動途中 SIGTERM 清理另先 RED。初始化前註冊 SIGINT／SIGTERM，pending stop 優先於 READY，資源建立完成後統一清理，不讓 parent attach，GREEN。
- Pane picker 原本會列出 dock，真實 PTY 先 RED，查詢 owner 並排除後 GREEN。
- Reviewer 唯讀復核生命週期修正，獨立重跑慢速啟動與 SIGTERM 回歸均 PASS，沒有 remaining blocking issue。

## 真實操作範圍

隔離 Linux tmux 3.4／3.7c 與 xterm-256color PTY。覆蓋兩個工作 panes、active／last 切換、focus 還原、方向鍵仍送到工作區、bar Enter 不貼上、單行 byte-exact 與不補 Enter；多行在 3.4 降級為 buffer、3.7c bracketed paste 保留中文／emoji／尾端 LF。另覆蓋同步 pane 不廣播、resize、保存後重新載入、新 window、重複 attach／多 client 與最後 detach 清理、session mouse 還原。

另一 fixture 從 tmux 內啟動 manager，switch-client 後 manager 退出，bar 仍能填入工作區；含空白的 binary 路徑與 detach 復原通過。慢速 fixture 檢查六個 windows 完整啟動，以及中止時 owner panes／mouse 清理。

跨平台、人工終端操作與 Claude 的待驗收仍沿用原計畫 Task 8。本輪不宣稱正式平台驗收或已發布。

## 最終檢查

`cargo fmt --all --check`、`cargo clippy --offline --locked --all-targets -- -D warnings` 通過；完整 Rust suite 79 項通過，Bats 133 項通過。八項真實 PTY 另在 tmux 3.4／3.7c 各全數通過。啟動中止 fixture 等待 manager 顯示 attach 失敗再送退出鍵，避免在終端暫時交接時遺失輸入。

`cargo build --release --offline --locked` 完成，兩版 tmux 另外使用 `target/release/tmux-manager` 執行 dock_smoke 皆通過。版本 2.0.0，Git HEAD 仍為 `6ff30a26da6bfe49ead4af12873eb8f4c8f48ef0`，`git diff --check` 通過，保留未 commit 工作區。

[80 欄底部列預覽](assets/prompt-dock-wide.png)／[40 欄預覽](assets/prompt-dock-narrow.png)由實際 Ratatui renderer 與測試資料產生，背景與字型為預覽設定。
