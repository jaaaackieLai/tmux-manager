# Session Action Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. 本計畫原先只撰寫，使用者其後已於 2026-10-03 指示「執行吧」；執行仍保持未 commit。

**Goal:** 將獨立 session 詳細頁改為列表上的小型操作選單，讓使用者操作時仍看得到 Sessions 與 Preview。

**Architecture:** 沿用 `Screen::Detail` 作為操作選單開啟狀態，保留既有 action dispatch；manager 始終先畫列表／Preview，再繪製浮動選單。選單 renderer 提供含 session ID 的獨立 hit map，modal 事件優先處理，避免點擊穿透及錯誤操作背景 session。背景刷新在選單開啟時仍更新 Preview／摘要。

**Tech Stack:** 現有 Rust 1.88／Ratatui 0.29／Crossterm 0.28／Tokio；不新增依賴。

**Spec:** 本檔「已確認互動設計」，依使用者於 2026-10-03 的確認與修正：接受 Enter／左鍵浮動選單；刪除「→ 按兩次」提案；只寫計畫，等待後續執行指示。

## 已確認互動設計

- 列表 ↑↓／Tab 選 session；Enter 或左鍵單擊 session 開啟浮動選單。第一次開啟只顯示選單，不直接 attach。
- 選單四項為 `attach`、`rename`、`kill`、`back`；`back` 即關閉選單。預設選中 attach，↑↓／Tab 選項，Enter 或左鍵單擊項目執行。
- Esc／q／點擊選單外關閉選單，保留所選 session、Preview 與面板比例。同一筆外部點擊只關閉，不再觸發背景 session、prompt 入口或 divider。
- 沿用選單內 `a/r/k` 快捷操作；`p` 仍先選明確工作 pane，再開 prompt。列表 `p` 仍進入沒有 paste target 的管理模式。
- `kill` 沿用原確認表單；rename／kill 取消後回選單。操作失敗仍留在 manager，呈現錯誤；attach 成功後 detach 回列表。
- 移除整頁詳細頁畫面。浮動選單置中於 manager 內容區，正常最大 56 欄 × 15 行；名稱與 stable ID、windows／created、摘要為精簡資訊，列表與 Preview 在選單周圍仍可見。
- 小尺寸向可用 Rect 夾緊；低高度優先保留可捲動的操作項目，省略資訊／提示，選中項在可用內容行中可見。沒有可用內容行時不產生可點擊 action Rect；Esc 仍可關閉，1×1 不 panic。
- **不實作向右按兩次，也不增加右方向鍵的開啟或執行功能。** 底部常駐 prompt dock 的操作方式維持原狀。

## Global Constraints

- 沿用分支 `feat/rust-prompt-slots`，保留現有未提交變更；不 commit／push／發布／正式安裝，不重做 Rust 遷移或底部 dock。
- 使用者已指示開始執行；完成情況以各 task checkbox 與驗證紀錄為準。執行採 native、TDD；獨立 code review 沿用使用者已授權的 reviewer 流程，不為每個 task 新開 agent。
- 不修改 tmux.conf、鍵盤 binding、paste 安全契約、UUID prompt 身分或 AI API 契約。
- 點擊以實際 render Rect 與 stable session ID 判斷；release／drag／右鍵不得觸發 action。
- 選單期間禁止背景 session 導航／divider 拖曳與比例快捷鍵；滾輪只移動選單項目。hover 不清畫面。
- 保留已修復的 nonce batch 分界及 Preview／摘要真實擷取回歸，不回退 `src/tmux/session.rs`。
- 各 task 先新增或更新一個行為測試，確認 RED，再做最小實作至 GREEN；其餘案例逐一重複。測試失敗最多修正／重跑五次再回報。

## Review Focus

1. 選單外點擊恰好落在其他 session／prompt header／divider：僅關閉，原選取與比例不變。
2. 選單開啟後背景 session 改名、排序或消失：保持同一 stable ID；目標消失關閉選單，舊 hit map 不得操作替代 session。
3. 80×24 → 40×12、長中文／emoji、1×1：選單／hit map 跟著 geometry 更新，不使用舊固定座標，不 panic。
4. 選單開啟時工作區持續輸出／摘要返回：Preview／AI 持續更新；關閉後仍是最新內容，不因 Detail 狀態停止 capture。
5. 表單取消、Ctrl-C、kill 否決、attach 失敗與 detach：恢復正確選單／列表，管理器與終端狀態正常，底部 dock 不受影響。

