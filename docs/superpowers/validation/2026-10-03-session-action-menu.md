# Session 浮動操作選單驗證

使用者確認 Enter／左鍵開小型選單，移除「→ 按兩次」提案；原先只撰寫計畫，後續於 2026-10-03 指示執行。沿用 `feat/rust-prompt-slots` 與既有未提交 Rust 工作區。

## 互動

Manager 持續繪製 Sessions 與 Preview，原 `Screen::Detail` 改為覆蓋其上的選單狀態。選單在內容區置中，最大 56×15（初版為56×12，驗收後增大間距），包含 session 精簡資訊及 attach／rename／kill／back；低高度優先保留操作，有可用內容行時選中項能捲動至可見。

Enter／左鍵點 session 只開啟選單；方向鍵／Tab／滾輪選操作，Enter／左鍵執行。Esc／q／選單外左鍵只關閉，不操作背景 session、prompt header 或 divider，選取與手動比例保留。邊框、release／drag／右鍵不觸發 action。Modal hit map 綁 stable session ID，背景改名／重排維持目標，移除時關閉，不使用舊 hit map 操作替代 session。

原 a/r/k／p 流程沿用。Rename 或 kill 取消回選單，kill 仍需確認；attach 失敗保留 manager 與錯誤，detach 回列表。Modal p 選明確工作 pane；列表 p 仍進 prompt 管理模式。底部常駐 prompt dock 未改動。

## TDD 與回歸

- 第一個 renderer 測試先 RED：Enter 後 Sessions／Preview 被整頁取代；浮動 renderer GREEN。
- 選單外 session 點擊先 RED：沒有關閉；modal 優先路由、只消耗該 click 後 GREEN。
- 真實 PTY 先 RED：選單期間工作區印出 `MODAL-PREVIEW-UPDATE`，manager 不擷取；runtime 在 List／Detail 均查 selected stable ID 後 GREEN，並確認 marker 在未被選單遮住的 Preview 行實際可見。
- 新增六項 manager actions 行為測試，含背景列表、比例保留、40／24／12／8／5／1 行等 geometry 邊界、中文／emoji 長名稱、可見 action hits、忽略事件、ID 重排／移除及摘要更新保留選單。既有 action mouse 測試依實際 menu Rect 定位。
- 真實 PTY 覆蓋第一次 Enter 不 attach、左鍵開啟、外部 header click 不開 prompts、選單開啟期間背景更新、40×12 → 80×24 縮放後 rename 點擊、表單 Esc／Ctrl-C、kill 否決、pane picker 返回、attach／detach、失效 session attach 失敗、termios 復原。

## 已執行檢查

- `cargo test --offline --locked`：89 項一般 Rust 測試通過；10 項真實測試另外明確執行。
- `cargo test --offline --locked --test tmux_pty --test tmux_capture -- --ignored --nocapture`：Linux tmux 3.4／3.7c 各 10 項通過，包含 2 項擷取／本機 mock HTTP AI 與 8 項 PTY／paste／popup／dock 生命週期。
- `./tests/bats/bin/bats tests/`：133 項通過。
- `cargo fmt --all --check`、`cargo clippy --offline --locked --all-targets -- -D warnings`：通過。
- `cargo build --release --offline --locked`：完成。
- 最終 release 在兩版 tmux 各跑 `tests/terminal_smoke.py`、`tests/dock_smoke.py`：均通過，含浮動選單與 Preview 更新，既有底部 prompt 列的滑鼠貼入／active pane／focus／detach 清理無回歸。

Rust 本機使用 `/tmp/tmux-manager-cargo-run`，測試均使用隔離 socket／PTY／暫存設定，沒有操作使用者 server 或呼叫外部 AI API。macOS／WSL／SSH、實際終端人工效果、外部 API、hosted CI 與正式發布沿用原計畫的待驗收限制。

## 工作區與審查方式

沒有 commit，HEAD 為 `6ff30a26da6bfe49ead4af12873eb8f4c8f48ef0`；不 push、不正式安裝。計畫腳本的 commit review range 為空，改用列明本輪檔案的 review package，獨立 reviewer 直接讀工作樹與測試；不能以空 git diff 當作審查通過。保留計畫專屬 ledger，方便未提交狀態下接續工作。

獨立 reviewer 自行重跑 25 項 manager／actions／layout 與 tmux 3.4 的 release manager PTY，全部通過。Critical／Important 為零；沒有 Declined to judge。Reviewer 沒有重跑完整雙版本 suite，上述雙版本結果為作者實測，未混作獨立證據。

保留一項非阻擋 MINOR：超長 session 名稱未限寬，會把同一行尾端的 `($id)` 擠出選單。Reviewer 以隔離 PTY 重現。事件路由仍核對 stable ID，不會因截斷改指向其他 session；後續可為 ID 預留欄位並裁切名稱。這是已知顯示限制，未宣稱長名稱時所有身份資訊都可見。

## 驗收後間距／單行提示修正

使用者回報選項擁擠，要求間距及一行按鍵說明。正常尺寸每項label後留一行空白，最大高度改15行；空白行不產生action hit，選取底色不延伸到間隔。短視窗回compact，保持選中項可見。Footer僅一行，完整提示47欄，窄視窗使用31欄或更短文案。

兩項新測試各先RED後GREEN：spacing與空白click不執行、同一行呈現選擇／執行／關閉。完整Rust suite現91項PASS，fmt／clippy -Dwarnings PASS。獨立reviewer重跑8actions＋10manager＋9layout共27項PASS，沒有新增Critical／Important／Minor；原長名稱ID截斷限制保留。

PTY初次回歸失敗是較高的選單覆蓋marker尾端，實際仍擷取最新Preview（可見MODAL-PREVI前段）。fixture將該階段寬度改104欄，完整marker可出現在選單左側，保留持續capture斷言；其餘80×24與40×12操作、resize、表單取消與attach/detach仍驗證。release已重建；最終雙版本PTY結果另列於下方。

最終release `terminal_smoke.py` 在tmux3.4／3.7c各PASS，包含新增間距後的mouse action／resize與Preview持續更新；`git diff --check` PASS。分支與HEAD未變，仍未commit。
