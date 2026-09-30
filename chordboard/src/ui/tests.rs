use super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};

type Changes = Rc<RefCell<Vec<(ParamPtr, f32)>>>;

fn view(mode: i32, mpe: bool) -> ChordboardView {
    ChordboardView::new(
        Arc::new(ChordboardParams {
            mode: IntParam::new("Play mode", mode, IntRange::Linear { min: 0, max: 3 }),
            output_mode: IntParam::new(
                "Output protocol",
                if mpe { 1 } else { 2 },
                IntRange::Linear { min: 0, max: 2 },
            ),
            ..ChordboardParams::default()
        }),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    )
}
fn context() -> (Context, Entity, Changes) {
    let mut cx = Context::default();
    let target = Element::new(&mut cx).entity();
    BackendContext::new(&mut cx)
        .cache()
        .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, W, H));
    let changes: Changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    cx.add_global_listener(move |_, event| {
        event.map(|event: &RawParamEvent, _| {
            if let RawParamEvent::SetParameterNormalized(ptr, value) = event {
                observed.borrow_mut().push((*ptr, *value));
            }
        });
    });
    (cx, target, changes)
}
fn event(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    x: f32,
    y: f32,
    event: WindowEvent,
) {
    let mut backend = BackendContext::new_with_event_manager(cx);
    backend.emit_origin(WindowEvent::MouseMove(x, y));
    backend.process_events();
    view.window_event(&mut EventContext::new_with_current(cx, target), &event);
    BackendContext::new_with_event_manager(cx).process_events();
}
fn click(view: &mut ChordboardView, cx: &mut Context, target: Entity, r: Rect) {
    let x = r.0 + r.2 / 2.0;
    let y = r.1 + r.3 / 2.0;
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
}

#[test]
fn rhythm_controls_and_pointer_targets_follow_active_mode() {
    for mode in 1..4 {
        let mut view = view(mode, false);
        let ids: Vec<_> = view.base_controls().iter().map(|(c, _)| c.id).collect();
        for id in ["quality", "spread"] {
            assert!(ids.contains(&id), "mode {mode}: {id}");
        }
        for id in ["humanize", "gate", "swing"] {
            assert_eq!(ids.contains(&id), mode == 3, "mode {mode}: {id}");
        }
        assert!(!ids.contains(&"direction"));
        for id in ["strum_ms", "contour"] {
            assert_eq!(ids.contains(&id), mode == 1, "mode {mode}: {id}");
        }
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, rate_rect(2));
        assert_eq!(
            changes
                .borrow()
                .iter()
                .any(|(ptr, _)| *ptr == view.params.rate.as_ptr()),
            mode == 3
        );
    }
}

#[test]
fn output_protocol_menu_selects_three_modes() {
    for selected in 0..3 {
        let mut view = view(0, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, MPE);
        assert!(view.panel.is_none());
        assert!(view.menu == Some(Menu::Protocol));
        assert!(changes.borrow().is_empty());
        click(
            &mut view,
            &mut cx,
            target,
            Menu::Protocol.option_rect(selected),
        );
        assert!(view.menu.is_none());
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.output_mode.as_ptr(),
                view.params.output_mode.preview_normalized(selected as i32)
            )]
        );
    }
}

#[test]
fn mapping_anchor_edits_the_selected_axis_without_touching_playback() {
    let mut view = view(3, false);
    let (mut cx, target, changes) = context();
    let original_x = view.params.map_x.load(Ordering::Relaxed);
    click(&mut view, &mut cx, target, mapping_summary_rect(1));
    assert_eq!(view.panel, Some(Panel::Mapping));
    assert_eq!(view.mapping_axis, 1);
    click_menu_trigger(&mut view, &mut cx, target, Menu::MappingKind);
    click_menu_option(&mut view, &mut cx, target, Menu::MappingKind, 1);
    click_menu_trigger(&mut view, &mut cx, target, Menu::MappingCc(false));
    click_menu_option(&mut view, &mut cx, target, Menu::MappingCc(false), 74);
    assert_eq!(view.mapping().kind, 1);
    assert_eq!(view.mapping().number, 74);
    assert_eq!(view.params.map_x.load(Ordering::Relaxed), original_x);
    assert!(
        changes.borrow().is_empty(),
        "popup clicks changed covered performance parameters"
    );
    let learn = mapping_learn_rect(view.mapping_axis);
    click(&mut view, &mut cx, target, learn);
    assert!(
        std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::Learn(2)))
    );
    click(&mut view, &mut cx, target, mapping_summary_rect(0));
    assert_eq!(view.mapping_axis, 0);
    assert!(
        std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::Learn(0)))
    );
}

#[test]
fn local_editor_blocks_covered_controls_and_escape_has_no_action() {
    let mut view = view(3, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, OUTPUT);
    click(&mut view, &mut cx, target, rate_rect(7));
    assert!(changes.borrow().is_empty(), "covered arp rate was changed");
    assert!(view
        .placed_controls()
        .iter()
        .all(|(c, _)| !matches!(c.id, "humanize" | "gate" | "swing")));
    event(
        &mut view,
        &mut cx,
        target,
        800.0,
        250.0,
        WindowEvent::KeyDown(Code::Digit1, None),
    );
    assert!(!view.pressed[0]);
    event(
        &mut view,
        &mut cx,
        target,
        800.0,
        250.0,
        WindowEvent::KeyDown(Code::Escape, None),
    );
    assert_eq!(view.panel, Some(Panel::Output));
    click(&mut view, &mut cx, target, OUTPUT);
    assert!(view.panel.is_none());
}

#[test]
fn output_options_match_the_active_midi_protocol() {
    for mpe in [false, true] {
        let mut view = view(0, mpe);
        view.set_panel(Some(Panel::Output));
        let ids: Vec<_> = view.panel_controls().iter().map(|(c, _)| c.id).collect();
        assert!(ids.contains(&"filter"));
        assert_eq!(ids.contains(&"members"), mpe);
        assert_eq!(ids.contains(&"output_channel"), !mpe);
        assert_eq!(ids.contains(&"split_channels"), !mpe);
    }
}

