mod support;
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    style::{Color, Modifier},
};
use support::{
    preview::{layout as pane, with_text},
    render::row_text,
};
use tmux_manager::{
    tmux::{PaneLayout, PanePreview},
    ui::{
        preview::render_panes,
        preview_layout::{PaneSlot, PreviewLayout, compute},
    },
};

fn slot(pane: usize, x: u16, y: u16, width: u16, height: u16) -> PaneSlot {
    PaneSlot {
        pane,
        area: Rect::new(x, y, width, height),
    }
}

#[test]
fn left_right_split_shares_one_vertical_separator() {
    let panes = [pane(0, 0, 0, 40, 24, true), pane(1, 41, 0, 39, 24, false)];
    let layout = compute(Rect::new(1, 1, 29, 10), &panes);
    assert!(!layout.stacked);
    assert_eq!(
        layout.slots,
        [slot(0, 1, 1, 14, 10), slot(1, 16, 1, 14, 10)]
    );
    assert_eq!(layout.separators, [Rect::new(15, 1, 1, 10)]);
    assert_eq!(layout.hidden, 0);
}

#[test]
fn top_bottom_split_shares_one_horizontal_separator() {
    let panes = [pane(0, 0, 0, 80, 12, true), pane(1, 0, 13, 80, 11, false)];
    let layout = compute(Rect::new(0, 0, 40, 11), &panes);
    assert_eq!(layout.slots, [slot(0, 0, 0, 40, 5), slot(1, 0, 6, 40, 5)]);
    assert_eq!(layout.separators, [Rect::new(0, 5, 40, 1)]);
}

#[test]
fn mixed_split_joins_separators_at_the_t_junction() {
    let layout = compute(Rect::new(1, 1, 28, 5), &three_panes());
    assert!(!layout.stacked);
    assert_eq!(
        layout.slots,
        [
            slot(0, 1, 1, 14, 5),
            slot(1, 16, 1, 13, 2),
            slot(2, 16, 4, 13, 2)
        ]
    );
    assert_eq!(
        layout.separators,
        [Rect::new(15, 1, 1, 5), Rect::new(15, 3, 14, 1)]
    );
}

#[test]
fn bounding_box_not_starting_at_origin_scales_like_the_same_shape_at_origin() {
    let at_origin = [pane(0, 0, 0, 40, 24, true), pane(1, 41, 0, 39, 24, false)];
    let shifted = [pane(0, 10, 5, 40, 24, true), pane(1, 51, 5, 39, 24, false)];
    let inner = Rect::new(1, 1, 29, 10);
    assert_eq!(compute(inner, &shifted), compute(inner, &at_origin));
}

#[test]
fn narrow_panes_fall_back_to_a_full_width_stack_ordered_by_index() {
    let panes = [
        pane(2, 54, 0, 26, 24, false),
        pane(0, 0, 0, 26, 24, false),
        pane(1, 27, 0, 26, 24, true),
    ];
    let layout = compute(Rect::new(1, 1, 20, 10), &panes);
    assert!(layout.stacked);
    assert_eq!(
        layout.slots,
        [
            slot(1, 1, 1, 20, 3),
            slot(2, 1, 5, 20, 3),
            slot(0, 1, 9, 20, 2)
        ]
    );
    assert_eq!(
        layout.separators,
        [Rect::new(1, 4, 20, 1), Rect::new(1, 8, 20, 1)]
    );
    assert_eq!(layout.hidden, 0);
}

#[test]
fn short_panes_fall_back_to_a_stack() {
    let panes = [pane(0, 0, 0, 80, 3, false), pane(1, 0, 4, 80, 20, true)];
    let layout = compute(Rect::new(0, 0, 40, 6), &panes);
    assert!(layout.stacked);
    assert_eq!(layout.slots, [slot(0, 0, 0, 40, 3), slot(1, 0, 4, 40, 2)]);
}

#[test]
fn overflowing_stack_keeps_the_active_pane_and_counts_hidden_ones() {
    let panes: Vec<_> = (0..5)
        .map(|i| pane(i, 0, (i * 5) as u16, 80, 4, i == 3))
        .collect();
    let layout = compute(Rect::new(0, 0, 40, 5), &panes);
    assert!(layout.stacked);
    assert_eq!(layout.slots, [slot(0, 0, 0, 40, 2), slot(3, 0, 3, 40, 2)]);
    assert_eq!(layout.separators, [Rect::new(0, 2, 40, 1)]);
    assert_eq!(layout.hidden, 3);
}

#[test]
fn single_row_still_shows_the_active_pane_title() {
    let panes = [pane(0, 0, 0, 40, 24, false), pane(1, 41, 0, 39, 24, true)];
    let layout = compute(Rect::new(0, 0, 40, 1), &panes);
    assert_eq!(layout.slots, [slot(1, 0, 0, 40, 1)]);
    assert_eq!(layout.hidden, 1);
}