## 檔案責任

| 檔案 | 變更責任 |
|---|---|
| `src/ui/manager_actions.rs`（新增） | 浮動選單 geometry、精簡資訊、操作列與 modal hit map |
| `src/ui/manager.rs` | 先畫列表／Preview，最後加選單；更新 footer 文案 |
| `src/ui/manager_mouse.rs` | 新增 `ActionMenuHitMap` 與 `ManagerHitMap.action_menu` |
| `src/ui/mod.rs` | 匯出新 renderer |
| `src/app/state.rs` | modal 滑鼠路由、開啟／關閉、stable ID 驗證；保留鍵盤 action |
| `src/app/layout.rs` | 確認選單開啟時比例／拖曳事件不會穿透 |
| `src/app/runtime.rs` | 選單開啟期間仍 capture 選中 session 的 Preview；沿用 action dispatch |
| `tests/manager_actions.rs`（新增） | geometry、背景可見、modal 點擊遮擋、選中項與 resize 行為 |
| `tests/manager.rs`、`tests/manager_layout.rs` | 更新舊詳細頁假設，保護導航／ID／比例／prompt 入口 |
| `tests/terminal_smoke.py` | 真實 PTY 以文字與 render 位置定位新選單；操作及背景刷新回歸 |
| `README.md`、`docs/zh-tw/README.md` | 將「詳細頁」改為「操作選單」，說明 Enter／滑鼠與取消方式 |
| `docs/superpowers/validation/2026-10-03-session-action-menu.md`（新增） | 記錄 RED／GREEN、獨立 review、release 驗證及限制 |

## Task 1：列表上繪製浮動操作選單與 modal 事件

**Files:** `src/ui/{manager_actions,manager,manager_mouse,mod}.rs`、`src/app/{state,layout}.rs`、`tests/{manager_actions,manager,manager_layout}.rs`。

**Interfaces:**
- Consumes: `AppState`、`Screen::{List,Detail}`、`DETAIL_ACTIONS`、`ManagerAction`、`SessionId`，沿用目前欄位與 enum，不為了改畫面重命名整套狀態。
- Produces: `ActionMenuHitMap { area: Rect, session_id: SessionId, rows: Vec<(Rect, usize)> }`，定義於 `manager_mouse.rs`；`ManagerHitMap` 增加 `action_menu: Option<ActionMenuHitMap>`。
- Produces: `manager_actions::render(frame: &mut Frame, area: Rect, app: &AppState) -> Option<ActionMenuHitMap>`；無有效選中 session 時回 `None`。`area` 是 manager 內容區，繪製 `Clear` 加 bordered Block，行命中使用 `ListState.offset()`。
- Retains: `manager::render(frame: &mut Frame, app: &AppState) -> ManagerHitMap`、`AppState::handle_mouse(event: MouseEvent, hits: &ManagerHitMap) -> ManagerAction`、`handle_key` 與 layout handler 簽名。

