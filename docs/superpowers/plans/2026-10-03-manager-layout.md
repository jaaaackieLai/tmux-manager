# Manager Layout Implementation Plan

> **For agentic workers:** 使用 superpowers:executing-plans 與 TDD，自行在目前分支執行；使用者要求保留未 commit 狀態。

**Goal:** 改善 session 列表可讀性，自動分配 Sessions／Preview，並支援拖曳及鍵盤調整。
**Architecture:** 純 geometry 函式決定兩個面板與 divider；獨立 session renderer 共用 metrics 產生實際行與 stable ID hit map；AppState 保存手動比例及拖曳狀態，runtime 僅在畫面狀態變更時重畫。
**Tech Stack:** 現有 Rust／Ratatui／Crossterm，不新增依賴。
**Spec:** 使用者本輪確認自動配置＋拖曳／鍵盤；名稱、windows、AI 欄位對齊，列表加列距與選取底色，窄畫面 AI 分行。

## Global Constraints

- 保留目前分支，不 commit／發布／正式安裝；保留 manager 內的 prompt 入口與返回。
- 明確 stable ID 與 render Rect 對應；捲動、resize、拖曳不得點到其他 session。
- 70 欄以上：單行內容＋間距，stride 2；窄畫面：名稱／windows、摘要、間距，stride 3；小高度可退回 compact。
- 自動列表高度依列數與 row metrics 決定，上限約可用高度 60%；兩個面板各保留至少 3 行，空間不足時安全退化。
- 手動比例只在本次執行保留。`[`／`]` 每次調整約 5 百分點且至少一行，從目前實際比例開始；`\` 回自動。
- divider 左鍵按下才開始拖曳；一般 drag／release 不執行 session action；hover 不清畫面。
- Preview 顯示目前高度可容納的最後幾行，最多 15 行。

## Review Focus

- 1×1／40×12／80×24／160×40，長中文／emoji名稱、session 大量或空列表。
- 更新比例後 mouse hit map 需跟著 redraw；分隔線不可當成 session 點擊。
- Session 捲動後 ID 與實際顯示列一致；空白間距與表頭不可觸發開啟。
- 手動比例遇到縮放需夾緊兩個面板，reset 後回到內容自動配置。
- 新視覺位置會變更既有 PTY 座標；fixture 依實際螢幕定位 session，不保留舊固定行號。

## Task 1：列表與面板調整

**Files:** 新增 src/ui/manager_layout.rs、src/ui/manager_sessions.rs、src/app/layout.rs；修改 src/ui/{manager,manager_mouse,mod}.rs、src/app/{state,runtime,mod}.rs；tests/manager_layout.rs、tests/manager.rs、tests/terminal_smoke.py、README.md、validation。
**Interfaces:** `Panels::new(body: Rect, count: usize, percent: Option<u16>)` 回 sessions/divider/preview Rect；`SessionMetrics` 定義 stride/header/capacity；`manager_sessions::render(frame, area, app) -> Vec<(Rect, ManagerHit)>`；`AppState::handle_layout_key(key, hits) -> bool`；hitmap 保存可用 body/panels geometry，供 drag 及鍵盤推導實際比例。

- [x] 先寫「大視窗只有少量 sessions 不保留巨大空面板」測試，看 RED，再實作 auto geometry。
- [x] 先寫欄位對齊／列距／選取底色與窄畫面測試，看 RED，再實作 session renderer，更新舊座標 fixture。
- [x] 先寫 divider drag、keyboard＋reset、resize clamp 與 selection 保留測試，看 RED，再接 AppState/runtime。
- [x] 完整 Rust suite、fmt／clippy、兩版 tmux 真實 PTY、最終 release 實測；獨立 reviewer 審查 resize 與 hit map。
- [x] 保留未提交修改與 reviewable release，README 說明控制方式、validation 紀錄實測限制。
