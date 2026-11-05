use super::*;

fn context() -> Context {
    Context {
        now: 0,
        beat: 0.0,
        sample_rate: 48_000.0,
        tempo: 120.0,
        key: 0,
        scale: 0,
        chord: Notes {
            values: [60, 64, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            len: 3,
        },
        mpe: false,
        upper: false,
        members: 15,
        bend_range: 48.0,
        master_range: 2.0,
        output_channel: 0,
        bass_channel: 1,
        melody_channel: 2,
        aligned: false,
        delay_map: std::array::from_fn(|i| i as u8),
    }
}
fn input(token: u64, note: u8, source: u8) -> Input {
    Input {
        token,
        source,
        channel: 0,
        note,
        velocity: 1.0,
        expression: Expression::default(),
        off: None,
        retrigger: false,
    }
}
fn ons(events: &[Out]) -> Vec<(u8, u8)> {
    events
        .iter()
        .filter_map(|e| match *e {
            Out::On(c, n, _) => Some((c, n)),
            _ => None,
        })
        .collect()
}
fn offs(events: &[Out]) -> Vec<(u8, u8)> {
    events
        .iter()
        .filter_map(|e| match *e {
            Out::Off(c, n, _) => Some((c, n)),
            _ => None,
        })
        .collect()
}
fn at(mut ctx: Context, beat: f64) -> Context {
    ctx.beat = beat;
    ctx.now = (beat * ctx.samples_per_beat()).round() as u64;
    ctx
}
#[test]
fn fx_harmony_pitch_modes_ties_and_boundaries() {
    let ctx = context();
    assert_eq!(harmony_pitch(60, 2, HarmonizerMode::Scale, ctx), Some(64));
    assert_eq!(harmony_pitch(61, 0, HarmonizerMode::Scale, ctx), Some(60));
    assert_eq!(harmony_pitch(60, -1, HarmonizerMode::Scale, ctx), Some(59));
    assert_eq!(harmony_pitch(60, 2, HarmonizerMode::Chord, ctx), Some(67));
    assert_eq!(harmony_pitch(62, 0, HarmonizerMode::Chord, ctx), Some(60));
    assert_eq!(harmony_pitch(0, -1, HarmonizerMode::Scale, ctx), None);
    assert_eq!(harmony_pitch(127, 1, HarmonizerMode::Fixed, ctx), None);
    assert_eq!(
        harmony_pitch(
            60,
            1,
            HarmonizerMode::Chord,
            Context {
                chord: Notes::default(),
                ..ctx
            }
        ),
        None
    );
}
#[test]
fn fx_harmonizer_sources_release_and_live_expression() {
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    let mut runtime = Runtime::default();
    let ctx = Context {
        mpe: true,
        ..context()
    };
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.capture(input(2, 48, midi_fx::source::BASS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    let notes = ons(&events);
    assert_eq!(notes.len(), 4);
    assert_eq!(notes.iter().filter(|(_, n)| *n == 48).count(), 1);
    assert!(notes.iter().any(|(_, n)| *n == 64));
    assert!(notes.iter().any(|(_, n)| *n == 67));
    events.clear();
    runtime.expression(
        1,
        Expression {
            pressure: 0.7,
            timbre: 0.9,
            bend: 0.6,
        },
    );
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Out::Pressure(_,v) if *v == 0.7))
            .count(),
        3
    );
    events.clear();
    runtime.release(1, ctx, &mut |e| events.push(e));
    assert_eq!(offs(&events).len(), 3);
    assert_eq!(
        runtime
            .owned_notes()
            .iter()
            .flatten()
            .filter(|&&v| v)
            .count(),
        1
    );
}
#[test]
fn fx_combined_source_mask_matches_once_and_duplicate_harmony_is_suppressed() {
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    state.harmonizer.sources = midi_fx::source::BASS | midi_fx::source::CHORDS;
    state.harmonizer.voices[1] = state.harmonizer.voices[0];
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    let ctx = context();
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    runtime.capture(input(1, 60, state.harmonizer.sources), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(0, 60), (0, 64)]);
    runtime.release(1, ctx, &mut |e| events.push(e));
    assert_eq!(offs(&events), [(0, 64), (0, 60)]);
}
#[test]
fn fx_gate_pulses_release_and_tempo_deadlines() {
    let mut state = MidiFxState::default();
    state.rhythm_gate.enabled = true;
    state.rhythm_gate.pattern.fill(1.0);
    let ctx = context();
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    runtime.capture(input(1, 60, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(0, 60)]);
    assert_eq!(runtime.deadline(ctx), Some(3000));
    assert_eq!(runtime.deadline(Context { tempo: 60.0, ..ctx }), Some(6000));
    runtime.flush(at(ctx, 0.125), &mut |e| events.push(e));
    assert_eq!(offs(&events), [(0, 60)]);
    runtime.capture(input(2, 64, midi_fx::source::CHORDS), at(ctx, 0.15));
    events.clear();
    runtime.flush(at(ctx, 0.15), &mut |e| events.push(e));
    assert!(ons(&events).is_empty());
    runtime.flush(at(ctx, 0.25), &mut |e| events.push(e));
    assert_eq!(ons(&events), [(0, 60), (0, 64)]);
    runtime.release(1, at(ctx, 0.3), &mut |e| events.push(e));
    runtime.release(2, at(ctx, 0.3), &mut |e| events.push(e));
    assert_eq!(offs(&events).len(), 2);
    assert_eq!(runtime.deadline(at(ctx, 0.3)), None);
}
#[test]
fn fx_splitter_ranks_simultaneous_notes_and_keeps_assignments() {
    let mut state = MidiFxState::default();
    state.voice_splitter.enabled = true;
    let ctx = context();
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    for (token, note) in [(1, 67), (2, 60), (3, 64)] {
        runtime.capture(input(token, note, midi_fx::source::CHORDS), ctx);
    }
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(3, 67), (1, 60), (2, 64)]);
    events.clear();
    runtime.release(2, ctx, &mut |e| events.push(e));
    runtime.capture(input(4, 72, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(offs(&events), [(1, 60)]);
    assert_eq!(ons(&events), [(3, 72)]);
    assert!(runtime.owned_notes()[2][64]);
    assert!(runtime.owned_notes()[3][67]);
}
#[test]
fn fx_mpe_pools_expression_and_exhaustion_are_isolated() {
    let mut state = MidiFxState::default();
    state.voice_splitter.enabled = true;
    state.voice_splitter.pools = [1, 1, 1];
    let ctx = Context {
        mpe: true,
        members: 4,
        ..context()
    };
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    for (token, note) in [(1, 60), (2, 64), (3, 67)] {
        runtime.capture(input(token, note, midi_fx::source::CHORDS), ctx);
    }
    runtime.capture(input(4, 48, midi_fx::source::BASS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(1, 60), (2, 64), (3, 67), (4, 48)]);
    for (channel, note) in ons(&events) {
        let on = events
            .iter()
            .position(|e| *e == Out::On(channel, note, 1.0))
            .unwrap();
        assert!(events[..on].contains(&Out::Bend(channel, 0.5)));
        assert!(events[..on].contains(&Out::Pressure(channel, 0.0)));
    }
    events.clear();
    runtime.capture(input(5, 72, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert!(ons(&events).is_empty());
    assert!(offs(&events).is_empty());
    runtime.release(3, ctx, &mut |e| events.push(e));
    runtime.capture(input(6, 74, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(3, 74)]);
}
#[test]
fn fx_harmony_and_delay_order_preserve_source_and_change_echo_voice_count() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    state.harmonizer.mode = HarmonizerMode::Chord;
    state.harmonizer.voices[0].interval = 1;
    state.harmonizer.voices[1].enabled = false;
    state.delay.enabled = true;
    state.delay.sources = midi_fx::source::MELODY;
    state.delay.taps[0].feedback = 0.0;
    state.delay.taps[0].time = 0.5;
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    runtime.release(1, at(ctx, 0.25), &mut |e| events.push(e));
    events.clear();
    let silent_chord = Context {
        chord: Notes::default(),
        ..at(ctx, 0.5)
    };
    runtime.flush(silent_chord, &mut |e| events.push(e));
    assert_eq!(
        ons(&events).iter().map(|(_, n)| *n).collect::<Vec<_>>(),
        [60, 64]
    );
    runtime.flush(at(ctx, 0.75), &mut |e| events.push(e));
    assert_eq!(offs(&events).len(), 2);
    state.order = vec![
        MidiFxId::Delay,
        MidiFxId::Harmonizer,
        MidiFxId::RhythmGate,
        MidiFxId::VoiceSplitter,
    ];
    runtime.clear(ctx, &mut |_| {});
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(2, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    runtime.release(2, at(ctx, 0.25), &mut |_| {});
    events.clear();
    runtime.flush(silent_chord, &mut |e| events.push(e));
    assert_eq!(
        ons(&events).iter().map(|(_, n)| *n).collect::<Vec<_>>(),
        [60]
    );
}
#[test]
fn fx_bypass_rebuilds_tracked_roots_and_known_offs_expire() {
    let ctx = context();
    let mut state = MidiFxState::default();
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |_| {});
    let mut note = input(1, 60, midi_fx::source::MELODY);
    note.off = Some(6000);
    runtime.capture(note, ctx);
    state.harmonizer.enabled = true;
    runtime.configure(&state, ctx, &mut |e| events.push(e));
    assert_eq!(offs(&events), [(0, 60)]);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events).len(), 3);
    events.clear();
    state.harmonizer.enabled = false;
    runtime.configure(&state, at(ctx, 0.1), &mut |e| events.push(e));
    runtime.flush(at(ctx, 0.1), &mut |e| events.push(e));
    assert_eq!(offs(&events).len(), 3);
    assert_eq!(ons(&events), [(0, 60)]);
    runtime.configure(&state, at(ctx, 0.25), &mut |_| {});
    assert!(!runtime.has_root(1));
}
#[test]
fn fx_invalid_pool_configuration_bypasses_only_splitter() {
    let ctx = Context {
        mpe: true,
        members: 4,
        ..context()
    };
    let mut state = MidiFxState::default();
    state.voice_splitter.enabled = true;
    state.harmonizer.enabled = true;
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(1, 60), (2, 64), (3, 67)]);
}
#[test]
fn fx_panic_clears_pending_echoes_and_retains_configuration() {
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    state.delay.enabled = true;
    state.delay.sources = midi_fx::source::MELODY;
    let ctx = context();
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    let mut events = Vec::new();
    runtime.clear(ctx, &mut |e| events.push(e));
    assert_eq!(offs(&events).len(), 3);
    assert_eq!(runtime.deadline(ctx), None);
    assert!(runtime.active());
    runtime.capture(input(2, 62, midi_fx::source::MELODY), ctx);
    events.clear();
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events).len(), 3);
}

