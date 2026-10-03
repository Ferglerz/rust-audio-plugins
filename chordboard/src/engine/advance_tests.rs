use super::*;
use nih_plug::prelude::NoteEvent;

fn engine(mode: u8, pattern: u8) -> Engine {
    let mut engine = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    engine.configure(
        Config {
            mode,
            arp_pattern: pattern,
            strum_sync: false,
            strum_ms: 13.25,
            length_ms: 19.0,
            swing: 0.43,
            humanize: 0.8,
            octaves: 3,
            ..Config::default()
        },
        &mut |_| {},
    );
    engine.midi_note(true, 1, 60, 0.8, &mut |_| {});
    engine
}

fn assert_state(actual: &Engine, reference: &Engine) {
    assert_eq!(actual.now, reference.now);
    assert_eq!(actual.snapshot(), reference.snapshot());
    assert_eq!(actual.config, reference.config);
    assert_eq!(actual.host_config, reference.host_config);
    assert_eq!(actual.needs_setup, reference.needs_setup);
    assert_eq!(
        actual
            .voices
            .map(|v| v.map(|v| (v.note, v.channel, v.started, v.off, v.layer))),
        reference
            .voices
            .map(|v| v.map(|v| (v.note, v.channel, v.started, v.off, v.layer)))
    );
    assert_eq!(
        actual
            .scheduled
            .map(|e| e.map(|e| (e.at, e.note, e.velocity, e.duration, e.gate_step))),
        reference
            .scheduled
            .map(|e| e.map(|e| (e.at, e.note, e.velocity, e.duration, e.gate_step)))
    );
    assert_eq!(
        (
            actual.arp_next,
            actual.arp_step,
            actual.arp_cycle_step,
            actual.arp_octaves,
            actual.arp_pattern,
            actual.arp_fraction,
            actual.arp_pair_swing,
            actual.arp_pair_rate,
            actual.rng
        ),
        (
            reference.arp_next,
            reference.arp_step,
            reference.arp_cycle_step,
            reference.arp_octaves,
            reference.arp_pattern,
            reference.arp_fraction,
            reference.arp_pair_swing,
            reference.arp_pair_rate,
            reference.rng
        ),
    );
}

fn compare_span(actual: &mut Engine, reference: &mut Engine, samples: usize) -> Vec<(usize, Out)> {
    let mut expected = Vec::new();
    for offset in 0..samples {
        reference.tick(&mut |event| expected.push((offset, event)));
    }
    let mut observed = Vec::new();
    actual.advance(samples, &mut |offset, event| observed.push((offset, event)));
    assert_eq!(observed, expected);
    assert_state(actual, reference);
    observed
}

#[test]
fn idle_and_zero_spans_match_tick_without_advancing_on_zero() {
    let mut actual = Engine::default();
    let mut reference = Engine::default();
    for samples in [0, 1, 0, 7, 128, 8192] {
        assert!(compare_span(&mut actual, &mut reference, samples).is_empty());
    }
    let mut actual = engine(ARP, 6);
    let mut reference = engine(ARP, 6);
    assert!(compare_span(&mut actual, &mut reference, 0).is_empty());
}

#[test]
fn arp_clock_rng_swing_and_fraction_match_in_every_performance_mode() {
    for mode in [CHORD, AUTO, MANUAL, ARP] {
        for pattern in 0..=6 {
            let mut actual = engine(mode, pattern);
            let mut reference = engine(mode, pattern);
            for samples in [1, 0, 3, 64, 17, 511, 1000] {
                compare_span(&mut actual, &mut reference, samples);
            }
            assert!(actual.arp_step > 0);
            assert_ne!(actual.rng, Engine::default().rng);
        }
    }
}

#[test]
fn overdue_simultaneous_slots_and_voice_stealing_keep_tick_order() {
    fn fixture() -> Engine {
        let mut engine = Engine {
            now: 100,
            needs_setup: false,
            ..Engine::default()
        };
        for (index, off, layer) in [
            (0, 90, false),
            (1, 100, false),
            (2, 100, true),
            (3, u64::MAX, false),
        ] {
            engine.voices[index] = Some(Voice {
                note: 40 + index as u8,
                channel: 0,
                started: 0,
                off,
                layer,
            });
        }
        for (index, at, note, duration) in [
            (0, 100, 60, 0),
            (17, 90, 61, 1),
            (90, 100, 60, 1),
            (127, 103, 62, 2),
        ] {
            engine.scheduled[index] = Some(Scheduled {
                at,
                note,
                velocity: 0.75,
                duration,
                gate_step: None,
            });
        }
        engine
    }
    let mut actual = fixture();
    let mut reference = fixture();
    let events = compare_span(&mut actual, &mut reference, 1);
    let notes: Vec<_> = events
        .iter()
        .filter_map(|&(offset, event)| match event {
            Out::On(_, note, _) => Some((offset, true, note)),
            Out::Off(_, note, _) => Some((offset, false, note)),
            _ => None,
        })
        .collect();
    assert_eq!(
        notes,
        [
            (0, false, 40),
            (0, false, 41),
            (0, true, 60),
            (0, true, 61),
            (0, false, 60),
            (0, true, 60)
        ]
    );
    compare_span(&mut actual, &mut reference, 100);
    assert!(actual
        .voices
        .iter()
        .flatten()
        .any(|v| v.layer && v.note == 42));
    assert!(actual.voices.iter().flatten().any(|v| v.off == u64::MAX));
}

