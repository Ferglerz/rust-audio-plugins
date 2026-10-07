use super::super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};
use super::support::*;

#[test]
fn strum_range_grips_clamp_without_playing_notes() {
    for (right, destination, expected) in [(false, 2.0, 0.75), (true, -1.0, 0.25)] {
        let mut view = view(2, false);
        let (mut cx, target, changes) = context();
        let r = strum_bound_rect(if right { 1.0 } else { 0.0 });
        event(
            &mut view,
            &mut cx,
            target,
            r.0 + r.2 / 2.0,
            r.1 + r.3 / 2.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(matches!(view.drag, Some(Drag::StrumBound(false, _))));
        let x = PLAY_PAD.0 + destination * PLAY_PAD.2;
        event(
            &mut view,
            &mut cx,
            target,
            x,
            r.1 + 13.0,
            WindowEvent::MouseMove(x, r.1 + 13.0),
        );
        let (ptr, norm) = changes.borrow().last().copied().unwrap();
        let param = if right {
            &view.params.x_max
        } else {
            &view.params.x_min
        };
        assert_eq!(ptr, param.as_ptr());
        assert!((param.preview_plain(norm) - expected).abs() < 0.00001);
        assert!(view.bridge.commands.pop().is_none());
        event(
            &mut view,
            &mut cx,
            target,
            x,
            r.1 + 13.0,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(view.drag.is_none());
    }
    let mut view = view(2, false);
    view.panel = Some(Panel::Mapping);
    view.mapping_axis = 0;
    assert_eq!(
        view.panel_controls()
            .iter()
            .map(|(c, _)| c.id)
            .collect::<Vec<_>>(),
        ["x_reverse"]
    );
}
#[test]
fn computer_keys_ignore_legacy_disable_but_require_focus() {
    let mut view = view(0, false);
    view.params = Arc::new(ChordboardParams {
        keyboard: BoolParam::new("Keyboard play", false),
        ..ChordboardParams::default()
    });
    let (mut cx, target, _) = context();
    view.focused = true;
    assert!(view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(CODES[0], None)
    ));
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::KeyDown(0, _, _))
    ));
    view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyUp(CODES[0], None),
    );
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::KeyUp(0))
    ));
    view.focused = false;
    assert!(!view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(CODES[0], None)
    ));
    assert!(view.bridge.commands.pop().is_none());
}
#[test]
fn memory_row_keys_recall_once_per_press_and_leave_escape_to_host() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    view.focused = true;
    for (slot, code) in MEMORY_CODES.into_iter().enumerate() {
        let word = SavedChord {
            root: 60 + slot as u8,
            second: None,
            quality: 0,
            inversion: 0,
            spread: 0,
            transpose: 0,
        }
        .encode();
        view.params.slot(slot).store(word, Ordering::Relaxed);
        for _ in 0..3 {
            let mut event = Event::new(WindowEvent::KeyDown(code, None));
            View::event(
                &mut view,
                &mut EventContext::new_with_current(&mut cx, target),
                &mut event,
            );
            BackendContext::new_with_event_manager(&mut cx).process_events();
        }
        let recalls = std::iter::from_fn(|| view.bridge.commands.pop())
            .filter(|command| matches!(command, Command::RecallMemory(index, saved) if *index == slot && *saved == word))
            .count();
        assert_eq!(recalls, 1, "memory {slot} retriggered on key repeat");
        assert!(view.window_event(
            &mut EventContext::new_with_current(&mut cx, target),
            &WindowEvent::KeyUp(code, None)
        ));
        assert!(!view.memory_held[slot]);
    }
    assert!(!view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::Escape, None)
    ));
    view.focused = false;
    assert!(!view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::KeyZ, None)
    ));
}
#[test]
fn memory_shortcuts_capture_with_shift_and_do_not_interrupt_text_entry() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    view.focused = true;
    view.snapshot.captured = SavedChord {
        root: 60,
        second: None,
        quality: 0,
        inversion: 0,
        spread: 0,
        transpose: 0,
    }
    .encode();
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, true);
    assert!(view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::KeyZ, None)
    ));
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::Capture(0))
    ));
    view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyUp(Code::KeyZ, None),
    );
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, false);
    view.edit = Some(ValueEdit::new(
        "transpose",
        (0.0, 0.0, 100.0, 30.0),
        "0".into(),
    ));
    assert!(view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::KeyZ, None)
    ));
    assert!(!view.memory_held[0]);
    assert!(view.bridge.commands.pop().is_none());
}
#[test]
fn expression_range_grips_drag_vertically_without_playing_notes() {
    for (maximum, destination, expected) in [(false, 2.0, 0.99), (true, -1.0, 0.01)] {
        let mut view = view(2, false);
        let (mut cx, target, changes) = context();
        let (ax, ay, pointer) = strum_bound_anchor(true, if maximum { 1.0 } else { 0.0 });
        let body = pleasant_ui::handles::tag_body_rect(ax, ay, pointer);
        let x = body.0 + body.2 / 2.0;
        event(
            &mut view,
            &mut cx,
            target,
            x,
            ay,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(matches!(view.drag, Some(Drag::StrumBound(true, end)) if end == maximum));
        let y = PLAY_PAD.1 + (1.0 - destination) * PLAY_PAD.3;
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        let (ptr, norm) = changes.borrow().last().copied().unwrap();
        let param = view.bound_param(true, maximum);
        assert_eq!(ptr, param.as_ptr());
        assert!((param.preview_plain(norm) - expected).abs() < 0.00001);
        assert!(view.bridge.commands.pop().is_none());
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(view.drag.is_none());
    }
    let mut view = view(2, false);
    view.panel = Some(Panel::Mapping);
    view.mapping_axis = 1;
    assert!(!view
        .panel_controls()
        .iter()
        .any(|(c, _)| matches!(c.id, "y_min" | "y_max")));
}
#[test]
fn manual_xy_cc_selection_clears_channel_lock_and_has_no_channel_menu() {
    for axis in 0..2 {
        let mut view = view(2, false);
        view.mapping_axis = axis;
        view.panel = Some(Panel::Mapping);
        let (mut cx, target, _) = context();
        view.params
            .mapping(axis)
            .store(crate::engine::Mapping::cc(7, 3).encode(), Ordering::Relaxed);
        click_menu_trigger(&mut view, &mut cx, target, Menu::MappingChannel(false));
        assert!(view.menu.is_none());
        assert_eq!(
            view.mapping().channel,
            3,
            "displaying learned mapping must preserve its channel"
        );
        view.select_menu(
            &mut EventContext::new_with_current(&mut cx, target),
            Menu::MappingCc(false),
            11,
        );
        assert_eq!(view.mapping(), crate::engine::Mapping::cc(11, 16));
        view.params
            .mapping(axis)
            .store(crate::engine::Mapping::cc(7, 3).encode(), Ordering::Relaxed);
        view.select_menu(
            &mut EventContext::new_with_current(&mut cx, target),
            Menu::MappingKind,
            1,
        );
        assert_eq!(view.mapping().kind, 1);
        assert_eq!(view.mapping().channel, 16);
    }
}
#[test]
fn spread_keyboard_cycles_all_voicings_without_drag_or_text_entry() {
    let (mut cx, target, changes) = context();
    for current in 0..harmony::VOICING_NAMES.len() as i32 {
        let mut view = view(2, false);
        view.params = Arc::new(ChordboardParams {
            mode: IntParam::new("Mode", 2, IntRange::Linear { min: 0, max: 3 }),
            spread: IntParam::new(
                "Spread",
                current,
                IntRange::Linear {
                    min: 0,
                    max: (harmony::VOICING_NAMES.len() - 1) as i32,
                },
            ),
            ..ChordboardParams::default()
        });
        let r = view.voicing_bounds().expect("voicing bounds");
        click(&mut view, &mut cx, target, r);
        assert_eq!(
            changes.borrow().last().copied(),
            Some((
                view.params.spread.as_ptr(),
                view.params
                    .spread
                    .preview_normalized((current + 1) % harmony::VOICING_NAMES.len() as i32)
            ))
        );
        assert!(view.drag.is_none());
        assert!(view.edit.is_none());
        view.snapshot.full_notes = harmony::voice(60, 3, None, 0, 0, current as u8);
        view.snapshot.notes = harmony::filter_notes(view.snapshot.full_notes, 1);
        assert_eq!(
            view.spread_preview_notes(),
            view.snapshot.full_notes,
            "keyboard must include filtered-out voices"
        );
    }
}
#[test]
fn defaults_use_auto_strum_zero_sweep_and_unassigned_y_controller() {
    let params = ChordboardParams::default();
    assert_eq!(params.mode.value(), 1);
    assert_eq!(params.strum_ms.value(), 125.0);
    assert_eq!(
        params.config().mappings,
        [
            crate::engine::Mapping::cc(1, 16),
            crate::engine::Mapping::default()
        ]
    );
    assert!(HEADER_SEQ_BYPASS.0 + HEADER_SEQ_BYPASS.2 < TEMPO_SYNC.0);
}
#[test]
fn direct_direction_selects_exact_values() {
    for selected in 0..5 {
        let mut view = view(1, false);
        open_arp(&mut view);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, pattern_rect(selected));
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.arp_pattern.as_ptr(),
                view.params.arp_pattern.preview_normalized(selected as i32)
            ),]
        );
        assert!(view.drag.is_none() && view.edit.is_none());
    }
}
#[test]
fn menu_open_releases_notes_and_memory_repeat_guard_before_swallowing_keyup() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    view.pressed[0] = true;
    view.memory_held[0] = true;
    view.open_menu(Menu::Scale);
    assert!(!view.pressed[0] && !view.memory_held[0]);
    assert!(std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::ReleaseKeyboard)));
    assert!(view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyUp(CODES[0], None)
    ));
    assert!(view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::Escape, None)
    ));
    assert!(view.menu.is_none());
}
#[test]
fn dropdowns_dismiss_without_close_buttons_or_parameter_changes() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    for menu in [Menu::Key, Menu::Scale] {
        view.open_menu(menu);
        let bounds = view.menu_bounds(menu);
        let header = (bounds.0 + bounds.2 - 80.0, bounds.1 + 4.0, 72.0, 30.0);
        click(&mut view, &mut cx, target, header);
        assert_eq!(view.menu, Some(menu));
        click(&mut view, &mut cx, target, menu.trigger_rect());
        assert!(view.menu.is_none());

        view.open_menu(menu);
        click(&mut view, &mut cx, target, (24.0, 100.0, 50.0, 28.0));
        assert!(view.menu.is_none());

        view.open_menu(menu);
        view.window_event(
            &mut EventContext::new_with_current(&mut cx, target),
            &WindowEvent::KeyDown(Code::Escape, None),
        );
        assert!(view.menu.is_none());
        assert!(view.drag.is_none());
    }
    assert!(changes.borrow().is_empty());
}
#[test]
fn panel_close_buttons_consume_without_changing_parameters() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    for panel in [Panel::Routes, Panel::Mapping] {
        view.set_panel(Some(panel));
        let close = view.panel_close_rect(panel);
        click(&mut view, &mut cx, target, close);
        assert!(view.panel.is_none());
        assert!(view.drag.is_none());
    }
    assert!(changes.borrow().is_empty());
}
#[test]
fn save_requires_a_chord_and_confirms_only_after_slot_update() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, memory_rect(0));
    assert!(!view.memory_ui.armed);
    view.request_capture(0);
    assert!(view.bridge.commands.pop().is_none());
    let word = SavedChord {
        root: 60,
        second: None,
        quality: 0,
        inversion: 0,
        spread: 0,
        transpose: 0,
    }
    .encode();
    view.snapshot.captured = word;
    click(&mut view, &mut cx, target, memory_rect(0));
    assert!(!view.memory_ui.armed);
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::Capture(0))
    ));
    view.tick_memories(0.1);
    assert_eq!(view.memory_ui.flash[0], 0.0);
    view.params.slot(0).store(word, Ordering::Relaxed);
    view.tick_memories(0.1);
    assert!(view.memory_ui.flash[0] > 0.0);
}
#[test]
fn left_performance_targets_do_not_overlap() {
    let mut targets = vec![
        LATCH,
        ORDER,
        Menu::Key.trigger_rect(),
        Menu::Scale.trigger_rect(),
    ];
    targets.extend([ALWAYS_BASS, AFFECT_CHORDS]);
    targets.push(protocol_rect(0));
    targets.extend((0..KEY_COUNT).map(key_rect));
    targets.extend((0..MEMORY_COUNT).map(memory_rect));
    targets.extend((0..2).map(inversion_rect));
    targets.extend((0..TRANSPOSE_STEPS.len()).map(transpose_rect));
    for (i, a) in targets.iter().enumerate() {
        assert!(a.0 >= 0.0 && a.1 >= 0.0 && a.0 + a.2 <= W && a.1 + a.3 <= H);
        for (j, b) in targets.iter().enumerate().skip(i + 1) {
            let overlap = a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3;
            assert!(
                !overlap,
                "performance targets {i} and {j} overlap: {a:?} {b:?}"
            );
        }
    }
}
#[test]
fn direct_selection_pairs_parameter_gestures() {
    let mut view = view(1, false);
    let (mut cx, target, _) = context();
    let gestures = Rc::new(RefCell::new(Vec::new()));
    let observed = gestures.clone();
    cx.add_global_listener(move |_, event| {
        event.map(|event: &RawParamEvent, _| {
            let phase = match event {
                RawParamEvent::BeginSetParameter(ptr) => Some((*ptr, 0)),
                RawParamEvent::SetParameterNormalized(ptr, _) => Some((*ptr, 1)),
                RawParamEvent::EndSetParameter(ptr) => Some((*ptr, 2)),
                _ => None,
            };
            if let Some(phase) = phase {
                observed.borrow_mut().push(phase);
            }
        });
    });
    for (menu, ptr) in [(Menu::Scale, view.params.scale.as_ptr())] {
        view.open_menu(menu);
        click(&mut view, &mut cx, target, menu.option_rect(1));
        assert_eq!(&*gestures.borrow(), &[(ptr, 0), (ptr, 1), (ptr, 2)]);
        gestures.borrow_mut().clear();
    }
    for (r, ptr) in [
        (view.voicing_bounds().unwrap(), view.params.spread.as_ptr()),
        (
            (
                PIANO_LEADING.0 + 100.0,
                PIANO_LEADING.1,
                20.0,
                PIANO_LEADING.3,
            ),
            view.params.voice_leading.as_ptr(),
        ),
    ] {
        click(&mut view, &mut cx, target, r);
        assert_eq!(&*gestures.borrow(), &[(ptr, 0), (ptr, 1), (ptr, 2)]);
        gestures.borrow_mut().clear();
    }
    open_arp(&mut view);
    let ptr = view.params.arp_pattern.as_ptr();
    click(&mut view, &mut cx, target, pattern_rect(2));
    assert_eq!(&*gestures.borrow(), &[(ptr, 0), (ptr, 1), (ptr, 2)]);
}
#[test]
fn displayed_units_round_trip_through_value_entry() {
    let view = view(1, false);
    for (id, text, expected) in [
        (
            "velocity",
            "42%",
            view.params.velocity.preview_normalized(0.42),
        ),
        (
            "humanize",
            "30",
            view.params.humanize.preview_normalized(0.3),
        ),
        (
            "contour",
            "-25%",
            view.params.contour.preview_normalized(-0.25),
        ),
        (
            "length_ms",
            "350 ms",
            view.params.length_ms.preview_normalized(350.0),
        ),
        (
            "strum_ms",
            "80",
            view.params.strum_ms.preview_normalized(80.0),
        ),
        (
            "bend_range",
            "48 st",
            view.params.bend_range.preview_normalized(48.0),
        ),
    ] {
        let c = view.control(id).unwrap();
        assert!(
            (view.parse_display_value(&c, text).unwrap() - expected).abs() < 0.00001,
            "{id}: {text}"
        );
        assert!(
            (view
                .parse_display_value(&c, &view.display_value(&c))
                .unwrap()
                - c.norm)
                .abs()
                < 0.00001,
            "{id} display failed round trip"
        );
    }
    let c = view.control("velocity").unwrap();
    assert!(view.parse_display_value(&c, "NaN%").is_none());
    assert!(view.parse_display_value(&c, "loud").is_none());
}
#[test]
fn all_popover_choices_fit_inside_the_window_and_clear_the_heading() {
    for menu in [
        Menu::Key,
        Menu::Scale,
        Menu::Scale,
        Menu::YTarget,
        Menu::StrumRate,
        Menu::MappingKind,
        Menu::MappingChannel(true),
        Menu::MappingChannel(false),
        Menu::MappingCc(true),
        Menu::MappingCc(false),
    ] {
        let bounds = menu.bounds();
        assert!(
            bounds.0 >= 0.0
                && bounds.1 >= 0.0
                && bounds.0 + bounds.2 <= W
                && bounds.1 + bounds.3 <= H,
            "{menu:?}"
        );
        for i in 0..menu.items().len() {
            let r = menu.option_rect(i);
            assert!(
                r.0 >= bounds.0
                    && r.1 >= bounds.1 + 34.0
                    && r.0 + r.2 <= bounds.0 + bounds.2
                    && r.1 + r.3 <= bounds.1 + bounds.3,
                "{menu:?} choice {i}"
            );
        }
    }
}
#[test]
fn hover_and_action_animation_settle_without_midi_commands() {
    let mut view = view(1, false);
    view.pointer = Some((ORDER.0 + 2.0, ORDER.1 + 2.0));
    assert!(view.tick_motion(0.1));
    assert_eq!(view.hover_amount(ORDER), 1.0);
    assert!(!view.tick_motion(0.1));
    view.flash_press(ORDER.0 + 2.0, ORDER.1 + 2.0);
    assert!(view.tick_motion(0.18));
    assert!(view.action_flash.is_none());
    view.pointer = None;
    assert!(view.tick_motion(0.1));
    assert_eq!(view.hover_amount(ORDER), 0.0);
    assert!(!view.tick_motion(0.1));
    assert!(view.bridge.commands.pop().is_none());
}
#[test]
fn split_drags_suppress_other_hovers_until_release() {
    let mut view = view(1, false);
    let split = view.split_marker();
    view.pointer = Some((split.0 + 1.0, split.1 + 1.0));
    view.tick_motion(0.1);
    assert_eq!(view.hover_amount(split), 1.0);

    for drag in [Drag::Split, Drag::BassSplit] {
        view.drag = Some(drag);
        // Even a previously animated hover disappears immediately on drag capture.
        assert_eq!(view.hover_amount(split), 0.0);
        view.pointer = Some((PIANO_LEADING.0 + 2.0, PIANO_LEADING.1 + 2.0));
        assert_eq!(view.hover_pointer(), None);
        view.tick_motion(0.1);
        assert_eq!(view.hover_amount(PIANO_LEADING), 0.0);
        view.drag = None;
        assert_eq!(view.hover_amount(split), 0.0);
        assert_eq!(view.hover_amount(PIANO_LEADING), 0.0);
    }

    view.drag = None;
    assert_eq!(view.hover_pointer(), view.pointer);
    view.tick_motion(0.1);
    assert_eq!(view.hover_amount(PIANO_LEADING), 1.0);
    assert!(view.bridge.commands.pop().is_none());
}