#[test]
fn mapping_menu_blocks_scroll_and_text_edit_on_covered_controls() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, mapping_summary_rect(0));
    click_menu_trigger(&mut view, &mut cx, target, Menu::MappingCc(false));
    assert!(view.menu.is_some());
    let r = mapping_control_rect(0, "x_reverse");
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + 25.0,
        r.1 + 15.0,
        WindowEvent::MouseScroll(0.0, 1.0),
    );
    let menu = view.menu_bounds(view.menu.unwrap());
    event(
        &mut view,
        &mut cx,
        target,
        menu.0 + 12.0,
        menu.1 + 12.0,
        WindowEvent::MouseDoubleClick(MouseButton::Left),
    );
    assert!(changes.borrow().is_empty());
    assert!(view.edit.is_none());
}

#[test]
fn chord_clicks_work_with_popovers_and_hold_until_release() {
    for panel in [None, Some(Panel::Output), Some(Panel::Mapping)] {
        let mut view = view(0, false);
        view.set_panel(panel);
        let (mut cx, target, _) = context();
        let r = key_rect(0);
        let x = r.0 + r.2 / 2.0;
        let y = r.1 + r.3 / 2.0;
        while view.bridge.commands.pop().is_some() {}
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(view.panel.is_none());
        assert!(std::iter::from_fn(|| view.bridge.commands.pop())
            .any(|c| matches!(c, Command::KeyDown(POINTER_KEY_OFFSET, _, 0))));
        assert!(matches!(view.drag, Some(Drag::Key(POINTER_KEY_OFFSET))));
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::KeyUp(POINTER_KEY_OFFSET))
        ));
    }
}

#[test]
fn direct_learn_y_dropdown_and_transpose_buttons() {
    let mut view = view(0, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, LEARN_OCTAVE);
    assert!(view.panel.is_none());
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::Learn(4))
    ));
    click(&mut view, &mut cx, target, mapping_summary_rect(1));
    click_menu_trigger(&mut view, &mut cx, target, Menu::YTarget);
    click_menu_option(&mut view, &mut cx, target, Menu::YTarget, 5);
    assert!(changes
        .borrow()
        .contains(&(view.params.y_target.as_ptr(), 1.0)));
    changes.borrow_mut().clear();
    for (i, step) in TRANSPOSE_STEPS.iter().enumerate() {
        click(&mut view, &mut cx, target, transpose_rect(i));
        assert_eq!(
            changes.borrow().last().copied(),
            Some((
                view.params.transpose.as_ptr(),
                view.params.transpose.preview_normalized(*step)
            ))
        );
    }
}

#[test]
fn controller_options_live_only_in_their_axis_popup() {
    let mut view = view(0, false);
    assert!(!view
        .base_controls()
        .iter()
        .any(|(c, _)| matches!(c.id, "keyboard_octave" | "x_reverse" | "y_reverse" | "y_cc")));
    view.set_panel(Some(Panel::Mapping));
    for axis in 0..2 {
        view.mapping_axis = axis;
        let controls = view.panel_controls();
        assert_eq!(controls.iter().any(|(c, _)| c.id == "x_reverse"), axis == 0);
        assert_eq!(controls.iter().any(|(c, _)| c.id == "y_reverse"), axis == 1);
        for (_, r) in controls {
            let p = view.panel_rect(Panel::Mapping);
            assert!(r.0 >= p.0 && r.1 >= p.1 && r.0 + r.2 <= p.0 + p.2 && r.1 + r.3 <= p.1 + p.3);
        }
    }
}

// Use Vizia's event manager, which classifies native double/triple clicks before
// dispatch. Calling window_event directly cannot exercise that path.
#[test]
fn native_repeated_clicks_reach_all_chord_and_parameter_buttons() {
    for button in 0..68 {
        if button == 44 || button == 64 {
            continue;
        }
        let mut cx = Context::default();
        let params = Arc::new(ChordboardParams {
            mode: IntParam::new("Play mode", 3, IntRange::Linear { min: 0, max: 3 }),
            ..ChordboardParams::default()
        });
        let bridge = Arc::new(Bridge::default());
        let target = ChordboardView::new(params.clone(), bridge.clone(), Arc::new(PreviewContext))
            .build(&mut cx, |_| {})
            .focusable(true)
            .entity();
        BackendContext::new(&mut cx)
            .cache()
            .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, W, H));
        let changes: Changes = Rc::new(RefCell::new(Vec::new()));
        let observed = changes.clone();
        cx.add_global_listener(move |_, event| {
            event.map(|event: &RawParamEvent, _| {
                if let RawParamEvent::SetParameterNormalized(ptr, value) = event {
                    observed.borrow_mut().push((*ptr, *value));
                }
            });
        });
        let (r, expected) = match button {
            0..=35 => (key_rect(button), None),
            36..=40 => (transpose_rect(button - 36), Some(params.transpose.as_ptr())),
            41..=43 => (mode_rect(button - 41), Some(params.mode.as_ptr())),
            45..=46 => (inversion_rect(button - 45), Some(params.inversion.as_ptr())),
            47..=51 => (pattern_rect(button - 47), Some(params.arp_pattern.as_ptr())),
            52..=59 => (rate_rect(button - 52), Some(params.rate.as_ptr())),
            60..=63 => (octave_rect(button - 60), Some(params.octaves.as_ptr())),
            64 => (MPE, Some(params.output_mode.as_ptr())),
            65 => (LATCH, Some(params.latch.as_ptr())),
            66 => (QWERTY, Some(params.keyboard.as_ptr())),
            _ => (ORDER, Some(params.fifths.as_ptr())),
        };
        for press in 1..=3 {
            EventContext::new_with_current(&mut cx, target).capture();
            let mut backend = BackendContext::new_with_event_manager(&mut cx);
            backend.emit_origin(WindowEvent::MouseMove(r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
            backend.emit_origin(WindowEvent::MouseDown(MouseButton::Left));
            backend.process_events();
            if expected.is_none() {
                assert!(
                    std::iter::from_fn(|| bridge.commands.pop())
                        .any(|c| matches!(c, Command::KeyDown(key, _, _) if key == button as u8 + POINTER_KEY_OFFSET)),
                    "chord click {press} was lost by native dispatch"
                );
            } else {
                assert_eq!(
                    changes
                        .borrow()
                        .iter()
                        .filter(|(ptr, _)| Some(*ptr) == expected)
                        .count(),
                    press,
                    "button {button} click {press} was lost by native dispatch"
                );
            }
            let mut backend = BackendContext::new_with_event_manager(&mut cx);
            backend.emit_origin(WindowEvent::MouseUp(MouseButton::Left));
            backend.process_events();
        }
    }
}

