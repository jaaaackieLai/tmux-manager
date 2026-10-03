# Preview／摘要擷取回歸

使用者驗收底部 prompt dock 後回報 Preview 全空白、所有 AI 摘要失敗。分支 `feat/rust-prompt-slots`，保留未 commit 工作區。

## 根因與修正

Preview 的 `snapshot` 與摘要的 `capture_many` 都把多條 tmux 指令合併為單次呼叫，以 `display-message -p` 印出控制字元分隔行。真實 tmux 3.4／3.7c 會將 `\x01` 印成文字 `\001`，原解析器只辨識原始控制字元。結果 snapshot 將分隔行與 pane 內容當成 session 欄位而失敗；摘要則因缺少分段，在發送 HTTP 前就回報「tmux 擷取輸出不完整」。

共用解析器現在接受原始分隔行與 tmux 印出的八進位 escape 形式。每次批次請求產生 UUID 分界，避免工作區內容恰巧包含舊分界文字而截斷或錯配 pane；分段數不符會明確報錯。既有單次呼叫、active pane target、尾端空白清除與行數限制維持原契約。

## TDD 與驗證

- adapter 的真實輸出格式先 RED：「無法解析 tmux session」；修正後 GREEN。補充 escaped 分隔行的多 pane 擷取測試。
- 獨立 reviewer 重現固定分界文字出現在 pane A 內容時，pane A 尾段被當成 pane B 而丟失真正 B 內容的 MINOR。先補分段錯配 RED，再加入每次請求 UUID 分界及完整分段數檢查，GREEN；另測試舊分界全文保留與兩次請求 marker 不同。
- 真實隔離 tmux snapshot 先 RED：「無法解析 tmux session」；AiService 先 RED：「tmux 擷取輸出不完整」。修正後兩者 GREEN。
- 新 fixture 包含工作 pane 與帶 owner 標記的底部 dock；工作輸出刻意包含舊分界字串及其後文字。確認 Preview 原樣保留、兩個工作 panes 的 batch 內容不錯配、AI 擷取排除 dock，完整 AiService 請求傳到本機 HTTP fixture 且回傳解析後摘要。兩項測試在 Linux tmux 3.4／3.7c 各通過；未呼叫外部 Anthropic API。
- 原生 PTY manager smoke 新增工作區輸出 marker，必須在 Preview 畫面實際看見；使用重建的 release binary，兩版 tmux 各通過。既有列表／詳細選單滑鼠、prompt 進出、分隔線調整、resize、attach／detach 與終端復原一起通過。
- release 底部 dock smoke 兩版 tmux 各通過，涵蓋 active／last pane、僅滑鼠、方向鍵 focus、貼上與清理。
- 完整 Rust suite 83 項通過；兩項新真實 tmux 測試明確另跑，不將 ignored 視為通過。Bats 133 項通過，fmt／clippy `-D warnings` 通過。
- CI 的真實 tmux 步驟新增 `--test tmux_capture`，以後會明確執行新回歸。Hosted CI 本輪未執行。

`cargo build --release --offline --locked` 已完成。驗證結果僅涵蓋本機 Linux、上述 tmux 版本與模擬 API；外部 API 回應與原計畫的跨平台人工驗收仍各自待驗證。

獨立 reviewer 最終唯讀複查確認原 MINOR 已解決，沒有新增回歸或 blocking issue；另自行重跑 18 項 adapter、5 項 AI 與 2 項真實 tmux 整合測試全部通過。Git HEAD 維持 `6ff30a26da6bfe49ead4af12873eb8f4c8f48ef0`，沒有 commit、push 或正式安裝。
