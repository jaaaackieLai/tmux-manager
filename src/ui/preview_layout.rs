use crate::tmux::PaneLayout;
use ratatui::layout::Rect;

/// 一個 pane 在預覽中的位置；`pane` 為輸入 slice 的索引。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneSlot {
    pub pane: usize,
    pub area: Rect,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreviewLayout {
    pub slots: Vec<PaneSlot>,
    /// 寬或高為 1 的分隔線；交叉處可能重複，但不與 pane 重疊。
    pub separators: Vec<Rect>,
    pub stacked: bool,
    /// 空間不足而未顯示的 pane 數。
    pub hidden: usize,
}

/// pane 至少要有的寬度與高度（標題 + 1 行），不足就改成上下堆疊。
pub const MIN_WIDTH: u16 = 8;
pub const MIN_HEIGHT: u16 = 2;

/// 依 tmux 座標把 panes 縮放進 `inner`；放不下時改成上下堆疊。
pub fn compute(inner: Rect, panes: &[PaneLayout]) -> PreviewLayout {
    if panes.is_empty() || inner.is_empty() {
        return PreviewLayout::default();
    }
    let layout = grid(inner, panes);
    let fits = layout
        .slots
        .iter()
        .all(|s| s.area.width >= MIN_WIDTH && s.area.height >= MIN_HEIGHT);
    if fits { layout } else { stack(inner, panes) }
}

/// 全寬上下堆疊，依 index 排序；放不下時優先保留 active pane。
fn stack(inner: Rect, panes: &[PaneLayout]) -> PreviewLayout {
    let mut order: Vec<usize> = (0..panes.len()).collect();
    order.sort_by_key(|&i| panes[i].index);
    // k 個 pane 需要 k * MIN_HEIGHT + (k - 1) 列；至少顯示一個。
    let fit = (usize::from(inner.height) + 1) / (usize::from(MIN_HEIGHT) + 1);
    let count = fit.clamp(1, order.len());
    if count < order.len() {
        let active = order.iter().position(|&i| panes[i].active).unwrap_or(0);
        if active >= count {
            order[count - 1] = order[active];
        }
        order.truncate(count);
    }
    let rows = inner.height - (count as u16 - 1);
    let (base, extra) = (rows / count as u16, rows % count as u16);
    let mut layout = PreviewLayout {
        stacked: true,
        hidden: panes.len() - count,
        ..PreviewLayout::default()
    };
    let mut y = inner.y;
    for (n, &pane) in order.iter().enumerate() {
        if n > 0 {
            layout
                .separators
                .push(Rect::new(inner.x, y, inner.width, 1));
            y += 1;
        }
        let height = base + u16::from((n as u16) < extra);
        let area = Rect::new(inner.x, y, inner.width, height);
        layout.slots.push(PaneSlot { pane, area });
        y += height;
    }
    layout
}

/// 一個軸向的線性映射：tmux 分隔線位置（含兩側虛擬外框）對到預覽分隔線位置。
struct Axis {
    origin: i32,
    tmux_span: i32,
    view_span: i32,
}

impl Axis {
    /// 兩側外框的 tmux 位置 first、last 分別對到 -1 與 size。
    fn new(first: i32, last: i32, size: u16) -> Self {
        Self {
            origin: first,
            tmux_span: last - first,
            view_span: i32::from(size) + 1,
        }
    }

    /// 四捨五入的單調映射，回傳相對 inner 的座標。
    fn map(&self, line: i32) -> i32 {
        let (offset, view, tmux) = (
            i64::from(line - self.origin),
            i64::from(self.view_span),
            i64::from(self.tmux_span),
        );
        ((2 * offset * view + tmux) / (2 * tmux) - 1) as i32
    }
}

/// pane 兩側分隔線的 tmux 座標：(left-1, top-1, right, bottom)。
fn edges(pane: &PaneLayout) -> [i32; 4] {
    let (left, top) = (i32::from(pane.left), i32::from(pane.top));
    [
        left - 1,
        top - 1,
        left + i32::from(pane.width),
        top + i32::from(pane.height),
    ]
}

fn grid(inner: Rect, panes: &[PaneLayout]) -> PreviewLayout {
    let all: Vec<_> = panes.iter().map(edges).collect();
    let bound = |side: usize| all.iter().map(move |e| e[side]);
    let (left, right) = (bound(0).min(), bound(2).max());
    let (top, bottom) = (bound(1).min(), bound(3).max());
    let xs = Axis::new(left.unwrap_or(0), right.unwrap_or(1), inner.width);
    let ys = Axis::new(top.unwrap_or(0), bottom.unwrap_or(1), inner.height);
    let (ix, iy) = (i32::from(inner.x), i32::from(inner.y));
    let (iw, ih) = (i32::from(inner.width), i32::from(inner.height));
    let mut layout = PreviewLayout::default();
    for (pane, e) in all.iter().enumerate() {
        let [x0, y0, x1, y1] = [xs.map(e[0]), ys.map(e[1]), xs.map(e[2]), ys.map(e[3])];
        let area = rect(ix + x0 + 1, iy + y0 + 1, x1 - x0 - 1, y1 - y0 - 1);
        layout.slots.push(PaneSlot { pane, area });
        // 只記右、下兩邊（含兩端交叉點），裁到 inner 內。
        if x1 < iw {
            let (from, to) = (y0.max(0), y1.min(ih - 1));
            layout
                .separators
                .push(rect(ix + x1, iy + from, 1, to - from + 1));
        }
        if y1 < ih {
            let (from, to) = (x0.max(0), x1.min(iw - 1));
            layout
                .separators
                .push(rect(ix + from, iy + y1, to - from + 1, 1));
        }
    }
    layout
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    let clamp = |value: i32| value.clamp(0, i32::from(u16::MAX)) as u16;
    Rect::new(clamp(x), clamp(y), clamp(width), clamp(height))
}