#[test]
fn slider_drags_from_title_track_and_value_without_jumping() {
    for origin in [(20.0, 12.0), (20.0, 28.0), (130.0, 12.0)] {
        let mut view = view(1, false);
        let (mut cx, target, changes) = context();
        let r = strum_controls()
            .into_iter()
            .find(|(id, _)| *id == "strum_ms")
            .unwrap()
            .1;
        let (x, y) = (r.0 + origin.0, r.1 + origin.1);
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(changes.borrow().is_empty(), "press jumped the value");
        event(
            &mut view,
            &mut cx,
            target,
            x + 40.0,
            y,
            WindowEvent::MouseMove(x + 40.0, y),
        );
        let (ptr, value) = changes.borrow().last().copied().unwrap();
        assert_eq!(ptr, view.params.strum_ms.as_ptr());
        assert!(value > 0.0 && value <= 1.0);
        event(
            &mut view,
            &mut cx,
            target,
            x + 40.0,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(view.edit.is_none(), "drag opened text entry");
        assert!(view.drag.is_none());
    }
}

#[test]
fn clicking_a_slider_value_edits_but_clicking_its_title_does_not() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    let r = strum_controls()
        .into_iter()
        .find(|(id, _)| *id == "strum_ms")
        .unwrap()
        .1;
    click(&mut view, &mut cx, target, (r.0, r.1, 60.0, 20.0));
    assert!(view.edit.is_none());
    click(
        &mut view,
        &mut cx,
        target,
        pleasant_ui::slider_value_rect(r),
    );
    assert_eq!(view.edit.as_ref().map(|edit| edit.target), Some("strum_ms"));
    assert!(changes.borrow().is_empty());
}

#[test]
fn mode_slide_retargets_from_current_position_and_blocks_moving_controls() {
    let mut view = view(0, false);
    let (mut cx, target, changes) = context();
    view.params = super::tests::view(3, false).params.clone();
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.page_position > 0.0 && view.page_position < 3.0);
    click(&mut view, &mut cx, target, rate_rect(2));
    assert!(changes.borrow().is_empty());
    let interrupted = view.page_position;
    view.params = super::tests::view(1, false).params.clone();
    view.last_frame = Instant::now();
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.page_start, interrupted);
    for _ in 0..3 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.page_position, 1.0);
}

#[test]
fn latched_input_readout_survives_pointer_release() {
    let mut view = view(0, false);
    view.params = Arc::new(ChordboardParams {
        latch: BoolParam::new("Latch", true),
        ..ChordboardParams::default()
    });
    let mut engine = crate::engine::Engine::default();
    engine.configure(view.params.config(), &mut |_| {});
    engine.command(Command::KeyDown(POINTER_KEY_OFFSET, 60, 0), &mut |_| {});
    engine.command(Command::KeyUp(POINTER_KEY_OFFSET), &mut |_| {});
    view.snapshot = engine.snapshot();
    assert_eq!(view.snapshot.root, -1);
    assert_eq!(view.display_input_notes(), (60, -1));
    engine.panic(&mut |_| {});
    view.snapshot = engine.snapshot();
    assert_eq!(view.display_input_notes(), (-1, -1));
}

#[test]
fn selected_key_transposes_all_chord_rows_and_pointer_notes() {
    let mut view = view(0, false);
    view.params = Arc::new(ChordboardParams {
        key: IntParam::new("Key", 2, IntRange::Linear { min: 0, max: 11 }),
        ..ChordboardParams::default()
    });
    let (mut cx, target, _) = context();
    for row in 0..3 {
        let key = row * KEY_COLUMNS;
        assert_eq!(view.keyboard_root(0), Some(2));
        click(&mut view, &mut cx, target, key_rect(key));
        assert!(std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::KeyDown(_, note, quality) if note == 50 && quality == harmony::row_quality(row))));
        view.play_key(key);
        assert!(std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::KeyDown(_, note, quality) if note == 50 && quality == harmony::row_quality(row))));
    }
}

#[test]
fn tempo_and_sweep_sync_buttons_and_beat_dropdown_emit_parameters() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, TEMPO_SYNC);
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.tempo_sync.as_ptr(), 0.0))
    );
    click(&mut view, &mut cx, target, STRUM_SYNC);
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.strum_sync.as_ptr(), 1.0))
    );
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Mode", 1, IntRange::Linear { min: 0, max: 3 }),
        strum_sync: BoolParam::new("Sweep sync", true),
        ..ChordboardParams::default()
    });
    assert!(!view.base_controls().iter().any(|(c, _)| c.id == "strum_ms"));
    click(&mut view, &mut cx, target, Menu::StrumRate.trigger_rect());
    click(&mut view, &mut cx, target, Menu::StrumRate.option_rect(0));
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.strum_beats.as_ptr(), 1.0))
    );
}

