use super::*;

fn on(timing: u32, channel: u8, note: u8) -> NoteEvent<()> {
    NoteEvent::NoteOn {
        timing,
        voice_id: None,
        channel,
        note,
        velocity: 0.8,
    }
}

fn off(timing: u32, channel: u8, note: u8) -> NoteEvent<()> {
    NoteEvent::NoteOff {
        timing,
        voice_id: None,
        channel,
        note,
        velocity: 0.2,
    }
}

#[test]
fn dry_midi_preserves_notes_controllers_pitch_and_timestamps() {
    for enabled in [false, true] {
        let mut tape = MidiTape::new();
        let mut output = Vec::new();
        tape.begin_block(enabled, &mut |e| output.push(e));
        let input = [
            on(3, 2, 60),
            NoteEvent::MidiCC {
                timing: 7,
                channel: 2,
                cc: 74,
                value: 0.3,
            },
            NoteEvent::MidiPitchBend {
                timing: 8,
                channel: 2,
                value: 0.7,
            },
            off(15, 2, 60),
        ];
        for i in 0..20 {
            for event in input.iter().filter(|e| e.timing() == i) {
                tape.push(*event, &mut |e| output.push(e));
            }
            tape.sample(i, 1.0, false, true, &mut |e| output.push(e));
        }
        assert_eq!(output, input);
    }
}

#[test]
fn half_speed_delays_events_across_blocks_and_bends_an_octave() {
    let mut tape = MidiTape::new();
    let mut output = Vec::new();
    for block in 0..3 {
        tape.begin_block(true, &mut |_| {});
        for i in 0..8 {
            let at = block * 8 + i;
            if at == 0 {
                tape.push(on(i, 3, 60), &mut |_| {});
            }
            if at == 5 {
                tape.push(on(i, 7, 64), &mut |_| {});
            }
            if at == 9 {
                tape.push(off(i, 3, 60), &mut |_| {});
            }
            tape.sample(i, 0.5, true, false, &mut |e| output.push((block, e)));
        }
    }
    assert!(
        output.contains(&(1, on(2, 7, 64))),
        "source sample 5 must play at sample 10"
    );
    assert!(
        output.contains(&(2, off(2, 3, 60))),
        "source sample 9 must release at sample 18"
    );
    for channel in [3, 7] {
        assert!(output.iter().any(
            |(_, e)| matches!(e, NoteEvent::MidiPitchBend { channel: c, value, .. }
            if *c == channel && (*value - 0.375).abs() < 1e-6)
        ));
        let note_pos = output
            .iter()
            .position(|(_, e)| matches!(e, NoteEvent::NoteOn { channel: c, .. } if *c == channel))
            .unwrap();
        assert!(output[..note_pos].iter().any(|(_, e)| matches!(e,
            NoteEvent::MidiCC { cc: 6, value, channel: c, .. } if *c == channel && (*value * 127.0 - 48.0).abs() < 1e-5)));
    }
}

#[test]
fn stopped_tape_holds_events_and_return_releases_in_order() {
    let mut tape = MidiTape::new();
    let mut output = Vec::new();
    tape.push(on(0, 0, 60), &mut |_| {});
    tape.sample(0, 0.0, true, false, &mut |e| output.push(e));
    for i in 1..20 {
        if i == 2 {
            tape.push(off(i, 0, 60), &mut |_| {});
        }
        if i == 3 {
            tape.push(on(i, 1, 64), &mut |_| {});
        }
        tape.sample(i, 0.0, true, false, &mut |e| output.push(e));
    }
    assert!(!output
        .iter()
        .any(|e| matches!(e, NoteEvent::NoteOff { .. })));
    output.clear();
    for i in 0..12 {
        tape.sample(i, 3.0, false, false, &mut |e| output.push(e));
    }
    let notes: Vec<_> = output
        .into_iter()
        .filter(|e| matches!(e, NoteEvent::NoteOn { .. } | NoteEvent::NoteOff { .. }))
        .collect();
    assert_eq!(notes, [off(1, 0, 60), on(1, 1, 64)]);
}

