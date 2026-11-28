//! Rhodes MK8 S dry-send and standard analogue signal path.
pub mod action;
pub mod field;
pub mod fx;
pub mod live;
mod modes;
pub mod panel;

#[cfg(test)]
mod tests {
    use super::{
        action::Action,
        live::Engine,
        panel::{Controls, Panel, Wave},
    };
    use std::f64::consts::TAU;

    #[test]
    fn disabling_extension_on_a_native_128_key_compass_preserves_both_endpoints() {
        use pleasant_dsp::struck_keys::StrikeKeys;
        let mut keys = StrikeKeys::new(48000., 0, 127, |n| Action::new(n, 192000., 48000.));
        keys.set_extended(true);
        keys.note_on(0, 0.7);
        keys.note_on(127, 0.7);
        keys.set_extended(false);
        assert_eq!(keys.held_notes(), 1 | 1u128 << 127);
    }
    #[test]
    fn range_midi_and_audition_preserve_note_ownership_and_real_pitch() {
        for rate in [44100, 48000, 96000] {
            let mut e = Engine::new(f64::from(rate));
            e.note_on(0, 0.7);
            assert_eq!(e.held_notes(), 0);
            e.set_extended(true);
            for n in [0, 28, 60, 100, 127] {
                e.reset();
                e.note_on(n, 0.6);
                let f = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
                let mut bins = [(0., 0.); 3];
                for i in 0..rate {
                    let x = e.next_outputs();
                    assert!(x.send_v.is_finite());
                    let t = TAU * f64::from(i) / f64::from(rate);
                    for ((a, b), ratio) in bins.iter_mut().zip([0.90, 1., 1.10]) {
                        let (s, c) = (f * ratio * t).sin_cos();
                        let w = 0.5 - 0.5 * t.cos();
                        *a += w * x.send_v * c;
                        *b += w * x.send_v * s;
                    }
                }
                let b = bins.map(|(a, b)| f64::hypot(a, b));
                assert!(b[1] > b[0] && b[1] > b[2], "{n} {rate} bins {b:?}");
            }
            e.note_on(54, 0.6);
            e.audition(54, 0.7);
            e.note_off(54);
            e.set_extended(false);
            assert_eq!(e.held_notes(), 1 << 54);
            e.set_sustain(true);
            e.audition_off();
            assert!(e.sounding());
            e.set_sustain(false);
            e.choke(54);
            let mut buf = [0.; 256];
            for _ in 0..200 {
                e.render(&mut buf);
                if !e.sounding() {
                    break;
                }
            }
            assert!(!e.sounding());
            e.reset();
            e.render(&mut buf);
            assert_eq!(buf, [0.; 256]);
        }
    }

    #[test]
    fn pickup_setback_changes_voltage_and_preserves_the_two_axes() {
        let mut stock = Action::new(60, 192000., 48000.);
        let mut far = Action::new(60, 192000., 48000.);
        far.set_pickup_gap_mm(1.5);
        stock.strike(1.1);
        far.strike(1.1);
        let mut energy = [0., 0.];
        let mut order = [[0.; 2]; 2];
        let f = 440. * 2f64.powf(-9. / 12.);
        for i in 0..38400 {
            let x = stock.step();
            let y = far.step();
            assert_eq!(
                (x.tine_mm, x.lateral_mm, x.contact_n),
                (y.tine_mm, y.lateral_mm, y.contact_n)
            );
            for ((en, amps), v) in energy
                .iter_mut()
                .zip(order.iter_mut())
                .zip([x.emf_v, y.emf_v])
            {
                *en += v * v;
                for (k, a) in amps.iter_mut().enumerate() {
                    *a += v * (TAU * (k + 1) as f64 * f * i as f64 / 192000.).sin();
                }
            }
        }
        assert!(energy[1] < energy[0] * 0.75);
        assert!(
            ((order[0][1] / order[0][0]) / (order[1][1] / order[1][0]))
                .abs()
                .log10()
                .abs()
                > 0.05
        );
    }

    #[test]
    fn eq_drive_and_stereo_modulation_leave_the_send_independent() {
        let c = Controls {
            drive: 0.7,
            low_db: 15.,
            mid_db: 12.,
            high_db: -9.,
            envelope: 0.8,
            pan_rate_hz: 311.,
            pan_depth: 1.,
            pan_enabled: true,
            wave: Wave::Sine,
            ..Controls::default()
        };
        let mut p = Panel::new(96000., c);
        let mut flat = Panel::new(96000., Controls::default());
        let mut diff = 0.;
        let mut pan = 0.;
        for i in 0..24000 {
            let v = 0.2 * (TAU * 164. * i as f64 / 96000.).sin();
            let x = p.tick(v);
            let y = flat.tick(v);
            assert_eq!(x.send_v, y.send_v);
            assert_eq!(y.left_v, y.right_v);
            diff += (x.left_v - y.left_v).powi(2);
            pan += (x.left_v - x.right_v).powi(2);
        }
        assert!(pan > 0.1 && diff > 0.1);
        p.set_controls(Controls { volume: 0., ..c });
        let first = p.tick(0.5);
        assert_eq!(first.send_v, flat.tick(0.5).send_v);
        assert_ne!((first.left_v, first.right_v), (0., 0.));
        for _ in 1..480 {
            let x = p.tick(0.5);
            assert_eq!(x.send_v, flat.tick(0.5).send_v);
        }
        let muted = p.tick(0.5);
        assert_eq!((muted.left_v, muted.right_v), (0., 0.));
    }
    #[test]
    fn three_eq_centers_reach_the_panel_marks_at_return() {
        for (low, mid, high, f) in [
            (15., 0., 0., 75.),
            (0., -15., 0., 480.),
            (0., 0., 15., 3000.),
        ] {
            let mut p = Panel::new(
                48000.,
                Controls {
                    low_db: low,
                    mid_db: mid,
                    high_db: high,
                    ..Controls::default()
                },
            );
            let mut ratio = 0.;
            let mut energy = 0.;
            for i in 0..48000 {
                let x = 0.01 * (TAU * f * i as f64 / 48000.).sin();
                let y = p.process_return(x)[0];
                if i >= 24000 {
                    ratio += y * y;
                    energy += x * x;
                }
            }
            assert!((10. * (ratio / energy).log10() - (low + mid + high)).abs() < 0.02);
        }
    }
}
