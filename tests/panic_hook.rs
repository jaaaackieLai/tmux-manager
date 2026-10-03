// panic hook 是全域狀態：獨立成一個測試 binary，避免與其他測試互相干擾。
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tmux_manager::ui::terminal::install_panic_hook_with;

#[test]
fn only_a_panic_on_the_ui_thread_restores_the_terminal() {
    let restored = Arc::new(AtomicUsize::new(0));
    let counter = restored.clone();
    install_panic_hook_with(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    // 背景 task（例如 AI worker）panic 時 TUI 仍在執行，不可離開 raw mode／alternate screen。
    assert!(std::thread::spawn(|| panic!("background")).join().is_err());
    assert_eq!(restored.load(Ordering::SeqCst), 0);
    assert!(std::panic::catch_unwind(|| panic!("ui")).is_err());
    assert_eq!(restored.load(Ordering::SeqCst), 1);
}
