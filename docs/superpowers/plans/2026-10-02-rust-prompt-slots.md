# Rust 遷移與 Prompt Slots Implementation Plan

> **For agentic workers:** 實作時使用 superpowers:executing-plans；只有使用者選擇委派時才使用 superpowers:subagent-driven-development。依下列核取項目逐項執行。本文件是規劃草案，沒有開始產品實作。

**Goal:** 用 Rust 保留 tmux-manager 的既有功能，讓使用者在 AI CLI 中點選已儲存 prompt 並貼回指定 pane。

**Architecture:** 一個 Cargo package、單一執行檔，同時提供 manager TUI、prompt popup 與設定／安裝 CLI。tmux adapter、AI worker 和 prompt repository 各自隔離，畫面透過事件更新；popup 是按需開啟的獨立程式。

**Tech Stack:** Rust、Ratatui/Crossterm、Clap、Tokio、Reqwest/Rustls、Serde/TOML；依需要加入 Unicode、文字編輯、UUID、檔案鎖、tempfile、SHA-256 函式庫。任務 1 選定相容穩定版本並鎖 Cargo.lock。

**Spec:** [設計草案](../specs/2026-10-02-rust-prompt-slots-design.md)。使用者已確認 tmux AI CLI 貼上情境，其餘建議以此草案為準，開始實作前先審閱。

## Global Constraints

- 溝通與文件使用繁體中文，commit message 使用英文。
- 支援 Linux、macOS、Windows 的 WSL；Windows 原生 tmux 整合不在第一版範圍。
- 所有產品執行邏輯、HTTP、設定、更新與安裝操作用 Rust；tmux 仍是外部必要工具，AI 不再依賴 curl/jq。
- 先以 tmux 3.3+ 為測試基線，啟動時檢查 popup 與貼上相關能力；多行直貼另需可判定 bracketed paste 的能力。
- CLI > 環境變數 > TOML > 預設值；五個現有設定鍵與 POLL_INTERVAL=0.2 保留。
- Prompt schema_version=1；body 上限 64 KiB UTF-8；title/body 不可只有空白；CRLF 正規化為 LF；拒絕其他控制字元，允許 LF/Tab。
- AI 背景任務上限 4，同一輪 request timeout 15 秒；預設 provider/model 保留，無 key 時其他功能完整可用。
- 不自動送 Enter/C-m；只對明確 pane ID 貼上，不因 pane 消失而改貼其他 pane。
- 設定與 prompt 寫入需原子替換；不執行舊 config.sh；保留舊設定與使用者 prompts。
- 不使用 PowerShell Remove-Item；此計畫不要求刪除舊碼或 submodule，實作期間採保留與備份。

## Review Focus

1. 多行／尾端換行與未知 paste 能力：不得把換行變成送出；由任務 1、5、8 驗證。
2. Popup 改變前景 pane、目標關閉／切換：始終使用原 pane ID/socket，失效即停止；由任務 5、7 驗證。
3. 中文、emoji、引號、控制字元及小終端：完整保存、正確顯示、禁入控制碼；由任務 4、6 驗證。
4. Manager 與 popup 同時編輯、檔案損毀：不丟資料、不覆寫損毀原檔；由任務 4 驗證。
5. 舊安裝是 symlink、下載失敗／checksum 不符：不覆寫 symlink 目標、不破壞現有可執行版本；由任務 8 驗證。

## 檔案與責任

