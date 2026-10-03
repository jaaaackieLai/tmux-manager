# tmux-manager v2.0.0

Rust 遷移版與 Prompt Slots 的操作、設定遷移、安裝和驗證說明已統一於 [主文件](../../README.md)。Bash 版（v1.x）執行 `tmux-manager --update` 即改裝為 Rust 版，舊 `config.sh` 會自動轉換成 `config.toml`。

```bash
cargo build --release --locked
./target/release/tmux-manager prompts
./target/release/tmux-manager bindings --print
```

新增 prompt：n → 填標題／標籤／多行內容（Tab 切欄位）→ Ctrl-S。tmux prefix + 大寫 P 開 popup；Enter 或滑鼠單擊貼回原 pane，沒有自動送出。管理器列表按 Enter 或左鍵開啟 session 浮動操作選單，背景列表與 Preview 保持可見；選單內 p 先選 pane。Esc／q／點選單外可關閉，kill 仍需確認。

tmux 3.4 無法查詢 mode 2004，多行會顯示尚未貼上並保留具名 buffer；Linux tmux 3.7c 已實測多行直貼與同步 pane 隔離。實際 AI CLI 與其他平台的驗收見 [驗證紀錄](../superpowers/validation/2026-10-03-rust-prompt-slots.md)。
