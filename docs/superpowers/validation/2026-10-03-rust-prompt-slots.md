# Rust Prompt Slots 驗證紀錄

分支：`feat/rust-prompt-slots`。版本：Cargo v2.0.0，Bash 歷史版 v1.2.1。未 commit、未發布、不覆蓋正式安裝。

## 任務進度

| Task | 實作與驗收狀態 |
|---|---|
| 1 tmux adapter | 已實作；Linux 3.4／3.7c 本機驗證通過，其他平台待驗收 |
| 2 設定／遷移 | 已實作，本機行為測試通過 |
| 3 Manager／AI | 已實作；補修滑鼠事件、manager 內 prompt 入口與內容自動／手動佈局，本機測試通過 |
| 4 Prompt 儲存 | 已實作，本機原子寫入／衝突／損毀保護測試通過 |
| 5 指定 pane 貼上 | 已實作；兩個 tmux 版本的實際 PTY 驗證通過 |
| 6 Prompt UI／編輯 | 已實作；popup 單擊及編輯通過，pane 選擇器單擊本輪補修 |
| 7 Popup 入口 | 已實作；真實 binding、原 pane/socket、滑鼠貼上通過 |
| 8 安裝／CI／完整驗收 | 安裝與更新已實作、本機測試通過；Claude／macOS／WSL／SSH、hosted CI 與正式切換仍待驗收 |

上次回報的「已實作」不是「所有操作與平台全部驗收」。原測試只涵蓋 Prompt Slots 的滑鼠點擊，沒有驗證 manager 列表／詳細選單；本輪已用 PTY 失敗測試重現並補修。

## 環境

Linux x86_64（Ubuntu 24.04），Rust 1.88.0；系統 tmux 3.4，官方 tmux 3.7c 另建置於 `/tmp/tmux-manager-newtmux/install/bin`。CLI：Claude Code 2.1.287、Codex 0.160.0。PTY 使用 xterm-256color、80×24 與 40×12；Rust render 另涵蓋 12×5、1×1。

## 已驗證

初次驗證命令：`cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings` 通過；`cargo test --locked` 為 44 個 PASS、5 個獨立 PTY 測試 ignored。另明確執行 `cargo test --locked --test tmux_pty -- --ignored --nocapture`，tmux 3.4 與 3.7c 各 5 個 PASS；Bats 歷史參考測試 128 個 PASS。Rust 命令本機使用 `/tmp` 的工具鏈與 offline cache，測試 socket 使用隔離環境。

初次 `cargo build --release --locked` 通過，`target/release/tmux-manager --version` 為 2.0.0；用 release binary 重跑 Codex 實際 popup／滑鼠／中文多行未送出驗證通過。Git HEAD 仍為 `6ff30a26da6bfe49ead4af12873eb8f4c8f48ef0`，所有變更未 commit。

### 滑鼠修正回歸

Manager 與 pane picker 原先只處理鍵盤，忽略 `Event::Mouse`；不是 release 路徑或終端設定造成。PTY 測試先重現單擊 session 沒反應，再重現 pane picker 單擊沒反應；修正後進入 detail、改名表單、kill 確認／取消、back、pane picker、滑鼠 attach／detach 回列表全部通過。新增三項 manager 測試保護捲動後列對應、stable ID、忽略邊框／放開／拖曳／右鍵，以及滾輪只選擇不執行。

修正後完整 Rust suite 為 47 個 PASS；tmux 3.4／3.7c 真實 PTY suite 各 5 個 PASS；clippy `-D warnings` 通過，release 已重新建置。沒有改動 Bash runtime，128 個歷史測試沿用上輪驗證結果。

依使用者要求另開獨立 agent 審查；發現無效 mouse event 清除重畫的 IMPORTANT，已以 PTY RED → GREEN 修正。多 pane／manager resize 後點擊為 MINOR 驗證缺口，詳見[滑鼠審查紀錄](2026-10-03-manager-mouse-review.md)。

### Manager 內的 prompt 管理入口

依使用者追加需求，列表頁可按 `p` 或單擊上方 `[p] Prompt Slots` 進管理模式，沒有 session 也能使用；Esc／q 返回原 manager，不重設 session 選取或 AI generation。Detail 的 `p` 維持 pane picker 與指定 target，管理模式不猜測貼上目的地。

