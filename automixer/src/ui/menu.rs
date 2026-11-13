use super::*;

impl AutomixerView {
    pub(super) fn menu_spec(kind: MenuKind) -> (Rect, i32, i32) {
        match kind {
            MenuKind::Channel(_) => (Self::channel_rect(), CHANNELS, CHANNELS),
            MenuKind::Cc(_) => (Self::cc_rect(), CC_COUNT, CC_VISIBLE),
        }
    }
    fn menu_value(&self, kind: MenuKind) -> i32 {
        let mic = match kind {
            MenuKind::Channel(mic) | MenuKind::Cc(mic) => mic,
        };
        let mic = &self.params.shared.mics[mic];
        match kind {
            MenuKind::Channel(_) => mic.midi_channel.value(),
            MenuKind::Cc(_) => mic.midi_cc.value(),
        }
    }
    fn menu_body(kind: MenuKind, rows: i32) -> Rect {
        let (anchor, _, _) = Self::menu_spec(kind);
        let height = rows as f32 * ROW + 8.0;
        let y = (anchor.1 - 6.0 - height).max(8.0);
        (anchor.0, y, anchor.2, height)
    }
    fn menu_row(body: Rect, row: i32) -> Rect {
        (
            body.0 + 4.0,
            body.1 + 4.0 + row as f32 * ROW,
            body.2 - 8.0,
            ROW - 1.0,
        )
    }
    pub(super) fn menu_index_at(&self, point: (f32, f32)) -> Option<i32> {
        let menu = self.menu.as_ref()?;
        let (_, count, visible) = Self::menu_spec(menu.kind);
        let rows = visible.min(count);
        let body = Self::menu_body(menu.kind, rows);
        if !point_in_rect(point.0, point.1, body) {
            return None;
        }
        let row = ((point.1 - (body.1 + 4.0)) / ROW) as i32;
        if row < 0 || row >= rows {
            return None;
        }
        let index = menu.scroll + row;
        (index >= 0 && index < count).then_some(index)
    }
    pub(super) fn open_menu(&mut self, kind: MenuKind) {
        let current = self.menu_value(kind);
        let (_, count, visible) = Self::menu_spec(kind);
        let max_scroll = (count - visible).max(0);
        self.menu = Some(OpenMenu {
            kind,
            scroll: (current - visible / 2).clamp(0, max_scroll),
        });
    }
    pub(super) fn scroll_menu(&mut self, dy: f32) {
        let Some(kind) = self.menu.as_ref().map(|menu| menu.kind) else {
            return;
        };
        let (_, count, visible) = Self::menu_spec(kind);
        let max_scroll = (count - visible).max(0);
        let step = if dy > 0.0 { -1 } else if dy < 0.0 { 1 } else { 0 };
        if let Some(menu) = &mut self.menu {
            menu.scroll = (menu.scroll + step).clamp(0, max_scroll);
        }
    }
    pub(super) fn choose_menu(&mut self, cx: &mut EventContext, index: i32) {
        let Some(kind) = self.menu.as_ref().map(|menu| menu.kind) else {
            return;
        };
        let mic = match kind {
            MenuKind::Channel(mic) | MenuKind::Cc(mic) => mic,
        };
        let param = match kind {
            MenuKind::Channel(_) => &self.params.shared.mics[mic].midi_channel,
            MenuKind::Cc(_) => &self.params.shared.mics[mic].midi_cc,
        };
        if let Some(value) = param.string_to_normalized_value(&index.to_string()) {
            param::set_normalized_once(cx, param.as_ptr(), value);
        }
    }
    pub(super) fn paint_dropdown(
        &self,
        d: &mut Draw,
        rect: Rect,
        label: &str,
        value: &str,
        open: bool,
    ) {
        d.button_surface(
            rect,
            TEAL,
            open as u8 as f32,
            d.is_hovered(rect) as u8 as f32,
            0.0,
        );
        d.text(rect.0 + 12.0, rect.1 + 18.0, label, 9.5, MUTED);
        d.text(rect.0 + 12.0, rect.1 + 44.0, value, 16.0, TEXT);
        let x = rect.0 + rect.2 - 22.0;
        let y = rect.1 + 40.0;
        d.poly(
            &[(x, y - 4.0), (x + 10.0, y - 4.0), (x + 5.0, y + 3.0)],
            TEXT,
            1.5,
        );
    }
    pub(super) fn paint_menu(&self, d: &mut Draw) {
        let Some(menu) = &self.menu else {
            return;
        };
        let (_, count, visible) = Self::menu_spec(menu.kind);
        let rows = visible.min(count);
        let body = Self::menu_body(menu.kind, rows);
        d.rect(body.0, body.1, body.2, body.3, PANEL);
        d.outline(body, LINE);
        let current = self.menu_value(menu.kind);
        for row in 0..rows {
            let index = menu.scroll + row;
            let item = Self::menu_row(body, row);
            if d.hover
                .is_some_and(|point| point_in_rect(point.0, point.1, item))
                || index == current
            {
                d.rect(item.0, item.1, item.2, item.3, LINE);
            }
            let label = if matches!(menu.kind, MenuKind::Channel(_)) && index == 0 {
                "Off".to_string()
            } else {
                index.to_string()
            };
            d.text_middle(
                item.0 + 10.0,
                item.1 + item.3 * 0.5,
                &label,
                12.0,
                if index == current { GOLD } else { TEXT },
            );
        }
    }
}