#[test]
fn learn_and_pointer_feedback_work_without_audio_processing() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, LEARN_OCTAVE);
    assert_eq!(
        view.learning(),
        4,
        "Learn must arm without an audio snapshot"
    );
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.learning(), 4, "idle UI ticks must preserve Learn");
    click(&mut view, &mut cx, target, LEARN_OCTAVE);
    assert_eq!(
        view.learning(),
        0,
        "Learn must cancel without audio processing"
    );
    for key in 0..KEY_COUNT {
        let r = key_rect(key);
        event(
            &mut view,
            &mut cx,
            target,
            r.0 + r.2 / 2.0,
            r.1 + r.3 / 2.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(
            view.key_active(key),
            "pointer key {key} must react immediately"
        );
        view.last_frame = Instant::now() - Duration::from_millis(50);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
        assert!(view.key_anim[key] > 0.0);
        event(
            &mut view,
            &mut cx,
            target,
            r.0 + r.2 / 2.0,
            r.1 + r.3 / 2.0,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(
            !view.key_active(key),
            "pointer key {key} must release locally"
        );
    }
}

#[test]
fn learn_feedback_returns_to_audio_state_after_acknowledgement_or_completion() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    view.bridge.visible.store(true, Ordering::Relaxed);
    view.request_learn(4);
    view.bridge.publish(Snapshot {
        learning: 4,
        ..Snapshot::default()
    });
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.pending_learn.is_none());
    assert_eq!(view.learning(), 4);
    view.bridge.publish(Snapshot::default());
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.learning(), 0);
    // A note can complete Learn in the same audio block that arms it.
    view.request_learn(4);
    view.params.control_base.store(36, Ordering::Relaxed);
    view.bridge.publish(Snapshot::default());
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.learning(), 0);
}

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
            .filter(|command| matches!(command, Command::Recall(saved) if *saved == word))
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
            2,
        );
        assert_eq!(view.mapping().kind, 2);
        assert_eq!(view.mapping().channel, 16);
    }
}

#[test]
fn spread_keyboard_cycles_all_voicings_without_drag_or_text_entry() {
    let (mut cx, target, changes) = context();
    for current in 0..3 {
        let mut view = view(2, false);
        view.params = Arc::new(ChordboardParams {
            mode: IntParam::new("Mode", 2, IntRange::Linear { min: 0, max: 3 }),
            spread: IntParam::new("Spread", current, IntRange::Linear { min: 0, max: 2 }),
            ..ChordboardParams::default()
        });
        let r = voicing_controls()[1].1;
        click(&mut view, &mut cx, target, r);
        assert_eq!(
            changes.borrow().last().copied(),
            Some((
                view.params.spread.as_ptr(),
                view.params.spread.preview_normalized((current + 1) % 3)
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
fn defaults_use_auto_strum_zero_sweep_and_expression_cc_on_any_channel() {
    let params = ChordboardParams::default();
    assert_eq!(params.mode.value(), 1);
    assert_eq!(params.strum_ms.value(), 0.0);
    assert_eq!(
        params.config().mappings,
        [
            crate::engine::Mapping::cc(1, 16),
            crate::engine::Mapping::cc(11, 16)
        ]
    );
    assert_eq!(MODE_LABELS, ["Auto Strum", "Manual Strum", "Arpeggiator"]);
}

#[test]
fn quality_menu_selects_without_drag_text_or_scroll_changes() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, Menu::Quality.trigger_rect());
    assert!(view.menu == Some(Menu::Quality));
    assert!(view.drag.is_none() && view.edit.is_none());
    click(&mut view, &mut cx, target, Menu::Quality.option_rect(3));
    assert_eq!(
        changes.borrow().last().copied(),
        Some((
            view.params.quality.as_ptr(),
            view.params.quality.preview_normalized(3)
        ))
    );
    changes.borrow_mut().clear();
    let r = Menu::Quality.trigger_rect();
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + r.2 / 2.0,
        r.1 + r.3 / 2.0,
        WindowEvent::MouseScroll(0.0, 1.0),
    );
    assert!(changes.borrow().is_empty());
}

#[test]
fn direct_direction_selects_exact_values() {
    for selected in 0..3 {
        let mut view = view(1, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, direction_rect(selected));
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.direction.as_ptr(),
                view.params.direction.preview_normalized(selected as i32)
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
    view.open_menu(Menu::Quality);
    assert!(!view.pressed[0] && !view.memory_held[0]);
    assert!(std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::ReleaseKeyboard)));
    assert!(view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyUp(CODES[0], None)
    ));
    assert!(!view.window_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::Escape, None)
    ));
}