#[test]
fn fx_delay_before_gate_starts_pulse_on_echo_deadline() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.order = vec![
        MidiFxId::Delay,
        MidiFxId::RhythmGate,
        MidiFxId::Harmonizer,
        MidiFxId::VoiceSplitter,
    ];
    state.rhythm_gate.enabled = true;
    state.rhythm_gate.sources = midi_fx::source::MELODY;
    state.rhythm_gate.pattern.fill(1.0);
    state.delay.enabled = true;
    state.delay.sources = midi_fx::source::MELODY;
    state.delay.taps[0].time = 0.5;
    state.delay.taps[0].feedback = 0.0;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    runtime.release(1, at(ctx, 0.1), &mut |_| {});
    let mut events = Vec::new();
    runtime.flush(at(ctx, 0.5), &mut |e| events.push(e));
    assert_eq!(ons(&events).len(), 1);
    assert_eq!(ons(&events)[0].1, 60);
    assert!(runtime
        .deadline(at(ctx, 0.5))
        .is_some_and(|deadline| deadline > 12000));
}

#[test]
fn fx_splitter_before_harmony_ignores_downstream_notes_when_ranking_new_attacks() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.order = vec![
        MidiFxId::VoiceSplitter,
        MidiFxId::Harmonizer,
        MidiFxId::RhythmGate,
        MidiFxId::Delay,
    ];
    state.voice_splitter.enabled = true;
    state.harmonizer.enabled = true;
    state.harmonizer.sources = midi_fx::source::CHORDS;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |_| {});
    let mut events = Vec::new();
    runtime.capture(input(2, 62, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert!(ons(&events).contains(&(3, 62)));
    assert!(!ons(&events).iter().any(|(channel, _)| *channel != 3));
}

#[test]
fn fx_explicit_delay_channels_cannot_escape_mpe_destination_pool() {
    let ctx = Context {
        mpe: true,
        members: 7,
        ..context()
    };
    let mut state = MidiFxState::default();
    state.voice_splitter.enabled = true;
    state.voice_splitter.pools = [1, 1, 2];
    state.delay.enabled = true;
    state.delay.sources = midi_fx::source::CHORDS;
    state.delay.taps[0].time = 0.5;
    state.delay.taps[0].feedback = 0.0;
    state.delay.taps[0].channel = 7;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |_| {});
    let mut events = Vec::new();
    runtime.flush(at(ctx, 0.5), &mut |e| events.push(e));
    assert!(ons(&events).is_empty());
    assert!(runtime.owned_notes()[3][60]);
    runtime.clear(ctx, &mut |_| {});
    state.delay.taps[0].channel = 5;
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(2, 60, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |_| {});
    runtime.flush(at(ctx, 0.5), &mut |e| events.push(e));
    assert_eq!(ons(&events), [(4, 60)]);
}

