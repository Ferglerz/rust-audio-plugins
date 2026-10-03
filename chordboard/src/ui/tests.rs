use super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};

type Changes = Rc<RefCell<Vec<(ParamPtr, f32)>>>;

fn view(mode: i32, mpe: bool) -> ChordboardView {
    let view = ChordboardView::new(
        Arc::new(ChordboardParams {
            mode: IntParam::new(
                "Play mode",
                mode.min(3),
                IntRange::Linear { min: 0, max: 3 },
            ),
            output_mode: IntParam::new(
                "Output protocol",
                if mpe { 1 } else { 2 },
                IntRange::Linear { min: 0, max: 2 },
            ),
            ..ChordboardParams::default()
        }),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    view
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
    // Ordinary interaction tests operate after the quarter-second page slide.
    view.route_elapsed = pleasant_ui::page_slide::DURATION;
    view.route_progress = if view.panel == Some(Panel::Routes) {
        1.0
    } else {
        0.0
    };
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
        for id in ["spread"] {
            assert!(
                view.route_targets()
                    .iter()
                    .any(|(i, _)| crate::engine::routing::TARGETS[*i].id == id),
                "mode {mode}: {id}"
            );
        }
        for id in ["humanize", "gate", "swing"] {
            assert_eq!(
                ids.contains(&id),
                if id == "gate" {
                    mode == 3
                } else {
                    matches!(mode, 1 | 3)
                },
                "mode {mode}: {id}"
            );
        }
        assert!(!ids.contains(&"direction"));
        assert!(!ids.contains(&"strum_ms"));
        assert!(!ids.contains(&"contour"));
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, rate_rect(2));
        assert_eq!(
            changes
                .borrow()
                .iter()
                .any(|(ptr, _)| *ptr == view.params.rate.as_ptr()),
            matches!(mode, 1 | 3)
        );
    }
}

#[test]
fn output_protocol_segments_select_three_modes() {
    for selected in 0..3 {
        let mut view = view(0, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, protocol_rect(selected));
        assert!(view.panel.is_none());
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
    view.mapping_axis = 1;
    view.set_panel(Some(Panel::Mapping));
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
    assert_eq!(Menu::MappingKind.items().len(), 3);
    assert!(!std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::Learn(1 | 2))));
    view.mapping_axis = 0;
    view.set_panel(Some(Panel::Mapping));
    assert_eq!(view.mapping_axis, 0);
    assert!(
        std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::Learn(0)))
    );
}

#[test]
fn keyboard_channel_dropdown_edits_the_correct_parameter() {
    let mut view = view(3, false);
    let (mut cx, target, changes) = context();
    click(
        &mut view,
        &mut cx,
        target,
        keyboard_control_rect("bass_channel", false),
    );
    let menu = Menu::KeyboardParam("bass_channel");
    assert_eq!(view.menu, Some(menu));
    click(&mut view, &mut cx, target, menu.option_rect(4));
    let (ptr, norm) = changes.borrow()[0];
    assert_eq!(ptr, view.params.bass_channel.as_ptr());
    assert_eq!(view.params.bass_channel.preview_plain(norm), 5);
    assert!(view.menu.is_none());
}

#[test]
fn output_options_match_the_active_midi_protocol() {
    for mpe in [false, true] {
        let view = view(0, mpe);
        let ids: Vec<_> = view.keyboard_controls().iter().map(|(c, _)| c.id).collect();
        assert!(!ids.contains(&"filter"));
        assert!(!ids.contains(&"split_channels"));
        assert_eq!(ids.contains(&"members"), mpe);
        assert_eq!(ids.contains(&"output_channel"), !mpe);
    }
}

