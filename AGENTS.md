# tmux-manager: Interactive tmux session manager

PTT 風格的 tmux session 管理工具，專為同時跑多個 Claude Code session 設計。v2 起為 Rust 實作；Bash 版（v1.x）已移除，需要時從 `v1.2.1` tag 取得。

## 專案結構

```
tmux-tool/
  Cargo.toml / Cargo.lock / rust-toolchain.toml   # Rust 1.88.0
  install.sh            # 下載入口：下載 release binary 後交給 `tmux-manager install`
  lib/constants.sh      # 只剩 VERSION：Bash 版 --update 讀取它判斷是否升級
  src/
    main.rs / entry.rs / cli.rs   # CLI 解析與子指令分派
    app/                # manager 狀態、事件迴圈、session 操作
    ui/                 # ratatui 繪製、滑鼠 hit test、文字截斷（text.rs）
    tmux/               # tmux 指令封裝（TmuxClient）、session/pane 查詢、能力偵測
    prompts/            # Prompt Slots：儲存、編輯器、popup、底部列（dock）、貼上
    ai/                 # Anthropic API 摘要（背景 worker）
    config/             # TOML 設定、環境變數覆寫、舊 config.sh 遷移
    distribution/       # install / update / uninstall 與 manifest
    storage.rs          # 原子寫入、檔案鎖、SHA-256
  tests/
    *.rs                # Rust 整合測試；需要真實 tmux 的標 #[ignore]
    *_smoke.py          # 真實 tmux/PTY 情境，由 tests/tmux_pty.rs 呼叫
    test_install_rust.bats  # install.sh
    bats/               # BATS (git submodule)
```

## 執行測試

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test tmux_pty --test tmux_capture -- --ignored   # 需要 tmux、python3
./tests/bats/bin/bats tests/
```

## 技術架構

- **語言**: Rust（tokio、ratatui、crossterm、reqwest/rustls）；執行時只需要 tmux
- **AI 摘要**: Anthropic API（`ANTHROPIC_API_KEY`，沒設就停用 AI 功能）
- **設定**: `${XDG_CONFIG_HOME:-~/.config}/tmux-manager/config.toml`，prompts 存在同目錄 `prompts.toml`
- **底部 prompt 列**: attach 時由 `prompt-dock-session` supervisor（獨立 process group）管理，最後一個 client detach 後清除並還原 mouse 設定

## 安裝與發布

```bash
./install.sh                            # 安裝到 ~/.local（預設）
INSTALL_PREFIX=/usr/local ./install.sh  # prefix 不可寫入時自動使用 sudo
tmux-manager --update
tmux-manager --uninstall                # 加 --purge 一併刪除設定與 prompts
```

推送 `v*` tag 時 `.github/workflows/release.yml` 建置各平台 binary 並建立 GitHub Release。Bash 版使用者執行 `--update` 會比對 main 上 `lib/constants.sh` 的 VERSION，不同就改跑 `install.sh` 下載 latest release，所以版本 bump 合併到 main 前必須先完成發布。

## 版本管理

使用 [Semantic Versioning](https://semver.org)：`MAJOR.MINOR.PATCH`

| 位置 | 何時 bump | 範例 |
|------|-----------|------|
| MAJOR | 破壞性變更（不向下相容） | 改 CLI 介面、砍舊功能 |
| MINOR | 新增功能（向下相容） | 加按鍵、新操作 |
| PATCH | 修 bug | 修顯示錯誤 |

## 開發備註

- AI model: `claude-haiku-4-5-20251001`
- 每次 commit 前先更改版本，三處須一致（`tests/distribution.rs` 會檢查）：`Cargo.toml` 的 `version`、`lib/constants.sh` 的 `VERSION`、`README.md:3` 的 badge
