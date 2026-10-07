use super::FundamentView;
use crate::dsp::MAX_HARMONICS;
use crate::telemetry::FundamentTelemetry;
use nih_plug_vizia::vizia::vg::Color;
use pleasant_dsp::axis::{db_y, freq_x, x_freq};
use pleasant_dsp::units::linear_to_db_with_floor;
use pleasant_ui::graph::Viewport;
use pleasant_ui::theme::{rgb, COLORS, LINE, MUTED, TEAL};
use pleasant_ui::Draw;
use std::sync::atomic::Ordering;

pub(super) const GRAPH_X: f32 = 48.0;
pub(super) const GRAPH_Y: f32 = 50.0;
pub(super) const GRAPH_W: f32 = 796.0;
pub(super) const GRAPH_H: f32 = 274.0;

const FREQ_MIN: f64 = 20.0;
const FREQ_MAX: f64 = 8_000.0;
const DB_MIN: f64 = -90.0;
const DB_MAX: f64 = 0.0;

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

const GRID_HZ: [f64; 9] = [
    20.0, 50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0,
];

/// `db_y` is centered on 0 dB. Shift so -90 sits on the floor and 0 on the top.
pub(super) fn spectrum_db_y(db: f64, y: f32, height: f32) -> f32 {
    let mid = (DB_MIN + DB_MAX) * 0.5;
    let range = (DB_MAX - DB_MIN) * 0.5;
    db_y(db - mid, range, y, height)
}

pub(super) fn freq_px(hz: f64) -> f32 {
    freq_x(
        hz.clamp(FREQ_MIN, FREQ_MAX),
        GRAPH_X,
        GRAPH_W,
        FREQ_MIN,
        FREQ_MAX,
    )
}

pub(super) fn px_freq(px: f32) -> f64 {
    x_freq(px, GRAPH_X, GRAPH_W, FREQ_MIN, FREQ_MAX)
}

pub(super) fn graph_viewport() -> Viewport {
    Viewport::new(GRAPH_X, GRAPH_Y, GRAPH_W, GRAPH_H)
}

/// Note name plus the measured frequency, e.g. `A2 110.0`.
pub(super) fn note_name(hz: f32) -> String {
    if !hz.is_finite() || hz <= 0.0 {
        return String::new();
    }
    let midi = 69.0 + 12.0 * (hz / 440.0).log2();
    let rounded = midi.round();
    if !rounded.is_finite() {
        return String::new();
    }
    let note = rounded as i32;
    let name = NOTE_NAMES[note.rem_euclid(12) as usize];
    let octave = note.div_euclid(12) - 1;
    format!("{name}{octave} {hz:.1}")
}

fn clamp_db_y(db: f64) -> f32 {
    spectrum_db_y(db, GRAPH_Y, GRAPH_H).clamp(GRAPH_Y, GRAPH_Y + GRAPH_H)
}

fn in_axis(hz: f64) -> bool {
    hz.is_finite() && (FREQ_MIN..=FREQ_MAX).contains(&hz)
}

fn notch_px(cut_db: f32, presence: f32) -> f32 {
    let depth = (cut_db * presence).abs();
    if !depth.is_finite() {
        return 0.0;
    }
    (depth / 36.0).clamp(0.0, 1.0) * 16.0
}

fn with_alpha(mut color: Color, alpha: f32) -> Color {
    color.a = alpha.clamp(0.0, 1.0);
    color
}

fn search_hz(low: f32, high: f32) -> (f32, f32) {
    let high = if low >= high { low * 2.0 } else { high };
    (low, high)
}

