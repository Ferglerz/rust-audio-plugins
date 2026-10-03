// Frozen reference from commit 3009591: preserve ordering and interpolation semantics.
use flattery::strength::StrengthNode;
use pleasant_dsp::axis::bell_influence;

/// Per-bin multiplier. Empty = flat 1.0 (global strength).
///
/// Each node is an independent bell, summed around the rest line so neighbors
/// do not pull a spline through each other.
pub fn weight_at(nodes: &[StrengthNode], freq: f64, _min_freq: f64, _max_freq: f64) -> f64 {
    if nodes.is_empty() {
        return 1.0;
    }
    let mut w = 1.0;
    for node in nodes {
        let inf = bell_influence(freq, node.freq, node.q);
        w += (node.weight - 1.0) * inf;
    }
    w.max(0.0)
}

pub fn fill_bin_weights(
    nodes: &[StrengthNode],
    bin_count: usize,
    bin_hz: f64,
    min_freq: f64,
    max_freq: f64,
    out: &mut [f64],
) {
    let n = bin_count.min(out.len());
    for (k, slot) in out.iter_mut().enumerate().take(n) {
        let freq = (k as f64 + 0.5) * bin_hz;
        *slot = weight_at(nodes, freq, min_freq, max_freq);
    }
}

/// Interpolate radius between nodes in log-frequency space using smoothstep.
pub fn radius_at(nodes: &[StrengthNode], freq: f64, default_radius: usize) -> f64 {
    if nodes.is_empty() {
        return default_radius as f64;
    }
    if nodes.len() == 1 {
        return nodes[0].radius as f64;
    }

    let mut sorted: Vec<&StrengthNode> = nodes.iter().collect();
    sorted.sort_by(|a, b| {
        a.freq
            .partial_cmp(&b.freq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if freq <= sorted[0].freq {
        return sorted[0].radius as f64;
    }
    if freq >= sorted.last().unwrap().freq {
        return sorted.last().unwrap().radius as f64;
    }

    for i in 0..sorted.len() - 1 {
        let n0 = sorted[i];
        let n1 = sorted[i + 1];
        if freq >= n0.freq && freq <= n1.freq {
            let f0_ln = n0.freq.ln();
            let f1_ln = n1.freq.ln();
            let t = if (f1_ln - f0_ln).abs() < 1e-6 {
                0.0
            } else {
                ((freq.ln() - f0_ln) / (f1_ln - f0_ln)).clamp(0.0, 1.0)
            };
            let smooth_t = t * t * (3.0 - 2.0 * t);
            return n0.radius as f64 + (n1.radius as f64 - n0.radius as f64) * smooth_t;
        }
    }

    sorted.last().unwrap().radius as f64
}

pub fn fill_bin_radii(
    nodes: &[StrengthNode],
    bin_count: usize,
    bin_hz: f64,
    default_radius: usize,
    out: &mut [usize],
) {
    let n = bin_count.min(out.len());
    for (k, slot) in out.iter_mut().enumerate().take(n) {
        let freq = (k as f64 + 0.5) * bin_hz;
        let r = radius_at(nodes, freq, default_radius).round();
        *slot = (r as usize).clamp(1, 12);
    }
}