#[test]
fn mapping_menu_blocks_scroll_and_text_edit_on_covered_controls() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    view.mapping_axis = 0;
    view.set_panel(Some(Panel::Mapping));
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
    for panel in [None, Some(Panel::Mapping)] {
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
    {
        let r = view.control_octave_bounds();
        click(&mut view, &mut cx, target, r);
    }
    assert!(view.panel.is_none());
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::Learn(4))
    ));
    view.mapping_axis = 1;
    view.set_panel(Some(Panel::Mapping));
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
    for button in 0..69 {
        if button == 43 || button == 44 || button == 66 {
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
            47..=53 => (pattern_rect(button - 47), Some(params.arp_pattern.as_ptr())),
            54..=61 => (rate_rect(button - 54), Some(params.rate.as_ptr())),
            62..=65 => (octave_rect(button - 62), Some(params.octaves.as_ptr())),
            66 => (protocol_rect(1), Some(params.output_mode.as_ptr())),
            67 => (LATCH, Some(params.latch.as_ptr())),
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
        Arc::get_mut(&mut view.params).unwrap().strum_sync = BoolParam::new("Sync", false);
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
    Arc::get_mut(&mut view.params).unwrap().strum_sync = BoolParam::new("Sync", false);
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
    view.params = super::tests::view(2, false).params.clone();
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.page_position > 1.0 && view.page_position < 2.0);
    click(&mut view, &mut cx, target, ROOT_ON_SELECT);
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
        assert_eq!(view.keyboard_chord(0).map(|c| c.root), Some(2));
        click(&mut view, &mut cx, target, key_rect(key));
        assert!(std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::KeyDown(_, note, quality) if note == 50 && quality == harmony::row_quality(row))));
        view.play_key(key);
        assert!(std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(c, Command::KeyDown(_, note, quality) if note == 50 && quality == harmony::row_quality(row))));
    }
}

#[test]
fn tempo_and_rate_sync_controls_emit_the_shared_parameters() {
    for mode in [1, 3] {
        let mut view = view(mode, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, TEMPO_SYNC);
        assert_eq!(
            changes.borrow().last().copied(),
            Some((view.params.tempo_sync.as_ptr(), 0.0))
        );
        click(&mut view, &mut cx, target, RATE_SYNC);
        assert_eq!(
            changes.borrow().last().copied(),
            Some((view.params.strum_sync.as_ptr(), 0.0))
        );
        click(&mut view, &mut cx, target, rate_rect(0));
        assert_eq!(
            changes.borrow().last().copied(),
            Some((view.params.rate.as_ptr(), 1.0))
        );
        Arc::get_mut(&mut view.params).unwrap().strum_sync = BoolParam::new("Sync", false);
        assert!(view.base_controls().iter().any(|(c, _)| c.id == "strum_ms"));
    }
}

#[test]
fn learn_and_pointer_feedback_work_without_audio_processing() {
    let mut view = view(0, false);
    let (mut cx, target, _) = context();
    {
        let r = view.control_octave_bounds();
        click(&mut view, &mut cx, target, r);
    }
    assert_eq!(
        view.learning(),
        4,
        "Learn must arm without an audio snapshot"
    );
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.learning(), 4, "idle UI ticks must preserve Learn");
    {
        let r = view.control_octave_bounds();
        click(&mut view, &mut cx, target, r);
    }
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
    assert_eq!(MODE_LABELS, ["Arpeggiator", "Manual Strum"]);
}

