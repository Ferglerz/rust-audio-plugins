//! The popup uses the same host parameters as the moving pickups in the action views.
use super::{Instrument, PianoView, Rect, Target};
use crate::params::PianoType;
use crate::regulation::{adjustments, param, Adjustment};
use nih_plug::prelude::Param;
use pleasant_ui::{
    draw::Draw,
    point_in_rect,
    theme::{GOLD, LINE, PANEL, TEAL, TEXT},
};

pub(super) const BUTTON: Rect = (250., 425., 116., 32.);
const WIDTH: f32 = 248.;
const ROW: f32 = 42.;

pub(super) fn panel(model: Instrument) -> Rect {
    let h = 12. + ROW * adjustments(model).len() as f32;
    (BUTTON.0 - 4., BUTTON.1 - h - 5., WIDTH, h)
}

pub(super) fn row(model: Instrument, control: Adjustment) -> Rect {
    let slot = adjustments(model)
        .iter()
        .position(|&p| p == control)
        .unwrap();
    let (x, y, w, _) = panel(model);
    (x + 6., y + 6. + slot as f32 * ROW, w - 12., ROW)
}

pub(super) fn slider_value(model: Instrument, control: Adjustment, x: f32) -> f32 {
    let (left, _, w, _) = row(model, control);
    ((x - left - 7.) / (w - 14.)).clamp(0., 1.)
}

impl PianoView {
    pub(super) fn regulation_hit(&self, model: Instrument, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, BUTTON) {
            return Some(Target::PickupMenu);
        }
        adjustments(model)
            .iter()
            .find(|&&control| point_in_rect(x, y, row(model, control)))
            .map(|&control| Target::PickupSetting(model, control))
    }
    pub(super) fn paint_regulation(&self, d: &mut Draw, model: Instrument) {
        let area = panel(model);
        d.panel(area, PANEL, LINE);
        for &control in adjustments(model) {
            let (x, y, w, _) = row(model, control);
            let p = param(&self.params, model, control);
            let n = p.unmodulated_normalized_value();
            d.text(x + 7., y + 13., control.label(model), 12., TEXT);
            d.text_right(
                x + w - 7.,
                y + 13.,
                &p.normalized_value_to_string(n, true),
                11.,
                GOLD,
            );
            d.line(x + 7., y + 28., x + w - 7., y + 28., LINE, 4.);
            let cx = x + 7. + n * (w - 14.);
            d.square_rect(cx - 2., y + 22., 4., 12., TEAL);
        }
    }
    pub(super) fn paint_pickup_button(&self, d: &mut Draw, model: Instrument) {
        let label = if model == Instrument::Piano(PianoType::Grand) {
            "Bonded piezos"
        } else {
            "Pickups"
        };
        d.button(
            BUTTON,
            label,
            self.regulation_open && !adjustments(model).is_empty(),
            TEAL,
        );
    }
}
