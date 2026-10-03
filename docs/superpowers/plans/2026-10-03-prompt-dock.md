# Prompt Dock Implementation Plan

> **For agentic workers:** 使用 superpowers:executing-plans 原地執行並採 TDD。使用者已確認底部常駐列、僅滑鼠、貼入上方 active pane；沿用目前分支與不 commit 的要求。

**Goal:** 從 manager attach 後，在 tmux window 底部顯示已保存 prompt，滑鼠點選填入最近 active 的工作 pane。
**Architecture:** 既有 attach 啟動獨立 session dock supervisor，建立／清理有本次 owner 標記的 full-width 底部 pane。Bar 使用現有 PromptStore/PasteService 與 Ratatui mouse hit map；每次點擊重新查詢同 window 的 active/last 工作 pane，貼上後返回焦點。Supervisor 隨 session clients 存活，支援切換／新增 window，不修改 tmux.conf 或按鍵 binding。
**Tech Stack:** 現有 Rust、tmux 3.3+、Ratatui、fs2/sha2/uuid，無新增依賴。
**Spec:** 使用者本輪確認的示意圖與「僅能用滑鼠按、往上方 active pane 填入」；保存資料與既有貼上能力判斷不變。

## Global Constraints

- 分支 feat/rust-prompt-slots，所有變更保留未 commit，不安裝到正式 prefix。
- 底部高度四行，滿 window 寬，split 不選中 bar；高度不足時保留工作區並回報狀態。
- Bar 不處理方向鍵／Enter 為 prompt 操作。只接受左鍵 Down 在真實 button Rect 上，release/drag/右鍵不貼上；滑鼠滾輪／左右按鈕換頁。
- 每次點擊依同 window 的 active 工作 pane，bar active 時改用 last 工作 pane；不得選其他 window/pane 作任意 fallback。
- payload 經既有 PasteService，指定 socket/pane ID，不補送 Enter、不 broadcast；未知多行能力仍僅存 buffer，錯誤在 bar 保留。
- 每個 session 僅一個 supervisor（fs2 lock）；只移除 owner 匹配的 bar panes，恢復暫時 mouse 設定，不修改其他 panes/bindings/config。
- 既有直接管理與 popup 入口保留，prompts 文件變更自動重新載入。

## Review Focus

- 點擊先讓 bar active，active/last 判定必須仍貼向最近工作 pane；兩個 pane 與同步開啟時 byte-exact/no Enter。
- target 死亡、copy mode、pane input off、沒有 last、或別的 dock pane 不得被當成工作目標。
- 多 prompts、40 欄/1欄、中文/emoji 裁切、空列表、換頁後 UUID 對應。
- attach/detach、重複 attach、window 新建/切換、inside-tmux switch-client 與 supervisor 清理。
- user 已有 mouse/binding/布局；僅暫時啟用 session mouse，清理後工作 panes 仍在，不殺 user process。

## Task 1: Mouse-only renderer and target resolution

**Files:** src/prompts/dock_ui.rs、src/tmux/dock.rs、tests/prompt_dock.rs。
**Interfaces:** DockView::render(frame, slots, status) 與 DockView::handle_mouse(event)->Option<Uuid>；TmuxClient::dock_target(bar:&PaneId)->Result<PaneId>。
- [x] 寫 renderer/hit map keyboard-independent、page、Unicode、target active/last 排除 dock 的失敗測試，看 RED。
- [x] 實作顯示標題 buttons、scroll/page hit map、實際 tmux targets 查詢，看 GREEN。

## Task 2: Dock lifetime and attach integration

**Files:** src/prompts/dock.rs、src/prompts/dock_session.rs、src/cli.rs、src/entry.rs、src/app/runtime.rs、tests/dock_smoke.py、tests/tmux_pty.rs。
**Interfaces:** dock::run(tmux, store, initial_pane)->Result<()>；dock_session::start(tmux, config_path, session)->Result<Child>；dock_session::run(tmux, config_path, session)->Result<()>。
- [x] 真實 PTY 先證明 manager attach 沒有底部 prompt；再實作 supervisor／full-width pane 與 hidden CLI，確保 MouseDown 貼單行不送出、focus restore。
- [x] 拓展真實 PTY：兩個工作 panes、最近 active 改變、multi-line mode、release/drag/key 無貼上、prompt reload、windows 切換與 detach 清理，逐項 RED→GREEN。
- [x] supervisor 清理與 mouse 設定還原，重複 session lock；inside-tmux 不把生命週期綁到 manager UI。

## Task 3: Verification and reviewable binary

**Files:** README.md、validation 紀錄與本 plan。
- [x] 執行 fmt、clippy -D warnings、完整 Rust suite、兩版 tmux ignored PTY，Bats 適當檢查。
- [x] 沿用已授權的獨立 reviewer 審查 target/lifetime；必要修正先 RED。
- [x] 重建 release 並用使用者執行的 binary 跑 dock 真實 PTY；附操作說明，保留未 commit 狀態。
