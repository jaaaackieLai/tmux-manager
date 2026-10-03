#!/usr/bin/env bash
# 版本號 beacon：Bash 版（v1.x）的 `tmux-manager --update` 會讀取這一行判斷是否有新版，
# 版本不同時執行 install.sh 改裝 Rust binary。必須與 Cargo.toml 的版本一致。
readonly VERSION="2.0.0"