#[test]
fn full_scheduler_and_arp_share_the_original_capacity_and_slot_order() {
    fn fixture() -> Engine {
        let mut engine = engine(ARP, 6);
        engine.voices.fill(None);
        for (index, slot) in engine.scheduled.iter_mut().enumerate() {
            *slot = Some(Scheduled {
                at: 0,
                note: 36 + (index % 48) as u8,
                velocity: 0.8,
                duration: 3,
                gate_step: None,
            });
        }
        engine
    }
    let mut actual = fixture();
    let mut reference = fixture();
    let events = compare_span(&mut actual, &mut reference, 32);
    let first: Vec<_> = events
        .iter()
        .filter_map(|&(offset, event)| match event {
            Out::On(_, note, _) if offset == 0 => Some(note),
            _ => None,
        })
        .collect();
    assert_eq!(
        first,
        (0..128).map(|i| 36 + (i % 48) as u8).collect::<Vec<_>>()
    );
}

#[test]
fn new_modes_patterns_tempo_and_transport_recompute_deadlines_between_spans() {
    let mut actual = engine(MANUAL, 6);
    let mut reference = engine(MANUAL, 6);
    for (index, mode) in [MANUAL, ARP, AUTO, CHORD, ARP].into_iter().enumerate() {
        compare_span(&mut actual, &mut reference, 137);
        let config = Config {
            mode,
            arp_pattern: index as u8,
            octaves: 1 + index as u8 % 4,
            swing: -0.25,
            rate: 0.03125,
            strum_sync: true,
            ..actual.host_config
        };
        let mut observed = Vec::new();
        let mut expected = Vec::new();
        actual.configure(config, &mut |event| observed.push(event));
        reference.configure(config, &mut |event| expected.push(event));
        actual.transport(
            index % 2 == 0,
            97.5 + index as f64,
            index == 4,
            &mut |event| observed.push(event),
        );
        reference.transport(
            index % 2 == 0,
            97.5 + index as f64,
            index == 4,
            &mut |event| expected.push(event),
        );
        assert_eq!(observed, expected);
        compare_span(&mut actual, &mut reference, 211);
    }
}

#[test]
fn clock_wrap_and_saturated_note_deadlines_match_reference_tick() {
    fn fixture() -> Engine {
        let mut engine = Engine {
            now: u64::MAX - 4,
            needs_setup: false,
            ..Engine::default()
        };
        engine.voices[0] = Some(Voice {
            note: 60,
            channel: 0,
            started: 0,
            off: u64::MAX,
            layer: false,
        });
        engine.voices[1] = Some(Voice {
            note: 64,
            channel: 0,
            started: 0,
            off: 0,
            layer: true,
        });
        engine.scheduled[90] = Some(Scheduled {
            at: u64::MAX,
            note: 67,
            velocity: 0.8,
            duration: 2,
            gate_step: None,
        });
        engine
    }
    let mut actual = fixture();
    let mut reference = fixture();
    let events = compare_span(&mut actual, &mut reference, 11);
    assert!(events.contains(&(4, Out::Off(0, 60, 0.0))));
    assert_eq!(actual.now, 6);
    assert!(actual
        .voices
        .iter()
        .flatten()
        .any(|v| v.note == 67 && v.off == u64::MAX));
    let mut actual = Engine {
        now: u64::MAX,
        ..Engine::default()
    };
    let mut reference = Engine {
        now: u64::MAX,
        ..Engine::default()
    };
    compare_span(&mut actual, &mut reference, 0);
    compare_span(&mut actual, &mut reference, 2);
}

#[test]
fn deterministic_sparse_schedules_match_across_block_partitions() {
    fn fixture(seed: u32) -> Engine {
        let mut engine = engine((seed % 4) as u8, (seed % 7) as u8);
        let mut random = seed.wrapping_add(1);
        for index in (0..128).step_by(7) {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            engine.scheduled[index] = Some(Scheduled {
                at: (random % 400) as u64,
                note: 36 + (random % 60) as u8,
                velocity: 0.7,
                duration: (random % 29) as u64,
                gate_step: Some(13),
            });
        }
        engine
    }
    for seed in 0..32 {
        let mut actual = fixture(seed);
        let mut reference = fixture(seed);
        for samples in [0, 1, 127, 3, 0, 256, 513] {
            compare_span(&mut actual, &mut reference, samples);
        }
    }
}

