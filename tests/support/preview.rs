use tmux_manager::tmux::{PaneId, PaneLayout, PanePreview};

/// 高 24 的單列 pane；`left`/`width` 為 tmux 座標。
pub fn pane(
    index: u32,
    left: u16,
    width: u16,
    active: bool,
    command: &str,
    text: &str,
) -> PanePreview {
    PanePreview {
        layout: PaneLayout {
            id: PaneId::parse(&format!("%{index}")).unwrap(),
            index,
            left,
            top: 0,
            width,
            height: 24,
            active,
            command: command.into(),
        },
        text: text.into(),
    }
}
