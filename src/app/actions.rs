use crate::{
    Result,
    config::Config,
    tmux::{Pane, SessionId, TmuxClient},
    ui::{
        form, hit_test,
        terminal::{Input, UiTerminal},
    },
};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{List, ListState, Paragraph},
};
pub async fn new_session(
    terminal: &mut UiTerminal,
    input: &mut Input,
    tmux: &TmuxClient,
    config: &Config,
) -> Result<Option<SessionId>> {
    let Some(name) = form::text(terminal, input, "新 session 名稱（空白取消）", "")
        .await?
        .filter(|s| !s.trim().is_empty())
    else {
        return Ok(None);
    };
    let mut directory = config.new_default_dir.clone();
    let mut command = config.new_default_cmd.clone();
    if config.new_ask_dir {
        let Some(value) =
            form::text(terminal, input, "工作目錄（空字串保留預設）", &directory).await?
        else {
            return Ok(None);
        };
        if !value.is_empty() {
            directory = value;
        }
    }
    if config.new_ask_cmd {
        let Some(value) = form::text(terminal, input, "啟動指令（- 表示停用）", &command).await?
        else {
            return Ok(None);
        };
        if value == "-" {
            command.clear();
        } else if !value.is_empty() {
            command = value;
        }
    }
    tmux.create(&name, &directory, &command).await.map(Some)
}
pub async fn pane_picker(
    terminal: &mut UiTerminal,
    input: &mut Input,
    panes: Vec<Pane>,
) -> Result<Option<Pane>> {
    if panes.is_empty() {
        return Err(crate::error::error("此 session 沒有可用 pane"));
    }
    let mut selected = 0;
    let mut dirty = true;
    let mut hits = Vec::new();
    loop {
        if dirty {
            terminal.draw(|frame| {
                let parts = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2),
                        Constraint::Min(1),
                        Constraint::Length(2),
                    ])
                    .split(frame.area());
                frame.render_widget(Paragraph::new("明確選擇貼上目的 pane"), parts[0]);
                let mut list_state = ListState::default().with_selected(Some(selected));
                frame.render_stateful_widget(
                    List::new(
                        panes
                            .iter()
                            .map(|p| format!("{} {}", p.id.as_str(), p.description)),
                    )
                    .highlight_symbol("> ")
                    .highlight_style(Style::default().fg(Color::Yellow)),
                    parts[1],
                    &mut list_state,
                );
                hits = hit_test::list_rows(parts[1], list_state.offset(), 0..panes.len()).collect();
                frame.render_widget(
                    Paragraph::new("[↑↓/Tab/滾輪] 選擇 [Enter/單擊] 確認 [Esc] 返回"),
                    parts[2],
                );
            })?;
            dirty = false;
        }
        match input.next_or_abort().await? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                    KeyCode::Enter => return Ok(Some(panes[selected].clone())),
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down | KeyCode::Tab => selected = (selected + 1) % panes.len(),
                    _ => (),
                }
                dirty = true;
            }
            Event::Mouse(mouse) => {
                let before = selected;
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(index) = hit_test::click(&hits, mouse) {
                            return Ok(Some(panes[*index].clone()));
                        }
                    }
                    MouseEventKind::ScrollUp => selected = selected.saturating_sub(1),
                    MouseEventKind::ScrollDown => selected = (selected + 1).min(panes.len() - 1),
                    _ => (),
                }
                dirty |= selected != before;
            }
            Event::Resize(_, _) => dirty = true,
            _ => (),
        }
    }
}
/// 先啟動底部列再 attach。底部列只是輔助功能：啟動失敗仍照常 attach，
/// 並回傳警告讓狀態列顯示。
pub async fn attach_with_dock<S>(
    dock: impl Future<Output = Result<S>>,
    attach: impl Future<Output = Result<()>>,
) -> (Option<S>, Result<Option<String>>) {
    let (supervisor, warning) = match dock.await {
        Ok(supervisor) => (Some(supervisor), None),
        Err(e) => (
            None,
            Some(format!("底部 prompt 列未啟動（{e}）；已直接 attach")),
        ),
    };
    (supervisor, attach.await.map(|()| warning))
}