#[test]
fn empty_panes_or_area_produce_an_empty_layout() {
    assert_eq!(
        compute(Rect::new(0, 0, 40, 10), &[] as &[PaneLayout]),
        PreviewLayout::default()
    );
    let layout = compute(Rect::new(0, 0, 0, 0), &three_panes());
    assert!(layout.slots.is_empty() && layout.separators.is_empty());
}

#[test]
fn single_pane_fills_the_whole_area() {
    let layout = compute(Rect::new(1, 1, 30, 8), &[pane(4, 0, 0, 120, 40, true)]);
    assert_eq!(layout.slots, [slot(0, 1, 1, 30, 8)]);
    assert!(layout.separators.is_empty());
}

fn three_panes() -> [PaneLayout; 3] {
    [
        pane(0, 0, 0, 40, 24, true),
        pane(1, 41, 0, 39, 12, false),
        pane(2, 41, 13, 39, 11, false),
    ]
}

/// 以遞迴切割產生 tmux 式 layout；切割處留 1 cell 分隔線。
fn split(rng: &mut u64, area: (u16, u16, u16, u16), depth: u32, out: &mut Vec<PaneLayout>) {
    let mut next = || {
        *rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*rng >> 33) as u16
    };
    let (left, top, width, height) = area;
    let vertical = next() % 2 == 0;
    let size = if vertical { width } else { height };
    if depth == 0 || size < 5 || next() % 4 == 0 {
        let index = out.len() as u32;
        out.push(pane(index, left, top, width, height, index == 0));
        return;
    }
    let first = 2 + next() % (size - 3);
    let second = size - first - 1;
    if vertical {
        split(rng, (left, top, first, height), depth - 1, out);
        split(rng, (left + first + 1, top, second, height), depth - 1, out);
    } else {
        split(rng, (left, top, width, first), depth - 1, out);
        split(rng, (left, top + first + 1, width, second), depth - 1, out);
    }
}

fn assert_exact_cover(inner: Rect, layout: &PreviewLayout) {
    for y in inner.top()..inner.bottom() {
        for x in inner.left()..inner.right() {
            let inside =
                |r: &Rect| x >= r.left() && x < r.right() && y >= r.top() && y < r.bottom();
            let panes = layout.slots.iter().filter(|s| inside(&s.area)).count();
            let lines = layout.separators.iter().any(inside);
            assert_eq!(
                panes + usize::from(lines),
                1,
                "cell ({x},{y}) covered by {panes} panes, separator={lines}: {layout:?}"
            );
        }
    }
    let areas = layout.slots.iter().map(|s| s.area);
    for r in areas.chain(layout.separators.iter().copied()) {
        assert!(
            r.area() > 0 && inner.union(r) == inner,
            "{r:?} outside {inner:?}"
        );
    }
}

#[test]
fn random_layouts_are_covered_exactly_without_gaps_or_overlaps() {
    let mut rng = 7u64;
    let (mut grids, mut stacks) = (0, 0);
    for case in 0..600u16 {
        let mut panes = Vec::new();
        let (width, height) = (60 + (case % 7) * 20, 20 + (case % 5) * 8);
        split(&mut rng, (3, 2, width, height), 4, &mut panes);
        let inner = Rect::new(2, 1, 20 + (case % 9) * 15, 4 + (case % 6) * 6);
        let layout = compute(inner, &panes);
        assert_exact_cover(inner, &layout);
        assert_eq!(layout.slots.len() + layout.hidden, panes.len());
        if layout.stacked {
            stacks += 1;
        } else {
            grids += 1;
        }
    }
    assert!(grids > 50 && stacks > 50, "grids={grids} stacks={stacks}");
}

fn preview(layout: PaneLayout, command: &str, text: &str) -> PanePreview {
    let layout = PaneLayout {
        command: command.into(),
        ..layout
    };
    with_text(layout, text)
}

fn draw(width: u16, height: u16, panes: &[PanePreview]) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| render_panes(frame, frame.area(), " P ", panes))
        .unwrap();
    terminal
}