#[test]
fn direct_direction_selects_exact_values() {
    for selected in 0..5 {
        let mut view = view(1, false);
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
    targets.extend((0..3).map(protocol_rect));
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
        (pattern_rect(2), view.params.arp_pattern.as_ptr()),
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
        view.mapping_axis = axis;
        view.set_panel(Some(Panel::Mapping));
        let panel = view.panel_rect(Panel::Mapping);
        assert_eq!(panel.2, 384.0);
        assert_eq!(panel.3, if axis == 0 { 104.0 } else { 180.0 });
        assert_eq!(panel.1 + panel.3 + 6.0, mapping_summary_rect(axis).1);
        let mut rects: Vec<_> = view.panel_controls().iter().map(|(_, r)| *r).collect();
        rects.extend([
            view.menu_trigger_rect(Menu::MappingKind),
            view.menu_trigger_rect(Menu::MappingCc(false)),
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
    let mut params = ChordboardParams::default();
    params.routes[7].source = IntParam::new("Source", 8, IntRange::Linear { min: 0, max: 9 });
    params.mode = IntParam::new("Mode", 2, IntRange::Linear { min: 0, max: 3 });
    view.params = Arc::new(params);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, meter_rect(7));
    assert_eq!(view.panel, Some(Panel::Routes));
    let r = view
        .editor_route_chips()
        .into_iter()
        .find(|(slot, _)| *slot == 7)
        .unwrap()
        .1;
    click(&mut view, &mut cx, target, r);
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
    click(&mut view, &mut cx, target, (900.0, 392.0, 16.0, 16.0));
    assert_eq!(view.panel, Some(Panel::Routes));
}

#[test]
fn route_ranges_use_destination_units_and_route_slots_fit() {
    let mut view = view(2, false);
    view.route_slot = 1;
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
fn source_meter_click_opens_routes_and_selection_mode_is_manual_only() {
    for mode in 1..=3 {
        let mut view = view(mode, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, ROOT_ON_SELECT);
        assert_eq!(
            changes
                .borrow()
                .iter()
                .any(|(p, v)| *p == view.params.root_on_select.as_ptr() && *v == 1.0),
            mode == 2
        );
        changes.borrow_mut().clear();
        click(&mut view, &mut cx, target, meter_rect(3));
        assert_eq!(view.panel, Some(Panel::Routes));
        assert_eq!(view.selected_modulator, Some(3));
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
    for (source, id) in [(0, "strings"), (3, "spread"), (8, "velocity")] {
        let mut view = view(2, false);
        let (mut cx, entity, changes) = context();
        let r = meter_rect(source);
        let (_, destination) = view
            .route_targets()
            .into_iter()
            .find(|(i, _)| crate::engine::routing::TARGETS[*i].id == id)
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
        let route = &view.params.routes[view.route_slot];
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
            (730.0, 538.0)
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
        let mut view = view(1, false);
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
        let rect = output_controls(1)
            .into_iter()
            .find(|(id, _)| *id == "strings")
            .unwrap()
            .1;
        route_drag_to(
            &mut view,
            &mut cx,
            entity,
            source,
            rect.0 + rect.2 / 2.0,
            rect.1 + 18.0,
        );
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
fn route_drag_exposes_keyboard_output_destinations() {
    let mut view = view(1, false);
    let (mut cx, entity, changes) = context();
    let (_, rect) = view
        .keyboard_controls()
        .into_iter()
        .find(|(c, _)| c.id == "output_channel")
        .unwrap();
    route_drag_to(&mut view, &mut cx, entity, 1, rect.0 + 25.0, rect.1 + 20.0);
    assert_eq!(view.panel, Some(Panel::Routes));
    assert!(changes
        .borrow()
        .iter()
        .any(|(p, _)| *p == view.params.routes[view.route_slot].source.as_ptr()));
    changes.borrow_mut().clear();
    // Starting a new drag folds the route editor away so Strings is reachable.
    let rect = output_controls(1)
        .into_iter()
        .find(|(id, _)| *id == "strings")
        .unwrap()
        .1;
    route_drag_to(
        &mut view,
        &mut cx,
        entity,
        2,
        rect.0 + rect.2 / 2.0,
        rect.1 + 18.0,
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|(p, _)| *p == view.params.routes[view.route_slot].target.as_ptr()));
}

#[test]
fn route_drag_with_no_free_slots_does_not_overwrite_an_unrelated_link() {
    let mut view = view(2, false);
    for route in &mut Arc::get_mut(&mut view.params).unwrap().routes {
        route.source = IntParam::new("Source", 1, IntRange::Linear { min: 0, max: 9 });
    }
    let (mut cx, entity, changes) = context();
    route_drag_to(&mut view, &mut cx, entity, 1, 200.0, PIANO_KEYS.1 + 20.0); // Spread; every slot is assigned.
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
        let route = &view.params.routes[view.route_slot];
        if id != "x" {
            assert!(changes.borrow().contains(&(
                route.target.as_ptr(),
                route.target.preview_normalized(target as i32)
            )));
        }
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
        Arc::get_mut(&mut view.params).unwrap().routes[7].source =
            IntParam::new("Source", 2, IntRange::Linear { min: 0, max: 9 });
        let (mut cx, target, changes) = context();
        let (x, y) = view.route_nodes()[node];
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(matches!(view.drag, Some(Drag::RouteNode(..))));
        let y = ROUTE_GRAPH.1 + ROUTE_GRAPH.3 * 0.75;
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        let r = &view.params.routes[7];
        let (ptr, expected) = match node {
            0 => (r.min.as_ptr(), 0.25),
            2 => (r.max.as_ptr(), 0.25),
            _ => (r.curve.as_ptr(), r.curve.preview_normalized(1.0 / 3.0)),
        };
        assert!(changes
            .borrow()
            .iter()
            .any(|(p, v)| *p == ptr && (v - expected).abs() < 0.0001));
        assert!(view.drag.is_none());
        assert!(!std::iter::from_fn(|| view.bridge.commands.pop())
            .any(|c| matches!(c, Command::BeginGesture(..))));
    }
}
#[test]
fn route_graph_curve_handles_reversed_and_flat_ranges() {
    let mut view = view(2, false);
    let r = &mut Arc::get_mut(&mut view.params).unwrap().routes[0];
    r.min = FloatParam::new("Minimum", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    r.max = FloatParam::new("Maximum", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    let value = view
        .route_node_value(1, ROUTE_GRAPH.1 + ROUTE_GRAPH.3 * 0.25)
        .unwrap();
    assert!((value - view.params.routes[0].curve.preview_normalized(1.0 / 3.0)).abs() < 0.0001);
    Arc::get_mut(&mut view.params).unwrap().routes[0].max =
        FloatParam::new("Maximum", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    assert_eq!(view.route_node_value(1, ROUTE_GRAPH.1), None);
}

#[test]
fn route_destination_menu_skips_touch_bounds_without_shifting_saved_ids() {
    let mut view = view(2, false);
    view.route_slot = 1;
    let (mut cx, entity, changes) = context();
    for (menu_index, (saved_index, target)) in
        crate::engine::routing::available_targets().enumerate()
    {
        assert_eq!(Menu::RouteTarget.items()[menu_index], target.name);
        view.select_menu(
            &mut EventContext::new_with_current(&mut cx, entity),
            Menu::RouteTarget,
            menu_index,
        );
        BackendContext::new_with_event_manager(&mut cx).process_events();
        let p = &view.params.routes[1].target;
        assert!(changes
            .borrow()
            .contains(&(p.as_ptr(), p.preview_normalized(saved_index as i32))));
    }
    assert!(view
        .route_targets()
        .iter()
        .all(|(i, _)| crate::engine::routing::TARGETS[*i].available()));
}

#[test]
fn leading_animation_restarts_when_a_new_transition_arrives() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    view.bridge
        .snapshots
        .push(Snapshot {
            leading_serial: 1,
            ..Snapshot::default()
        })
        .unwrap();
    view.last_frame = Instant::now() - Duration::from_millis(50);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.leading_progress < 0.2);
    for _ in 0..8 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.leading_progress, 1.0);
    view.bridge
        .snapshots
        .push(Snapshot {
            leading_serial: 2,
            ..Snapshot::default()
        })
        .unwrap();
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
        rects.extend((0..3).map(|i| output_protocol_rect(mpe, i)));
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
        (PERF_SURFACE, 112.0, (0..3).map(mode_rect).collect()),
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
fn strum_corner_controls_remain_adjacent_through_expansion() {
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let pad = pad_rect(t);
        let zoom = expand_rect(t);
        let latch = strum_latch_rect(t);
        assert_eq!(pad.0 + pad.2 - zoom.0 - zoom.2, 12.0);
        assert_eq!(zoom.1 - pad.1, 11.0);
        assert_eq!(zoom.0 - latch.0 - latch.2, 8.0);
        assert_eq!(zoom.1 + zoom.3 * 0.5, latch.1 + latch.3 * 0.5);
    }
    assert_eq!(initial_editor_size(), (W as u32, H as u32));
}

#[test]
fn destination_chips_open_the_exact_route_and_remain_reachable() {
    let mut view = view(1, false);
    let (mut cx, target, _) = context();
    let source = view.params.routes[0].source.value() as usize - 1;
    let meter = meter_rect(source);
    view.update_route_hover(meter.0 + 5.0, meter.1 + 5.0);
    let (slot, chip) = view.route_chips()[0];
    view.update_route_hover(chip.0 + 5.0, meter.1 + meter.3 + 3.0);
    assert_eq!(view.route_hover, Some(source));
    click(&mut view, &mut cx, target, chip);
    assert_eq!(view.route_slot, slot);
    assert_eq!(view.panel, Some(Panel::Routes));
    assert_eq!(view.panel_rect(Panel::Routes), CHORDS_SURFACE);
    assert!(view
        .placed_controls()
        .iter()
        .any(|(c, _)| c.id == "humanize"));
}

#[test]
fn wires_face_destinations_and_sliding_route_page_blocks_hidden_controls() {
    let r = meter_rect(0);
    assert_eq!(routing::route_anchor(r, 0.0), (r.0 + r.2 / 2.0, r.1));
    assert_eq!(routing::route_anchor(r, H), (r.0 + r.2 / 2.0, r.1 + r.3));
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    view.set_panel(Some(Panel::Routes));
    let r = transpose_rect(3);
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + 2.0,
        r.1 + 2.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(changes.borrow().is_empty());
    assert_eq!(view.route_elapsed, 0.0);
    assert_eq!(W, 1621.0); // Preserve Chordboard’s design width.
}

#[test]
fn transpose_reset_emits_zero_from_a_nonzero_saved_value() {
    let mut view = view(1, false);
    view.params = Arc::new(ChordboardParams {
        transpose: IntParam::new("Transpose", 19, IntRange::Linear { min: -24, max: 24 }),
        ..ChordboardParams::default()
    });
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, transpose_rect(2));
    assert_eq!(
        changes.borrow().last().copied(),
        Some((
            view.params.transpose.as_ptr(),
            view.params.transpose.preview_normalized(0)
        ))
    );
}

#[test]
fn modulator_selection_toggles_and_xy_tiles_open_routes() {
    let (mut cx, target, changes) = context();
    for source in 0..crate::engine::routing::SOURCE_COUNT {
        let mut view = view(3, false);
        click(&mut view, &mut cx, target, meter_rect(source));
        assert_eq!(view.panel, Some(Panel::Routes));
        assert_eq!(view.selected_modulator, Some(source));
        click(&mut view, &mut cx, target, meter_rect(source));
        assert_eq!(view.panel, None);
        assert_eq!(view.selected_modulator, None);
    }
    assert!(changes.borrow().is_empty());
}

#[test]
fn route_overlay_removal_requires_the_confirmation_check() {
    let mut view = view(3, false);
    let (mut cx, target, changes) = context();
    view.route_hover = Some(7);
    let (slot, chip) = view.route_chips()[0];
    let remove = ChordboardView::route_remove_rect(chip);
    click(&mut view, &mut cx, target, remove);
    assert_eq!(view.pending_route_remove, Some(slot));
    assert!(changes.borrow().is_empty());
    click(
        &mut view,
        &mut cx,
        target,
        (chip.0, chip.1, chip.2 - 28.0, chip.3),
    );
    assert_eq!(view.pending_route_remove, Some(slot));
    assert!(changes.borrow().is_empty());
    click(&mut view, &mut cx, target, remove);
    assert_eq!(view.pending_route_remove, None);
    assert_eq!(
        changes.borrow().as_slice(),
        &[(view.params.routes[slot].source.as_ptr(), 0.0)]
    );
}

#[test]
fn modulated_rate_clicks_explain_ownership_without_editing() {
    let mut view = view(3, false);
    let destination = crate::engine::routing::TARGETS
        .iter()
        .position(|t| t.id == "rate")
        .unwrap();
    let mut params = ChordboardParams::default();
    params.routes[1].source = IntParam::new("Source", 6, IntRange::Linear { min: 0, max: 9 });
    params.routes[1].target = IntParam::new(
        "Destination",
        destination as i32,
        IntRange::Linear {
            min: 0,
            max: crate::engine::routing::TARGETS.len() as i32 - 1,
        },
    );
    params.mode = IntParam::new("Mode", 3, IntRange::Linear { min: 0, max: 3 });
    view.params = Arc::new(params);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, rate_rect(0));
    assert!(changes.borrow().is_empty());
    assert!(view
        .status
        .contains("Rate (synced) is controlled by Timbre"));
    assert_eq!(view.route_hover, Some(5));
    assert!(view.drag.is_none());
}

#[test]
fn routing_lists_group_every_option_once_and_fit_without_overlaps() {
    for menu in [Menu::RouteSource, Menu::RouteTarget] {
        let mut grouped: Vec<_> = menu
            .groups()
            .into_iter()
            .flat_map(|(_, items)| items)
            .collect();
        grouped.sort_unstable();
        assert_eq!(grouped, (0..menu.items().len()).collect::<Vec<_>>());
        let bounds = menu.bounds();
        let cells: Vec<_> = (0..menu.items().len())
            .map(|i| menu.option_rect(i))
            .collect();
        for (i, r) in cells.iter().enumerate() {
            assert!(
                r.0 >= bounds.0
                    && r.1 >= bounds.1
                    && r.0 + r.2 <= bounds.0 + bounds.2
                    && r.1 + r.3 <= bounds.1 + bounds.3
            );
            for other in cells.iter().skip(i + 1) {
                assert!(
                    r.0 + r.2 <= other.0
                        || other.0 + other.2 <= r.0
                        || r.1 + r.3 <= other.1
                        || other.1 + other.3 <= r.1
                );
            }
        }
    }
}

#[test]
fn split_learning_and_pitch_meter_use_their_correct_colors_and_center() {
    let mut view = view(3, false);
    view.snapshot.learning = 5;
    assert_eq!(view.learn_highlight_color(), GOLD);
    view.snapshot.learning = 6;
    assert_eq!(view.learn_highlight_color(), TEAL);
    assert_eq!(super::performance::meter_fill(0, 0.5), (0.5, 0.5));
    assert_eq!(super::performance::meter_fill(0, 0.0), (0.0, 0.5));
    assert_eq!(super::performance::meter_fill(0, 1.0), (0.5, 1.0));
}

#[test]
fn current_chord_highlight_follows_midi_latch_and_recall_and_clears_after_silence() {
    let mut view = view(3, false);
    for (root, quality) in [(60, 0), (62, 1), (67, 2)] {
        let chord = SavedChord {
            root,
            second: None,
            quality,
            inversion: 0,
            spread: 0,
            transpose: 0,
        };
        view.snapshot.captured = chord.encode();
        view.snapshot.root = root as i16;
        let key = view.current_chord_key().unwrap();
        assert_eq!(view.keyboard_chord(key).unwrap().root, root % 12);
        assert_eq!(view.keyboard_chord(key).unwrap().quality, quality);
        view.snapshot.root = -1;
        view.snapshot.notes = harmony::voice(root, quality, None, 0, 0, 0);
        assert_eq!(view.current_chord_key(), Some(key));
        view.snapshot.notes = harmony::Notes::default();
        assert_eq!(view.current_chord_key(), None);
    }
}

#[test]
fn once_and_loop_share_a_page_with_mode_specific_timing_controls() {
    let once = view(1, false);
    let looped = view(3, false);
    assert_eq!(once.page_position, looped.page_position);
    for v in [&once, &looped] {
        let ids = v
            .base_controls()
            .into_iter()
            .map(|(c, _)| c.id)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"humanize") && ids.contains(&"swing"));
        assert_eq!(ids.contains(&"gate"), v.mode() == 3);
        assert!(!ids.contains(&"strum_ms"));
        assert!(v
            .route_targets()
            .iter()
            .any(|(i, _)| crate::engine::routing::TARGETS[*i].id == "octaves"));
    }
    for mode in [1, 3] {
        let mut v = view(mode, false);
        let (mut cx, target, changes) = context();
        click(&mut v, &mut cx, target, repeat_rect(mode == 1));
        assert_eq!(
            &*changes.borrow(),
            &[(
                v.params.mode.as_ptr(),
                v.params
                    .mode
                    .preview_normalized(if mode == 1 { 3 } else { 1 })
            )]
        );
    }
}

