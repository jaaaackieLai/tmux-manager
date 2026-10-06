use tmux_manager::tmux::{PaneId, PaneLayout, PanePreview};

/// pane ID 取 `%{index}`，command 為 zsh；位置大小為 tmux 座標。
pub fn layout(
    index: u32,
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    active: bool,
) -> PaneLayout {
    PaneLayout {
        id: PaneId::parse(&format!("%{index}")).unwrap(),
        index,
        left,
        top,
        width,
        height,
        active,
        command: "zsh".into(),
    }
}

pub fn with_text(layout: PaneLayout, text: &str) -> PanePreview {
    PanePreview {
        layout,
        text: text.into(),
    }
}

/// 高 24 的單列 pane。
pub fn pane(
    index: u32,
    left: u16,
    width: u16,
    active: bool,
    command: &str,
    text: &str,
) -> PanePreview {
    let layout = PaneLayout {
        command: command.into(),
        ..layout(index, left, 0, width, 24, active)
    };
    with_text(layout, text)
}
