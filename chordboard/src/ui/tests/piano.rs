use super::super::*;
use super::support::*;

#[test]
fn full_piano_fits_below_performance_and_never_plays_notes() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    for note in 0..128u8 {
        let r = piano_key_rect(note);
        assert!(r.1 >= EXPANDED_SURFACE.1 + EXPANDED_SURFACE.3);
        assert!(r.1 + r.3 < H);
        assert!(hit(PIANO_SURFACE, r.0 + r.2, r.1 + r.3));
        assert_eq!(piano_note_at(r.0 + r.2 / 2.0, r.1 + r.3 - 2.0), Some(note));
        click(&mut view, &mut cx, target, r);
    }
    assert!(!std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::KeyDown(..) | Command::BeginGesture(..))));
}
#[test]
fn full_piano_buttons_work_in_every_mode_and_expanded_manual() {
    for mode in 1..4 {
        let mut view = view(mode, false);
        if mode == 2 {
            view.expand_progress = 1.0;
        }
        let (mut cx, target, changes) = context();
        for (r, ptr) in [
            (ALWAYS_BASS, view.params.always_bass.as_ptr()),
            (
                key_split_rect(view.params.mpe_enabled()),
                view.params.key_split.as_ptr(),
            ),
        ] {
            click(&mut view, &mut cx, target, r);
            assert_eq!(changes.borrow().last().copied(), Some((ptr, 1.0)));
        }
    }
}
#[test]
fn piano_split_drag_selects_accidentals_and_clamps_at_midi_edges() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        key_split: BoolParam::new("Key split", true),
        ..ChordboardParams::default()
    });
    let (mut cx, target, changes) = context();
    let r = view.split_marker();
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + r.2 / 2.0,
        r.1 + 8.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::Split)));
    event(
        &mut view,
        &mut cx,
        target,
        piano_note_x(61),
        PIANO_SPLITS.1 + 14.0,
        WindowEvent::MouseMove(0.0, 0.0),
    );
    assert_eq!(
        changes.borrow().last().copied(),
        Some((
            view.params.split_note.as_ptr(),
            view.params.split_note.preview_normalized(61)
        ))
    );
    for (x, note) in [(-10.0, 13), (W + 10.0, 127)] {
        event(
            &mut view,
            &mut cx,
            target,
            x,
            PIANO_SPLITS.1,
            WindowEvent::MouseMove(x, PIANO_SPLITS.1),
        );
        assert_eq!(
            changes.borrow().last().copied(),
            Some((
                view.params.split_note.as_ptr(),
                view.params.split_note.preview_normalized(note)
            ))
        );
    }
    event(
        &mut view,
        &mut cx,
        target,
        W + 10.0,
        PIANO_SPLITS.1,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(view.drag.is_none());
    assert!(!std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::KeyDown(..) | Command::BeginGesture(..))));
}
#[test]
fn clicking_split_when_off_enables_key_split_and_moves_it() {
    let mut view = view(1, false);
    assert!(!view.params.key_split.value());
    let (mut cx, target, changes) = context();
    let r = view.split_marker();
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + r.2 / 2.0,
        r.1 + 8.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::Split)));
    event(
        &mut view,
        &mut cx,
        target,
        piano_note_x(65),
        PIANO_SPLITS.1 + 14.0,
        WindowEvent::MouseMove(0.0, 0.0),
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|&(ptr, val)| ptr == view.params.key_split.as_ptr() && val == 1.0));
    assert_eq!(
        changes.borrow().last().copied(),
        Some((
            view.params.split_note.as_ptr(),
            view.params.split_note.preview_normalized(65)
        ))
    );
}
#[test]
fn piano_tooltips_distinguish_silent_controls_and_live_notes() {
    let mut view = view(1, false);
    view.params.control_base.store(24, Ordering::Relaxed);
    let r = piano_key_rect(27);
    let hint = view.piano_hint(r.0 + r.2 / 2.0, r.1 + 8.0).unwrap();
    assert!(
        hint.contains("MIDI 27")
            && hint.contains("silent")
            && hint.contains(harmony::CONTROL_LABELS[3])
    );
    view.snapshot.held_notes[60] = true;
    view.snapshot.sounding_notes[60] = true;
    let r = piano_key_rect(60);
    let hint = view.piano_hint(r.0 + r.2 / 2.0, r.1 + r.3 - 2.0).unwrap();
    assert!(
        hint.contains("C4")
            && hint.contains("MIDI 60")
            && hint.contains("Held input")
            && hint.contains("Sounding output")
    );
}
#[test]
fn routed_controls_use_parameter_scale_and_preserve_the_saved_base() {
    let mut view = view(1, false);
    for id in ["length_ms", "bass_channel", "velocity", "strings"] {
        let (index, target) = crate::engine::routing::TARGETS
            .iter()
            .enumerate()
            .find(|(_, t)| t.id == id)
            .unwrap();
        view.snapshot.routed[index] = Some(0.5);
        let base = view.control(id).unwrap();
        let routed = view.routed_control(&base).unwrap();
        let plain = target.plain(0.5) + if id == "bass_channel" { 1.0 } else { 0.0 };
        assert_eq!(routed.norm, unsafe { base.ptr.preview_normalized(plain) });
        assert_eq!(view.control(id).unwrap().norm, base.norm);
        assert_eq!(routed.value, target.label(0.5));
        view.snapshot.routed[index] = None;
        assert!(view.routed_control(&base).is_none());
    }
}
#[test]
fn module_grid_and_keyboard_midi_controls_fit_without_overlapping() {
    assert!(PIANO_SPLITS.1 + PIANO_SPLITS.3 <= PIANO_KEYS.1);
    assert!(PIANO_LEADING.1 + PIANO_LEADING.3 + 2.0 < PIANO_SPLITS.1);
    assert!(PIANO_KEYS.1 + PIANO_KEYS.3 < BASS_CONTAINER.1);
    for mpe in [false, true] {
        let melody = melody_container(mpe);
        let chord = chord_container(mpe);
        assert!(chord.1 > PIANO_KEYS.1 + PIANO_KEYS.3);
        assert!(melody.1 > PIANO_KEYS.1 + PIANO_KEYS.3);
        assert_eq!(BASS_CONTAINER.0, PIANO_KEYS.0);
        assert_eq!(melody.0 + melody.2, PIANO_KEYS.0 + PIANO_KEYS.2);
        assert!((chord.0 + chord.2 + 12.0 - melody.0).abs() < 0.001);
        assert_eq!(BASS_CONTAINER.2, chord.2);
        assert_eq!(chord.2, melody.2);
        let bypass = key_split_rect(mpe);
        assert_eq!(bypass.0, melody.0 + 10.0);
        assert!(bypass.0 + bypass.2 <= melody.0 + melody.2);
        for r in [BASS_CONTAINER, chord, melody] {
            assert!(r.1 + r.3 <= PIANO_SURFACE.1 + PIANO_SURFACE.3);
        }
    }

    assert_eq!(CHORDS_SURFACE.1, PERF_SURFACE.1);
    assert_eq!(MOD_SURFACE.1 - PERF_SURFACE.1 - PERF_SURFACE.3, 12.0);
    assert_eq!(
        CHORDS_SURFACE.1 + CHORDS_SURFACE.3,
        PERF_SURFACE.1 + PERF_SURFACE.3
    );
    assert_eq!(PIANO_SURFACE.1 - MOD_SURFACE.1 - MOD_SURFACE.3, 12.0);
    assert_eq!(CHORDS_SURFACE.0, PIANO_SURFACE.0);
    assert_eq!(PERF_SURFACE.0 - CHORDS_SURFACE.0 - CHORDS_SURFACE.2, 12.0);
    assert_eq!(MOD_SURFACE.0, PIANO_SURFACE.0);
    assert_eq!(MOD_SURFACE.2, PIANO_SURFACE.2);
    for mpe in [false, true] {
        let view = view(1, mpe);
        for (id, module) in [
            ("bass_channel", BASS_CONTAINER),
            ("output_channel", chord_container(mpe)),
            ("upper_channel", melody_container(mpe)),
        ] {
            let r = keyboard_control_rect(id, mpe);
            assert_eq!(r.0 + r.2, module.0 + module.2 - 12.0);
            assert_eq!(r.1 + r.3 * 0.5, module_header_mid(module.1));
        }
        let mut rects: Vec<_> = view.keyboard_controls().iter().map(|(_, r)| *r).collect();
        rects.push(output_protocol_rect(mpe, 0));
        rects.extend([
            inversion_control_rect(mpe, 0),
            inversion_control_rect(mpe, 1),
            ALWAYS_BASS,
            affect_chords_rect(mpe),
            key_split_rect(mpe),
            melody_split_rect(mpe),
        ]);
        for (i, r) in rects.iter().enumerate() {
            assert!(
                [BASS_CONTAINER, chord_container(mpe), melody_container(mpe)]
                    .iter()
                    .any(|module| r.0 >= module.0
                        && r.1 >= module.1
                        && r.0 + r.2 <= module.0 + module.2
                        && r.1 + r.3 <= module.1 + module.3),
                "control outside its module: {r:?}"
            );
            assert!(r.1 > PIANO_KEYS.1 + PIANO_KEYS.3);
            assert!(r.0 >= PIANO_SURFACE.0 && r.0 + r.2 <= PIANO_SURFACE.0 + PIANO_SURFACE.2);
            assert!(r.1 + r.3 <= PIANO_SURFACE.1 + PIANO_SURFACE.3);
            for other in rects.iter().skip(i + 1) {
                assert!(
                    !(r.0 < other.0 + other.2
                        && other.0 < r.0 + r.2
                        && r.1 < other.1 + other.3
                        && other.1 < r.1 + r.3)
                );
            }
        }
    }
}
#[test]
fn bass_marker_drag_keeps_one_key_below_the_right_hand_split() {
    let mut view = view(3, false);
    view.params.control_base.store(36, Ordering::Relaxed);
    let (mut cx, target, changes) = context();
    let r = view.bass_marker().unwrap();
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + r.2 / 2.0,
        r.1 + r.3 / 2.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::BassSplit)));
    for note in [24, 60] {
        event(
            &mut view,
            &mut cx,
            target,
            piano_note_x(note),
            PIANO_SPLITS.1 + 14.0,
            WindowEvent::MouseMove(0.0, 0.0),
        );
        assert_eq!(
            changes.borrow().last().copied(),
            Some((
                view.params.bass_split.as_ptr(),
                view.params
                    .bass_split
                    .preview_normalized((note as i32).min(59))
            ))
        );
    }
    assert!(changes
        .borrow()
        .iter()
        .all(|(ptr, _)| *ptr != view.params.split_note.as_ptr()));
    event(
        &mut view,
        &mut cx,
        target,
        piano_note_x(60),
        PIANO_KEYS.1,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(view.drag.is_none());
}
#[test]
fn bass_marker_follows_the_default_control_octave() {
    let view = view(3, false);
    assert_eq!(view.params.bass_split.value(), -1);
    assert_eq!(view.params.control_base.load(Ordering::Relaxed), 12);
    let marker = view.bass_marker();
    assert!(
        marker.is_some(),
        "bass marker must appear by default even before learn"
    );
    assert_eq!(view.bass_boundary(), Some(12));
}
#[test]
fn onscreen_piano_key_learns_control_octave_and_stops_listening() {
    let mut view = view(3, false);
    let (mut cx, target, _) = context();
    {
        let r = view.control_octave_bounds();
        click(&mut view, &mut cx, target, r);
    }
    assert_eq!(view.learning(), 4);

    let key_r = piano_key_rect(52);
    click(
        &mut view,
        &mut cx,
        target,
        (key_r.0 + key_r.2 * 0.5, key_r.1 + key_r.3 * 0.5, 2.0, 2.0),
    );
    assert_eq!(view.params.control_base.load(Ordering::Relaxed), 48);
    assert_eq!(view.learning(), 0);
}
#[test]
fn keyboard_clicks_and_outside_moves_do_not_set_splits() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    for note in [36, 60, 90] {
        click(&mut view, &mut cx, target, piano_key_rect(note));
        assert!(view.drag.is_none());
    }
    assert!(changes
        .borrow()
        .iter()
        .all(|(ptr, _)| *ptr != view.params.split_note.as_ptr()
            && *ptr != view.params.bass_split.as_ptr()));
    let marker = view.split_marker();
    click(&mut view, &mut cx, target, marker);
    changes.borrow_mut().clear();
    view.move_split(
        &mut EventContext::new_with_current(&mut cx, target),
        piano_note_x(90),
        PIANO_KEYS.1 + 10.0,
    );
    view.move_bass_split(
        &mut EventContext::new_with_current(&mut cx, target),
        piano_note_x(90),
        PIANO_KEYS.1 + 10.0,
    );
    assert!(changes.borrow().is_empty());
}