#[test]
fn fx_custom_controllers_use_actual_destinations_once() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    let mut events = Vec::new();
    runtime.controller(1, 11, 0.75, &mut |e| events.push(e));
    assert_eq!(events, [Out::Cc(0, 11, 0.75)]);
}

#[test]
fn fx_gate_timing_does_not_depend_on_host_block_size() {
    fn run(block: u64) -> Vec<(u64, Out)> {
        let mut state = MidiFxState::default();
        state.rhythm_gate.enabled = true;
        state.rhythm_gate.pattern.fill(1.0);
        let ctx = context();
        let mut runtime = Runtime::default();
        runtime.configure(&state, ctx, &mut |_| {});
        runtime.capture(input(1, 60, midi_fx::source::CHORDS), ctx);
        let mut now = 0;
        let mut events = Vec::new();
        while now < 24000 {
            let ctx = Context {
                now,
                beat: now as f64 / 24000.0,
                ..ctx
            };
            runtime.flush(ctx, &mut |event| events.push((now, event)));
            now = (now + block)
                .min(runtime.deadline(ctx).unwrap_or(24000))
                .min(24000);
        }
        events
    }
    assert_eq!(run(17), run(1024));
}

#[test]
fn fx_shared_bass_chord_root_ranks_with_chord_tones() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.voice_splitter.enabled = true;
    let mut runtime = Runtime::default();
    let mut events = Vec::new();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(
        input(1, 60, midi_fx::source::BASS | midi_fx::source::CHORDS),
        ctx,
    );
    runtime.capture(input(2, 64, midi_fx::source::CHORDS), ctx);
    runtime.capture(input(3, 67, midi_fx::source::CHORDS), ctx);
    runtime.flush(ctx, &mut |e| events.push(e));
    assert_eq!(ons(&events), [(1, 60), (2, 64), (3, 67)]);
}