#[test]
fn selected_modulator_connections_survive_pointer_leaving_and_removal_icons_fit() {
    let mut v = view(3, false);
    v.toggle_modulator(7);
    v.route_hover = None;
    v.pointer = None;
    assert!(!v.route_chips().is_empty());
    for (_, r) in v.route_chips() {
        let x = ChordboardView::route_remove_rect(r);
        assert!(x.0 > r.0 && x.1 > r.1 && x.0 + x.2 < r.0 + r.2 && x.1 + x.3 < r.1 + r.3);
    }
    v.toggle_modulator(7);
    assert!(v.route_chips().is_empty());
}

#[test]
fn output_controls_are_on_the_playback_pages_without_a_settings_page() {
    for mode in [1, 2, 3] {
        let v = view(mode, false);
        assert_eq!(v.mode(), mode);
        let ids: Vec<_> = v.base_controls().into_iter().map(|(c, _)| c.id).collect();
        assert!(ids.contains(&"velocity"));
        assert_eq!(ids.contains(&"length_ms"), mode != 3);
        assert_eq!(ids.contains(&"strings"), mode != 3);
        assert_eq!(ids.contains(&"strings_played"), mode == 1);
        assert!(!ids.contains(&"contour"));
    }
}

#[test]
fn every_modulator_route_has_an_independent_contour_drag() {
    for source in 1..=9 {
        let mut v = view(1, false);
        let params = Arc::get_mut(&mut v.params).unwrap();
        params.routes[2].source =
            IntParam::new("Source", source, IntRange::Linear { min: 0, max: 9 });
        v.route_slot = 2;
        v.set_panel(Some(Panel::Routes));
        v.selected_modulator = Some(source as usize - 1);
        v.route_progress = 1.0;
        v.route_elapsed = pleasant_ui::page_slide::DURATION;
        let (mut cx, target, changes) = context();
        let (x, y) = (ROUTE_GRAPH.0 + 150.0, ROUTE_GRAPH.1 + 30.0);
        event(
            &mut v,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(matches!(v.drag, Some(Drag::RouteContour { slot: 2, .. })));
        event(
            &mut v,
            &mut cx,
            target,
            x - 100.0,
            y,
            WindowEvent::MouseMove(x - 100.0, y),
        );
        let edits = changes.borrow();
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].0, v.params.routes[2].min.as_ptr());
        assert_eq!(edits[1].0, v.params.routes[2].max.as_ptr());
        assert!(edits[0].1 > 0.0 && edits[1].1 < 1.0);
        drop(edits);
        event(
            &mut v,
            &mut cx,
            target,
            x - 100.0,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(v.drag.is_none());
    }
}

