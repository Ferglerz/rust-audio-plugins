//! Exercise real source routing and transport, beyond the standalone FX graph.
use super::*;
use crate::midi_fx::{source, MidiFxId};

fn engine(mpe: bool) -> Engine {
    let mut e = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    e.arp.data.midi_fx.harmonizer.enabled = true;
    e.configure(
        Config {
            key_split: true,
            split_note: 60,
            mpe,
            arp_enabled: false,
            mode: MANUAL,
            ..Config::default()
        },
        &mut |_| {},
    );
    e
}
fn on(e: &mut Engine, note: u8) -> Vec<Out> {
    let mut events = Vec::new();
    e.midi_note(true, 1, note, 0.8, &mut |v| events.push(v));
    e.advance(1, &mut |_, v| events.push(v));
    events
}
fn notes(events: &[Out]) -> Vec<(u8, u8)> {
    events
        .iter()
        .filter_map(|e| {
            if let Out::On(c, n, _) = *e {
                Some((c, n))
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn fx_engine_melody_harmonizes_and_releases_every_owned_note() {
    let mut e = engine(false);
    let events = on(&mut e, 72);
    assert_eq!(notes(&events), vec![(1, 72), (1, 76), (1, 79)]);
    let mut off = Vec::new();
    e.midi_note(false, 1, 72, 0.0, &mut |v| off.push(v));
    for n in [72, 76, 79] {
        assert!(off.contains(&Out::Off(1, n, 0.0)));
    }
    assert!(!e.owned_output_notes().into_iter().flatten().any(|n| n));
}
#[test]
fn fx_engine_mpe_pool_expression_follows_input_across_opposite_zones() {
    let mut e = engine(true);
    e.arp.data.midi_fx.voice_splitter.enabled = true;
    e.arp.data.midi_fx.voice_splitter.sources = source::MELODY;
    e.configure(
        Config {
            upper: true,
            ..e.host_config
        },
        &mut |_| {},
    );
    let events = on(&mut e, 72);
    let assigned = notes(&events);
    assert_eq!(assigned.len(), 3);
    let masks = e
        .arp
        .data
        .midi_fx
        .voice_splitter
        .pool_ranges(15, true)
        .unwrap();
    for ((channel, _), pool) in assigned.iter().zip([0, 1, 2]) {
        assert_ne!(masks[pool] & (1 << channel), 0);
    }
    let mut events = Vec::new();
    e.pressure(1, None, 0.7, &mut |v| events.push(v));
    e.bend(1, 0.6, &mut |v| events.push(v));
    e.advance(1, &mut |_, v| events.push(v));
    for (channel, _) in assigned {
        assert!(events.contains(&Out::Pressure(channel, 0.7)));
        assert!(events
            .iter()
            .any(|v| matches!(*v, Out::Bend(c,b) if c==channel && (b-0.6).abs()<1e-5)));
    }
    e.panic(&mut |_| {});
    assert!(!e.owned_output_notes().into_iter().flatten().any(|n| n));
}
#[test]
fn fx_engine_mpe_master_sustain_holds_retriggers_and_releases_members() {
    let mut e = engine(true);
    on(&mut e, 72);
    e.control(0, 64, 1.0, &mut |_| {});
    e.midi_note(false, 1, 72, 0.0, &mut |_| {});
    assert!(e.owned_output_notes().into_iter().flatten().any(|n| n));
    assert_eq!(
        notes(&on(&mut e, 72)).len(),
        3,
        "repress under pedal must attack again"
    );
    e.midi_note(false, 1, 72, 0.0, &mut |_| {});
    e.control(0, 64, 0.0, &mut |_| {});
    assert!(!e.owned_output_notes().into_iter().flatten().any(|n| n));
}
#[test]
fn fx_engine_last_bypass_rebuilds_held_notes_without_mpe_channel_collisions() {
    let mut e = engine(true);
    on(&mut e, 72);
    on(&mut e, 76);
    e.arp.data.midi_fx.harmonizer.enabled = false;
    let mut events = Vec::new();
    e.configure(e.host_config, &mut |v| events.push(v));
    e.advance(1, &mut |_, v| events.push(v));
    let active = e.owned_output_notes();
    assert_eq!(active.iter().flatten().filter(|&&n| n).count(), 2);
    assert!(active
        .iter()
        .all(|notes| notes.iter().filter(|&&n| n).count() <= 1));
    e.midi_note(false, 1, 72, 0.0, &mut |_| {});
    e.midi_note(false, 1, 76, 0.0, &mut |_| {});
    assert!(!e.owned_output_notes().into_iter().flatten().any(|n| n));
}
#[test]
fn fx_engine_gate_is_block_size_independent_and_clears_on_stop() {
    fn setup() -> Engine {
        let mut e = engine(false);
        e.arp.data.midi_fx.harmonizer.enabled = false;
        e.arp.data.midi_fx.rhythm_gate.enabled = true;
        e.arp.data.midi_fx.rhythm_gate.sources = source::MELODY;
        e.arp.data.midi_fx.rhythm_gate.pattern.fill(1.0);
        e.configure(e.host_config, &mut |_| {});
        e.midi_note(true, 1, 72, 0.8, &mut |_| {});
        e
    }
    let mut a = setup();
    let mut b = setup();
    let mut expected = Vec::new();
    let mut actual = Vec::new();
    for i in 0..1000 {
        a.advance(1, &mut |_, v| expected.push((i, v)));
    }
    b.advance(1000, &mut |i, v| actual.push((i, v)));
    assert_eq!(expected, actual);
    assert_eq!(
        notes(&actual.iter().map(|(_, v)| *v).collect::<Vec<_>>()).len(),
        8
    );
    b.transport(true, 120.0, false, &mut |_| {});
    b.transport(false, 120.0, false, &mut |_| {});
    assert!(!b.owned_output_notes().into_iter().flatten().any(|n| n));
}
#[test]
fn fx_engine_reorder_preserves_source_filters_and_releases_old_outputs() {
    let mut e = engine(false);
    e.arp.data.midi_fx.rhythm_gate.enabled = true;
    e.arp.data.midi_fx.rhythm_gate.sources = source::BASS;
    on(&mut e, 72);
    let mut events = Vec::new();
    e.arp.data.midi_fx.order = vec![
        MidiFxId::RhythmGate,
        MidiFxId::Harmonizer,
        MidiFxId::VoiceSplitter,
        MidiFxId::Delay,
    ];
    e.configure(e.host_config, &mut |v| events.push(v));
    e.advance(1, &mut |_, v| events.push(v));
    assert_eq!(notes(&events).len(), 3);
    assert!(events.contains(&Out::Off(1, 72, 0.0)));
    e.panic(&mut |_| {});
    e.advance(2000, &mut |_, v| assert!(!matches!(v, Out::On(..))));
}

#[test]
fn fx_engine_promoting_a_timed_voice_keeps_the_held_layer_alive() {
    let mut e = engine(false);
    e.allocate_voice(60, 0.8, 10, false, &mut |_| {});
    e.advance(1, &mut |_, _| {});
    let index = e.voices.iter().position(Option::is_some).unwrap();
    e.voices[index].as_mut().unwrap().layer = true;
    e.voices[index].as_mut().unwrap().off = u64::MAX;
    e.advance(30, &mut |_, _| {});
    assert!(e.owned_output_notes().iter().any(|notes| notes[60]));
    e.end_voice(index, 0.0, &mut |_| {});
    assert!(!e.owned_output_notes().iter().any(|notes| notes[60]));
}

#[test]
fn fx_engine_sustained_strum_rearticulates_without_doubling_or_losing_layer() {
    for mpe in [false, true] {
        let mut e = engine(mpe);
        e.allocate_voice(60, 0.8, u64::MAX, true, &mut |_| {});
        e.advance(1, &mut |_, _| {});
        let index = e.voices.iter().position(Option::is_some).unwrap();
        let mut events = Vec::new();
        e.strike_prepared(60, 0.9, 20, 0, &mut |v| events.push(v));
        e.advance(1, &mut |_, v| events.push(v));
        assert_eq!(notes(&events).len(), 1, "a held-string pluck must reattack");
        assert_eq!(
            e.owned_output_notes()
                .iter()
                .filter(|notes| notes[60])
                .count(),
            1
        );
        e.advance(100, &mut |_, _| {});
        assert!(e.owned_output_notes().iter().any(|notes| notes[60]));
        e.end_voice(index, 0.0, &mut |_| {});
        assert!(!e.owned_output_notes().iter().any(|notes| notes[60]));
    }
}