#[test]
fn popover_close_buttons_consume_without_changing_parameters() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    for menu in [Menu::Quality, Menu::Protocol] {
        view.open_menu(menu);
        click(&mut view, &mut cx, target, menu.close_rect());
        assert!(view.menu.is_none());
        assert!(view.drag.is_none());
    }
    for panel in [Panel::Output, Panel::Mapping] {
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
    click(&mut view, &mut cx, target, SAVE_MEMORY);
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
    click(&mut view, &mut cx, target, SAVE_MEMORY);
    assert!(view.memory_ui.armed);
    view.request_capture(0);
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
        QWERTY,
        ORDER,
        SAVE_MEMORY,
        Menu::Key.trigger_rect(),
        Menu::Scale.trigger_rect(),
        Menu::Quality.trigger_rect(),
        voicing_controls()[1].1,
        voicing_controls()[2].1,
    ];
    targets.extend((0..KEY_COUNT).map(key_rect));
    targets.extend((0..8).map(memory_rect));
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
    for (menu, ptr) in [
        (Menu::Quality, view.params.quality.as_ptr()),
        (Menu::Protocol, view.params.output_mode.as_ptr()),
    ] {
        view.open_menu(menu);
        click(&mut view, &mut cx, target, menu.option_rect(1));
        assert_eq!(&*gestures.borrow(), &[(ptr, 0), (ptr, 1), (ptr, 2)]);
        gestures.borrow_mut().clear();
    }
    for (r, ptr) in [
        (voicing_controls()[1].1, view.params.spread.as_ptr()),
        (voicing_controls()[2].1, view.params.voice_leading.as_ptr()),
        (direction_rect(2), view.params.direction.as_ptr()),
    ] {
        click(&mut view, &mut cx, target, r);
        assert_eq!(&*gestures.borrow(), &[(ptr, 0), (ptr, 1), (ptr, 2)]);
        gestures.borrow_mut().clear();
    }
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
fn all_popover_choices_fit_inside_the_window_and_clear_the_close_button() {
    for menu in [
        Menu::Key,
        Menu::Scale,
        Menu::Quality,
        Menu::Protocol,
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
        let close = menu.close_rect();
        for i in 0..menu.items().len() {
            let r = menu.option_rect(i);
            assert!(
                r.0 >= bounds.0
                    && r.1 >= close.1 + close.3
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

fn click_menu_trigger(view: &mut ChordboardView, cx: &mut Context, target: Entity, menu: Menu) {
    let r = view.menu_trigger_rect(menu);
    click(view, cx, target, r);
}
fn click_menu_option(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    menu: Menu,
    index: usize,
) {
    let r = view.menu_option_rect(menu, index);
    click(view, cx, target, r);
}

#[test]
fn compact_mapping_controls_fit_each_axis_and_close_without_parameter_changes() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    for axis in 0..2 {
        click(&mut view, &mut cx, target, mapping_summary_rect(axis));
        let panel = view.panel_rect(Panel::Mapping);
        assert_eq!(panel.2, 384.0);
        assert_eq!(panel.3, if axis == 0 { 104.0 } else { 180.0 });
        assert_eq!(panel.1 + panel.3 + 6.0, mapping_summary_rect(axis).1);
        let mut rects: Vec<_> = view.panel_controls().iter().map(|(_, r)| *r).collect();
        rects.extend([
            view.menu_trigger_rect(Menu::MappingKind),
            view.menu_trigger_rect(Menu::MappingCc(false)),
            mapping_learn_rect(axis),
            view.panel_close_rect(Panel::Mapping),
        ]);
        if axis == 1 {
            rects.push(view.menu_trigger_rect(Menu::YTarget));
        }
        for r in &rects {
            assert!(
                r.0 >= panel.0
                    && r.1 >= panel.1
                    && r.0 + r.2 <= panel.0 + panel.2
                    && r.1 + r.3 <= panel.1 + panel.3
            );
        }
        for (i, a) in rects.iter().enumerate() {
            for b in rects.iter().skip(i + 1) {
                assert!(
                    a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1,
                    "overlapping compact mapping controls: {a:?}, {b:?}"
                );
            }
        }
        let close = view.panel_close_rect(Panel::Mapping);
        click(&mut view, &mut cx, target, close);
        assert!(view.panel.is_none());
        assert!(changes.borrow().is_empty());
    }
}

fn pad_center() -> (f32, f32) {
    (PLAY_PAD.0 + PLAY_PAD.2 / 2.0, PLAY_PAD.1 + PLAY_PAD.3 / 2.0)
}

fn drain_commands(view: &ChordboardView) -> Vec<Command> {
    std::iter::from_fn(|| view.bridge.commands.pop()).collect()
}

#[test]
fn trackpad_latch_button_toggles_hover_strum_without_a_click() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, STRUM_LATCH);
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.strum_latch.as_ptr(), 1.0))
    );

    let (x, y) = pad_center();
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert!(drain_commands(&view).is_empty());

    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert!(view.pad_hover);
    assert!(view.drag.is_none());
    let commands = drain_commands(&view);
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::BeginGesture(px, py) if (px - 0.5).abs() < 0.001 && (py - 0.5).abs() < 0.001)),
        "{commands:?}"
    );
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.y.as_ptr(), 0.5))
    );

    let nx = PLAY_PAD.0 + PLAY_PAD.2 * 0.75;
    event(
        &mut view,
        &mut cx,
        target,
        nx,
        y,
        WindowEvent::MouseMove(nx, y),
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, value)| { *ptr == view.params.x.as_ptr() && (*value - 0.75).abs() < 0.001 }));
    assert!(drain_commands(&view)
        .iter()
        .all(|c| !matches!(c, Command::BeginGesture(_, _) | Command::EndGesture)));

    event(
        &mut view,
        &mut cx,
        target,
        PLAY_PAD.0 - 20.0,
        y,
        WindowEvent::MouseMove(PLAY_PAD.0 - 20.0, y),
    );
    assert!(!view.pad_hover);
    assert!(drain_commands(&view)
        .iter()
        .any(|c| matches!(c, Command::EndGesture)));
}

#[test]
fn trackpad_latch_hover_promotes_to_a_captured_drag_once() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    let (mut cx, target, _) = context();
    let (x, y) = pad_center();
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert_eq!(
        drain_commands(&view)
            .iter()
            .filter(|c| matches!(c, Command::BeginGesture(_, _)))
            .count(),
        1
    );
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::Pad)));
    assert!(!view.pad_hover);
    assert!(drain_commands(&view)
        .iter()
        .all(|c| !matches!(c, Command::BeginGesture(_, _) | Command::EndGesture)));
}

#[test]
fn trackpad_latch_skips_range_grips_and_still_lets_them_calibrate() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    let (mut cx, target, _) = context();
    let r = strum_bound_rect(0.0);
    let x = r.0 + r.2 / 2.0;
    let y = r.1 + r.3 / 2.0;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert!(!view.pad_hover);
    assert!(drain_commands(&view).is_empty());
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::StrumBound(false, false))));
    assert!(drain_commands(&view).is_empty());
}

#[test]
fn trackpad_latch_hover_follows_an_expanded_strum_field() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.expand_elapsed = pleasant_ui::page_slide::DURATION;
    let (mut cx, target, _) = context();
    let play = view.play_pad();
    let x = play.0 + play.2 / 2.0;
    let y = play.1 + play.3 / 2.0;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert_eq!(play, EXPANDED_PLAY_PAD);
    assert!(view.pad_hover);
    assert!(drain_commands(&view)
        .iter()
        .any(|c| matches!(c, Command::BeginGesture(_, _))));
}

#[test]
fn collapsed_strum_geometry_is_unchanged() {
    assert_eq!(pad_rect(0.0), PAD);
    assert_eq!(play_pad_rect(0.0), PLAY_PAD);
    assert_eq!(auto_field_rect(0.0), AUTO_FIELD);
    assert_eq!(expand_rect(0.0), EXPAND);
    assert_eq!(perf_surface_rect(0.0), PERF_SURFACE);
    assert_eq!(strum_sync_rect(0.0), STRUM_SYNC);
    assert_eq!(strum_latch_rect(0.0), STRUM_LATCH);
    let expand = expand_rect(0.0);
    let sync = strum_sync_rect(0.0);
    let latch = strum_latch_rect(0.0);
    assert!(
        expand.0 + expand.2 <= sync.0 || sync.0 + sync.2 <= expand.0,
        "expand button overlaps sweep sync"
    );
    assert!(
        expand.0 + expand.2 <= latch.0 || latch.0 + latch.2 <= expand.0,
        "expand button overlaps trackpad latch"
    );
}

