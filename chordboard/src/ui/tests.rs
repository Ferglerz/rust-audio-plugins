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
        for id in ["direction", "strum_ms", "contour"] {
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
fn output_protocol_button_cycles_three_modes() {
    for mode in 0..3 {
        let mut view = view(0, false);
        view.params = Arc::new(ChordboardParams {
            output_mode: IntParam::new(
                "Output protocol",
                mode,
                IntRange::Linear { min: 0, max: 2 },
            ),
            ..ChordboardParams::default()
        });
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, MPE);
        assert!(view.panel.is_none());
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.output_mode.as_ptr(),
                view.params.output_mode.preview_normalized((mode + 1) % 3)
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
    click(&mut view, &mut cx, target, Menu::MappingKind.trigger_rect());
    click(&mut view, &mut cx, target, Menu::MappingKind.option_rect(1));
    click(
        &mut view,
        &mut cx,
        target,
        Menu::MappingCc(false).trigger_rect(),
    );
    click(
        &mut view,
        &mut cx,
        target,
        Menu::MappingCc(false).option_rect(74),
    );
    assert_eq!(view.mapping().kind, 1);
    assert_eq!(view.mapping().number, 74);
    assert_eq!(view.params.map_x.load(Ordering::Relaxed), original_x);
    assert!(
        changes.borrow().is_empty(),
        "popup clicks changed covered performance parameters"
    );
    click(&mut view, &mut cx, target, LEARN);
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
    click(
        &mut view,
        &mut cx,
        target,
        Menu::MappingCc(false).trigger_rect(),
    );
    assert!(view.menu.is_some());
    let r = mapping_control_rect(0);
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + 25.0,
        r.1 + 15.0,
        WindowEvent::MouseScroll(0.0, 1.0),
    );
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + 25.0,
        r.1 + 15.0,
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
    click(&mut view, &mut cx, target, Menu::YTarget.trigger_rect());
    click(&mut view, &mut cx, target, Menu::YTarget.option_rect(5));
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
            let p = Panel::Mapping.rect();
            assert!(r.0 >= p.0 && r.1 >= p.1 && r.0 + r.2 <= p.0 + p.2 && r.1 + r.3 <= p.1 + p.3);
        }
    }
}

// Use Vizia's event manager, which classifies native double/triple clicks before
// dispatch. Calling window_event directly cannot exercise that path.
#[test]
fn native_repeated_clicks_reach_all_chord_and_parameter_buttons() {
    for button in 0..68 {
        if button == 44 {
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
            _ => ((32.0, 190.0, 154.0, 28.0), Some(params.fifths.as_ptr())),
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
        let r = voicing_controls()[0].1;
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
        assert_eq!(ptr, view.params.quality.as_ptr());
        assert!((value - 0.16).abs() < 0.00001);
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
    let r = voicing_controls()[0].1;
    click(&mut view, &mut cx, target, (r.0, r.1, 60.0, 20.0));
    assert!(view.edit.is_none());
    click(
        &mut view,
        &mut cx,
        target,
        pleasant_ui::slider_value_rect(r),
    );
    assert_eq!(view.edit.as_ref().map(|edit| edit.target), Some("quality"));
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
        click(
            &mut view,
            &mut cx,
            target,
            Menu::MappingChannel(false).trigger_rect(),
        );
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
    assert_eq!(MODE_LABELS, ["AUTO STRUM", "MANUAL STRUM", "ARPEGGIATOR"]);
}
