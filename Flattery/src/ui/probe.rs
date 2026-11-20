use super::*;
use pleasant_ui::spectrum::smooth_bins_f32;

const INPUT_FLOOR_DB: f32 = -120.0;

/// Editor-owned copies keep telemetry locks out of drawing and use the same
/// values for the traces and the probe.
#[derive(Default)]
pub(super) struct DisplayFrame {
    pub(super) spectrum: Vec<f32>,
    pub(super) filters: Vec<(f32, f32)>,
}

impl DisplayFrame {
    pub(super) fn refresh(&mut self, shared: &Shared, fft_size: usize, sample_rate: f64) {
        let on_grid = || {
            shared.fft_size.load(Ordering::Relaxed) == fft_size
                && f64::from(shared.sample_rate.load(Ordering::Relaxed)) == sample_rate
        };
        self.spectrum.clear();
        self.filters.clear();
        if !on_grid() || fft_size < 2 || sample_rate <= 0.0 || !sample_rate.is_finite() {
            return;
        }
        if let Ok(raw) = shared.spectrum_mags_db.read() {
            if raw.len() == fft_size / 2 {
                self.spectrum.resize(raw.len(), INPUT_FLOOR_DB);
                smooth_bins_f32(&raw, &mut self.spectrum);
            }
        }
        if let Ok(filters) = shared.filter_display.read() {
            let bin_hz = sample_rate / fft_size as f64;
            // A mode change can publish the new grid before its first filters.
            // Discard those old centers instead of pairing them with new bins.
            if filters.iter().all(|&(hz, _)| {
                let bin = f64::from(hz) / bin_hz;
                bin >= 0.5 - 1e-3 && bin < (fft_size / 2) as f64 && (bin.fract() - 0.5).abs() < 1e-3
            }) {
                self.filters.extend_from_slice(&filters);
            }
        }
        if !on_grid() {
            self.spectrum.clear();
            self.filters.clear();
        }
    }

    pub(super) fn reading(
        &self,
        layout: &GraphLayout,
        x: f32,
        fft_size: usize,
        bin_hz: f64,
        bypassed: bool,
    ) -> Option<Reading> {
        if !bin_hz.is_finite() || bin_hz <= 0.0 || fft_size < 2 {
            return None;
        }
        let hz = layout.x_to_freq(x).clamp(layout.min_freq, layout.max_freq);
        if hz >= fft_size as f64 * bin_hz * 0.5 {
            return None;
        }
        let first = (layout.min_freq / bin_hz - 0.5).ceil().max(0.0) as usize;
        let last = (layout.max_freq / bin_hz - 0.5).floor().max(0.0) as usize;
        let last = last.min(fft_size / 2 - 1);
        if first > last || layout.max_freq < bin_hz * 0.5 {
            return None;
        }
        let filter = if bypassed {
            None
        } else {
            nearest_visible_filter(&self.filters, layout, hz, bin_hz)
        };
        let bin = (filter.map_or(hz, |&(center, _)| f64::from(center)) / bin_hz).floor() as usize;
        let bin = bin.clamp(first, last);
        Some(Reading {
            frequency: (bin as f64 + 0.5) * bin_hz,
            input_db: self.spectrum.get(bin).copied().filter(|v| v.is_finite()),
            gain_db: if bypassed {
                Some(0.0)
            } else {
                filter.map(|&(_, gain)| gain).filter(|v| v.is_finite())
            },
            bypassed,
        })
    }
}

fn nearest_visible_filter<'a>(
    filters: &'a [(f32, f32)],
    layout: &GraphLayout,
    hz: f64,
    bin_hz: f64,
) -> Option<&'a (f32, f32)> {
    let first = filters.partition_point(|&(center, _)| f64::from(center) < layout.min_freq);
    let last = filters.partition_point(|&(center, _)| f64::from(center) <= layout.max_freq);
    let visible = &filters[first..last];
    let half_bin = bin_hz * (0.5 + 1e-6);
    if hz < f64::from(visible.first()?.0) - half_bin || hz > f64::from(visible.last()?.0) + half_bin
    {
        return None;
    }
    let right = visible.partition_point(|&(center, _)| f64::from(center) < hz);
    let left = &visible[right.saturating_sub(1)];
    let right = visible.get(right).unwrap_or(left);
    if (f64::from(left.0) - hz).abs() <= (f64::from(right.0) - hz).abs() {
        Some(left)
    } else {
        Some(right)
    }
}

pub(super) struct Reading {
    frequency: f64,
    input_db: Option<f32>,
    gain_db: Option<f32>,
    bypassed: bool,
}

