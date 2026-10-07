use super::{ControlId, FundamentView, VIEW_H, VIEW_W};
use nih_plug::prelude::{IntParam, Param, ParamPtr};
use nih_plug_vizia::vizia::vg::Color;
use pleasant_ui::theme::{COLORS, LINE, MUTED};
use pleasant_ui::Draw;

pub(super) const HEADER_H: f32 = 44.0;
pub(super) const CONTROLS_TOP: f32 = 346.0;
pub(super) const ROW_H: f32 = 52.0;
pub(super) const ROW_GAP: f32 = 4.0;
const TOP_COUNT: usize = 8;

pub(super) const BYPASS_BUTTON: (f32, f32, f32, f32) = (168.0, 9.0, 26.0, 26.0);
pub(super) const THEME_BUTTON: (f32, f32, f32, f32) = (VIEW_W - 84.0, 9.0, 72.0, 26.0);
pub(super) const IN_METER: (f32, f32, f32, f32) = (VIEW_W - 196.0, 13.0, 78.0, 6.0);
pub(super) const OUT_METER: (f32, f32, f32, f32) = (VIEW_W - 196.0, 25.0, 78.0, 6.0);

pub(super) const CONTROLS: [ControlId; 15] = [
    ControlId::Voices,
    ControlId::Harmonics,
    ControlId::Low,
    ControlId::High,
    ControlId::Sensitivity,
    ControlId::CutDepth,
    ControlId::CutWidth,
    ControlId::SynthLevel,
    ControlId::Tone,
    ControlId::Waveform,
    ControlId::Attack,
    ControlId::Release,
    ControlId::Glide,
    ControlId::Mix,
    ControlId::Output,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Hit {
    Bypass,
    Theme,
    Value(ControlId),
    Body(ControlId),
}

macro_rules! each_param {
    ($self:ident, $id:expr, $p:ident => $body:expr) => {{
        match $id {
            ControlId::Voices => {
                let $p = &$self.params.max_voices;
                $body
            }
            ControlId::Harmonics => {
                let $p = &$self.params.harmonics;
                $body
            }
            ControlId::Low => {
                let $p = &$self.params.low_hz;
                $body
            }
            ControlId::High => {
                let $p = &$self.params.high_hz;
                $body
            }
            ControlId::Sensitivity => {
                let $p = &$self.params.sensitivity_db;
                $body
            }
            ControlId::CutDepth => {
                let $p = &$self.params.cut_depth_db;
                $body
            }
            ControlId::CutWidth => {
                let $p = &$self.params.cut_q;
                $body
            }
            ControlId::SynthLevel => {
                let $p = &$self.params.synth_level_db;
                $body
            }
            ControlId::Tone => {
                let $p = &$self.params.tone;
                $body
            }
            ControlId::Waveform => {
                let $p = &$self.params.waveform;
                $body
            }
            ControlId::Attack => {
                let $p = &$self.params.attack_ms;
                $body
            }
            ControlId::Release => {
                let $p = &$self.params.release_ms;
                $body
            }
            ControlId::Glide => {
                let $p = &$self.params.glide_ms;
                $body
            }
            ControlId::Mix => {
                let $p = &$self.params.mix;
                $body
            }
            ControlId::Output => {
                let $p = &$self.params.output_db;
                $body
            }
        }
    }};
}

pub(super) fn inside(px: f32, py: f32, rect: (f32, f32, f32, f32)) -> bool {
    px >= rect.0 && px <= rect.0 + rect.2 && py >= rect.1 && py <= rect.1 + rect.3
}

pub(super) fn control_slot(index: usize) -> (f32, f32, f32, f32) {
    let (row, col, cols) = if index < TOP_COUNT {
        (0usize, index, TOP_COUNT)
    } else {
        (1, index - TOP_COUNT, CONTROLS.len() - TOP_COUNT)
    };
    let margin = 12.0;
    let gap = 6.0;
    let cols_f = cols as f32;
    let width = (VIEW_W - margin * 2.0 - gap * (cols_f - 1.0)) / cols_f;
    let row_w = cols_f * width + (cols_f - 1.0) * gap;
    let x = (VIEW_W - row_w) * 0.5 + col as f32 * (width + gap);
    let y = CONTROLS_TOP + row as f32 * (ROW_H + ROW_GAP);
    (x, y, width, ROW_H)
}

pub(super) fn value_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (slot.0 + 4.0, slot.1 + 14.0, slot.2 - 8.0, 16.0)
}

pub(super) fn bar_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (slot.0 + 8.0, slot.1 + slot.3 - 12.0, slot.2 - 16.0, 5.0)
}

pub(super) fn hit(x: f32, y: f32) -> Option<Hit> {
    if inside(x, y, BYPASS_BUTTON) {
        return Some(Hit::Bypass);
    }
    if inside(x, y, THEME_BUTTON) {
        return Some(Hit::Theme);
    }
    for (index, id) in CONTROLS.iter().enumerate() {
        let slot = control_slot(index);
        if !inside(x, y, slot) {
            continue;
        }
        if inside(x, y, value_rect(slot)) {
            return Some(Hit::Value(*id));
        }
        return Some(Hit::Body(*id));
    }
    None
}

