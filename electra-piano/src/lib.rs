//! Hohner Electra-Piano three-fork reference and 72-key real-time engine. Units are metres, seconds and volts.
pub mod circuit;
pub mod fork;
pub mod live;

#[cfg(test)]
mod tests {
    use super::{
        circuit::{group, winding_sign, Controls, Panel},
        fork::{Fork, KEYS},
    };

    #[test]
    fn fork_rebounds_and_restrikes_moving_prongs_at_each_rate() {
        for rate in [44100., 48000., 96000.] {
            for spec in KEYS {
                let mut fork = Fork::new(spec, rate);
                fork.strike(1.7);
                let mut contact_frames = 0;
                let mut arm_m = 0_f64;
                let mut return_m = 0_f64;
                for i in 0..(rate * 0.060) as usize {
                    let m = fork.tick();
                    assert!(m.pickup_v.is_finite() && m.sensed_m.abs() < 0.013);
                    if m.contact_n > 0. {
                        contact_frames += 1;
                    }
                    if i as f64 > rate * 0.012 {
                        assert_eq!(m.contact_n, 0.);
                    }
                    arm_m = arm_m.max(m.sensed_m.abs());
                    return_m = return_m.max(m.return_m.abs());
                }
                assert!(contact_frames > 4 && arm_m > 1e-5 && return_m > 1e-8 && arm_m > return_m);
                let before = fork.tick().sensed_m;
                fork.release();
                fork.strike(0.85);
                let after = fork.tick().sensed_m;
                assert!((after - before).abs() < 0.22 * arm_m);
            }
        }
    }

    #[test]
    fn right_pedal_catches_release_and_reapplication_damps_both_arms() {
        let rate = 48000.;
        for spec in KEYS {
            let mut held = Fork::new(spec, rate);
            let mut caught = Fork::new(spec, rate);
            let mut damped = Fork::new(spec, rate);
            for fork in [&mut held, &mut caught, &mut damped] {
                fork.strike(0.85);
            }
            let (mut control, mut ring, mut quiet) = (0., 0., 0.);
            for i in 0..36000 {
                if i == 4800 {
                    caught.sustain(true);
                    caught.release();
                    damped.release();
                }
                if i == 22000 {
                    caught.sustain(false);
                }
                let (a, b, c) = (held.tick(), caught.tick(), damped.tick());
                if (4800..22000).contains(&i) {
                    assert_eq!(a.sensed_m, b.sensed_m);
                }
                if i > 30000 {
                    control += a.sensed_m.powi(2) + a.return_m.powi(2);
                    ring += b.sensed_m.powi(2) + b.return_m.powi(2);
                    quiet += c.sensed_m.powi(2) + c.return_m.powi(2);
                }
            }
            assert!(control > 0. && ring < control * 0.001 && quiet < ring);
        }
    }

    #[test]
    fn series_groups_cancel_adjacent_common_flux_and_soft_pedal_attenuates() {
        assert_eq!(group(28), None);
        assert_eq!(group(101), None);
        assert_eq!(
            (group(38), group(39), group(88), group(89)),
            (Some(0), Some(1), Some(5), Some(6))
        );
        let controls = Controls::default();
        let mut panel = Panel::new(48000., controls);
        let mut common = [0.; 7];
        for note in 29..=100 {
            common[group(note).unwrap()] += winding_sign(note);
        }
        assert_eq!(common, [0.; 7]);
        assert_eq!(panel.tick(common).speaker_v, 0.);
        let mut reference = Panel::new(48000., controls);
        let mut original = 0.;
        let mut soft = 0.;
        for i in 0..60000 {
            if i == 16000 {
                panel.set_controls(Controls {
                    soft_pedal: true,
                    ..controls
                });
            }
            let mut input = [0.; 7];
            input[3] = (std::f64::consts::TAU * 350. * i as f64 / 48000.).sin();
            let expected = reference.tick(input);
            let actual = panel.tick(input);
            if i < 16000 {
                assert_eq!(actual.auxiliary_v, expected.auxiliary_v);
            }
            if i > 40000 {
                original += expected.auxiliary_v.powi(2);
                soft += actual.auxiliary_v.powi(2);
            }
        }
        assert!(soft > 0.02 * original && soft < 0.35 * original);
    }
}