#[test]
fn disabling_or_resetting_releases_voices_clears_queue_and_restores_bend() {
    for reset in [false, true] {
        let mut tape = MidiTape::new();
        let mut output = Vec::new();
        tape.push(on(0, 4, 60), &mut |_| {});
        tape.sample(0, 0.5, true, false, &mut |_| {});
        tape.push(on(1, 4, 62), &mut |_| {});
        if reset {
            tape.reset();
        }
        tape.begin_block(reset, &mut |e| output.push(e));
        assert!(output.iter().any(|e| matches!(
            e,
            NoteEvent::NoteOff {
                channel: 4,
                note: 60,
                ..
            }
        )));
        assert!(output.contains(&NoteEvent::MidiPitchBend {
            timing: 0,
            channel: 4,
            value: 0.5
        }));
        assert!(output.iter().any(|e| matches!(e, NoteEvent::MidiCC { cc: 6, value, .. } if (*value * 127.0 - 2.0).abs() < 1e-6)));
        assert!(tape.queue.is_empty());
        output.clear();
        tape.sample(0, 1.0, false, true, &mut |e| output.push(e));
        assert!(output.is_empty(), "discarded delayed notes must not return");
    }
}

#[test]
fn incoming_bend_and_custom_range_are_combined_then_restored() {
    let mut tape = MidiTape::new();
    tape.begin_block(false, &mut |_| {});
    let mut passthrough = Vec::new();
    let mut incoming = Vec::new();
    for (cc, value) in [
        (101, 0),
        (100, 0),
        (6, 12),
        (38, 50),
        (101, 127),
        (100, 127),
    ] {
        let event = NoteEvent::MidiCC {
            timing: 0,
            channel: 2,
            cc,
            value: value as f32 / 127.0,
        };
        incoming.push(event);
        tape.push(event, &mut |e| passthrough.push(e));
    }
    assert_eq!(
        passthrough, incoming,
        "disabled MIDI must preserve RPN messages exactly"
    );
    tape.push(
        NoteEvent::MidiPitchBend {
            timing: 0,
            channel: 2,
            value: 0.75,
        },
        &mut |_| {},
    );
    tape.begin_block(true, &mut |_| {});
    let mut output = Vec::new();
    tape.sample(0, 0.5, true, false, &mut |e| output.push(e));
    let expected = 0.5 + (6.25 - 12.0) / 96.0;
    assert!(output.iter().any(
        |e| matches!(e, NoteEvent::MidiPitchBend { value, .. } if (*value - expected).abs() < 1e-6)
    ));
    output.clear();
    tape.sample(1, 1.0, false, true, &mut |e| output.push(e));
    assert!(output.contains(&NoteEvent::MidiPitchBend {
        timing: 1,
        channel: 2,
        value: 0.75
    }));
    for (cc, expected) in [(6, 12.0), (38, 50.0)] {
        assert!(output.iter().any(|e| matches!(e, NoteEvent::MidiCC { cc: c, value, .. } if *c == cc && (*value * 127.0 - expected).abs() < 1e-5)));
    }
}

#[test]
fn overflow_is_bounded_and_fails_to_passthrough_without_stuck_notes() {
    let mut tape = MidiTape::new();
    tape.push(on(0, 0, 60), &mut |_| {});
    tape.sample(0, 0.0, true, false, &mut |_| {});
    for _ in 0..CAPACITY {
        tape.push(on(1, 1, 61), &mut |_| {});
    }
    let capacity = tape.queue.capacity();
    let mut output = Vec::new();
    tape.push(on(2, 2, 62), &mut |e| output.push(e));
    assert!(tape.overflow);
    assert!(tape.queue.is_empty());
    assert_eq!(tape.queue.capacity(), capacity);
    assert!(output.iter().any(|e| matches!(
        e,
        NoteEvent::NoteOff {
            channel: 0,
            note: 60,
            ..
        }
    )));
    assert_eq!(output.last(), Some(&on(2, 2, 62)));
}

#[test]
fn retrigger_rejoins_live_tape_and_pitch_output_is_rate_limited() {
    let mut tape = MidiTape::new();
    tape.set_sample_rate(48_000.0);
    tape.push(on(0, 0, 60), &mut |_| {});
    let mut bends = 0;
    for i in 0..48_000 {
        tape.sample(i, 1.0 - i as f32 / 48_000.0, true, false, &mut |e| {
            if matches!(e, NoteEvent::MidiPitchBend { .. }) {
                bends += 1;
            }
        });
    }
    assert!(bends <= 251, "pitch output must stay bounded: {bends}");
    tape.sample(0, 0.2, false, false, &mut |_| {});
    tape.push(on(1, 1, 64), &mut |_| {});
    let mut output = Vec::new();
    tape.sample(1, 0.99, true, false, &mut |e| output.push(e));
    assert!(
        output.contains(&on(1, 1, 64)),
        "new brake must follow the audio head's live join"
    );
}