#[test]
fn route_contour_can_cross_flat_reverse_and_lock_the_drag_axis() {
    let origin = (ROUTE_GRAPH.0 + 100.0, ROUTE_GRAPH.1 + 40.0);
    for (dx, expected) in [
        (0.0, (0.0, 1.0)),
        (-ROUTE_GRAPH.2 * 0.5, (0.5, 0.5)),
        (-ROUTE_GRAPH.2, (1.0, 0.0)),
    ] {
        assert_eq!(
            ChordboardView::route_contour_range(
                origin,
                (origin.0 + dx, origin.1),
                0.0,
                1.0,
                false,
                &mut Default::default()
            ),
            expected
        );
    }
    let mut axis = Default::default();
    let first = ChordboardView::route_contour_range(
        origin,
        (origin.0 + 1.0, origin.1 - 20.0),
        0.0,
        1.0,
        false,
        &mut axis,
    );
    let sideways = ChordboardView::route_contour_range(
        origin,
        (origin.0 + 200.0, origin.1 - 20.0),
        0.0,
        1.0,
        false,
        &mut axis,
    );
    assert_eq!(first, sideways);
    assert!(first.0 > 0.0 && first.1 < 1.0);
}

#[test]
fn arp_strings_cover_octaves_and_flash_short_notes_after_they_end() {
    let mut v = view(3, false);
    Arc::get_mut(&mut v.params).unwrap().octaves =
        IntParam::new("Octaves", 4, IntRange::Linear { min: 1, max: 4 });
    v.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
    assert_eq!(v.arp_string_count(3), 12);
    assert_eq!(v.arp_string_count(1), 17);
    let (mut cx, target, _) = context();
    let mut snapshot = v.snapshot;
    snapshot.note_strikes[96] = 1;
    snapshot.sounding_notes[96] = false;
    v.bridge.visible.store(true, Ordering::Relaxed);
    v.bridge.publish(snapshot);
    v.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(v.note_anim[96] > 0.0);
}