#[test]
fn fx_delay_power_changes_preserve_dry_notes_and_gate_phase() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.rhythm_gate.enabled = true;
    state.rhythm_gate.pattern.fill(1.0);
    state.delay.enabled = true;
    state.delay.sources = midi_fx::source::CHORDS;
    state.delay.taps[0].time = 0.5;
    state.delay.taps[0].feedback = 0.0;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::CHORDS), ctx);
    for beat in [0.0, 0.125, 0.25, 0.375, 0.5] {
        runtime.flush(at(ctx, beat), &mut |_| {});
    }
    assert!(runtime.owned_notes()[0][60]);
    assert!(runtime.owned_notes().iter().skip(1).any(|notes| notes[60]));
    let phase = runtime.gate_clock.unwrap().next;
    let mut events = Vec::new();
    state.delay.enabled = false;
    assert!(!runtime.configure(&state, at(ctx, 0.55), &mut |e| events.push(e)));
    runtime.flush(at(ctx, 0.55), &mut |e| events.push(e));
    assert!(ons(&events).is_empty());
    assert!(!offs(&events).contains(&(0, 60)));
    assert!(runtime.owned_notes()[0][60]);
    assert_eq!(runtime.gate_clock.unwrap().next, phase);
    assert!(runtime.delay.deadline(13200, 0.55, 24000.0).is_none());
    state.delay.enabled = true;
    assert!(!runtime.configure(&state, at(ctx, 0.6), &mut |_| {}));
    assert!(runtime.delay.deadline(14400, 0.6, 24000.0).is_none());
}

