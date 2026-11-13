use crate::{
    params::EngineKind,
    telemetry::POINTS,
};
use pleasant_ui::draw::KnobLayout;

pub(super) type Rect = (f32, f32, f32, f32);
pub(super) const SIZE: (f32, f32) = (800.0, 620.0);
pub(super) const KNOB: Rect = (600.0, 146.0, 160.0, 198.0);
pub(super) const THEME: Rect = (688.0, 23.0, 76.0, 28.0);
pub(super) const DPDFNET: Rect = (200.0, 23.0, 86.0, 28.0);
pub(super) const NARA_WPE: Rect = (294.0, 23.0, 99.0, 28.0);
pub(super) const DTLN: Rect = (401.0, 23.0, 62.0, 28.0);
pub(super) const LIVE: Rect = (478.0, 23.0, 62.0, 28.0);
pub(super) const HIGH_QUALITY: Rect = (548.0, 23.0, 120.0, 28.0);
pub(super) const ENGINE_BUTTONS: [(Rect, &str, EngineKind); 3] = [
    (DPDFNET, "DPDFNET", EngineKind::DpdfNet),
    (NARA_WPE, "NARA-WPE", EngineKind::NaraWpe),
    (DTLN, "DTLN", EngineKind::Dtln),
];
pub(super) const GRAPH: Rect = (64.0, 432.0, 496.0, 88.0);
pub(super) const HISTORY_FLOOR: f32 = -90.0;
pub(super) const CURVE: Rect = (64.0, 142.0, 496.0, 210.0);
pub(super) const RELATIVE: Rect = (302.0, 86.0, 120.0, 28.0);
pub(super) const ABSOLUTE: Rect = (432.0, 86.0, 128.0, 28.0);
pub(super) const FLAT: Rect = (600.0, 370.0, 70.0, 28.0);
pub(super) const TRANSIENT_GUARD: Rect = (600.0, 517.0, 164.0, 34.0);
pub(super) const GUARD_TRACK_LEFT: f32 = TRANSIENT_GUARD.0 + 4.0;
pub(super) const GUARD_TRACK_WIDTH: f32 = TRANSIENT_GUARD.2 - 8.0;
pub(super) const LOUDER_BUTTON: Rect = (680.0, 370.0, 84.0, 28.0);

pub(super) fn value_rect() -> Rect {
    KnobLayout::new(KNOB).value_rect(KNOB.0 + 20.0, KNOB.2 - 40.0)
}

pub(super) fn recent_power(history: &[f32; POINTS]) -> f64 {
    let recent = &history[POINTS - 20..];
    recent
        .iter()
        .map(|&rms| f64::from(rms).powi(2))
        .sum::<f64>()
        / recent.len() as f64
}

pub(super) fn history_level(power: f64) -> String {
    if power > 0.0 {
        format!("{:.1} dBFS", 10.0 * power.log10())
    } else {
        "−∞ dBFS".into()
    }
}