- [x] **Step 1 — Renderer RED：** 新增 `action_menu_keeps_list_and_preview_visible`：80×24、兩個 sessions、Preview marker；Enter 後選單存在，兩個面板與未被遮住的背景 session 仍存在，menu 最大 56×15。新增 `small_action_menu_keeps_selected_action_visible`：40×12／12×5／1×1，geometry 在內容區內；有內容行時最後一項也可捲動至可見，沒有超界 hits。
- [x] **Step 2 — 確認 RED：** `cargo test --locked --test manager_actions`；預期目前沒有 modal geometry，或 renderer 仍為整頁詳細頁而失敗。
- [x] **Step 3 — Renderer GREEN：** 實作新 renderer 與 hit map；`manager::render` 在 List／Detail 都繪製原本 Sessions／Preview，Detail 才覆蓋 modal。列表 footer 用 `[Enter] 操作`；選單提示包含 `[p] prompts` 與 `[Esc] 關閉`。
- [x] **Step 4 — Modal RED：** 逐一加入 `outside_click_only_dismisses_menu`、`menu_wheel_does_not_change_session`、`stale_menu_hit_cannot_retarget_another_session`。斷言選單外 session／prompt header／divider 點擊只關閉且沒有 action；右鍵／release／drag 忽略；列表刷新重排或改名保留 ID，目標移除關閉且舊 action hits 不得生效。在既有導航測試加入兩次 `KeyCode::Right` 仍為 List，沒有 action。
- [x] **Step 5 — Modal GREEN：** `handle_mouse` 在 Detail 優先處理 modal，不先呼叫 divider／背景 hit handler。可見 action 左鍵 Down 經 session ID 核對後共用 Enter 行為；外部左鍵 Down 關閉且消耗該事件。邊框左鍵不執行；滾輪只移動 action；layout handler 保持 Detail 不受理比例調整。
- [x] **Step 6 — Task 驗證：** `cargo test --locked --test manager_actions --test manager --test manager_layout` 全數 PASS。既有測試依實際 `action_menu.rows` 選座標，不能只替換固定 y 值來勉強通過。保留未 commit。

## Task 2：選單中的 Preview 更新與完整操作回歸

**Files:** `src/app/runtime.rs`、`tests/terminal_smoke.py`；必要時補充 `tests/manager_actions.rs` 的背景狀態更新案例。

**Interfaces:**
- Consumes: Task 1 的 `ActionMenuHitMap`；原 `AppEvent::Refresh`、`AppEvent::AiResult`、`TmuxClient::snapshot` 與既有 action／form API。
- Produces: poll 在 List／Detail 都使用 `app.selected_id().cloned()`；不再因選單開啟而停抓 Preview。不改 tmux adapter API 或 AI generation 規則。
- Retains: rename／kill 取消留在 Detail；成功回 List；attach 失敗留 manager，detach 回 List；modal `p` 開明確 pane picker，返回後恢復選單。

- [x] **Step 1 — 真實 PTY RED：** 修改既有 fixture：左鍵開選單後，同時確認 `Sessions`／`Preview`；依實際螢幕定位選單項目。選單保持開啟時由測試 tmux pane 印出新的 `MODAL-PREVIEW-UPDATE`，要求 manager 擷取並渲染該 marker；讓 marker 位於選單未遮住的 Preview 行。原 runtime 因 Detail 不 capture 而應失敗。另外用 AppEvent 的實際更新入口驗證摘要抵達時保留 modal target／action selection。
- [x] **Step 2 — 執行 RED：** `cargo test --locked --test tmux_pty real_pty_manager_editor_restart_resize_ctrl_c_and_terminal_restore -- --ignored --nocapture`；只能使用 fixture 自己的隔離 socket，不操作使用者 tmux server。
- [x] **Step 3 — GREEN：** 移除 runtime 中「Detail 不抓 Preview」分支，背景刷新照 stable ID 套用。更新 PTY 的 `[Enter] 詳細` 等舊文案與整頁假設，維持 marker、termios 及錯誤斷言。
- [x] **Step 4 — 逐一補回歸：** PTY 覆蓋 Enter 開啟不 attach、左鍵 attach／detach、rename Esc／Ctrl-C、kill 否決、外部點擊關閉、resize 後 action 點擊、`p` pane picker 返回選單、失效 session attach 錯誤。每個新增案例先確認失敗，再做必要修補；不得移除既有 kill 確認／終端復原測試。
- [x] **Step 5 — Task 驗證：** 上述真實 manager PTY 在 tmux 3.4／3.7c 都 PASS；`cargo test --locked --test tmux_capture -- --ignored` 兩版都 PASS，保護 Preview／摘要 batch 擷取與 dock 排除。保留未 commit。

## Task 3：文件、獨立 review 與可試用 release

**Files:** `README.md`、`docs/zh-tw/README.md`、新增 validation 紀錄；前兩 task 受 review 影響的檔案。

**Interfaces:** Consumes: Task 1／2 最終互動與測試證據。Produces: 更新後的 `target/release/tmux-manager` 與可查閱驗證紀錄，供使用者人工確認；不安裝到正式 prefix。

