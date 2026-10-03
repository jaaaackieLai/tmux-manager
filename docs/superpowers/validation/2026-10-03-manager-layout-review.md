# Manager 動態佈局與獨立審查

分支 `feat/rust-prompt-slots`，保留未 commit。使用者確認採內容自動配置，並支援拖曳分隔線與鍵盤調整 Sessions／Preview。

## 畫面與操作

寬畫面將 session、windows、AI 摘要對齊並加列距，選取列用底色凸顯；內寬不足 70 欄時摘要移到下一行。長中文／emoji 名稱與摘要依顯示寬度裁切，不切開 grapheme。面板太小時退回 compact 顯示。

自動 Sessions 高度依內容計算，上限約可用高度 60%，空間足夠時兩個面板各至少三行。左鍵按下分隔線才啟動拖曳；`[`／`]` 每次調整約五百分點且至少一行，`\` 回到自動。比例僅本次執行保留，resize 重新夾緊高度。Preview 依實際高度顯示最新尾端輸出，最多 15 行。

## TDD 與審查

內容高度、列距／欄位／底色、短 Preview 的最新輸出、拖曳、鍵盤與 reset 先寫失敗測試，再實作；補驗窄畫面與 resize 上下限。既有 mouse hit map fixture 依新位置更新，session ID、表頭與列間空白的判定保留。

沿用使用者要求的獨立 agent 唯讀審查：沒有 CRITICAL／IMPORTANT，一項 MINOR「拖曳途中開 Prompt Slots，mouseup 由子畫面消耗，返回後誤把一般 drag 當作 resize」。Reviewer 以實際 PTY 重現 Preview 起始列 9 誤移到 14。

兩項回歸測試分別 RED → GREEN：keyboard action 清除進行中的 drag；新 left-down 清除舊 drag，再判斷是否點中 divider。PTY 加入 divider-down → p → prompt 內 release → Esc → preview-down／drag 不改比例。Reviewer 以最新 debug binary 復核，Preview 起始列維持 9，本項已解決，沒有 remaining blocking issue。

## 驗證範圍

新增九項 layout 測試，搭配既有九項 manager 測試，涵蓋內容高度、Unicode、選取底色、Preview 最新行、divider drag、鍵盤＋reset、短視窗步進、縮放夾緊與拖曳狀態清除。

真實 manager PTY 加入拖曳、鍵盤調整／自動復原、縮小到 40×12 後單擊進 detail 與返回；既有 prompt 進出、rename／kill 取消、pane 單擊、attach／detach 與 termios 復原繼續執行。多 pane 捲動點選、其他平台與人工終端操作仍未完整驗收，原計畫 Task 8 的待辦保留。

最終 `cargo fmt --all --check`、`cargo clippy --offline --locked --all-targets -- -D warnings` 通過；完整 Rust suite 58 項通過。五項真實 PTY 測試另獨立在 tmux 3.4／3.7c 各全數通過。`cargo build --release --offline --locked` 完成，使用同一個 `target/release/tmux-manager` 再跑 `tests/terminal_smoke.py`，兩個 tmux 版本皆通過。Bash 沒有本輪變更，128 項歷史測試沿用前次結果。

Git HEAD 仍為 `6ff30a26da6bfe49ead4af12873eb8f4c8f48ef0`，未 commit、未發布、未覆蓋正式安裝；保留工作區讓使用者確認實際畫面。
