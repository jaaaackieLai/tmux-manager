use ratatui::layout::Rect;

pub fn preferred_stride(width: u16) -> u16 {
    if width >= 70 { 2 } else { 3 }
}

#[derive(Clone, Copy, Debug)]
pub struct Panels {
    pub body: Rect,
    pub sessions: Rect,
    pub divider: Rect,
    pub preview: Rect,
}

impl Panels {
    pub fn new(body: Rect, count: usize, percent: Option<u16>) -> Self {
        let available = body.height.saturating_sub(1);
        let minimum = available.min(3);
        let maximum = available.saturating_sub(available.saturating_sub(minimum).min(3));
        let desired = if let Some(percent) = percent {
            (u32::from(available) * u32::from(percent.min(100)) / 100) as u16
        } else {
            let content = count
                .max(1)
                .saturating_mul(usize::from(preferred_stride(body.width.saturating_sub(2))))
                .saturating_add(4);
            content.min(usize::from(available) * 3 / 5) as u16
        };
        let height = desired.clamp(minimum, maximum);
        let sessions = Rect::new(body.x, body.y, body.width, height);
        let divider = Rect::new(body.x, sessions.bottom(), body.width, body.height.min(1));
        let preview = Rect::new(body.x, divider.bottom(), body.width, available - height);
        Self {
            body,
            sessions,
            divider,
            preview,
        }
    }

    pub fn percent(&self) -> u16 {
        self.percent_at(self.sessions.bottom())
    }

    pub fn percent_at(&self, row: u16) -> u16 {
        let available = self.body.height.saturating_sub(1);
        if available == 0 {
            return 50;
        }
        (u32::from(row.saturating_sub(self.body.y).min(available)) * 100)
            .div_ceil(u32::from(available)) as u16
    }
}
