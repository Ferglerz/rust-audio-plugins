use super::super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};
use super::support::*;

#[test]
fn rhythm_controls_and_pointer_targets_follow_active_mode() {
    for mode in 1..4 {
        let mut view = view(mode, false);
        for id in ["spread"] {
            assert!(
                view.route_targets()
                    .iter()
                    .any(|(i, _)| crate::engine::routing::TARGETS[*i].id == id),
                "mode {mode}: {id}"
            );
        }
        let ids: Vec<_> = view.base_controls().iter().map(|(c, _)| c.id).collect();
        assert!(!ids.contains(&"humanize") && !ids.contains(&"swing"));
        open_arp(&mut view);
        let ids: Vec<_> = view.base_controls().iter().map(|(c, _)| c.id).collect();
        // Gate is drawn as a rail beside swing rather than as a generic slider.
        for (id, present) in [("humanize", true), ("swing", true), ("gate", false)] {
            assert_eq!(ids.contains(&id), present, "mode {mode}: {id}");
        }
        assert!(!ids.contains(&"direction"));
        assert!(!ids.contains(&"strum_ms"));
        assert!(!ids.contains(&"contour"));
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, rate_rect(2));
        assert!(changes
            .borrow()
            .iter()
            .any(|(ptr, _)| *ptr == view.params.rate.as_ptr()));
    }
}
#[test]
fn interlock_sits_in_the_voice_leading_row_without_changing_leading() {
    assert!(hit(PIANO_LEADING, INTERLOCK.0, INTERLOCK.1));
    assert!(hit(
        PIANO_LEADING,
        INTERLOCK.0 + INTERLOCK.2,
        INTERLOCK.1 + INTERLOCK.3
    ));
    for leading in 0..3 {
        let mut view = view(0, false);
        Arc::get_mut(&mut view.params).unwrap().voice_leading =
            IntParam::new("Voice leading", leading, IntRange::Linear { min: 0, max: 2 });
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, INTERLOCK);
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.interlock.as_ptr(),
                view.params.interlock.preview_normalized(1)
            )]
        );
    }
}
#[test]
fn mpe_button_toggles_explicit_on_and_off() {
    for mpe in [false, true] {
        let mut view = view(0, mpe);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, protocol_rect(0));
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.output_mode.as_ptr(),
                view.params
                    .output_mode
                    .preview_normalized(if mpe { 2 } else { 1 })
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
#[test]
fn native_repeated_clicks_reach_all_chord_and_parameter_buttons() {
    for button in 0..69 {
        if (42..=44).contains(&button) || button == 66 {
            continue;
        }
        let mut cx = Context::default();
        let params = Arc::new(ChordboardParams {
            mode: IntParam::new("Play mode", 3, IntRange::Linear { min: 0, max: 3 }),
            ..ChordboardParams::default()
        });
        let bridge = Arc::new(Bridge::default());
        let mut view = ChordboardView::new(params.clone(), bridge.clone(), Arc::new(PreviewContext));
        if (47..=65).contains(&button) {
            open_arp(&mut view);
        }
        let target = view
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
            41 => (HEADER_SEQ_BYPASS, Some(params.seq_enabled.as_ptr())),
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
        open_arp(&mut view);
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
    open_arp(&mut view);
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
fn sequencer_page_slide_retargets_from_current_position_and_blocks_moving_controls() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    assert_eq!(view.page_position, 2.0);
    view.sequencer_ui.open = true;
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.page_position > 1.0 && view.page_position < 2.0);
    click(&mut view, &mut cx, target, LATCH);
    assert!(changes.borrow().is_empty());
    let interrupted = view.page_position;
    view.sequencer_ui.open = false;
    view.last_frame = Instant::now();
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.page_start, interrupted);
    for _ in 0..3 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.page_position, 2.0);
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
        open_arp(&mut view);
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