fn symbol(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> &str {
    terminal.backend().buffer()[(x, y)].symbol()
}

#[test]
fn separators_join_each_other_and_the_outer_border() {
    let [a, b, c] = three_panes();
    let panes = [
        preview(a, "claude", "> thinking..."),
        preview(b, "zsh", "$ ls"),
        preview(c, "vim", "fn main()"),
    ];
    let terminal = draw(30, 7, &panes);
    let at = |x, y| symbol(&terminal, x, y);
    assert_eq!(
        [at(15, 0), at(15, 1), at(15, 3), at(29, 3), at(15, 6)],
        ["┬", "│", "├", "┤", "┴"]
    );
    assert_eq!(at(20, 3), "─");
    assert!(row_text(&terminal, 0).starts_with("┌ P ─"));
    assert_eq!(row_text(&terminal, 1), "│0: claude     │1: zsh       │");
    assert_eq!(row_text(&terminal, 2), "│> thinking... │$ ls         │");
    assert_eq!(row_text(&terminal, 4), "│              │2: vim       │");
}

#[test]
fn four_way_split_draws_a_cross_and_side_junctions() {
    let panes = [
        preview(pane(0, 0, 0, 40, 12, true), "a", ""),
        preview(pane(1, 41, 0, 39, 12, false), "b", ""),
        preview(pane(2, 0, 13, 40, 11, false), "c", ""),
        preview(pane(3, 41, 13, 39, 11, false), "d", ""),
    ];
    let terminal = draw(30, 9, &panes);
    let at = |x, y| symbol(&terminal, x, y);
    assert_eq!(
        [at(15, 4), at(0, 4), at(29, 4), at(15, 0), at(15, 8)],
        ["┼", "├", "┤", "┬", "┴"]
    );
}

#[test]
fn title_text_wins_over_a_border_junction() {
    let panes = [
        preview(pane(0, 0, 0, 40, 24, true), "a", ""),
        preview(pane(1, 41, 0, 39, 24, false), "b", ""),
    ];
    let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    terminal
        .draw(|frame| render_panes(frame, frame.area(), " Preview · work-session ", &panes))
        .unwrap();
    assert!(row_text(&terminal, 0).starts_with("┌ Preview · work-session ─"));
    assert_eq!(symbol(&terminal, 15, 5), "┴");
}

#[test]
fn active_pane_title_is_cyan_bold_and_others_are_not() {
    let [a, b, c] = three_panes();
    let panes = [
        preview(a, "x", ""),
        preview(b, "y", ""),
        preview(c, "z", ""),
    ];
    let terminal = draw(30, 7, &panes);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 1)].fg, Color::Cyan);
    assert!(buffer[(1, 1)].modifier.contains(Modifier::BOLD));
    assert_ne!(buffer[(16, 1)].fg, Color::Cyan);
    assert!(!buffer[(16, 1)].modifier.contains(Modifier::BOLD));
}

#[test]
fn long_titles_and_lines_are_clipped_on_the_right_without_wrapping() {
    let [a, b, c] = three_panes();
    let panes = [
        preview(
            a,
            "claude",
            "中文中文中文中文中文\nabcdefghijklmnopqrstuvwxyz",
        ),
        preview(b, "very-long-command-name", ""),
        preview(c, "vim", ""),
    ];
    let terminal = draw(30, 7, &panes);
    assert_eq!(row_text(&terminal, 1), "│0: claude     │1: very-long…│");
    assert_eq!(row_text(&terminal, 2), "│中文中文中文… │             │");
    assert_eq!(row_text(&terminal, 3), "│abcdefghijklm…├─────────────┤");
    assert_eq!(row_text(&terminal, 4), "│              │2: vim       │");
}

#[test]
fn only_the_last_lines_that_fit_are_shown() {
    let text = (1..=9).map(|n| format!("line {n}")).collect::<Vec<_>>();
    let panes = [preview(
        pane(0, 0, 0, 80, 24, true),
        "zsh",
        &text.join("\n"),
    )];
    let terminal = draw(20, 6, &panes);
    let rows: Vec<_> = (1..5).map(|y| row_text(&terminal, y)).collect();
    assert_eq!(
        rows,
        [
            "│0: zsh            │",
            "│line 7            │",
            "│line 8            │",
            "│line 9            │"
        ]
    );
}

#[test]
fn hidden_panes_are_counted_after_the_last_visible_title() {
    let panes: Vec<_> = (0..5)
        .map(|i| preview(pane(i, 0, (i * 5) as u16, 80, 4, i == 3), "zsh", ""))
        .collect();
    let terminal = draw(30, 7, &panes);
    assert_eq!(row_text(&terminal, 1), "│0: zsh                      │");
    assert_eq!(row_text(&terminal, 3), "├────────────────────────────┤");
    assert_eq!(row_text(&terminal, 4), "│3: zsh … +3                 │");
}

#[test]
fn no_panes_draws_only_the_border() {
    let terminal = draw(12, 4, &[]);
    assert_eq!(row_text(&terminal, 1), "│          │");
    assert_eq!(row_text(&terminal, 2), "│          │");
    assert_eq!(row_text(&terminal, 3), "└──────────┘");
}