Keyboard／mouse 兩項新 manager 測試 RED → GREEN；真實 PTY 先重現無 session 時入口無效，再驗證 manager 內新增／Ctrl-S 保存／Esc 回列表／滑鼠再次進入／q 回列表，使用同一程序與 terminal。Rust suite 現為 49 項，真實 PTY 仍獨立執行兩個 tmux 版本。

### Sessions／Preview 動態佈局

使用者確認內容自動配置＋拖曳／鍵盤調整。列表新增列距、欄位對齊及選取底色，窄畫面摘要分行；自動高度依列數，手動比例僅本次執行保留，`[`／`]` 調整、`\` 復原。Preview 空間不足時顯示最新尾端輸出。

獨立 reviewer 發現拖曳途中進 Prompt Slots 可能殘留 drag 狀態，已用兩項 RED → GREEN 測試與 PTY 重現修正；唯讀復核已解決，沒有 blocking issue。詳細內容、畫面預覽與驗證範圍見[佈局審查紀錄](2026-10-03-manager-layout-review.md)。

本輪最終完整 Rust suite 58 項通過，fmt／clippy `-D warnings` 通過；tmux 3.4／3.7c 真實 PTY suite 各 5 項通過。Release 已重建，兩版 tmux 另用此 release 實測新佈局、拖曳／鍵盤、自動復原、縮放後點擊與 prompt 進出皆通過。仍未 commit；跨平台與 Claude 驗收保留待辦。

### Attach 後常駐底部 prompt 列

使用者確認只用滑鼠、填入上方最近 active 工作 pane；從 manager attach 後自動建立，不需另設定快捷鍵。支援新 window、prompt 重新載入、翻頁、focus 返回與最後 client detach 清理。底部 pane 被排除於工作 pane picker／AI 擷取清單。

獨立 reviewer 的 resize 高度與全程 timeout 清理兩項 IMPORTANT 均先 RED → GREEN 修正，另驗證啟動中止的 owner 清理。唯讀復核沒有 blocking issue。最新完整 Rust suite 79 項、Bats 133 項、tmux 3.4／3.7c 各八項 PTY 通過；release 重新建置，兩版 tmux 另實測 release 的底部列均通過。詳見[底部列驗證紀錄](2026-10-03-prompt-dock.md)。

### Preview／摘要擷取回歸

使用者回報 Preview 空白與摘要全失敗，真實 tmux 重現批次分隔行的控制字元被印為 `\001`，原解析器未辨識，兩者在擷取階段失敗。已補修共用解析器；兩項真實 tmux 測試先 RED 再 GREEN，包含完整 AiService 到本機模擬 HTTP 的工作內容與摘要，並排除底部 dock。獨立 reviewer 另重現固定分界與工作文字碰撞的 MINOR，已先 RED 再以每次請求 UUID 分界與分段檢查修正。完整 Rust suite 更新為 83 項通過、Bats 133 項通過；tmux 3.4／3.7c 的新擷取測試、release manager Preview 畫面與底部 dock smoke 各通過。詳細證據與限制見[回歸驗證紀錄](2026-10-03-preview-summary-regression.md)。

### Session 浮動操作選單

原整頁詳細畫面改為列表上的小型操作選單，Enter／左鍵開啟，保留 Sessions／Preview；Esc／q／選單外點擊只關閉，外部點擊不穿透，Preview／摘要持續更新。使用者已排除右方向鍵捷徑，沒有新增。表單取消、kill 確認、明確 pane prompt、attach／detach 與終端復原沿用原流程。

完整 Rust suite 89 項、Bats 133 項通過；tmux 3.4／3.7c 各 10 項真實測試與最終 release manager／dock smoke 通過。獨立 reviewer 重跑 25 項行為測試與 tmux 3.4 release PTY，沒有阻擋問題；保留超長名稱遮掉 ID 的非阻擋 MINOR。詳見[浮動選單驗證紀錄](2026-10-03-session-action-menu.md)。

- tmux adapter：stable session/pane ID、特殊名稱、零 session、錯誤傳遞、最低版本、help/version 無 tmux；preview 查詢 session 的作用中 window/pane。
- 設定：優先序、自訂路徑、TOML 空字串與多行、bool／正數驗證、拒絕執行 config.sh、遷移行號與原檔保留。
- 管理器：列表／詳細操作、ID 選取、AI generation、縮放 render；PTY 實際進詳細頁、pane picker、prompt 管理與退出。
- AI：無 key、mock HTTP 正常／401／429／malformed、多 text blocks；8 個任務並行時最多 4 requests，15 秒 timeout 不重送。
- Prompt：Unicode、多行、CRLF、64 KiB、控制碼、schema、UUID、0600、衝突不丟草稿、損毀不覆寫；PTY 新增／Ctrl-S／重啟讀回／Ctrl-C 復原。
- 貼上：payload 只走 stdin，唯一 buffer、重查固定 pane/socket、失敗無重送；真實中文多行與尾端 LF byte-for-byte，無額外 Enter；synchronize-panes 開啟時第二 pane 未收到內容；目標關閉後停止。
- tmux 3.4：mode 2004 查詢未知，多行只建 buffer，單行直貼。
- tmux 3.7c：`bracket_paste_flag` 可辨識開關，多行直貼通過 PTY。
- Codex CLI 0.160.0／tmux 3.7c／Linux PTY xterm-256color：實際 prefix + P 開 popup、滑鼠單擊中文與 emoji 多行 prompt，包含尾端 LF，完整內容仍停留在輸入區，沒有送出或發出模型請求。
- 安裝／更新：自訂 prefix、重裝、舊 symlink target 保留與備份、checksum、語意版本、遠端版本記錄；mock 下載中斷／checksum 不符保留 binary 和 manifest；卸載不移除被修改檔案、不清 prompts。
- 終端：一般退出、Esc、Ctrl-C 復原 termios／raw／游標／滑鼠／bracketed paste／alternate screen；錯誤與 panic 的其他實測另記。

## 審查與修正

一次獨立程式碼審查沒有 CRITICAL，三項 IMPORTANT 均先以失敗測試重現再修正：長行編輯保持游標附近文字可見；窄畫面固定顯示錯誤與 buffer 名稱，PageUp/PageDown 可捲動完整提示；預覽改抓作用中 pane。舊設定 `$HOME_PROJECT` 誤展開另提升為 IMPORTANT，變數邊界測試 RED → GREEN，現在只接受 `$HOME`／`${HOME}`。

保留一項 MINOR：內容已成功送入 pane，但刪除本次 buffer 失敗時，popup 關閉前的清理警告不會留在畫面。內容只貼一次，buffer 保留；沒有自動重試。暫不擴增成功流程，待使用者確認效果後處理。

## 待驗收與執行判定

- Claude Code 2.1.287 停在工作目錄信任確認；未代替使用者接受信任設定，實際 popup 貼上待確認。Codex 的使用者主動送出也保留人工驗收，本次只驗證內容停留且未自動送出。
- macOS、WSL、SSH 與實際終端模擬器人工操作；CI 定義不等同這些平台已跑過。
- GitHub hosted CI／release artifacts，因本次沒有 push 或發布。

依使用者要求直接在新分支修改、沒有另外開 worktree、沒有 commit 或正式安裝；因此未套用計畫各任務的 commit 步驟，工作區保留供試用。跨平台與 Claude 驗收尚未完成，所以這是可試用實作，不宣稱計畫所有平台驗收均已完成。

tmux `display-popup` 的 argv 不會展開 format。binding 採 `run-shell -C` 先展開 tmux 指令，再直接 argv 啟動 popup；prompt 內容不經 shell。此判定依真實失敗測試修正，最低基線仍為 tmux 3.3。

## 技術依據

tmux 3.4 的 `format.c` 無 mode 2004 查詢；3.7c 提供 `bracket_paste_flag`，較新 `pane_private_modes` 可辨識 2004。以執行時欄位值判斷，未知就降級。官方參考：[tmux 手冊](https://man.openbsd.org/tmux.1)、[tmux CHANGES](https://github.com/tmux/tmux/blob/master/CHANGES)、[paste-buffer 實作](https://github.com/tmux/tmux/blob/3.4/cmd-paste-buffer.c)。

驗收後依使用者要求將操作選項加空白間距、按鍵說明合併單行，正常modal改56×15，短視窗compact。完整Rust suite更新91項，獨立27項manager相關測試與最終release兩版manager PTY均PASS；fmt／clippy通過，仍未commit。證據見[浮動選單驗證紀錄](2026-10-03-session-action-menu.md)。
