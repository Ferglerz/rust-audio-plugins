use super::*;

/// Keep each half of the gain axis evenly divided, with at most three intervals.
pub(in crate::ui) fn gain_axis_values(range: f64) -> Vec<i32> {
    let intervals = ((range / 6.0).round() as i32).clamp(1, 3);
    (-intervals..=intervals)
        .map(|i| (range * i as f64 / intervals as f64).round() as i32)
        .collect()
}

pub(in crate::ui) fn freq_x_at(freq: f64, gx: f32, gw: f32) -> f32 {
    gx + gw * (freq / 20.0).log(1000.0).clamp(0.0, 1.0) as f32
}

pub(in crate::ui) fn x_freq_at(x: f32, gx: f32, gw: f32) -> f64 {
    20.0 * 1000.0_f64.powf(((x - gx) / gw).clamp(0.0, 1.0) as f64)
}

pub(in crate::ui) fn db_y(db: f64, range: f64) -> f32 {
    GY + GH * (0.5 - (db.clamp(-range, range) / (2.0 * range)) as f32)
}

pub(in crate::ui) fn y_db(y: f32, range: f64) -> f64 {
    let range = range.abs().max(1.0e-6);
    ((0.5 - (y - GY) as f64 / GH as f64) * 2.0 * range).clamp(-range, range)
}

#[derive(Default)]
pub(in crate::ui) struct EqDrawScratch {
    pub xs: Vec<f32>,
    pub coefficients: Vec<pleasant_eq::BandCoefficients>,
    pub frequencies: Vec<f64>,
    pub points: Vec<(f32, f32)>,
    pub sum_points: Vec<(f32, f32)>,
    pub selected_fill: Vec<(f32, f32)>,
}

/// Pixel-spaced x plus each band's exact frequency so peaks and notch zeros
/// land on a vertex instead of being skipped by a coarse chord.
pub(in crate::ui) fn fill_eq_curve_xs(
    xs: &mut Vec<f32>,
    gx: f32,
    gw: f32,
    extra_freqs: impl IntoIterator<Item = f64>,
) {
    xs.clear();
    let n = gw.max(2.0).ceil() as usize;
    xs.extend((0..=n).map(|i| gx + gw * (i as f32 / n as f32)));
    for f in extra_freqs {
        if f.is_finite() && f > 0.0 {
            let x = freq_x_at(f, gx, gw);
            if x >= gx && x <= gx + gw {
                xs.push(x);
            }
        }
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.15);
}

pub(in crate::ui) fn eq_curve_xs(
    gx: f32,
    gw: f32,
    extra_freqs: impl IntoIterator<Item = f64>,
) -> Vec<f32> {
    let mut xs = Vec::new();
    fill_eq_curve_xs(&mut xs, gx, gw, extra_freqs);
    xs
}

/// Preserve the exact graph grid and host Nyquist clamp while evaluating at
/// the processing rate. Each EQ page owns its independent cache.
#[expect(
    clippy::too_many_arguments,
    reason = "Explicitly passes scratch buffers alongside graph and rate parameters"
)]
pub(in crate::ui) fn update_eq_response_cache_into(
    cache: &mut pleasant_eq::ResponseCache,
    bands: &[Band],
    meters: &[(u64, f32)],
    xs: &[f32],
    (gx, gw): (f32, f32),
    (sr, eq_sr): (f64, f64),
    coefficients: &mut Vec<pleasant_eq::BandCoefficients>,
    frequencies: &mut Vec<f64>,
) {
    coefficients.clear();
    coefficients.extend(
        bands
            .iter()
            .filter(|b| b.enabled)
            .map(|b| BandCoeffs::make(&plot_band(b, meters), eq_sr).coefficients()),
    );
    frequencies.clear();
    let nyq = sr * 0.49;
    frequencies.extend(xs.iter().map(|&x| x_freq_at(x, gx, gw).min(nyq)));
    cache.update(coefficients, frequencies, eq_sr);
}

#[cfg(test)]
pub(in crate::ui) fn update_eq_response_cache(
    cache: &mut pleasant_eq::ResponseCache,
    bands: &[Band],
    meters: &[(u64, f32)],
    xs: &[f32],
    (gx, gw): (f32, f32),
    (sr, eq_sr): (f64, f64),
) {
    let mut coefficients = Vec::new();
    let mut frequencies = Vec::new();
    update_eq_response_cache_into(
        cache,
        bands,
        meters,
        xs,
        (gx, gw),
        (sr, eq_sr),
        &mut coefficients,
        &mut frequencies,
    );
}

pub(in crate::ui) fn eq_db_on_xs(
    coeff: &BandCoeffs,
    xs: &[f32],
    gx: f32,
    gw: f32,
    sr: f64,
    eq_sr: f64,
) -> Vec<f64> {
    let nyq = sr * 0.49;
    xs.iter()
        .map(|&x| coeff.response(x_freq_at(x, gx, gw).min(nyq), eq_sr))
        .collect()
}

pub(in crate::ui) fn fill_eq_points(
    points: &mut Vec<(f32, f32)>,
    xs: &[f32],
    dbs: &[f64],
    graph_db: f64,
) {
    points.clear();
    points.extend(xs.iter().zip(dbs).map(|(&x, &db)| (x, db_y(db, graph_db))));
}

pub(in crate::ui) fn eq_points(xs: &[f32], dbs: &[f64], graph_db: f64) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    fill_eq_points(&mut points, xs, dbs, graph_db);
    points
}

pub(in crate::ui) fn plot_band(b: &Band, meters: &[(u64, f32)]) -> Band {
    let mut plot = b.clone();
    if plot.dynamic && plot.shape.has_gain() {
        if let Some(uncapped) = uncapped_for(plot.id, meters) {
            plot.gain -= capped_reduction(plot.range, uncapped);
        }
    }
    plot
}

#[cfg(test)]
mod axis_tests {
    use super::*;
    #[test]
    fn every_gain_range_has_uniform_subdivisions_and_both_endpoints() {
        assert_eq!(gain_axis_values(18.0), vec![-18, -12, -6, 0, 6, 12, 18]);
        assert_eq!(gain_axis_values(72.0), vec![-72, -48, -24, 0, 24, 48, 72]);
        for range in SCALES {
            let values = gain_axis_values(range);
            assert_eq!(values.first(), Some(&-(range as i32)));
            assert_eq!(values.last(), Some(&(range as i32)));
            assert!(values.contains(&0));
            assert!(values.len() <= 7);
            let step = values[1] - values[0];
            assert!(values.windows(2).all(|pair| pair[1] - pair[0] == step));
        }
    }
}