impl FundamentView {
    pub(super) fn draw_spectrum(&self, d: &mut Draw) {
        let gx = GRAPH_X;
        let gy = GRAPH_Y;
        let gw = GRAPH_W;
        let gh = GRAPH_H;
        d.rect(gx, gy, gw, gh, rgb(23, 27, 33));

        let (low, high) = search_hz(self.params.low_hz.value(), self.params.high_hz.value());
        let x0 = freq_px(f64::from(low));
        let x1 = freq_px(f64::from(high));
        if x1 - x0 > 1.0 {
            d.rect(x0, gy, x1 - x0, gh, with_alpha(TEAL, 0.10));
        }

        for step in 0..=5 {
            let db = -18.0 * f64::from(step);
            let y = spectrum_db_y(db, gy, gh);
            d.line(gx, y, gx + gw, y, LINE, 1.0);
            let label_y = if step == 0 { y + 11.0 } else { y + 4.0 };
            d.text_right(gx - 6.0, label_y, &format!("{db:.0}"), 10.0, MUTED);
        }
        for (index, hz) in GRID_HZ.iter().enumerate() {
            let x = freq_px(*hz);
            d.line(x, gy, x, gy + gh, LINE, 1.0);
            let label = if *hz >= 1_000.0 {
                format!("{}k", (*hz / 1_000.0) as i32)
            } else {
                format!("{hz:.0}")
            };
            let y = gy + gh + 12.0;
            if index == 0 {
                d.text(x, y, &label, 10.0, MUTED);
            } else if index + 1 == GRID_HZ.len() {
                d.text_right(x, y, &label, 10.0, MUTED);
            } else {
                d.text_centered(x, y, &label, 10.0, MUTED);
            }
        }

        let mut bands = [0.0f32; FundamentTelemetry::SPECTRUM_BANDS];
        self.telemetry.spectrum(&mut bands);
        let mut pts = [(0.0f32, 0.0f32); FundamentTelemetry::SPECTRUM_BANDS];
        let mut count = 0usize;
        for (index, db) in bands.iter().enumerate() {
            let hz = f64::from(FundamentTelemetry::band_hz(index));
            if !in_axis(hz) {
                continue;
            }
            pts[count] = (freq_px(hz), clamp_db_y(f64::from(*db)));
            count += 1;
        }
        let live = !self.params.bypass.value();
        let trace = if live { TEAL } else { MUTED };
        if count >= 2 {
            d.area(&pts[..count], gy + gh, with_alpha(trace, 0.16));
            d.poly(&pts[..count], trace, 1.5);
        }

        let cut_db = self.telemetry.cut_depth_db.load(Ordering::Relaxed);
        let harmonics = self.params.harmonics.value().clamp(1, MAX_HARMONICS as i32) as usize;
        for voice_index in 0..crate::dsp::MAX_VOICES {
            let voice = self.telemetry.voice_snapshot(voice_index);
            if !voice.active {
                continue;
            }
            let alpha = if voice.presence.is_finite() {
                voice.presence.clamp(0.0, 1.0)
            } else {
                0.0
            };
            if alpha <= 0.0 || !in_axis(f64::from(voice.f0_hz)) {
                continue;
            }
            let color = with_alpha(COLORS[voice_index % COLORS.len()], alpha);
            let x = freq_px(f64::from(voice.f0_hz));
            d.line(x, gy, x, gy + gh, color, 1.0);
            let label = note_name(voice.f0_hz);
            if !label.is_empty() {
                let label_y = gy + 14.0 + (voice_index % 3) as f32 * 12.0;
                d.text_centered(x, label_y, &label, 11.0, color);
            }
            for partial in voice.partials.iter().take(harmonics) {
                if !(partial.mag > 0.0) || !in_axis(f64::from(partial.freq_hz)) {
                    continue;
                }
                let px = freq_px(f64::from(partial.freq_hz));
                let db = linear_to_db_with_floor(f64::from(partial.mag), 1.0e-6);
                let py = clamp_db_y(db);
                d.line(px - 4.0, py, px + 4.0, py, color, 1.0);
                d.circle(px, py, 2.6, color, true);
                let height = notch_px(cut_db, alpha).min((gy + gh - py - 3.0).max(0.0));
                if height > 0.5 {
                    d.fill_poly(
                        &[
                            (px - 3.5, py + 4.0),
                            (px + 3.5, py + 4.0),
                            (px, py + 4.0 + height),
                        ],
                        with_alpha(color, (alpha * 0.9).clamp(0.0, 1.0)),
                    );
                }
            }
        }

        d.outline((gx, gy, gw, gh), if live { TEAL } else { LINE });

        if let Some((hx, hy)) = self.idle_hover() {
            if graph_viewport().contains(hx, hy) {
                let hz = px_freq(hx);
                let label = if hz >= 1_000.0 {
                    format!("{:.2} kHz", hz / 1_000.0)
                } else {
                    format!("{hz:.1} Hz")
                };
                d.text_right(gx + gw - 8.0, gy + 16.0, &label, 11.0, MUTED);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names_follow_equal_temperament() {
        assert_eq!(note_name(110.0), "A2 110.0");
        assert_eq!(note_name(220.0), "A3 220.0");
        assert_eq!(note_name(440.0), "A4 440.0");
        assert_eq!(note_name(27.5), "A0 27.5");
        let c4 = 440.0 * 2f32.powf(-9.0 / 12.0);
        assert!(note_name(c4).starts_with("C4 "), "{c4}");
        assert_eq!(note_name(0.0), "");
        assert_eq!(note_name(-12.0), "");
        assert_eq!(note_name(f32::NAN), "");
    }

    #[test]
    fn spectrum_axes_map_edges_and_round_trip() {
        let top = spectrum_db_y(0.0, GRAPH_Y, GRAPH_H);
        let mid = spectrum_db_y(-45.0, GRAPH_Y, GRAPH_H);
        let bottom = spectrum_db_y(-90.0, GRAPH_Y, GRAPH_H);
        assert!((top - GRAPH_Y).abs() < 1.0e-3);
        assert!((bottom - (GRAPH_Y + GRAPH_H)).abs() < 1.0e-3);
        assert!((mid - (GRAPH_Y + GRAPH_H * 0.5)).abs() < 1.0e-3);
        for hz in [20.0, 110.0, 440.0, 1_000.0, 8_000.0] {
            let back = px_freq(freq_px(hz));
            assert!((back - hz).abs() / hz < 1.0e-5, "{hz} -> {back}");
        }
        assert!((freq_px(20.0) - GRAPH_X).abs() < 1.0e-3);
        assert!((freq_px(8_000.0) - (GRAPH_X + GRAPH_W)).abs() < 1.0e-2);
    }
}