- [x] **Step 1 — 更新文件：** 描述 Enter／左鍵小選單、外部點擊關閉、快捷鍵與 modal `p` 明確 pane；不記載已排除的右方向鍵操作。validation 區分本機實測、mock API 與未完成的平台驗收。
- [x] **Step 2 — 完整檢查：** `cargo fmt --all --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked`、`./tests/bats/bin/bats tests/` 全 PASS。真實測試明確另跑 `cargo test --locked --test tmux_pty --test tmux_capture -- --ignored --nocapture`，tmux 3.4／3.7c 各 PASS，不以 ignored 計為通過。
- [x] **Step 3 — 獨立 code review：** 沿用使用者已要求的獨立 agent，唯讀審查 modal 遮擋、stable ID、resize、背景刷新及 action 返回。發現確定 bug 時先寫 RED、修補 GREEN，再請 reviewer 複核；不為本次計畫撰寫啟動 review。
- [x] **Step 4 — Release 驗證：** `cargo build --release --locked`；兩版 tmux 各執行 `python3 tests/terminal_smoke.py ./target/release/tmux-manager` 與 `python3 tests/dock_smoke.py ./target/release/tmux-manager`。要求新選單、真實 Preview、既有底部 dock 都 PASS。
- [x] **Step 5 — 留待使用者驗收：** `git diff --check` 通過，記錄 branch／HEAD／命令結果與 review。回報如何重新啟動試用，保持未 commit，不 push／發布／正式安裝。

## 接續執行資訊

- 本機可用 `/tmp/tmux-manager-cargo-run` wrapper 與 `--offline --locked`；若下一次已不存在，先確認 Rust 1.88 與 cache，不假設臨時環境永久存在。
- 系統 tmux 3.4；3.7c 曾位於 `/tmp/tmux-manager-newtmux/install/bin/tmux`。執行第二組測試前先查版本；不可將同一版本跑兩次當作雙版本驗證。
- PTY／隔離 socket／本機 mock HTTP 可能需要 sandbox escalation，限定測試資源；不得 kill 使用者 server。
- 目前 HEAD：`6ff30a26da6bfe49ead4af12873eb8f4c8f48ef0`；branch：`feat/rust-prompt-slots`。現有 Rust 實作多為未追蹤檔案，不能僅以 tracked `git diff` 判斷工作區內容。
- 前一輪驗證：83 Rust／133 Bats PASS，真實 tmux 3.4／3.7c Preview／摘要擷取與 release manager／dock smoke PASS。參考 `docs/superpowers/validation/2026-10-03-preview-summary-regression.md`；外部 AI API 與跨平台人工驗收不等於已完成。
- 本輪已收到執行指示；接續時讀取本計畫專屬 ledger，從第一個未完成 task 繼續，不重做已驗證部分。

## 計畫自查

- 已覆蓋只用 Enter／左鍵、背景可見、取消保留 state、kill 確認、不加右方向鍵序列等已確認需求。
- 五項 Review Focus 分別由 Task 1 modal／stable ID／geometry 與 Task 2 刷新／操作 PTY 測試負責。
- Task 1 的 hit map 與 renderer 簽名在後續沿用；Task 2 不引入新的 Screen variant 或改動 paste／AI API。
- 此計畫依後續指示執行；完成狀態需對照 checkbox 與驗證紀錄，不包含 commit 步驟。

## 執行結果

2026-10-03 三個 tasks 已執行完成；完整 Rust 89／Bats 133 與兩版 tmux 各10真實測試、release manager／dock smoke通過。獨立 reviewer 無阻擋問題；超長名稱可能遮掉選單 ID 的 MINOR 保留待確認。詳見 [驗證紀錄](../validation/2026-10-03-session-action-menu.md)。工作區保持未 commit。

## 驗收後間距修正

使用者要求選項有間距、按鍵說明改一行。正常選單上限調為56×15，每項兩行（label＋空白），hit map僅對應label；空間不足回到可捲動compact。Footer單行並依欄寬縮短文案。spacing與單行提示測試各RED→GREEN；獨立review無新增問題。