#[test]
fn fx_controller_cache_reseeds_new_outputs_and_runtime_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Runtime>();
    let ctx = context();
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    let serial = runtime.output_serial();
    assert_eq!(runtime.root_len, 1);
    assert_eq!(runtime.node_len, 3);
    let mut events = Vec::new();
    runtime.controller(1, 11, 0.75, &mut |e| events.push(e));
    runtime.controller(1, 11, 0.75, &mut |e| events.push(e));
    assert_eq!(events, [Out::Cc(0, 11, 0.75)]);
    runtime.release(1, ctx, &mut |_| {});
    assert_eq!(runtime.node_len, 0);
    assert_eq!(runtime.root_len, 0);
    runtime.capture(input(2, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    assert!(runtime.output_serial() > serial);
    events.clear();
    runtime.controller(2, 11, 0.75, &mut |e| events.push(e));
    assert_eq!(events, [Out::Cc(0, 11, 0.75)]);
}

#[test]
fn fx_updated_source_lifetime_does_not_extend_gate_pulses() {
    let ctx = context();
    let mut state = MidiFxState::default();
    state.rhythm_gate.enabled = true;
    state.rhythm_gate.pattern.fill(1.0);
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    let mut note = input(1, 60, midi_fx::source::CHORDS);
    note.off = Some(1000);
    runtime.capture(note, ctx);
    runtime.flush(ctx, &mut |_| {});
    runtime.set_off(1, None);
    assert_eq!(runtime.deadline(ctx), Some(3000));
    let mut events = Vec::new();
    runtime.flush(at(ctx, 0.125), &mut |e| events.push(e));
    assert_eq!(offs(&events), [(0, 60)]);
    assert!(runtime.has_root(1));
    runtime.flush(at(ctx, 0.25), &mut |e| events.push(e));
    assert_eq!(ons(&events), [(0, 60)]);
}

#[test]
fn fx_explicit_restrike_reuses_pitch_ownership_in_standard_and_mpe() {
    for mpe in [false, true] {
        let ctx = Context { mpe, ..context() };
        let mut state = MidiFxState::default();
        state.harmonizer.enabled = true;
        state.harmonizer.sources = 0;
        let mut runtime = Runtime::default();
        runtime.configure(&state, ctx, &mut |_| {});
        runtime.capture(input(1, 60, midi_fx::source::CHORDS), ctx);
        runtime.flush(ctx, &mut |_| {});
        let mut strike = input(2, 60, midi_fx::source::STRUM);
        strike.retrigger = true;
        runtime.capture(strike, ctx);
        let mut events = Vec::new();
        runtime.flush(ctx, &mut |e| events.push(e));
        let channel = if mpe { 1 } else { 0 };
        assert_eq!(offs(&events), [(channel, 60)]);
        assert_eq!(ons(&events), [(channel, 60)]);
        assert_eq!(
            runtime
                .owned_notes()
                .iter()
                .flatten()
                .filter(|&&v| v)
                .count(),
            1
        );
        events.clear();
        runtime.release(2, ctx, &mut |e| events.push(e));
        assert!(offs(&events).is_empty());
        runtime.release(1, ctx, &mut |e| events.push(e));
        assert_eq!(offs(&events), [(channel, 60)]);
    }
}

#[test]
fn fx_mpe_dry_attacks_reclaim_echo_channels_before_dropping() {
    let ctx = Context {
        mpe: true,
        members: 2,
        ..context()
    };
    let mut state = MidiFxState::default();
    state.harmonizer.enabled = true;
    state.harmonizer.sources = 0;
    state.delay.enabled = true;
    state.delay.sources = midi_fx::source::MELODY;
    state.delay.taps[0].time = 0.5;
    state.delay.taps[0].feedback = 0.0;
    let mut runtime = Runtime::default();
    runtime.configure(&state, ctx, &mut |_| {});
    runtime.capture(input(1, 60, midi_fx::source::MELODY), ctx);
    runtime.flush(ctx, &mut |_| {});
    runtime.flush(at(ctx, 0.5), &mut |_| {});
    assert!(runtime.owned_notes()[1][60]);
    assert!(runtime.owned_notes()[2][60]);
    runtime.capture(input(2, 62, midi_fx::source::MELODY), at(ctx, 0.5));
    let mut events = Vec::new();
    runtime.flush(at(ctx, 0.5), &mut |e| events.push(e));
    assert_eq!(offs(&events), [(2, 60)]);
    assert_eq!(ons(&events), [(2, 62)]);
    assert!(runtime.owned_notes()[1][60]);
}