impl Reading {
    pub(super) fn paint(&self, d: &mut Draw, layout: &GraphLayout) {
        let x = layout.freq_to_x(self.frequency);
        let top = layout.gy;
        let bottom = layout.gy + layout.gh;
        d.scissor(layout.gx, top, layout.gw, layout.gh);
        d.line(x, top, x, bottom, MUTED, 1.2);
        if let Some(gain) = self.gain_db {
            let y = layout
                .db_to_y(f64::from(gain))
                .clamp(top + 5.0, bottom - 5.0);
            let marker = [
                (x, y - 5.0),
                (x + 5.0, y),
                (x, y + 5.0),
                (x - 5.0, y),
                (x, y - 5.0),
            ];
            d.fill_poly(&marker, GOLD);
            d.poly(&marker, TEXT, 1.2);
        }
        if let Some(input) = self.input_db {
            let y = layout
                .mag_to_y(f64::from(input))
                .clamp(top + 5.0, bottom - 5.0);
            d.circle(x, y, 4.0, PANEL, true);
            d.circle(x, y, 4.0, TEXT, false);
        }
        d.reset_scissor();
    }

    pub(super) fn paint_footer(&self, d: &mut Draw, mut x: f32, y: f32) {
        let frequency = if self.frequency >= 1000.0 {
            format!("{:.3} kHz", self.frequency / 1000.0)
        } else {
            format!("{:.1} Hz", self.frequency)
        };
        let input = self.input_db.map_or_else(
            || "—".into(),
            |db| {
                if db <= INPUT_FLOOR_DB {
                    "≤ −120 dBFS".into()
                } else {
                    format!("{db:.1} dBFS")
                }
            },
        );
        let gain = self.gain_db.map_or_else(
            || "—".into(),
            |db| {
                if db.abs() < 0.05 {
                    "0.0 dB".into()
                } else {
                    format!("{db:+.1} dB")
                }
            },
        );
        for (text, label) in [
            (frequency.as_str(), false),
            ("  ·  Input ", true),
            (input.as_str(), false),
            ("  ·  Applied gain ", true),
            (gain.as_str(), false),
        ] {
            let mut run = |d: &mut Draw| {
                d.text(
                    x,
                    y,
                    text,
                    READABLE_LABEL_SIZE,
                    if label { MUTED } else { TEXT },
                );
                x += d.text_width(text, READABLE_LABEL_SIZE);
            };
            if label {
                d.with_label_font(run);
            } else {
                run(d);
            }
        }
        if self.bypassed {
            d.with_label_font(|d| d.text(x, y, "  ·  Bypassed", READABLE_LABEL_SIZE, TEXT));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_probe_reads_spectrum_and_gain_at_the_same_filter_center() {
        let layout = GraphLayout::default();
        let mut frame = DisplayFrame {
            spectrum: vec![-100.0; 256],
            filters: vec![(850.0, -3.2), (1150.0, 2.0)],
        };
        frame.spectrum[8] = -24.0;
        frame.spectrum[9] = -48.0;
        let reading = frame
            .reading(&layout, layout.freq_to_x(970.0), 512, 100.0, false)
            .unwrap();
        assert_eq!(
            (reading.frequency, reading.input_db, reading.gain_db),
            (850.0, Some(-24.0), Some(-3.2))
        );
    }

    #[test]
    fn frequency_probe_distinguishes_missing_filter_data_from_bypass() {
        let layout = GraphLayout::default();
        let frame = DisplayFrame {
            spectrum: vec![-60.0; 256],
            filters: vec![(850.0, -3.2)],
        };
        let x = layout.freq_to_x(1150.0);
        let outside = frame.reading(&layout, x, 512, 100.0, false).unwrap();
        assert_eq!(
            (outside.frequency, outside.input_db, outside.gain_db),
            (1150.0, Some(-60.0), None)
        );
        let bypassed = frame.reading(&layout, x, 512, 100.0, true).unwrap();
        assert_eq!(bypassed.gain_db, Some(0.0));
        assert!(bypassed.bypassed);
        assert!(frame
            .reading(&layout, layout.freq_to_x(20000.0), 512, 60.0, false)
            .is_none());
    }

    #[test]
    fn frequency_probe_rejects_old_fft_data_and_respects_the_zoomed_range() {
        let shared = Shared::new();
        shared.sample_rate.store(51200.0, Ordering::Relaxed);
        shared.fft_size.store(512, Ordering::Relaxed);
        *shared.spectrum_mags_db.write().unwrap() = vec![-42.0; 256];
        *shared.filter_display.write().unwrap() = vec![(350.0, 1.0), (850.0, -4.0), (1150.0, -2.0)];
        let mut frame = DisplayFrame::default();
        frame.refresh(&shared, 512, 51200.0);
        assert_eq!(frame.spectrum[8], -42.0);
        let layout = GraphLayout {
            min_freq: 800.0,
            max_freq: 1000.0,
            ..GraphLayout::default()
        };
        let reading = frame
            .reading(&layout, layout.freq_to_x(800.0), 512, 100.0, false)
            .unwrap();
        assert_eq!((reading.frequency, reading.gain_db), (850.0, Some(-4.0)));
        shared.fft_size.store(1024, Ordering::Relaxed);
        frame.refresh(&shared, 1024, 51200.0);
        assert!(frame.spectrum.is_empty() && frame.filters.is_empty());
    }
}