| 新檔案／目錄 | 責任 | 現有對應 |
|---|---|---|
| Cargo.toml、Cargo.lock、rust-toolchain.toml | 相依版本、可重現建置 | 無 |
| src/main.rs、src/lib.rs、src/cli.rs、src/error.rs | CLI、入口、錯誤與退出碼 | tmux-manager |
| src/config/{mod,legacy}.rs | TOML、環境變數、舊設定匯入 | lib/config.sh |
| src/tmux/{mod,command,model,session,capabilities}.rs | 子程式、ID、session、能力查詢 | lib/sessions.sh、lib/actions.sh |
| src/app/{mod,state,event,actions}.rs | 狀態、事件、功能協調 | lib/constants.sh、lib/input.sh |
| src/ui/{mod,terminal,manager,hit_test}.rs | 終端復原、管理畫面、滑鼠命中 | lib/render.sh、lib/utils.sh |
| src/ai/{mod,anthropic,worker}.rs | HTTP、摘要解析、取消與限流 | lib/ai.sh |
| src/prompts/{mod,model,store,paste,ui,editor,popup}.rs | 儲存、編輯、貼上與 popup | 新功能 |
| src/distribution/{mod,install,update,uninstall}.rs | 發布與安裝管理 | install.sh、lib/update.sh |
| tests/{config,tmux_adapter,manager,ai,prompts,paste,popup,distribution}.rs | 有價值的行為測試 | tests/*.bats 作為參考 |
| tests/support/{mod,runner,pty}.rs | 假 subprocess、隔離 tmux、PTY fixture | tests/test_helper.bash |
| .github/workflows/{ci,release}.yml | Linux/macOS 檢查與 release artifacts | 無 |

修改 `.gitignore`、README.md、docs/zh-tw/README.md、install.sh；既有主程式、lib/*.sh、Bats 暫時保留為遷移參考，不再用作 Rust runtime。

## 執行順序與里程碑

任務 1 → 2 → 3 建立可用的 Rust 管理器；任務 4 → 5 → 6 → 7 完成 prompt popup；任務 8 完成 v2 發布與遷移。
不讓新版覆蓋既有正式安裝，直到任務 8 的驗收通過。各任務先新增能捕捉使用者行為的失敗測試，再實作、驗證及提交。

### Task 1：可行性與 Rust tmux adapter

**Files:** Cargo.toml、Cargo.lock、rust-toolchain.toml、src/{main,lib,cli,error}.rs、src/tmux/*、tests/tmux_adapter.rs、tests/support/*、.gitignore。
**Interfaces:** `PaneId(String)` 僅接受 `%` 加數字；`SessionId(String)` 使用 tmux session ID；`TmuxClient { socket: Option<PathBuf> }`；`list_sessions() -> Result<Vec<Session>>`、`list_panes(&SessionId) -> Result<Vec<Pane>>`、`capabilities(&PaneId) -> Result<PaneCapabilities>`。所有 tmux subprocess 都經過同一 runner，支援注入假的 runner。

- [ ] 定義行為測試：零 sessions 是正常空結果；特殊 session 名稱不經 shell；不存在 tmux 返回可讀錯誤；pane IDs 拒絕任意文字；help/version 不依賴 tmux。
- [ ] 執行 `cargo test --test tmux_adapter`，確認測試先因尚未實作功能失敗。
- [ ] 建立最小 Cargo 專案，鎖 toolchain/依賴；用參數陣列、明確 format 與 stable ID 呼叫 tmux。設定 tmux command timeout 2 秒，attach 不套此 timeout。
- [ ] 建立隔離 socket 的 PTY fixture。驗證基線 tmux popup、滑鼠、多行 buffer、mode 2004 查詢；不要使用不存在的 pane_bracketed_paste 欄位。探測可用的 pane_private_modes 並記錄最早通過實測的版本。
- [ ] 以會開啟／關閉 mode 2004 的測試程式驗證中文多行內容、尾端 LF、未送 Enter；有 synchronize-panes 的另一 pane 不得收到內容。無能力時測試預期降級行為，將實際能力矩陣寫回 spec。
- [ ] `cargo test --test tmux_adapter` 通過；Linux/WSL 與 macOS 的實測結果分別記錄，無法取得的平台標示待驗收。
- [ ] Commit: `feat: establish Rust tmux adapter and capability checks`。

### Task 2：設定、CLI 與舊設定遷移

**Files:** src/config/*、src/cli.rs、tests/config.rs。
**Interfaces:** `Config::load(cli: &CliOverrides, env: &EnvMap) -> Result<Config>`；`migrate_legacy(source: &Path, destination: &Path) -> Result<MigrationReport>`。提供既有五個 key 的讀寫及 AI model 設定；prompts 檔案位於解析後 config 的同一目錄。

- [ ] 新增測試：XDG/自訂 config path、優先序、bool 字面值、正數 poll interval、寫入空字串、多行字串 roundtrip；未知鍵不寫檔。
- [ ] 新增遷移測試：簡單 assignment/$HOME 可匯入；`$(...)`、反引號、source 等拒絕且顯示行號；來源原檔及既有目的檔不變。
- [ ] `cargo test --test config` 先確認失敗，再實作 TOML 與明確的 migrate-config CLI，不啟動 Bash 讀設定。
- [ ] 保留 `--config [--list | KEY | KEY VALUE]`；舊 .sh override path 顯示遷移說明，不能誤當 TOML 或執行 shell。
- [ ] `cargo test --test config` 通過；Commit: `feat: add typed configuration and safe legacy migration`。

### Task 3：既有 TUI、session 操作與 AI 對等

**Files:** src/app/*、src/ui/*、src/tmux/session.rs、src/ai/*、tests/manager.rs、tests/ai.rs。
**Interfaces:** `AppState` 保存選定 SessionId、畫面及 AI 狀態；`AppEvent` 包含輸入、刷新、預覽與 `AiResult { session_id, generation, result }`；`TerminalGuard` 負責 enter/suspend/resume/drop；`AiService::refresh(sessions: Vec<Session>, generation: u64)` 管理取消與並行限制。

- [ ] 新增管理行為測試：list/detail 按鍵、空清單、new 後 attach、目錄／啟動指令、改名與 kill 確認、detach 後回到 list、不自動重啟 AI。
- [ ] 新增終端與 render 測試：一般退出／錯誤／panic／Ctrl-C 復原 raw mode、游標與滑鼠；中文寬度及 80×24、40×12、縮放不越界。
- [ ] 新增 AI mock HTTP 測試：無 key、正常 SUMMARY/NAME、401/429、timeout、malformed body、多 content blocks、刷新後舊結果、session 改名；同時最多 4 requests、15 秒 timeout。
- [ ] 執行 `cargo test --test manager --test ai` 確認失敗，再實作 Ratatui 管理器、session actions 與 Anthropic async workers。tmux 擷取與 HTTP 不在 UI thread 同步等待。
- [ ] Attach 前暫停 UI、復原終端、交接 stdin/out/err；在 tmux 內走 switch-client，避免巢狀 attach；返回後重建畫面。
- [ ] 預覽保持 15 行上限、多 pane 摘要保持 80 行總預算；依內容與事件重繪，避免空閒期間無條件畫面輸出。
- [ ] `cargo test --test manager --test ai` 通過並以隔離 session 手動驗收；Commit: `feat: port session manager and AI summaries to Rust`。

### Task 4：Prompt 資料模型與可靠儲存

**Files:** src/prompts/{mod,model,store}.rs、tests/prompts.rs。
**Interfaces:** `PromptSlot { id: Uuid, title: String, body: String, tags: Vec<String>, order: u32 }`；`PromptDocument { schema_version: u32, slots: Vec<PromptSlot> }`；`PromptSnapshot { document, revision }`；`PromptStore::load() -> Result<PromptSnapshot>`、`commit(expected_revision: &str, next: &PromptDocument) -> Result<PromptSnapshot>`，衝突返回明確 Conflict。

- [ ] 新增測試：中文多行/emoji/引號 roundtrip、CRLF、空內容、64 KiB 邊界、ESC/NUL/裸 CR、重複 UUID、未知 schema、排序與搜尋。
- [ ] 新增可靠性測試：寫入中斷仍保留原檔；損毀 TOML 不覆寫；兩個 store 同時修改返回 conflict；新增檔案 Unix mode 0600。
- [ ] `cargo test --test prompts` 先失敗；實作獨立 lockfile、鎖內重新讀取及 revision 比對、同目錄暫存檔和原子替換。title/tag 可重複，身份只由 UUID 決定。
- [ ] `cargo test --test prompts` 通過；Commit: `feat: persist prompt slots with atomic writes and conflict detection`。

### Task 5：指定 pane 的貼上服務

**Files:** src/prompts/paste.rs、src/tmux/{command,capabilities}.rs、tests/paste.rs。
**Interfaces:** `PasteTarget { socket: Option<PathBuf>, pane_id: PaneId }`；`PasteService::paste(target: &PasteTarget, body: &str) -> Result<PasteOutcome>`；`PasteOutcome` 區分 Delivered 與 BufferedOnly，不能將 buffer-only 顯示為已貼上。

- [ ] 新增假 runner 測試：prompt 只走 stdin、唯一 buffer 名稱、目標 ID 不受活動 pane 改變影響、無 Enter/C-m、同一次命令不重送、不動其他 buffers。
- [ ] 新增失敗測試：不存在／dead／input-off／copy-mode pane、不同 socket、缺 paste 能力、多行 mode 2004 關閉、paste 途中失敗。失敗不自動改目標或重試注入。
- [ ] `cargo test --test paste` 先失敗；實作能力檢查 → load-buffer stdin → paste-buffer -p -r 到指定 pane → 成功清理本次 buffer。失敗保留本次 buffer 並顯示名稱。
- [ ] 如果能力未知，多行僅建立具名 buffer 並提供明確提示；單行仍可直貼。不可悄悄將多行合併成單行或假裝完成貼上。
- [ ] 用任務 1 PTY fixture 驗證 byte-for-byte 多行及單 pane 行為；`cargo test --test paste` 通過。
- [ ] Commit: `feat: paste saved prompts into explicit tmux panes`。

### Task 6：Prompt 管理畫面、編輯器與滑鼠

**Files:** src/prompts/{ui,editor}.rs、src/ui/hit_test.rs、src/app/{state,event,actions}.rs、tests/prompts.rs、tests/manager.rs。
**Interfaces:** `PromptAction` 包含 Select/Paste/Create/Edit/Delete/Move/Search；`PromptView::render(...) -> HitMap`；`handle_mouse(MouseEvent, &HitMap) -> Option<PromptAction>`。編輯器提交至 PromptStore，貼上交給 PasteService。

- [ ] 新增 render/event 測試：列表/預覽與窄屏布局、搜尋後索引、捲動後點擊映射、單次左鍵按下只貼一次、滑鼠抬起/拖曳不貼、Enter 貼上、Esc 取消。
- [ ] 新增編輯測試：多行與終端貼上、Ctrl-S 儲存、未儲存離開確認、刪除確認、重排、conflict 不丟草稿。
- [ ] 執行 `cargo test --test prompts --test manager` 先確認新測試失敗；實作 slot 列表、預覽、編輯表單與基於 render Rect 的 hit map。管理器 detail 加 `p` 和明確的 pane picker。
- [ ] 貼上成功才顯示完成／關閉 popup；失敗留在畫面。目標 ID 與 session/window/pane 描述始終可見，沒有 target 時禁止貼上。
- [ ] 同一組測試通過；Commit: `feat: add editable prompt slots with mouse and keyboard controls`。

### Task 7：AI CLI 使用中的 popup 入口

**Files:** src/prompts/popup.rs、src/cli.rs、src/ui/terminal.rs、tests/popup.rs、README.md、docs/zh-tw/README.md。
**Interfaces:** `tmux-manager prompts --target-pane %12` 啟動 picker；socket 繼承原 tmux 環境或明確參數；`tmux-manager bindings --print` 輸出 prefix + P 的設定片段。支持沒有 target 的 prompt 管理模式，但不猜測貼上目的地。

- [ ] 新增測試：開 popup 前傳入原 pane ID、popup 本身的 TMUX_PANE 不改寫 target、非 tmux 環境顯示說明、缺 popup 能力顯示降級入口、Esc/貼上退出恢復焦點。
- [ ] 新增 tmux.conf fixture：載入生成片段、不改使用者其他 binding；binary path 有空白仍可執行；在至少兩個 pane 中驗證 prefix + P 與滑鼠點擊。
- [ ] `cargo test --test popup` 先失敗；實作按需獨立程式與 binding 片段，優先使用 tmux 可支援的直接 argument 呼叫，必要 shell quoting 僅處理 binary path，不包含 prompt 內容。
- [ ] 開啟兩個 popup 驗證儲存更新及 revision conflict；原 pane 已關閉時應停留顯示錯誤、不貼其他 pane。
- [ ] `cargo test --test popup` 通過；Commit: `feat: expose prompt slots through a tmux popup`。

### Task 8：安裝、更新、CI、完整驗收與切換

**Files:** src/distribution/*、src/cli.rs、tests/distribution.rs、.github/workflows/{ci,release}.yml、install.sh、README.md、docs/zh-tw/README.md。
**Interfaces:** `install(binary: &Path, prefix: &Path) -> Result<InstallReport>`；`update(current: &Path) -> Result<UpdateReport>`；`uninstall(manifest: &InstallManifest) -> Result<UninstallReport>`；CLI 保留既有入口，新增 Rust install 與 cargo install 說明。

- [ ] 新增測試：自訂 prefix、舊 symlink、安裝兩次、無權限、release artifact 缺失、checksum mismatch、下載中斷、更新版本比較、不改非本工具檔案、卸載保留 config/prompts。
- [ ] `cargo test --test distribution` 先失敗；實作 release artifact 對應、checksum、備份與原子切換；僅管理自己的安裝 manifest 路徑，不自動 sudo 或遞迴清設定。
- [ ] install.sh 縮成可選下載 bootstrap；舊程式/lib/Bats 留作參考但不加入 Rust 安裝包。新增忽略 target 及必要產物，不移除 Bats submodule。
- [ ] CI 在 Linux/macOS 執行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --locked`；release build 執行 `cargo build --release --locked`。真實 tmux/PTY 測試必須獨立可見，不能以全部 skip 的綠燈宣稱驗收完成。
- [ ] 手動驗收至少 Claude Code 與 Codex CLI 的當時版本：開 popup、滑鼠單擊中文多行 prompt、完整內容停留在輸入區、由使用者送出；記錄 CLI/tmux/terminal 版本。若其中未通過，列出限制並修正，不能只靠 tmux exit code 判定成功。
- [ ] 驗收 Linux/WSL 和 macOS、SSH、無 API key、多 pane、縮放、Ctrl-C、損毀設定、並行編輯、原版升級與失敗回復；文件同步更新版本、設定格式、能力矩陣與已知限制。
- [ ] 上述檢查全部通過後準備 v2.0.0 artifacts；發布到 GitHub 和覆蓋正式安裝只在實作階段獲授權後進行，本次不發布。
- [ ] Commit: `feat: ship Rust distribution and document prompt slot workflow`。

## 完成定義

1. 正式 binary 的產品功能全部由 Rust 實作，除了 tmux 不需外部 runtime；保留的 shell 僅作為可選下載入口與歷史參考。
2. 原有 session/AI/設定操作通過 Rust 行為測試與平台驗收。
3. 自訂 prompt 重啟後仍存在，可在 AI CLI 使用中開 popup 並滑鼠單擊貼到原 pane。
4. 支援的 AI CLI 多行貼上保持完整且不自動送出；不支援時顯示能力限制，沒有靜默改寫。
5. 既有設定可按明確規則遷移，升級失敗可回復，prompts 不因更新或卸載消失。

## 工期建議

以一名熟悉 Rust 的工程師估算：任務 1 約 1–2 工作天；任務 2–3 約 3–5 天；任務 4–7 約 3–5 天；任務 8 約 2–3 天，合計約 9–15 工作天。
此為排程估計而非承諾；主要變數是實際 AI CLI 的多行貼上行為、舊設定複雜度及跨平台發布環境。先完成任務 1 再更新工期。

## 計畫自我檢查

- 現有九個 lib 模組、CLI、安裝／更新／卸載和新增 prompt 功能都有對應任務。
- Target、ID、repository revision、UI action 與 worker generation 由各自任務定義，後續透過同一介面使用。
- 多行、popup 目標、同步輸入、損毀／並行檔案及 symlink 五類風險都有指定驗證責任。
- 這次只新增規劃文件；未改產品程式、未安裝工具、未執行建置或刪除操作。