fn note(timing: u32, on: bool, note: u8) -> NoteEvent<()> {
    if on {
        NoteEvent::NoteOn {
            timing,
            voice_id: None,
            channel: 1,
            note,
            velocity: 0.8,
        }
    } else {
        NoteEvent::NoteOff {
            timing,
            voice_id: None,
            channel: 1,
            note,
            velocity: 0.3,
        }
    }
}

fn reference_inputs(
    plugin: &mut crate::Chordboard,
    count: usize,
    inputs: &[NoteEvent<()>],
) -> Vec<(u32, Out)> {
    let mut inputs = inputs.iter().copied();
    let mut next = inputs.next();
    let mut emitted = Vec::new();
    for sample in 0..count.max(1) {
        while next.is_some_and(|event| event.timing() <= sample as u32) {
            if let Some(event) = next.take() {
                plugin.input(event, &mut |event| emitted.push((sample as u32, event)));
            }
            next = inputs.next();
        }
        if count > 0 {
            plugin
                .engine
                .tick(&mut |event| emitted.push((sample as u32, event)));
        }
    }
    while let Some(event) = next.take() {
        plugin.input(event, &mut |event| {
            emitted.push((count.saturating_sub(1) as u32, event))
        });
        next = inputs.next();
    }
    emitted
}

#[test]
fn midi_spans_preserve_zero_buffers_ties_end_boundaries_and_trailing_order() {
    for count in [0, 1, 8, 64] {
        for mode in [CHORD, AUTO, MANUAL, ARP] {
            let fixture = || crate::Chordboard {
                engine: engine(mode, 6),
                ..crate::Chordboard::default()
            };
            let mut actual = fixture();
            let mut reference = fixture();
            let inputs = [
                note(0, false, 60),
                note(0, true, 62),
                note(3, true, 66),
                note(3, false, 66),
                note(count as u32, false, 62),
                note(count as u32 + 20, true, 60),
                note(0, false, 60),
            ];
            let expected = reference_inputs(&mut reference, count, &inputs);
            let mut observed = Vec::new();
            let mut input = inputs.into_iter();
            actual.process_events(count, || input.next(), &mut |offset, event| {
                observed.push((offset, event))
            });
            assert_eq!(observed, expected);
            assert!(observed
                .iter()
                .all(|(offset, _)| *offset <= count.saturating_sub(1) as u32));
            assert_state(&actual.engine, &reference.engine);
            compare_span(&mut actual.engine, &mut reference.engine, 100);
        }
    }
}

#[test]
fn incoming_midi_release_cancels_a_strike_due_at_the_same_sample() {
    let fixture = || {
        let mut plugin = crate::Chordboard {
            engine: engine(AUTO, 0),
            ..crate::Chordboard::default()
        };
        plugin.engine.voices.fill(None);
        plugin.engine.scheduled.fill(None);
        plugin.engine.scheduled[127] = Some(Scheduled {
            at: 7,
            note: 90,
            velocity: 0.8,
            duration: 5,
            gate_step: None,
        });
        plugin
    };
    let mut actual = fixture();
    let mut reference = fixture();
    let inputs = [note(7, false, 60)];
    let expected = reference_inputs(&mut reference, 16, &inputs);
    let mut observed = Vec::new();
    let mut inputs = inputs.into_iter();
    actual.process_events(16, || inputs.next(), &mut |offset, event| {
        observed.push((offset, event))
    });
    assert_eq!(observed, expected);
    assert!(!observed
        .iter()
        .any(|(_, event)| matches!(event, Out::On(_, 90, _))));
    assert_state(&actual.engine, &reference.engine);
}

#[test]
#[ignore = "timing benchmark; run in release mode with --ignored --nocapture"]
fn deadline_advance_benchmark() {
    use std::{hint::black_box, time::Instant};
    for mode in [None, Some(AUTO), Some(ARP)] {
        let fixture = || {
            mode.map_or_else(Engine::default, |mode| {
                let mut engine = engine(mode, 6);
                engine.sample_rate = 48_000.0;
                engine
            })
        };
        let mut reference = fixture();
        let mut actual = fixture();
        let mut expected = Vec::new();
        let mut observed = Vec::new();
        let samples = 480_000;
        let started = Instant::now();
        for offset in 0..samples {
            reference.tick(&mut |event| expected.push((offset, black_box(event))));
        }
        let tick_time = started.elapsed();
        let started = Instant::now();
        for block in (0..samples).step_by(512) {
            actual.advance((samples - block).min(512), &mut |offset, event| {
                observed.push((block + offset, black_box(event)))
            });
        }
        let advance_time = started.elapsed();
        assert_eq!(observed, expected);
        assert_state(&actual, &reference);
        eprintln!(
            "mode={mode:?}: tick={tick_time:?}, advance={advance_time:?}, outputs={}",
            observed.len()
        );
    }
}
