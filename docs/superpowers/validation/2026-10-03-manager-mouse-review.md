# Manager 滑鼠修正與獨立審查

分支 `feat/rust-prompt-slots`，保持未 commit。使用者測試 `target/release/tmux-manager` 發現 session 列表／詳細操作不能點選。

## 根因與修正

Manager 與 pane picker 開啟 mouse capture，但輸入迴圈忽略 `Event::Mouse`。原實測只證明 Prompt Slots 的滑鼠操作，不能代表 manager 可點選。

- Session 列表單擊進 detail；hit map 使用實際 render Rect、ListState offset 與 stable SessionId。
- Detail 單擊 attach／rename／kill／back，走既有鍵盤 Enter action，kill 保留確認。
- Pane picker 單擊選擇明確 pane；滾輪只更改選取。
- Release、drag、右鍵、空白與邊框不執行操作。

PTY regression 先重現 session 單擊無反應（RED），修正後通過（GREEN）；pane 單擊再單獨 RED → GREEN。新增三項 manager 行為測試驗證 scrolled row、stable ID、detail action 與忽略事件。

## 獨立 agent 審查結果

沒有 CRITICAL，1 項 IMPORTANT、1 項 MINOR。

**IMPORTANT：無效滑鼠事件造成整頁清除重畫。** 初版修正讓所有 mouse event 都設 dirty 並 clear；crossterm 啟用 any-motion，因此指標移動就會重畫。Reviewer 以 PTY 重現 20 次 hover 造成 20 次 clear、35,440 bytes 輸出。新增 PTY 失敗測試確認後，runtime 改為 mouse action 與 selected／detail_selected／screen 都沒變時直接略過；pane picker 僅選取變更時 dirty。測試 RED → GREEN。

**MINOR：多項目與縮放的整合驗證缺口。** Rust 測試有 scrolled session row，真實 manager PTY 仍只涵蓋單 session、單 pane、80×24；detail 小尺寸捲動、manager resize 後點擊、多 pane 捲動後選擇仍待補驗。這是尚未驗證，沒有確認為 bug；不宣稱完整平台／尺寸驗收。

同一 reviewer 唯讀復核 IMPORTANT 修正：最新 debug binary 的列表 24 個、detail 23 個忽略事件均產生 0 bytes 輸出／0 次 clear；有效單擊仍能進 detail。審查結論為 IMPORTANT 已解決、沒有剩餘 blocking issue，MINOR 驗證缺口保留。

## 驗證

完整 Rust suite 47 個通過；真實 PTY suite 在 tmux 3.4 與 3.7c 各 5 個通過；fmt、clippy `-D warnings` 通過。PTY 覆蓋 session click、rename 取消、kill 確認取消且 session 仍存在、back、pane click、mouse attach、detach 回列表與 termios 復原。Release binary 重新建置，供使用者再次確認效果。

## 追加：從 manager 進出 Prompt Slots

列表 `p` 或 header 單擊直接開無 target 管理模式；Esc／q 回同一 manager。Detail 仍選定 pane 後貼上。兩項新 keyboard／mouse 測試、無 session 的實際 PTY 新增保存與返回通過，完整 Rust suite 更新為 49 項，兩個 tmux 版本的 PTY suite 各 5 項通過。

沿用同一獨立 reviewer 唯讀審查本輪入口，沒有 CRITICAL／IMPORTANT／新增 MINOR，也沒有 blocking issue。Reviewer 另用 PTY 實測 keyboard／header 開啟、Esc／q 返回、第二個 session 選取保留，以及管理模式禁止貼上。返回後沿用原 AppState，沒有重設 AI generation。