pub(super) fn control_at(x: f32, y: f32) -> Option<ControlId> {
    match hit(x, y) {
        Some(Hit::Value(id) | Hit::Body(id)) => Some(id),
        _ => None,
    }
}

impl FundamentView {
    pub(super) fn idle_hover(&self) -> Option<(f32, f32)> {
        pleasant_ui::idle_hover(self.hover, self.drag.is_some())
    }

    pub(super) fn control_name(&self, id: ControlId) -> &str {
        each_param!(self, id, p => p.name())
    }

    pub(super) fn control_value(&self, id: ControlId) -> String {
        each_param!(self, id, p => p.to_string())
    }

    pub(super) fn control_norm(&self, id: ControlId) -> f32 {
        each_param!(self, id, p => p.unmodulated_normalized_value())
    }

    pub(super) fn control_default_norm(&self, id: ControlId) -> f32 {
        each_param!(self, id, p => p.default_normalized_value())
    }

    pub(super) fn control_ptr(&self, id: ControlId) -> ParamPtr {
        each_param!(self, id, p => p.as_ptr())
    }

    pub(super) fn control_from_text(&self, id: ControlId, text: &str) -> Option<f32> {
        each_param!(self, id, p => p.string_to_normalized_value(text))
    }

    pub(super) fn accent(id: ControlId) -> Color {
        let index = CONTROLS.iter().position(|item| *item == id).unwrap_or(0);
        COLORS[index % COLORS.len()]
    }

    pub(super) fn step_int(param: &IntParam, up: bool, fine: bool) -> f32 {
        let plain = param.unmodulated_plain_value();
        let next = if up {
            param.next_step(plain, fine)
        } else {
            param.previous_step(plain, fine)
        };
        param.preview_normalized(next)
    }

    pub(super) fn draw_controls(&self, d: &mut Draw) {
        d.rect(
            0.0,
            CONTROLS_TOP - 4.0,
            VIEW_W,
            VIEW_H - (CONTROLS_TOP - 4.0),
            pleasant_ui::theme::PANEL,
        );
        for (index, id) in CONTROLS.iter().enumerate() {
            self.draw_control(d, *id, control_slot(index));
        }
    }

    fn draw_control(&self, d: &mut Draw, id: ControlId, slot: (f32, f32, f32, f32)) {
        let color = Self::accent(id);
        let norm = self.control_norm(id).clamp(0.0, 1.0);
        let bar = bar_rect(slot);
        let value_r = value_rect(slot);
        d.text_centered(
            slot.0 + slot.2 * 0.5,
            slot.1 + 12.0,
            self.control_name(id),
            11.0,
            MUTED,
        );
        d.rounded_rect(bar.0, bar.1, bar.2, bar.3, 2.5, LINE);
        let fill_w = bar.2 * norm;
        if fill_w > 0.5 {
            d.rounded_rect(bar.0, bar.1, fill_w, bar.3, 2.5, color);
        }
        let baseline = value_r.1 + 12.0;
        if let Some(edit) = self.edit.as_ref().filter(|edit| edit.target == id) {
            d.value_edit(edit, color);
        } else {
            let value = self.control_value(id);
            d.text_centered(value_r.0 + value_r.2 * 0.5, baseline, &value, 11.0, color);
            if self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if inside(hx, hy, value_r) {
                        d.value_underline(value_r, baseline, color);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::spectrum;
    use super::*;

    fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
        a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
    }

    #[test]
    fn hit_testing_finds_each_control_and_stays_inside_the_view() {
        assert_eq!(
            hit(BYPASS_BUTTON.0 + 4.0, BYPASS_BUTTON.1 + 4.0),
            Some(Hit::Bypass)
        );
        assert_eq!(
            hit(THEME_BUTTON.0 + 4.0, THEME_BUTTON.1 + 4.0),
            Some(Hit::Theme)
        );
        assert_eq!(hit(2.0, 2.0), None);
        assert!(spectrum::GRAPH_Y + spectrum::GRAPH_H + 12.0 <= CONTROLS_TOP);
        assert!(CONTROLS_TOP + ROW_H * 2.0 + ROW_GAP <= VIEW_H);
        assert!(!overlaps(BYPASS_BUTTON, THEME_BUTTON));
        assert!(!overlaps(THEME_BUTTON, IN_METER));

        for (index, id) in CONTROLS.iter().enumerate() {
            let slot = control_slot(index);
            assert!(slot.0 >= 0.0 && slot.0 + slot.2 <= VIEW_W + 0.01);
            assert!(slot.1 + slot.3 <= VIEW_H + 0.01);
            let value = value_rect(slot);
            let bar = bar_rect(slot);
            assert!(inside(value.0 + 1.0, value.1 + 1.0, slot));
            assert!(inside(bar.0 + 1.0, bar.1 + 1.0, slot));
            assert_eq!(
                hit(value.0 + value.2 * 0.5, value.1 + value.3 * 0.5),
                Some(Hit::Value(*id))
            );
            assert_eq!(
                hit(bar.0 + bar.2 * 0.5, bar.1 + bar.3 * 0.5),
                Some(Hit::Body(*id))
            );
            for other in (index + 1)..CONTROLS.len() {
                assert!(!overlaps(slot, control_slot(other)));
            }
        }
    }
}
