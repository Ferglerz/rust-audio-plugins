use super::super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use super::support::*;

#[test]
fn voice_leading_display_cycles_mode_and_modulation_is_one_column() {
    let mut view = view(1, false);
    assert!(PIANO_LEADING.1 + PIANO_LEADING.3 + 2.0 < PIANO_SPLITS.1);
    assert!(PIANO_SPLITS.1 + PIANO_SPLITS.3 < PIANO_KEYS.1);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, PIANO_LEADING);
    assert_eq!(
        changes.borrow().last().copied(),
        Some((
            view.params.voice_leading.as_ptr(),
            view.params
                .voice_leading
                .preview_normalized((view.params.voice_leading.value() + 1) % 3)
        ))
    );
    for i in 0..crate::engine::routing::SOURCE_COUNT {
        let r = meter_rect(i);
        assert_eq!(r.1, meter_rect(0).1);
        assert!(r.1 + r.3 < MOD_SURFACE.1 + MOD_SURFACE.3);
        if i > 0 {
            assert!(r.0 > meter_rect(i - 1).0 + meter_rect(i - 1).2);
        }
    }
}
#[test]
fn clicking_x_opens_the_default_connection_without_changing_parameters() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, meter_rect(7));
    assert_eq!(view.panel, Some(Panel::Routes));
    assert_eq!(view.route_slot, 0);
    assert!(changes.borrow().is_empty());
    assert_eq!(view.params.routes[0].source.value(), 8);
}
#[test]
fn self_modulation_is_rejected_by_drag_and_both_route_menus() {
    let mut view = view(2, false);
    view.panel = Some(Panel::Mapping);
    view.mapping_axis = 0;
    let r = mapping_control_rect(0, "x_reverse");
    let (mut cx, target, changes) = context();
    view.finish_route_drag(
        &mut EventContext::new_with_current(&mut cx, target),
        7,
        r.0 + 20.0,
        r.1 + 10.0,
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(changes.borrow().is_empty());
    assert!(view.status.contains("cannot modulate"));
    let index = crate::engine::routing::available_targets()
        .position(|(_, t)| t.id == "x_reverse")
        .unwrap();
    assert!(!view.route_menu_allowed(Menu::RouteTarget, index));
    view.select_menu(
        &mut EventContext::new_with_current(&mut cx, target),
        Menu::RouteTarget,
        index,
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(changes.borrow().is_empty());
    Arc::get_mut(&mut view.params).unwrap().routes[0].target = IntParam::new(
        "Destination",
        crate::engine::routing::TARGETS
            .iter()
            .position(|t| t.id == "x_reverse")
            .unwrap() as i32,
        IntRange::Linear {
            min: 0,
            max: (crate::engine::routing::TARGETS.len() - 1) as i32,
        },
    );
    assert!(!view.route_menu_allowed(Menu::RouteSource, 8));
    view.select_menu(
        &mut EventContext::new_with_current(&mut cx, target),
        Menu::RouteSource,
        8,
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(changes.borrow().is_empty());
}
#[test]
fn relocated_transpose_and_tempo_targets_fit_their_modules() {
    for mpe in [false, true] {
        let module = CHORDS_SURFACE;
        let first = transpose_control_rect(mpe, 0);
        let last = transpose_control_rect(mpe, 4);
        assert!(first.0 > ORDER.0 + ORDER.2);
        assert_eq!(first.0, TRANSPOSE.0);
        assert_eq!(last.0 + last.2, TRANSPOSE.0 + TRANSPOSE.2);
        for r in (0..TRANSPOSE_STEPS.len()).map(|i| transpose_control_rect(mpe, i)) {
            assert!(r.0 >= module.0 && r.0 + r.2 <= module.0 + module.2);
            assert!(r.1 >= module.1 && r.1 + r.3 <= module.1 + MODULE_HEADER_H);
        }
    }
    for r in [TEMPO_SYNC, TEMPO_CONTROL] {
        assert!(r.0 >= 0.0 && r.0 + r.2 < APPEARANCE.0);
        assert!(r.1 >= 0.0 && r.1 + r.3 < HEADER_H);
    }
    assert!(TEMPO_SYNC.0 + TEMPO_SYNC.2 <= TEMPO_CONTROL.0);
    assert_eq!(TEMPO_CONTROL.1, TEMPO_SYNC.1);
    assert_eq!(TEMPO_CONTROL.3, TEMPO_SYNC.3);
}
#[test]
fn chord_readout_tracks_slash_bass_inversions_transpose_and_sustain() {
    let mut view = view(0, false);
    let mut engine = crate::engine::Engine::default();
    let config = crate::engine::Config {
        bass_split: 36,
        ..Default::default()
    };
    assert_eq!(view.chord_readout(), "—");
    engine.configure(config, &mut |_| {});
    engine.midi_note(true, 0, 60, 0.8, &mut |_| {});
    view.snapshot = engine.snapshot();
    assert_eq!(view.chord_readout(), "C");

    engine.configure(
        crate::engine::Config {
            inversion: 1,
            ..config
        },
        &mut |_| {},
    );
    view.snapshot = engine.snapshot();
    assert_eq!(view.chord_readout(), "C/E");

    engine.midi_note(true, 0, 31, 0.8, &mut |_| {});
    engine.control(0, 64, 1.0, &mut |_| {});
    engine.midi_note(false, 0, 31, 0.0, &mut |_| {});
    view.snapshot = engine.snapshot();
    assert_eq!(view.snapshot.bass_note, Some(31));
    assert_eq!(view.chord_readout(), "C/G");
    engine.control(0, 64, 0.0, &mut |_| {});
    view.snapshot = engine.snapshot();
    assert_eq!(view.snapshot.bass_note, None);
    assert_eq!(view.chord_readout(), "C/E");

    engine.midi_note(true, 0, 24, 0.8, &mut |_| {});
    view.snapshot = engine.snapshot();
    assert_eq!(view.chord_readout(), "C");
    engine.midi_note(false, 0, 24, 0.0, &mut |_| {});
    engine.configure(
        crate::engine::Config {
            inversion: 1,
            transpose: 2,
            ..config
        },
        &mut |_| {},
    );
    view.snapshot = engine.snapshot();
    assert_eq!(view.chord_readout(), "D/F#");
}
#[test]
fn keyboard_settings_share_one_body_row_and_mpe_hides_bend_controls() {
    for mpe in [false, true] {
        let view = view(1, mpe);
        let controls = view.keyboard_controls();
        assert!(!controls
            .iter()
            .any(|(c, _)| matches!(c.id, "bend_range" | "master_range")));
        for (c, r) in controls {
            if !matches!(c.id, "bass_channel" | "output_channel" | "upper_channel") {
                assert_eq!(r.1, ALWAYS_BASS.1);
            }
        }
        for r in [
            ALWAYS_BASS,
            affect_chords_rect(mpe),
            melody_split_rect(mpe),
            inversion_control_rect(mpe, 0),
            output_protocol_rect(mpe, 0),
        ] {
            assert_eq!(r.1, ALWAYS_BASS.1);
        }
    }
}
#[test]
fn selection_button_cycles_all_three_modes_and_preserves_saved_flags() {
    for selected in 0..3 {
        let mut view = view(2, false);
        view.params = Arc::new(ChordboardParams {
            mode: IntParam::new("Mode", 2, IntRange::Linear { min: 0, max: 3 }),
            root_on_select: BoolParam::new("Root", selected == 1),
            always_chord: BoolParam::new("Chord", selected == 2),
            ..ChordboardParams::default()
        });
        assert_eq!(view.selection_mode(), selected);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, ROOT_ON_SELECT);
        let next = (selected + 1) % 3;
        for (ptr, enabled) in [
            (view.params.root_on_select.as_ptr(), next == 1),
            (view.params.always_chord.as_ptr(), next == 2),
        ] {
            assert!(changes.borrow().contains(&(ptr, enabled as u8 as f32)));
        }
    }
}
#[test]
fn scale_layout_pointer_and_typing_resolve_the_same_notes_without_overlapping_tiles() {
    for layout in 1..=2 {
        for scale in 0..8 {
            let mut view = view(2, false);
            view.params = Arc::new(ChordboardParams {
                scale_layout: IntParam::new("Layout", layout, IntRange::Linear { min: 0, max: 2 }),
                scale: IntParam::new("Scale", scale, IntRange::Linear { min: 0, max: 7 }),
                key: IntParam::new("Key", 1, IntRange::Linear { min: 0, max: 11 }),
                key_spelling: IntParam::new("Spelling", 2, IntRange::Linear { min: 0, max: 2 }),
                ..ChordboardParams::default()
            });
            let (mut cx, target, _) = context();
            for i in 0..KEY_COUNT {
                let Some(chord) = view.keyboard_chord(i) else {
                    view.play_key(i);
                    assert!(view.bridge.commands.pop().is_none());
                    continue;
                };
                let r = view.keyboard_key_rect(i);
                assert!(r.0 >= CHORDS_SURFACE.0 && r.0 + r.2 < CHORDS_SURFACE.0 + CHORDS_SURFACE.2);
                for j in i + 1..KEY_COUNT {
                    let other = view.keyboard_key_rect(j);
                    assert!(
                        other.2 == 0.0
                            || r.1 != other.1
                            || r.0 + r.2 <= other.0
                            || other.0 + other.2 <= r.0,
                        "overlapping keys {i} and {j}, layout {layout}, scale {scale}"
                    );
                }
                click(&mut view, &mut cx, target, r);
                let pointer: Vec<_> = std::iter::from_fn(|| view.bridge.commands.pop()).collect();
                assert!(pointer.iter().any(|c| matches!(c, Command::KeyDown(_, note, quality) if *note == 48 + chord.root && *quality == chord.quality)));
                view.play_key(i);
                assert!(
                    matches!(view.bridge.commands.pop(), Some(Command::KeyDown(_, note, quality)) if note == 48 + chord.root && quality == chord.quality)
                );
            }
        }
    }
}
#[test]
fn compact_layout_preserves_keyboard_velocity_heights_and_memory_gap() {
    assert_eq!(PIANO_SURFACE.3, 200.0);
    assert_eq!(PERF_SURFACE.3, CHORDS_SURFACE.3);
    let tile = key_rect(KEY_COLUMNS * 2);
    assert_eq!(memory_rect(0).1 - tile.1 - tile.3, 10.0);
    assert!(tile.3 / tile.2 < 1.3);
    assert_eq!(PIANO_SURFACE.2, W - 32.0);
    let first = key_rect(0);
    let last = memory_rect(MEMORY_COUNT - 1);
    let body_top = CHORDS_SURFACE.1 + MODULE_HEADER_H;
    let body_bottom = CHORDS_SURFACE.1 + CHORDS_SURFACE.3;
    assert_eq!(first.1 - body_top, body_bottom - last.1 - last.3);
    for r in (0..KEY_COUNT)
        .map(key_rect)
        .chain((0..MEMORY_COUNT).map(memory_rect))
    {
        assert_eq!((r.2, r.3), (66.0, 78.0));
        assert!(r.0 >= CHORDS_SURFACE.0 + 16.0);
        assert!(r.0 + r.2 <= CHORDS_SURFACE.0 + CHORDS_SURFACE.2 - 16.0);
    }
    let readout = chord_readout_rect();
    assert!(readout.0 > last.0 + last.2);
    assert!(readout.0 + readout.2 <= CHORDS_SURFACE.0 + CHORDS_SURFACE.2 - 16.0);
}
#[test]
fn all_layouts_keep_the_physical_keyboard_stagger() {
    for layout in 0..=2 {
        let mut view = view(1, false);
        view.params = Arc::new(ChordboardParams {
            scale_layout: IntParam::new("Layout", layout, IntRange::Linear { min: 0, max: 2 }),
            ..ChordboardParams::default()
        });
        for i in 0..KEY_COUNT {
            if view.keyboard_chord(i).is_some() {
                assert_eq!(view.keyboard_key_rect(i), key_rect(i));
            }
        }
    }
    assert_eq!(key_rect(KEY_COLUMNS).0 - key_rect(0).0, KEY_PITCH * 0.25);
    assert_eq!(
        key_rect(KEY_COLUMNS * 2).0 - key_rect(0).0,
        KEY_PITCH * 0.75
    );
}
#[test]
fn split_learning_buttons_swap_crossed_boundaries_and_cancel() {
    for (target_kind, key, expected) in [(5, 80, (60, 80)), (6, 20, (12, 20)), (5, 60, (60, 61))] {
        let mut view = view(2, false);
        let (mut cx, target, changes) = context();
        let r = if target_kind == 5 {
            keyboard_control_rect("bass_split", false)
        } else {
            melody_split_rect(false)
        };
        click(&mut view, &mut cx, target, r);
        assert_eq!(view.learning(), target_kind);
        click(&mut view, &mut cx, target, r);
        assert_eq!(view.learning(), 0);
        click(&mut view, &mut cx, target, r);
        let key_rect = piano_key_rect(key);
        click(
            &mut view,
            &mut cx,
            target,
            (key_rect.0, key_rect.1 + key_rect.3 - 4.0, key_rect.2, 2.0),
        );
        assert_eq!(view.learning(), 0);
        for (param, value) in [
            (&view.params.bass_split, expected.0),
            (&view.params.split_note, expected.1),
        ] {
            if param.value() != value {
                assert!(changes
                    .borrow()
                    .contains(&(param.as_ptr(), param.preview_normalized(value))));
            }
        }
    }
}
#[test]
fn expanded_manual_field_preserves_all_four_insets() {
    let insets = |pad: Rect, field: Rect| {
        (
            field.0 - pad.0,
            field.1 - pad.1,
            pad.0 + pad.2 - field.0 - field.2,
            pad.1 + pad.3 - field.1 - field.3,
        )
    };
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert_eq!(
            insets(pad_rect(t), play_pad_rect(t)),
            (60.0, 75.0, 20.0, 65.0)
        );
    }
}
#[test]
fn bass_bypass_and_inline_controls_share_the_header() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, BASS_BYPASS);
    assert!(changes
        .borrow()
        .contains(&(view.params.bass_enabled.as_ptr(), 0.0)));
    for r in [
        ALWAYS_BASS,
        SPLIT_NOTE,
        AFFECT_CHORDS,
        keyboard_control_rect("bass_split", false),
    ] {
        assert_eq!(r.1 + r.3 * 0.5, module_header_mid(BASS_CONTAINER.1));
    }
}
#[test]
fn split_drag_repairs_extreme_loaded_boundaries() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        bass_split: IntParam::new("Bass", 127, IntRange::Linear { min: -1, max: 127 }),
        split_note: IntParam::new("Melody", 0, IntRange::Linear { min: 0, max: 127 }),
        ..ChordboardParams::default()
    });
    let (mut cx, target, changes) = context();
    view.move_bass_split(
        &mut EventContext::new_with_current(&mut cx, target),
        piano_note_x(127),
        PIANO_SPLITS.1,
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(changes.borrow().contains(&(
        view.params.split_note.as_ptr(),
        view.params.split_note.preview_normalized(1)
    )));
    assert!(changes.borrow().contains(&(
        view.params.bass_split.as_ptr(),
        view.params.bass_split.preview_normalized(0)
    )));
    changes.borrow_mut().clear();
    view.move_split(
        &mut EventContext::new_with_current(&mut cx, target),
        piano_note_x(0),
        PIANO_SPLITS.1,
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(changes.borrow().contains(&(
        view.params.bass_split.as_ptr(),
        view.params.bass_split.preview_normalized(126)
    )));
    assert!(changes
        .borrow()
        .contains(&(view.params.split_note.as_ptr(), 1.0)));
}
#[test]
fn module_header_controls_fit_inline_without_overlap() {
    let chords = vec![
        Menu::Key.trigger_rect(),
        Menu::Scale.trigger_rect(),
        ORDER,
        TRANSPOSE,
    ];
    for (module, title_width, controls) in [
        (CHORDS_SURFACE, 78.0, chords),
    ] {
        let mut right = module.0 + title_width;
        for r in controls {
            assert!(r.0 >= right + 2.0, "header overlap: {r:?}");
            assert_eq!(r.1 + r.3 * 0.5, module_header_mid(module.1));
            right = r.0 + r.2;
            assert!(right <= module.0 + module.2 - 10.0);
        }
    }
    assert!(key_rect(0).1 >= CHORDS_SURFACE.1 + MODULE_HEADER_H);
    assert!(TEMPO_SYNC.1 + TEMPO_SYNC.3 < HEADER_H);
    assert!(TEMPO_CONTROL.0 >= TEMPO_SYNC.0 + TEMPO_SYNC.2 + 8.0);
    assert!(TEMPO_CONTROL.0 + TEMPO_CONTROL.2 < APPEARANCE.0);
}
#[test]
fn strum_bypass_header_toggles_in_collapsed_and_expanded_views() {
    for expanded in [false, true] {
        let mut view = view(2, false);
        if expanded {
            view.expand_progress = 1.0;
            view.expand_target = 1.0;
        }
        let (mut cx, target, changes) = context();
        let rect = strum_bypass_rect(view.expand_t());
        click(&mut view, &mut cx, target, rect);
        assert_eq!(
            &*changes.borrow(),
            &[(view.params.strum_enabled.as_ptr(), 0.0)]
        );
    }
    let mut view = view(1, false);
    assert!(
        view.params.strum_enabled.value(),
        "Strumfield must start enabled"
    );
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, STRUM_BYPASS);
    assert_eq!(
        &*changes.borrow(),
        &[(view.params.strum_enabled.as_ptr(), 0.0)]
    );
}
#[test]
fn strum_header_controls_remain_adjacent_through_expansion() {
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let surface = perf_surface_rect(t);
        let zoom = expand_rect(t);
        let latch = strum_latch_rect(t);
        assert_eq!(surface.0 + surface.2 - zoom.0 - zoom.2, 16.0);
        assert_eq!(zoom.1 + zoom.3 * 0.5, module_header_mid(surface.1));
        assert_eq!(zoom.3, latch.3);
        assert_eq!(zoom.0 - latch.0 - latch.2, 8.0);
        assert_eq!(zoom.1 + zoom.3 * 0.5, latch.1 + latch.3 * 0.5);
    }
    assert_eq!(initial_editor_size(), (W as u32, H as u32));
}
