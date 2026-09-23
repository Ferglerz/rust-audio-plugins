use super::*;

pub(super) fn hud_rect_for(b: &Band, range: f64) -> (f32, f32, f32, f32) {
    hud_rect_for_at(b, range, GX, GW)
}
pub(super) fn hud_value_rect(b: &Band, range: f64, i: usize) -> (f32, f32, f32, f32) {
    hud_value_rect_at(b, range, i, GX, GW)
}
pub(super) fn hud_rect_for_lift(b: &LiftBand, range: f64) -> (f32, f32, f32, f32) {
    hud_rect_for_lift_at(b, range, GX, GW)
}
pub(super) fn hud_value_rect_lift(b: &LiftBand, range: f64, i: usize) -> (f32, f32, f32, f32) {
    hud_value_rect_lift_at(b, range, i, GX, GW)
}


mod entries;
mod layout;
mod meters;
mod graph;
