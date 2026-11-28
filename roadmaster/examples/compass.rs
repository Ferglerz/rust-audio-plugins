use roadmaster::{live::Engine, HIGH, LOW};
fn main() {
    let rate: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut e = Engine::new(rate as f64);
    let mut best = [(0.0, vec![]), (0., vec![])];
    let mut strike = |notes: &[u8], v: f32| {
        e.reset();
        for &n in notes {
            e.note_on(n, v);
        }
        for _ in 0..(rate as f64 * 0.18) as usize {
            let x = e.next_outputs();
            for (slot, peak) in best.iter_mut().zip([x.jack_v.abs(), x.board_m_s.abs()]) {
                if peak > slot.0 {
                    slot.0 = peak;
                    slot.1 = notes.into();
                }
            }
        }
    };
    for n in LOW..=HIGH {
        strike(&[n], 1.);
    }
    for n in LOW..=HIGH - 28 {
        let notes = [0, 7, 12, 16, 19, 23, 26, 28].map(|p| n + p);
        strike(&notes, 0.75);
        strike(&notes, 1.);
    }
    println!("{rate} jack V / board m/s: {best:?}");
}
