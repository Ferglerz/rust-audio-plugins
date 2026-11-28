use storytone::{
    amp::{Amplifier, Controls},
    engine::{plate, Engine, HIGH, LOW, PLATES},
};

#[test]
fn plate_allocation_and_signed_voicing_reach_the_selected_primary() {
    let mut e = Engine::new(48000.);
    let mut count = [0; 8];
    for n in LOW..=HIGH {
        let p = plate(n).unwrap();
        count[p] += 1;
        e.reset();
        e.note_on(n, 0.7);
        let mut peak = 0_f64;
        for _ in 0..900 {
            let taps = e.next_outputs();
            peak = peak.max(taps.plate_emf_v[p].abs());
            assert_eq!(taps.plate_emf_v[p], taps.pickups_v[p / 4]);
            assert_eq!(taps.pickups_v[1 - p / 4], 0.);
            assert!(taps.amplifier.speaker_v.is_finite());
        }
        assert!(peak > 1e-9, "silent note {n}");
    }
    assert_eq!(count, PLATES);
    assert_eq!(plate(LOW - 1), None);
    assert_eq!(plate(HIGH + 1), None);
    let mut render = |sign| {
        e.reset();
        e.set_voicing(63, sign);
        e.note_on(63, 0.7);
        (0..480)
            .map(|_| e.next_outputs().pickups_v[0])
            .collect::<Vec<_>>()
    };
    let forward = render(1.);
    let reversed = render(-1.);
    assert!(forward.into_iter().zip(reversed).all(|(a, b)| a == -b));
}

#[test]
fn sustain_soft_and_electrical_swell_operate_at_different_points() {
    let mut e = Engine::new(44100.);
    let mut trial = |soft, sustain, swell| {
        e.reset();
        e.set_controls(Controls::default());
        e.set_soft(soft);
        e.set_sustain(sustain);
        e.note_on(60, 0.65);
        let mut energy = [0.; 3];
        for i in 0..10000 {
            if i == 4000 {
                e.note_off(60);
                e.set_controls(Controls {
                    swell,
                    ..Controls::default()
                });
            }
            let tap = e.next_outputs();
            if i < 3500 {
                energy[0] += tap.pickups_v[0].powi(2);
            }
            if i > 8000 {
                energy[1] += tap.pickups_v[0].powi(2);
                energy[2] += tap.amplifier.auxiliary_v.powi(2);
            }
        }
        energy
    };
    let normal = trial(false, false, 0.);
    let soft = trial(true, false, 0.);
    let sustain = trial(false, true, 0.);
    let swell = trial(false, true, 1.);
    assert!(soft[0] < 0.8 * normal[0]);
    assert!(sustain[1] > 100. * normal[1]);
    assert_eq!(swell[1], sustain[1]);
    assert!(swell[2] > 1.5 * sustain[2]);
}

#[test]
fn held_keys_restrikes_rate_changes_and_chokes_are_finite() {
    for rate in [44100., 48000., 96000.] {
        let mut e = Engine::new(rate);
        e.note_on(18, 1.);
        e.note_on(120, 1.);
        e.note_on(75, f32::NAN);
        assert!(!e.sounding());
        e.note_on(64, 0.6);
        e.audition(64, 0.7);
        e.note_off(64);
        assert_eq!(e.held_notes(), 1u128 << 64);
        e.note_on(64, 0.7);
        e.audition_off();
        assert_eq!(e.held_notes(), 1u128 << 64);
        let mut b = [0_f32; 256];
        e.render(&mut b);
        e.note_on(64, 1.);
        e.render(&mut b);
        assert!(b.iter().all(|x| x.is_finite()) && b.iter().any(|x| x.abs() > 1e-5));
        e.choke(64);
        assert_eq!(e.held_notes(), 0);
        for _ in 0..400 {
            e.render(&mut b);
            if !e.sounding() {
                break;
            }
        }
        assert!(!e.sounding());
        e.reset();
        e.render(&mut b);
        assert_eq!(b, [0.; 256]);
    }
}

#[test]
fn amp_observations_follow_the_aux_divider_and_remain_bounded_under_overdrive() {
    let mut a = Amplifier::new(48000., Controls::default());
    let mut reference = Amplifier::new(48000., Controls::default());
    let mut error = 0_f64;
    for i in 0..2400 {
        let x = [1e-8 * (i as f64 * 0.13).sin(), 0.];
        let y = a.tick(x);
        error = error.max((y.speaker_v - reference.linear(x).speaker_v).abs());
    }
    assert!(error < 1e-8);
    let mut peak = 0_f64;
    for (j, manual) in [0., 0.5, 1.].into_iter().enumerate() {
        for (k, swell) in [0., 0.5, 1.].into_iter().enumerate() {
            a.set_controls(Controls {
                optimum: 1.,
                manual,
                swell,
            });
            for i in 0..2400 {
                let x = 0.04 * (i as f64 * 0.09).sin();
                let y = a.tick([x, if (j + k) % 2 == 0 { 0. } else { x }]);
                assert!(y.speaker_v.is_finite() && y.speaker_v.abs() < 150.);
                peak = peak.max(y.speaker_v.abs());
                assert_eq!(y.auxiliary_v, y.speaker_v * 220. / 4120.);
            }
        }
    }
    assert!(peak > 20.);
}

#[test]
fn extended_pitch_and_gap_act_on_physical_strings_without_changing_native_ownership() {
    let mut e = Engine::new(48000.);
    e.note_on(0, 0.75);
    e.note_on(127, 0.75);
    assert_eq!(e.held_notes(), 0);
    e.set_extended(true);
    for note in [0u8, 127] {
        e.reset();
        e.note_on(note, 0.65);
        let mut bins = [(0_f64, 0_f64); 3];
        let f = 440. * 2f64.powf((f64::from(note) - 69.) / 12.);
        for i in 0..48000 {
            let s = e.next_outputs().pickups_v[usize::from(note > 64)];
            let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / 48000.).cos();
            for ((a, b), ratio) in bins.iter_mut().zip([0.9, 1., 1.1]) {
                let (sin, cos) = (std::f64::consts::TAU * f * ratio * i as f64 / 48000.).sin_cos();
                *a += s * w * sin;
                *b += s * w * cos;
            }
        }
        let levels = bins.map(|(a, b)| a.hypot(b));
        assert!(
            levels[1] > levels[0] && levels[1] > levels[2],
            "note {note} {levels:?}"
        );
    }
    e.audition(0, 0.7);
    e.note_on(60, 0.7);
    e.set_sustain(true);
    e.set_extended(false);
    assert_eq!(e.held_notes(), 1u128 << 60);
    e.note_on(127, 1.);
    assert_eq!(e.held_notes(), 1u128 << 60);
    let mut ring = |offset| {
        e.reset();
        e.set_pickup_gap_mm(offset);
        e.note_on(50, 0.3);
        (0..4000)
            .map(|_| e.next_outputs().pickups_v[0])
            .collect::<Vec<_>>()
    };
    let native = ring(0.);
    let near = ring(-0.25);
    let far = ring(2.);
    let restored = ring(0.);
    assert_eq!(native, restored);
    let energy = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>();
    assert!(energy(&near) > energy(&native) && energy(&far) < energy(&native));
}
