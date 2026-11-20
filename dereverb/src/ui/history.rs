use super::*;
use nih_plug_vizia::vizia::vg::{Paint, Path};

const HISTORY_INSET: f32 = 5.0;
const SAMPLE_SECONDS: f32 = 0.01;

pub(super) struct HistorySample {
    index: usize,
    input: f32,
    output: f32,
}

impl HistorySample {
    fn delta(&self) -> Option<f32> {
        (self.input >= HISTORY_FLOOR && self.output >= HISTORY_FLOOR)
            .then(|| self.output - self.input)
    }

    pub(super) fn tooltip(&self) -> String {
        let age = (POINTS - 1 - self.index) as f32 * SAMPLE_SECONDS;
        let time = if age == 0.0 {
            "Now".into()
        } else {
            format!("−{age:.2} s")
        };
        let mut text = format!(
            "{time}  ·  In {}  ·  Out {}",
            level(self.input),
            level(self.output)
        );
        if let Some(delta) = self.delta() {
            let value = if delta.abs() < 0.05 {
                "0.0".into()
            } else {
                format!("{delta:+.1}")
            };
            text.push_str(&format!("  ·  Change {value} dB"));
        }
        text
    }
}

impl DereverbView {
    pub(super) fn paint_history(
        &self,
        d: &mut Draw,
        input: &[f32; POINTS],
        output: &[f32; POINTS],
        input_power: f64,
        output_power: f64,
    ) -> Option<HistorySample> {
        let sample = d
            .hover
            .filter(|_| self.shared.status.load(Ordering::Relaxed) != telemetry::FAILED)
            .and_then(history_index)
            .map(|index| HistorySample {
                index,
                input: dbfs(input[index]),
                output: dbfs(output[index]),
            });
        d.with_label_font(|d| d.text(GRAPH.0, 414.0, "Level history", READABLE_LABEL_SIZE, TEXT));
        history_key(d, 258.0, 416.0, "In", input_power, MUTED);
        history_key(d, 432.0, 560.0, "Out", output_power, TEAL);

        for db in [-90, -60, -30, 0] {
            let y = level_y(db as f32);
            d.line(GRAPH.0, y, GRAPH.0 + GRAPH.2, y, LINE, 1.0);
            d.text_right(
                GRAPH.0 - 10.0,
                y + 4.0,
                &db.to_string(),
                READABLE_AXIS_SIZE,
                MUTED,
            );
        }
        d.scissor(GRAPH.0, GRAPH.1, GRAPH.2, GRAPH.3);
        let points = |history: &[f32; POINTS]| -> [(f32, f32); POINTS] {
            std::array::from_fn(|i| (history_x(i), level_y(dbfs(history[i]))))
        };
        let input = points(input);
        let output = points(output);
        if let Some(sample) = &sample {
            draw_reduction(d, &input, &output);
            let x = history_x(sample.index);
            d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3, MUTED, 1.5);
        }
        // Two sample segments on, one off. A wider dashed input remains visible
        // around the narrower output even when both histories coincide.
        for start in (0..POINTS - 1).step_by(3) {
            d.poly(&input[start..(start + 3).min(POINTS)], MUTED, 3.4);
        }
        if d.light {
            d.poly(&output, MUTED, 2.8);
        }
        d.poly(&output, TEAL, 1.8);
        if let Some(sample) = &sample {
            let x = history_x(sample.index);
            let input_y = level_y(sample.input);
            let output_y = level_y(sample.output);
            d.circle(x, input_y, 5.0, pleasant_ui::theme::BG, true);
            d.outline_rounded(x - 5.0, input_y - 5.0, 10.0, 10.0, 5.0, MUTED, 1.5);
            d.fill_poly(
                &[
                    (x, output_y - 4.0),
                    (x + 4.0, output_y),
                    (x, output_y + 4.0),
                    (x - 4.0, output_y),
                ],
                TEAL,
            );
        }
        d.reset_scissor();
        d.text(GRAPH.0, 538.0, "−1.6 s", READABLE_AXIS_SIZE, MUTED);
        d.with_label_font(|d| {
            d.text_right(GRAPH.0 + GRAPH.2, 538.0, "Now", READABLE_LABEL_SIZE, MUTED)
        });
        sample
    }
}

fn history_key(
    d: &mut Draw,
    left: f32,
    right: f32,
    name: &str,
    power: f64,
    color: nih_plug_vizia::vizia::vg::Color,
) {
    if color == MUTED {
        d.line(left, 410.0, left + 6.0, 410.0, color, 2.0);
        d.line(left + 10.0, 410.0, left + 16.0, 410.0, color, 2.0);
    } else {
        d.line(left, 410.0, left + 16.0, 410.0, color, 2.0);
    }
    d.with_label_font(|d| d.text(left + 23.0, 414.0, name, READABLE_LABEL_SIZE, TEXT));
    d.text_right(
        right,
        414.0,
        &level(dbfs(power.sqrt() as f32)),
        READABLE_AXIS_SIZE,
        TEXT,
    );
}

