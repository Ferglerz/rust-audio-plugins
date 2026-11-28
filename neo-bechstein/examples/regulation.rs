//! Evaluate the assumed keyboard-to-amplifier sensitivity on a reproducible chord.
use neo_bechstein::Engine;
fn main() {
    for rate in [44100, 48000, 96000] {
        for (keys, velocity) in [
            (&[48, 55, 60, 64][..], 100u8),
            (&[48, 55, 60, 64][..], 127),
            (&[60][..], 80),
            (&[60][..], 127),
        ] {
            let mut e = Engine::new(f64::from(rate));
            for &n in keys {
                e.keyboard.note_on(n, f32::from(velocity) / 127.);
            }
            let start = rate / 50;
            let end = start + rate / 10;
            let mut sum = [0.; 2];
            let mut peak = [0_f64; 2];
            for i in 0..end {
                let (t, s) = e.sample_with_taps();
                for (j, v) in [t.input_v, s].into_iter().enumerate() {
                    peak[j] = peak[j].max(v.abs());
                    if i >= start {
                        sum[j] += v * v;
                    }
                }
            }
            println!("{rate} {keys:?} v{velocity} rmsE1={:.6}V rms6ohm={:.4}V peakE1={:.5}V peak6ohm={:.4}V",
                (sum[0]/f64::from(end-start)).sqrt(),(sum[1]/f64::from(end-start)).sqrt(),peak[0],peak[1]);
        }
    }
}
