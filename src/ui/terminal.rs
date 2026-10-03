use crate::Result;
use crossterm::{
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, EventStream,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{Stdout, stdout};
pub type UiTerminal = Terminal<CrosstermBackend<Stdout>>;
pub struct TerminalGuard {
    active: bool,
}
impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        let mut guard = Self { active: false };
        guard.resume()?;
        Ok(guard)
    }
    pub fn resume(&mut self) -> Result<()> {
        if !self.active {
            enable_raw_mode()?;
            self.active = true;
            execute!(
                stdout(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableBracketedPaste
            )?;
        }
        Ok(())
    }
    pub fn suspend(&mut self) -> Result<()> {
        if self.active {
            self.active = false;
            restore();
        }
        Ok(())
    }
}
pub fn restore() {
    let _ = execute!(
        stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        Show,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.suspend();
    }
}
pub fn terminal() -> Result<UiTerminal> {
    Ok(Terminal::new(CrosstermBackend::new(stdout()))?)
}
pub fn install_panic_hook() {
    install_panic_hook_with(restore);
}
/// 只有呼叫此函式的 thread（執行 TUI 主迴圈）panic 時才還原終端。背景 task 在 tokio
/// worker thread 上 panic 時 TUI 仍在執行，若離開 raw mode／alternate screen 會讓畫面無法使用。
pub fn install_panic_hook_with(restore: impl Fn() + Send + Sync + 'static) {
    let ui_thread = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().id() == ui_thread {
            restore();
        }
        previous(info);
    }));
}
/// 事件驅動的終端輸入：沒事件時整個 task 休眠，不輪詢。
/// `EventStream` 同時只能存活一個，且它的背景執行緒會讀 stdin，
/// 所以全程共用同一個 `Input`，並在子程序接手 TTY 前 `suspend`。
#[derive(Default)]
pub struct Input(Option<EventStream>);
impl Input {
    pub fn new() -> Self {
        Self::default()
    }
    /// 丟掉 stream 讓讀取執行緒退出（例如 attach 前，避免搶走 tmux client 的輸入）；
    /// 下次 `next` 會自動重建。
    pub fn suspend(&mut self) {
        self.0 = None;
    }
    /// 取消安全：可放進 `select!`，被取消時不會遺失事件。
    pub async fn next(&mut self) -> Result<Event> {
        match self.0.get_or_insert_with(EventStream::new).next().await {
            Some(event) => Ok(event?),
            None => Err(crate::error::error("終端輸入已關閉")),
        }
    }
    /// 同 `next`，但外部 SIGINT 會中止操作。
    pub async fn next_or_abort(&mut self) -> Result<Event> {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => Err(crate::error::error("操作已中止")),
            event = self.next() => event,
        }
    }
}
