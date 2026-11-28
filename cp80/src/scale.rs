//! Yamaha service table (p. 14), key 1 = A0. Acoustic anchor values are described
//! in the research notes. Units are metres, newtons, kg, N/m² and dB/s.
use crate::KeySpec;
const CENTS: [i8; 88] = [
    -23, -21, -19, -17, -16, -15, -14, -13, -12, -11, -10, -9, -8, -7, -7, -6, -6, -5, -5, -5, -4,
    -4, -4, -4, -4, -3, -3, -3, -3, -3, -2, -2, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, 0, 0, 0, 0,
    0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 11, 12,
    14, 16, 18, 20, 21, 23, 25, 27, 29, 31, 33, 35,
];

struct Anchor {
    n: u8,
    b: f64,
    length: f64,
    tension: f64,
    k: f64,
    pos: f64,
    kg: f64,
    loss: [f64; 6],
}
const fn anchor(
    n: u8,
    b: f64,
    length: f64,
    tension: f64,
    hammer: [f64; 3],
    loss: [f64; 6],
) -> Anchor {
    Anchor {
        n,
        b,
        length,
        tension,
        k: hammer[0],
        pos: hammer[1],
        kg: hammer[2],
        loss,
    }
}
const SINGLE: &[Anchor] = &[
    anchor(
        21,
        0.0027,
        1.12,
        480.,
        [8.17e6, 0.080, 0.006],
        [6.5, 3.1, 3.1, 1.7, 3.5, 2.7],
    ),
    anchor(
        27,
        0.001439,
        1.05,
        480.,
        [1.1792e7, 0.11981, 0.003],
        [6.45, 3.07, 3.15, 1.7, 3.46, 2.66],
    ),
    anchor(
        42,
        0.00017,
        0.89,
        510.,
        [8.1717e6, 0.08115, 0.0041375],
        [2., 5.9, 2., 4.1, 10.5, 16.4],
    ),
];
const WOUND: &[Anchor] = &[
    anchor(
        43,
        0.00018,
        0.89,
        510.,
        [1.2054e7, 0.10402, 0.003521],
        [2., 5.9, 2., 4.1, 10.5, 16.4],
    ),
    anchor(
        50,
        0.00020,
        0.76,
        510.,
        [1.2054e7, 0.10402, 0.003521],
        [2.1, 5., 6.7, 16.5, 23., 17.],
    ),
    anchor(
        57,
        0.000258,
        0.65,
        530.,
        [8.643e6, 0.05852, 0.0050457],
        [5.64, 5.09, 13.28, 0.6, 12.28, 16.54],
    ),
    anchor(
        62,
        0.00035,
        0.55,
        560.,
        [8.643e6, 0.05852, 0.0050457],
        [4., 7., 11., 9., 13., 15.],
    ),
];
const PLAIN: &[Anchor] = &[
    anchor(
        63,
        0.000425,
        0.52,
        620.,
        [2.4589e7, 0.09515, 0.0068202],
        [3.4, 8., 8.4, 13., 13., 16.],
    ),
    anchor(
        72,
        0.000862,
        0.38,
        620.,
        [8.597e7, 0.18696, 0.0040552],
        [9.39, 10.53, 26.68, 15.65, 10.9, 34.26],
    ),
    anchor(
        80,
        0.0017347,
        0.282,
        595.,
        [7.0616e7, 0.21541, 0.0069857],
        [12.35, 28.65, 49.38, 46.09, 40.11, 50.],
    ),
    anchor(
        85,
        0.002457,
        0.22,
        580.,
        [2.9005e7, 0.12074, 0.0081171],
        [29.18, 50., 30.94, 50., 50., 50.],
    ),
    anchor(
        91,
        0.004669,
        0.17,
        570.,
        [2.1252e8, 0.09742, 0.0093662],
        [15.83, 33.85, 50., 50., 50., 50.],
    ),
    anchor(
        97,
        0.007557,
        0.12,
        560.,
        [1.4705e8, 0.16784, 0.0076357],
        [26.97, 50., 50., 50., 50., 50.],
    ),
    anchor(
        102,
        0.009,
        0.09,
        550.,
        [1.3853e7, 0.11870, 0.0067844],
        [40.06, 50., 50., 50., 50., 50.],
    ),
    anchor(107, 0.011, 0.069, 540., [1.2e8, 0.08, 0.003], [43.74; 6]),
    anchor(108, 0.0115, 0.065, 540., [1.2e8, 0.08, 0.003], [44.; 6]),
];
// Yamaha's wire numbers expressed as nominal European piano-wire diameters (mm).
fn diameter(n: u8) -> f64 {
    match n {
        63 => 1.025,
        64..=71 => 0.975,
        72..=77 => 0.95,
        78..=83 => 0.925,
        84..=91 => 0.90,
        92..=96 => 0.875,
        97..=100 => 0.85,
        101..=104 => 0.825,
        _ => 0.80,
    }
}
pub(crate) fn spec(midi: u8) -> Option<KeySpec> {
    let anchors = match midi {
        21..=42 => SINGLE,
        43..=62 => WOUND,
        63..=108 => PLAIN,
        _ => return None,
    };
    let i = anchors
        .partition_point(|a| a.n < midi)
        .min(anchors.len() - 1);
    let a = &anchors[i.saturating_sub(1)];
    let b = &anchors[i];
    let t = f64::from(midi - a.n) / f64::from((b.n - a.n).max(1));
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    let [contact_k, strike_fraction, hammer_kg] = crate::contact::CONTACT
        .binary_search_by_key(&midi, |(key, _)| *key)
        .map(|i| crate::contact::CONTACT[i].1)
        .unwrap_or_else(|_| [lerp(a.k, b.k), lerp(a.pos, b.pos), lerp(a.kg, b.kg)]);
    let mut spec = KeySpec {
        midi,
        strings: if midi <= 42 { 1 } else { 2 },
        cents: f64::from(CENTS[usize::from(midi - 21)]),
        stiffness: if midi >= 63 {
            diameter(midi).powi(4) * lerp(a.b / diameter(a.n).powi(4), b.b / diameter(b.n).powi(4))
        } else {
            lerp(a.b, b.b)
        },
        length: lerp(a.length, b.length),
        tension: lerp(a.tension, b.tension),
        hammer_kg,
        contact_k,
        contact_linear: 0.,
        strike_fraction,
        unison_cents: 0.8,
        bearing_imbalance: 0.,
        losses_db_s: std::array::from_fn(|i| lerp(a.loss[i], b.loss[i])),
    };
    if midi == 74 {
        // D5 has no native recording. Fit the ringing fundamental level and
        // relative partials to the two adjacent modeled keys across velocities.
        spec.contact_k = 2.496e7;
        spec.strike_fraction = 0.10932;
        spec.hammer_kg = 0.003;
    } else if midi == 82 {
        // The native MP fundamental has two lines 0.84 Hz apart, with unequal
        // amplitudes. Their partial cancellation and the slower H3 decay are
        // audible well after the hammer has left the strings.
        spec.unison_cents = 1.5;
        spec.bearing_imbalance = 0.40;
        spec.losses_db_s[..3].copy_from_slice(&[8., 20., 20.5]);
        // Initial felt stiffness constrains the third partial from MP to FF.
        spec.contact_linear = 16787.162;
        spec.contact_k = 265143.2;
        spec.strike_fraction = 0.17098166;
        spec.hammer_kg = 0.01177944;
    } else if midi == 95 {
        // Direct B6 MP/FF voicing. A finite initial stiffness keeps the force
        // spectrum much steadier with strike speed. Values are effective fits.
        spec.contact_linear = 25741.2;
        spec.contact_k = 290780.;
        spec.strike_fraction = 0.2106846;
        spec.hammer_kg = 0.00690504;
        // The recorded fundamental beats more slowly than the default pair.
        // H2 loses an additional 40 dB/s relative to H1 during the early ring.
        spec.unison_cents = 0.35;
        spec.losses_db_s[1] = spec.losses_db_s[0] + 40.;
    }
    Some(spec)
}