#[test]
fn expanded_strum_field_fills_the_body_below_the_header() {
    let pad = pad_rect(1.0);
    let play = play_pad_rect(1.0);
    let performance = perf_surface_rect(1.0);
    assert_eq!(pad, EXPANDED_PAD);
    assert_eq!(play, EXPANDED_PLAY_PAD);
    assert!(performance.1 >= HEADER_H);
    assert!(pad.1 >= HEADER_H);
    assert!(pad.0 + pad.2 <= W);
    assert!(pad.1 + pad.3 <= H);
    assert!(play.0 >= pad.0 && play.0 + play.2 <= pad.0 + pad.2);
    assert!(play.1 >= pad.1 && play.1 + play.3 <= pad.1 + pad.3);
    let expand = expand_rect(1.0);
    let latch = strum_latch_rect(1.0);
    assert!(
        expand.0 + expand.2 <= latch.0 || latch.0 + latch.2 <= expand.0,
        "expanded expand button overlaps trackpad latch"
    );
    assert!(pad.2 > PAD.2);
    assert!(pad.3 > PAD.3);
    assert!(pad.0 < PAD.0);
    assert_eq!(shrink_width(CHORDS_SURFACE, 1.0).2, 0.0);
}

#[test]
fn strum_expand_slides_like_composure_pages() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, expand_rect(0.0));
    assert_eq!(view.expand_target, 1.0);
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.expand_progress > 0.0 && view.expand_progress < 1.0);
    let interrupted = view.expand_progress;
    let collapse = view.expand_button();
    click(&mut view, &mut cx, target, collapse);
    assert_eq!(view.expand_target, 0.0);
    assert_eq!(view.expand_start, interrupted);
    for _ in 0..3 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.expand_progress, 0.0);
}

#[test]
fn expanded_strum_consumes_covered_chord_clicks_and_keeps_the_play_pad() {
    let mut view = view(2, false);
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.expand_elapsed = pleasant_ui::page_slide::DURATION;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, key_rect(0));
    assert!(std::iter::from_fn(|| view.bridge.commands.pop())
        .all(|c| !matches!(c, Command::KeyDown(..))));
    let play = view.play_pad();
    click(&mut view, &mut cx, target, play);
    assert!(std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::BeginGesture(..))));
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.x.as_ptr()));
}

#[test]
fn arp_mode_collapses_an_open_strum_field() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.expand_elapsed = pleasant_ui::page_slide::DURATION;
    view.params = super::tests::view(3, false).params.clone();
    view.last_frame = Instant::now();
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.expand_target, 0.0);
    for _ in 0..3 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.expand_progress, 0.0);
}

#[test]
fn routing_panel_owns_covered_targets_and_edits_the_selected_slot() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, ROUTES_BUTTON);
    assert_eq!(view.panel, Some(Panel::Routes));
    click(&mut view, &mut cx, target, route_slot_rect(7));
    assert_eq!(view.route_slot, 7);
    click(&mut view, &mut cx, target, ROUTE_SOURCE);
    assert_eq!(view.menu, Some(Menu::RouteSource));
    let choice = view.menu_option_rect(Menu::RouteSource, 2);
    click(&mut view, &mut cx, target, choice);
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.routes[7].source.as_ptr()));
    assert!(!changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.routes[0].source.as_ptr()));
    // Covered pad space cannot start a strum through the smaller editor.
    click(&mut view, &mut cx, target, (740.0, 392.0, 16.0, 16.0));
    assert_eq!(view.panel, Some(Panel::Routes));
}

#[test]
fn route_ranges_use_destination_units_and_route_slots_fit() {
    let view = view(2, false);
    let c = view.control("route_min").unwrap();
    assert_eq!(view.display_value(&c), "3");
    assert!((view.parse_display_value(&c, "8").unwrap() - 5.0 / 9.0).abs() < 1e-6);
    for i in 0..crate::engine::routing::ROUTE_COUNT {
        let r = route_slot_rect(i);
        assert!(hit(Panel::Routes.rect(), r.0, r.1));
        assert!(hit(Panel::Routes.rect(), r.0 + r.2 - 1.0, r.1 + r.3 - 1.0));
    }
    for menu in [Menu::RouteSource, Menu::RouteTarget] {
        let r = menu.bounds();
        assert!(r.0 >= 0.0 && r.1 >= 0.0 && r.0 + r.2 <= W && r.1 + r.3 <= H);
    }
}

#[test]
fn source_meter_click_is_inert_and_root_toggle_is_available_in_all_modes() {
    for mode in 1..=3 {
        let mut view = view(mode, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, ROOT_ON_SELECT);
        assert!(changes
            .borrow()
            .iter()
            .any(|(p, v)| *p == view.params.root_on_select.as_ptr() && *v == 1.0));
        changes.borrow_mut().clear();
        click(&mut view, &mut cx, target, meter_rect(3));
        assert_eq!(view.panel, None);
        assert!(view.drag.is_none());
        assert!(changes.borrow().is_empty());
    }
}

