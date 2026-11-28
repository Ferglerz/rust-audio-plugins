//! 2013 Vintage Vibe 64 Classic with passive or serviced StereoVibe Rev 4.1 electronics.
pub mod action;
pub mod live;
pub mod panel;
pub mod stereo;

#[cfg(test)]
mod tests {
    use super::{
        action::{length_mm, Action, HIGH, LOW},
        live::Engine,
        panel::{Controls, Panel},
    };
    use std::f64::consts::TAU;

    #[test]
    fn moving_the_pickup_changes_voltage_and_harmonics_without_moving_the_fork() {
        let mut actions = [
            Action::new(55, 192000., 48000.),
            Action::new(55, 192000., 48000.),
        ];
        for a in &mut actions {
            a.strike(1.2);
        }
        let hz = 440. * 2_f64.powf((55. - 69.) / 12.);
        let mut bins = [[(0., 0.); 2]; 2];
        let mut force = 0_f64;
        for i in 0..38400 {
            let a = actions[0].step(0.);
            let b = actions[1].step(0.0015);
            force = force.max(a.contact_n);
            assert_eq!(
                (a.tine_m, a.tonebar_m, a.contact_n),
                (b.tine_m, b.tonebar_m, b.contact_n)
            );
            if i > 8000 {
                for (m, v) in bins.iter_mut().zip([a.pickup_v, b.pickup_v]) {
                    for (j, (re, im)) in m.iter_mut().enumerate() {
                        let (s, c) = (TAU * hz * (j + 1) as f64 * i as f64 / 192000.).sin_cos();
                        *re += v * c;
                        *im += v * s;
                    }
                }
            }
        }
        let a = bins.map(|x| x.map(|(r, i)| f64::hypot(r, i)));
        assert!(force > 0.1 && actions[0].step(0.).contact_n == 0.);
        assert!(a[0][0] > a[1][0] * 1.5);
        assert!(((a[0][1] / a[0][0]) / (a[1][1] / a[1][0])).log10().abs() > 0.05);
    }
    #[test]
    fn releases_after_compass_changes_and_audition_keep_their_owners() {
        let mut e = Engine::new(44100.);
        e.note_on(0, 0.6);
        assert_eq!(e.held_notes(), 0);
        e.set_extended(true);
        e.note_on(0, 0.7);
        e.note_on(60, 0.8);
        e.audition(60, 0.7);
        e.note_off(60);
        e.set_extended(false);
        assert_eq!(e.held_notes(), 1 << 60);
        e.set_sustain(true);
        e.audition_off();
        assert_eq!(e.held_notes(), 0);
        let mut buf = [0.; 64];
        e.render(&mut buf);
        assert!(buf.iter().any(|y| *y != 0.));
        e.set_sustain(false);
        for _ in 0..5000 {
            e.render(&mut buf);
            if !e.sounding() {
                break;
            }
        }
        assert!(!e.sounding());
        e.render(&mut buf);
        assert_eq!(buf, [0.; 64]);
    }
    #[test]
    fn extension_holds_actual_pitch_with_nearest_physical_dimensions() {
        assert_eq!(length_mm(0), length_mm(LOW));
        assert_eq!(length_mm(127), length_mm(HIGH));
        for rate in [44100, 48000, 96000] {
            let mut e = Engine::new(f64::from(rate));
            e.set_extended(true);
            for n in [0, 33, 60, 96, 127] {
                e.reset();
                e.note_on(n, 0.6);
                let f = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
                let mut bins = [(0., 0.); 3];
                for i in 0..rate {
                    let y = e.next_outputs().jack_v;
                    assert!(y.is_finite());
                    for ((r, j), scale) in bins.iter_mut().zip([0.90, 1., 1.10]) {
                        let ph = TAU * f64::from(i) / f64::from(rate);
                        let (s, c) = (ph * f * scale).sin_cos();
                        let w = 0.5 - 0.5 * ph.cos();
                        *r += w * y * c;
                        *j += w * y * s;
                    }
                }
                let m = bins.map(|(r, i)| f64::hypot(r, i));
                assert!(m[1] > m[0] && m[1] > m[2], "{rate} {n} {m:?}");
            }
        }
    }
    #[test]
    fn passive_controls_selectively_cut_bass_and_treble_and_ground_the_jack() {
        let response = |c: Controls, f: f64| {
            let mut p = Panel::new(48000., c);
            let mut energy = 0.;
            for i in 0..24000 {
                let x = (i as f64 * TAU * f / 48000.).sin();
                let y = p.tick(x).jack_v;
                if i > 12000 {
                    energy += y * y;
                }
            }
            energy
        };
        let open = Controls::default();
        let bass = Controls { bass: 0., ..open };
        let treble = Controls { treble: 0., ..open };
        assert!(response(bass, 55.) < 0.25 * response(open, 55.));
        assert!(response(bass, 4000.) > 0.55 * response(open, 4000.));
        assert!(response(treble, 5000.) < 0.2 * response(open, 5000.));
        assert!(response(treble, 55.) > 0.90 * response(open, 55.));
        assert_eq!(response(Controls { volume: 0., ..open }, 220.), 0.);
    }
}
