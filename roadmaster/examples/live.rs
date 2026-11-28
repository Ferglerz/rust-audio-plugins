use roadmaster::live::Engine;
use std::time::Instant;
fn main() {
    for r in [44100, 48000, 96000] {
        let mut e = Engine::new(r as f64);
        e.set_extended(true);
        for notes in [
            &[60u8][..],
            &[36, 43, 48, 52, 55, 59, 62, 64][..],
            &[0, 127][..],
        ] {
            e.reset();
            let start = Instant::now();
            let mut peak = (0f64, 0f64);
            let mut rms = 0.;
            for &n in notes {
                e.note_on(n, 0.80);
            }
            for t in 0..r / 8 {
                if t == r / 10 {
                    for &n in notes {
                        e.note_off(n);
                    }
                }
                let tap = e.next_outputs();
                peak.0 = peak.0.max(tap.jack_v.abs());
                peak.1 = peak.1.max(tap.board_m_s.abs());
                rms += tap.jack_v.powi(2);
                assert!(tap.jack_v.is_finite() && tap.board_m_s.is_finite());
            }
            println!(
                "{r} {notes:?} jack peak/RMS {:.4}/{:.4} V, board {:.4} m/s; elapsed/period {:.1}%",
                peak.0,
                (rms / (r / 8) as f64).sqrt(),
                peak.1,
                start.elapsed().as_secs_f64() * 800.
            );
        }
    }
}