fn route_drag_to(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    source: usize,
    x: f32,
    y: f32,
) {
    let r = meter_rect(source);
    event(
        view,
        cx,
        target,
        r.0 + r.2 / 2.0,
        r.1 + r.3 / 2.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(view, cx, target, x, y, WindowEvent::MouseMove(x, y));
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
}

#[test]
fn route_drag_links_only_on_drop_and_never_moves_the_destination_control() {
    for (source, id) in [(0, "strings"), (3, "spread"), (8, "quality")] {
        let mut view = view(2, false);
        let (mut cx, entity, changes) = context();
        let r = meter_rect(source);
        let (_, destination) = view
            .base_controls()
            .into_iter()
            .find(|(c, _)| c.id == id)
            .unwrap();
        let (x, y) = (
            destination.0 + destination.2 / 2.0,
            destination.1 + destination.3 / 2.0,
        );
        event(
            &mut view,
            &mut cx,
            entity,
            r.0 + 20.0,
            r.1 + 20.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        assert!(changes.borrow().is_empty());
        assert!(matches!(view.drag, Some(Drag::Route(drag)) if drag.active));
        assert!(view.route_drag_hint().unwrap().starts_with("Link "));
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert_eq!(view.panel, Some(Panel::Routes));
        let route = &view.params.routes[0];
        let destination_index = crate::engine::routing::TARGETS
            .iter()
            .position(|t| t.id == id)
            .unwrap();
        assert!(changes.borrow().contains(&(
            route.source.as_ptr(),
            route.source.preview_normalized(source as i32 + 1)
        )));
        assert!(changes.borrow().contains(&(
            route.target.as_ptr(),
            route.target.preview_normalized(destination_index as i32)
        )));
        assert!(!changes
            .borrow()
            .iter()
            .any(|(p, _)| *p == view.control(id).unwrap().ptr));
        assert!(view.drag.is_none() && view.edit.is_none() && view.menu.is_none());
    }
}

#[test]
fn route_drag_cancel_jitter_and_focus_loss_never_assign_or_strum() {
    for cancel in [0, 1, 2, 3] {
        let mut view = view(2, false);
        Arc::get_mut(&mut view.params).unwrap().strum_latch =
            BoolParam::new("Trackpad latch", true);
        let (mut cx, entity, changes) = context();
        let r = meter_rect(1);
        event(
            &mut view,
            &mut cx,
            entity,
            r.0 + 20.0,
            r.1 + 20.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        let (x, y) = if cancel == 0 {
            (r.0 + 22.0, r.1 + 21.0)
        } else if cancel == 1 {
            (580.0, 580.0)
        } else {
            (850.0, 400.0)
        };
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        if cancel == 2 {
            event(
                &mut view,
                &mut cx,
                entity,
                x,
                y,
                WindowEvent::KeyDown(Code::Escape, None),
            );
        } else if cancel == 3 {
            event(&mut view, &mut cx, entity, x, y, WindowEvent::FocusOut);
        }
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(changes.borrow().is_empty());
        assert!(view.drag.is_none() && !view.pad_hover);
        assert!(
            !std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(
                c,
                Command::BeginGesture(_, _) | Command::X(_) | Command::Y(_)
            ))
        );
    }
}

#[test]
fn route_drag_reuses_existing_destination_and_preserves_range_and_curve() {
    for source in [0, 3] {
        let mut view = view(2, false);
        let route = &mut Arc::get_mut(&mut view.params).unwrap().routes[6];
        route.source = IntParam::new("Source", 1, IntRange::Linear { min: 0, max: 9 });
        route.min = FloatParam::new("Minimum", 0.25, FloatRange::Linear { min: 0.0, max: 1.0 });
        route.curve = FloatParam::new(
            "Curve",
            -0.4,
            FloatRange::Linear {
                min: -1.0,
                max: 1.0,
            },
        );
        let (mut cx, entity, changes) = context();
        route_drag_to(&mut view, &mut cx, entity, source, 1010.0, 500.0);
        assert_eq!(view.route_slot, 6);
        assert_eq!(view.panel, Some(Panel::Routes));
        let expected = if source == 0 {
            vec![]
        } else {
            vec![(view.params.routes[6].source.as_ptr(), 4.0 / 9.0)]
        };
        assert_eq!(*changes.borrow(), expected);
    }
}

#[test]
fn route_drag_respects_popover_occlusion_and_exposes_output_destinations() {
    let mut view = view(2, false);
    let (mut cx, entity, changes) = context();
    view.set_panel(Some(Panel::Output));
    assert_eq!(view.route_target_at(850.0, 135.0), None); // Mode buttons are covered.
    let (_, rect) = view
        .panel_controls()
        .into_iter()
        .find(|(c, _)| c.id == "output_channel")
        .unwrap();
    route_drag_to(&mut view, &mut cx, entity, 1, rect.0 + 25.0, rect.1 + 20.0);
    assert_eq!(view.panel, Some(Panel::Routes));
    assert!(changes
        .borrow()
        .iter()
        .any(|(p, _)| *p == view.params.routes[0].source.as_ptr()));
    changes.borrow_mut().clear();
    // Starting a new drag folds the route editor away so Strings is reachable.
    route_drag_to(&mut view, &mut cx, entity, 2, 1010.0, 500.0);
    assert!(changes
        .borrow()
        .iter()
        .any(|(p, _)| *p == view.params.routes[0].target.as_ptr()));
}

#[test]
fn route_drag_with_no_free_slots_does_not_overwrite_an_unrelated_link() {
    let mut view = view(2, false);
    for route in &mut Arc::get_mut(&mut view.params).unwrap().routes {
        route.source = IntParam::new("Source", 1, IntRange::Linear { min: 0, max: 9 });
    }
    let (mut cx, entity, changes) = context();
    route_drag_to(&mut view, &mut cx, entity, 1, 400.0, 570.0); // Spread, all slots target Strings.
    assert!(changes.borrow().is_empty());
    assert!(view.status.starts_with("All 16 routes"));
    assert!(view.panel.is_none());
}

#[test]
fn auto_strum_cannot_expand_and_collapses_the_manual_field() {
    let mut view = view(1, false);
    let (mut cx, target, _) = context();
    assert!(!view.can_expand_strum());
    click(&mut view, &mut cx, target, expand_rect(0.0));
    assert_eq!(view.expand_target, 0.0);
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.expand_target, 0.0);
}

#[test]
fn modulator_drop_targets_include_auto_amount_and_manual_sweep() {
    for (mode, id) in [(1, "strings_played"), (2, "x")] {
        let mut view = view(mode, false);
        let (mut cx, entity, changes) = context();
        let target = crate::engine::routing::TARGETS
            .iter()
            .position(|t| t.id == id)
            .unwrap();
        let (_, r) = view
            .route_targets()
            .into_iter()
            .find(|(i, _)| *i == target)
            .unwrap();
        view.finish_route_drag(
            &mut EventContext::new_with_current(&mut cx, entity),
            1,
            r.0 + r.2 / 2.0,
            r.1 + r.3 / 2.0,
        );
        BackendContext::new_with_event_manager(&mut cx).process_events();
        let route = &view.params.routes[0];
        assert!(changes.borrow().contains(&(
            route.target.as_ptr(),
            route.target.preview_normalized(target as i32)
        )));
        assert!(changes
            .borrow()
            .contains(&(route.source.as_ptr(), route.source.preview_normalized(2))));
    }
}

#[test]
fn route_graph_nodes_edit_selected_parameters_and_release_capture() {
    for node in 0..3 {
        let mut view = view(2, false);
        view.panel = Some(Panel::Routes);
        view.route_slot = 7;
        let (mut cx, target, changes) = context();
        let (x, y) = view.route_nodes()[node];
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        assert!(matches!(view.drag, Some(Drag::RouteNode(..))));
        let y = ROUTE_GRAPH.1 + ROUTE_GRAPH.3 * 0.75;
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseMove(x, y));
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseUp(MouseButton::Left));
        let r = &view.params.routes[7];
        let (ptr, expected) = match node {
            0 => (r.min.as_ptr(), 0.25),
            2 => (r.max.as_ptr(), 0.25),
            _ => (r.curve.as_ptr(), r.curve.preview_normalized(1.0 / 3.0)),
        };
        assert!(changes.borrow().iter().any(|(p,v)| *p == ptr && (v-expected).abs() < 0.0001));
        assert!(view.drag.is_none());
        assert!(!std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::BeginGesture(..))));
    }
}
#[test]
fn route_graph_curve_handles_reversed_and_flat_ranges() {
    let mut view = view(2, false);
    let r = &mut Arc::get_mut(&mut view.params).unwrap().routes[0];
    r.min = FloatParam::new("Minimum", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    r.max = FloatParam::new("Maximum", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    let value = view.route_node_value(1, ROUTE_GRAPH.1 + ROUTE_GRAPH.3 * 0.25).unwrap();
    assert!((value - view.params.routes[0].curve.preview_normalized(1.0 / 3.0)).abs() < 0.0001);
    Arc::get_mut(&mut view.params).unwrap().routes[0].max = FloatParam::new("Maximum", 1.0,
        FloatRange::Linear { min: 0.0, max: 1.0 });
    assert_eq!(view.route_node_value(1, ROUTE_GRAPH.1), None);
}

#[test]
fn route_destination_menu_skips_touch_bounds_without_shifting_saved_ids() {
    let mut view = view(2, false);
    let (mut cx, entity, changes) = context();
    for (menu_index, (saved_index, target)) in crate::engine::routing::available_targets().enumerate() {
        assert_eq!(Menu::RouteTarget.items()[menu_index], target.name);
        view.select_menu(&mut EventContext::new_with_current(&mut cx, entity), Menu::RouteTarget, menu_index);
        BackendContext::new_with_event_manager(&mut cx).process_events();
        let p = &view.params.routes[0].target;
        assert!(changes.borrow().contains(&(p.as_ptr(), p.preview_normalized(saved_index as i32))));
    }
    assert!(view.route_targets().iter().all(|(i, _)| crate::engine::routing::TARGETS[*i].available()));
}

#[test]
fn leading_animation_restarts_when_a_new_transition_arrives() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    view.bridge.snapshots.push(Snapshot { leading_serial: 1, ..Snapshot::default() }).unwrap();
    view.last_frame = Instant::now() - Duration::from_millis(50);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.leading_progress < 0.2);
    for _ in 0..8 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.leading_progress, 1.0);
    view.bridge.snapshots.push(Snapshot { leading_serial: 2, ..Snapshot::default() }).unwrap();
    view.last_frame = Instant::now() - Duration::from_millis(50);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.leading_progress < 0.2);
}

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
    assert!(!std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::KeyDown(..) | Command::BeginGesture(..))));
}