fn history_x(index: usize) -> f32 {
    GRAPH.0 + HISTORY_INSET + (GRAPH.2 - 2.0 * HISTORY_INSET) * index as f32 / (POINTS - 1) as f32
}

fn history_index((x, y): (f32, f32)) -> Option<usize> {
    point_in_rect(x, y, GRAPH).then(|| {
        let position = (x - history_x(0)) / (history_x(POINTS - 1) - history_x(0));
        (position.clamp(0.0, 1.0) * (POINTS - 1) as f32).round() as usize
    })
}

fn dbfs(rms: f32) -> f32 {
    if rms.is_finite() && rms > 0.0 {
        20.0 * rms.log10()
    } else {
        f32::NEG_INFINITY
    }
}

fn level(db: f32) -> String {
    if db < HISTORY_FLOOR {
        "≤ −90 dBFS".into()
    } else {
        format!("{db:.1} dBFS")
    }
}

fn level_y(db: f32) -> f32 {
    GRAPH.1
        + HISTORY_INSET
        + (GRAPH.3 - 2.0 * HISTORY_INSET) * db.clamp(HISTORY_FLOOR, 0.0) / HISTORY_FLOOR
}

fn reduction_range(start: f32, end: f32) -> Option<(f32, f32)> {
    match (start > 0.0, end > 0.0) {
        (true, true) => Some((0.0, 1.0)),
        (true, false) => Some((0.0, start / (start - end))),
        (false, true) => Some((start / (start - end), 1.0)),
        (false, false) => None,
    }
}

fn draw_reduction(d: &mut Draw, input: &[(f32, f32); POINTS], output: &[(f32, f32); POINTS]) {
    let mut path = Path::new();
    for (input, output) in input.windows(2).zip(output.windows(2)) {
        let Some((start, end)) =
            reduction_range(output[0].1 - input[0].1, output[1].1 - input[1].1)
        else {
            continue;
        };
        let point = |trace: &[(f32, f32)], fraction: f32| {
            let x = trace[0].0 + (trace[1].0 - trace[0].0) * fraction;
            let y = trace[0].1 + (trace[1].1 - trace[0].1) * fraction;
            (d.ox + x * d.s, d.oy + y * d.s)
        };
        let (x, y) = point(input, start);
        path.move_to(x, y);
        for (trace, fraction) in [(input, end), (output, end), (output, start)] {
            let (x, y) = point(trace, fraction);
            path.line_to(x, y);
        }
        path.close();
    }
    let fill = pleasant_ui::with_alpha(TEAL, 0.20);
    d.c.fill_path(&path, &Paint::color(d.color(fill)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_lens_maps_graph_edges_and_snaps_to_samples() {
        let y = GRAPH.1 + GRAPH.3 * 0.5;
        assert_eq!(history_index((GRAPH.0, y)), Some(0));
        assert_eq!(history_index((GRAPH.0 + GRAPH.2, y)), Some(POINTS - 1));
        assert_eq!(history_index((history_x(42) + 0.4, y)), Some(42));
        assert_eq!(history_index((GRAPH.0 - 1.0, y)), None);
    }

    #[test]
    fn history_lens_reports_signed_changes_and_limits_silence_to_the_floor() {
        let mut sample = HistorySample {
            index: POINTS - 2,
            input: -18.0,
            output: -24.0,
        };
        assert_eq!(sample.delta(), Some(-6.0));
        assert!(sample.tooltip().contains("−0.01 s"));
        sample.output = -18.0;
        assert_eq!(sample.delta(), Some(0.0));
        sample.output = -12.0;
        assert!(sample.tooltip().contains("Change +6.0 dB"));
        sample.output = dbfs(0.0);
        assert!(sample.tooltip().contains("Out ≤ −90 dBFS"));
        assert_eq!(sample.delta(), None);
    }

    #[test]
    fn history_lens_shades_only_the_reduction_side_of_a_crossing() {
        assert_eq!(reduction_range(2.0, -2.0), Some((0.0, 0.5)));
        assert_eq!(reduction_range(-2.0, 6.0), Some((0.25, 1.0)));
        assert_eq!(reduction_range(1.0, 2.0), Some((0.0, 1.0)));
        assert_eq!(reduction_range(0.0, -2.0), None);
    }
}