#[test]
fn full_piano_buttons_work_in_every_mode_and_expanded_manual() {
    for mode in 1..4 {
        let mut view = view(mode, false);
        if mode == 2 { view.expand_progress = 1.0; }
        let (mut cx, target, changes) = context();
        for (r, ptr) in [(ALWAYS_BASS, view.params.always_bass.as_ptr()), (ALWAYS_CHORD, view.params.always_chord.as_ptr()), (KEY_SPLIT, view.params.key_split.as_ptr())] {
            click(&mut view, &mut cx, target, r);
            assert_eq!(changes.borrow().last().copied(), Some((ptr, 1.0)));
        }
    }
}

#[test]
fn piano_split_drag_selects_accidentals_and_clamps_at_midi_edges() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams { key_split: BoolParam::new("Key split", true), ..ChordboardParams::default() });
    let (mut cx, target, changes) = context();
    let r = piano_key_rect(61);
    event(&mut view, &mut cx, target, r.0 + r.2 / 2.0, r.1 + 8.0, WindowEvent::MouseDown(MouseButton::Left));
    assert!(matches!(view.drag, Some(Drag::Split)));
    assert_eq!(changes.borrow().last().copied(), Some((view.params.split_note.as_ptr(), view.params.split_note.preview_normalized(61))));
    for (x, note) in [(-10.0, 0), (W + 10.0, 127)] {
        event(&mut view, &mut cx, target, x, PIANO_LEADING.1, WindowEvent::MouseMove(x, PIANO_LEADING.1));
        assert_eq!(changes.borrow().last().copied(), Some((view.params.split_note.as_ptr(), view.params.split_note.preview_normalized(note))));
    }
    event(&mut view, &mut cx, target, W + 10.0, PIANO_LEADING.1, WindowEvent::MouseUp(MouseButton::Left));
    assert!(view.drag.is_none());
    assert!(!std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::KeyDown(..) | Command::BeginGesture(..))));
}

#[test]
fn piano_tooltips_distinguish_silent_controls_and_live_notes() {
    let mut view = view(1, false);
    view.params.control_base.store(24, Ordering::Relaxed);
    let r = piano_key_rect(27);
    let hint = view.piano_hint(r.0 + r.2 / 2.0, r.1 + 8.0).unwrap();
    assert!(hint.contains("MIDI 27") && hint.contains("silent") && hint.contains(harmony::CONTROL_LABELS[3]));
    view.snapshot.held_notes[60] = true;
    view.snapshot.sounding_notes[60] = true;
    let r = piano_key_rect(60);
    let hint = view.piano_hint(r.0 + r.2 / 2.0, r.1 + r.3 - 2.0).unwrap();
    assert!(hint.contains("C4") && hint.contains("MIDI 60") && hint.contains("Held input") && hint.contains("Sounding output"));
}
