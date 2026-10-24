use super::super::*;
use super::support::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};

#[test]
fn value_edit_clicks_place_the_caret_and_command_a_selects_text() {
    let mut view = view(0, false);
    let (mut cx, target, changes) = context();
    let rect = pleasant_ui::slider_value_rect(output_controls()[0].1);
    view.edit = Some(ValueEdit::new("length_ms", rect, "350".into()));
    let (x, y) = (rect.0 + 4.0, rect.1 + rect.3 * 0.5);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    let edit = view
        .edit
        .as_ref()
        .expect("click inside the editor must not commit");
    assert_eq!((edit.cursor, edit.anchor), (0, 0));
    assert!(changes.borrow().is_empty());
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::LOGO, true);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::KeyA, None),
    );
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::LOGO, false);
    assert_eq!(view.edit.as_ref().unwrap().selection(), 0..3);
    // Some host key paths supply a physical key without a character event.
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::Digit4, None),
    );
    assert_eq!(view.edit.as_ref().unwrap().text, "4");
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::Enter, None),
    );
    assert!(view.edit.is_none());
    assert_eq!(
        &*changes.borrow(),
        &[(
            view.params.length_ms.as_ptr(),
            view.params.length_ms.preview_normalized(4.0)
        )]
    );
}

#[test]
fn slider_reset_gestures_close_text_editing_and_restore_original_defaults() {
    for double in [false, true] {
        for id in ["length_ms", "swing", "humanize", "arp_free_ms"] {
            let mut view = view(2, false);
            view.params.arpeggiator.edit(|s| {
                s.rows[0].settings.swing = 0.5;
                s.rows[0].settings.humanize = 0.8;
                s.rows[0].settings.sync = false;
                s.rows[0].settings.free_ms = 500.0;
            });
            view.refresh_arp_editor_params();
            let (mut cx, target, changes) = context();
            let (control, rect) = view
                .placed_controls()
                .into_iter()
                .find(|(c, _)| c.id == id)
                .unwrap();
            let value = pleasant_ui::slider_value_rect(rect);
            click(&mut view, &mut cx, target, value);
            assert!(view.edit.is_some());
            changes.borrow_mut().clear();
            BackendContext::new(&mut cx)
                .modifiers()
                .set(Modifiers::LOGO, !double);
            event(
                &mut view,
                &mut cx,
                target,
                value.0 + value.2 * 0.5,
                value.1 + value.3 * 0.5,
                if double {
                    WindowEvent::MouseDoubleClick(MouseButton::Left)
                } else {
                    WindowEvent::MouseDown(MouseButton::Left)
                },
            );
            let defaults = crate::arpeggiator::Settings::default();
            let expected = match id {
                "swing" => view.arp_params().swing.preview_normalized(defaults.swing),
                "humanize" => view
                    .arp_params()
                    .humanize
                    .preview_normalized(defaults.humanize),
                "arp_free_ms" => view
                    .arp_params()
                    .arp_free_ms
                    .preview_normalized(defaults.free_ms),
                _ => view.params.length_ms.default_normalized_value(),
            };
            assert_eq!(&*changes.borrow(), &[(control.ptr, expected)], "{id}");
            assert!(view.edit.is_none());
            assert!(view.drag.is_none());
        }
    }
}

#[test]
fn invalid_value_edit_blocks_outside_click_and_focus_loss_cancels() {
    let mut view = view(0, false);
    let (mut cx, target, changes) = context();
    let rect = pleasant_ui::slider_value_rect(output_controls()[0].1);
    view.edit = Some(ValueEdit::new("length_ms", rect, "invalid".into()));
    event(
        &mut view,
        &mut cx,
        target,
        LATCH.0 + 5.0,
        LATCH.1 + 5.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(view.edit.as_ref().unwrap().invalid);
    assert!(changes.borrow().is_empty());
    event(
        &mut view,
        &mut cx,
        target,
        rect.0,
        rect.1,
        WindowEvent::FocusOut,
    );
    assert!(view.edit.is_none());
    assert!(changes.borrow().is_empty());
}

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
        open_arp(&mut view);
        let ids: Vec<_> = view.base_controls().iter().map(|(c, _)| c.id).collect();
        // Direction is a dropdown in the arpeggiator row, not a slider.
        for (id, present) in [("humanize", true), ("swing", true), ("gate", false)] {
            assert_eq!(ids.contains(&id), present, "mode {mode}: {id}");
        }
        assert!(!ids.contains(&"direction"));
        assert!(!ids.contains(&"strum_ms"));
        assert!(!ids.contains(&"contour"));
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, arp_rate_rect());
        assert_eq!(view.menu, Some(Menu::ArpRate));
        click_menu_option(&mut view, &mut cx, target, Menu::ArpRate, 2);
        assert!(changes
            .borrow()
            .iter()
            .any(|(ptr, _)| *ptr == view.arp_params().rate.as_ptr()));
    }
}
#[test]
fn arp_modes_and_octaves_edit_the_selected_page_independently() {
    let mut view = view(0, false);
    view.arp_ui.edit_page = 3;
    let (mut cx, target, changes) = context();
    for mode in 0..4 {
        let r = arpeggiator::mode_rect();
        click(&mut view, &mut cx, target, r);
        click(
            &mut view,
            &mut cx,
            target,
            (r.0, r.1 + r.3 + 4.0 + mode as f32 * 22.0, r.2, 22.0),
        );
        let p = &view.params.arp_pages[3].arp_mode;
        assert_eq!(
            changes.borrow().last().copied(),
            Some((p.as_ptr(), p.preview_normalized(mode)))
        );
    }
    changes.borrow_mut().clear();
    click(&mut view, &mut cx, target, arpeggiator::octaves_rect());
    let p = &view.params.arp_pages[3].arp_octaves;
    assert_eq!(&*changes.borrow(), &[(p.as_ptr(), p.preview_normalized(2))]);
    assert!(!changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.octaves.as_ptr()));
}

#[test]
fn blank_voice_leading_diagram_does_not_toggle_a_hidden_setting() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    click(
        &mut view,
        &mut cx,
        target,
        (32.0, PIANO_LEADING.1, 20.0, 24.0),
    );
    assert!(changes.borrow().is_empty());
}

#[test]
fn strumfield_power_never_changes_arp_repeat_or_power() {
    for mode in 1..4 {
        let mut view = view(mode, false);
        Arc::get_mut(&mut view.params).unwrap().strum_enabled = BoolParam::new("Strumfield", false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, STRUM_BYPASS);
        assert_eq!(
            &*changes.borrow(),
            &[(view.params.strum_enabled.as_ptr(), 1.0)]
        );
        assert!(view.manual_playing() == view.params.strum_enabled.value());
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
fn route_menu_blocks_scroll_and_text_edit_on_covered_controls() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    view.set_panel(Some(Panel::Routes));
    click_menu_trigger(&mut view, &mut cx, target, Menu::RouteTarget);
    assert!(view.menu.is_some());
    let r = route_control_rect("route_smoothing_ms");
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
fn chord_clicks_hold_until_release() {
    let mut view = view(0, false);
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
#[test]
fn direct_learn_and_transpose_buttons() {
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
fn native_repeated_clicks_reach_all_chord_and_parameter_buttons() {
    for button in 0..69 {
        // Arp rate and ordering use dropdowns; obsolete slots are archived.
        // 47..=53 were the direction buttons; direction is a dropdown now.
        if (42..=44).contains(&button)
            || (47..=53).contains(&button)
            || (54..=61).contains(&button)
            || button == 66
        {
            continue;
        }
        let mut cx = Context::default();
        let params = Arc::new(ChordboardParams::default());
        let bridge = Arc::new(Bridge::default());
        let mut view =
            ChordboardView::new(params.clone(), bridge.clone(), Arc::new(PreviewContext));
        if button == 41 || (47..=65).contains(&button) {
            open_arp(&mut view);
        }
        let target = view.build(&mut cx, |_| {}).focusable(true).entity();
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
            41 => (arp_power_rect(), Some(params.arp_enabled.as_ptr())),
            45..=46 => (inversion_rect(button - 45), Some(params.inversion.as_ptr())),
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
        Arc::get_mut(&mut view.params).unwrap().arp_pages[0].arp_sync =
            BoolParam::new("Sync", false);
        let (mut cx, target, changes) = context();
        let r = arp_rate_rect();
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
        assert_eq!(ptr, view.arp_params().arp_free_ms.as_ptr());
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
    Arc::get_mut(&mut view.params).unwrap().arp_pages[0].arp_sync = BoolParam::new("Sync", false);
    let (mut cx, target, changes) = context();
    let r = arp_rate_rect();
    click(&mut view, &mut cx, target, (r.0, r.1, 60.0, 20.0));
    assert!(view.edit.is_none());
    click(
        &mut view,
        &mut cx,
        target,
        pleasant_ui::slider_value_rect(r),
    );
    assert_eq!(
        view.edit.as_ref().map(|edit| edit.target),
        Some("arp_free_ms")
    );
    assert!(changes.borrow().is_empty());
}
#[test]
fn arp_editor_module_keeps_the_chord_page_in_place() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    assert!(view
        .base_controls()
        .iter()
        .any(|(c, _)| c.id == "root_on_select"));
    assert!(view.arp_editor_open());
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view
        .base_controls()
        .iter()
        .any(|(c, _)| c.id == "root_on_select"));
    click(&mut view, &mut cx, target, LATCH);
    assert!(!changes.borrow().is_empty());
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
fn tempo_is_global_and_arp_rate_sync_is_page_specific() {
    for mode in [1, 3] {
        let mut view = view(mode, false);
        open_arp(&mut view);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, TEMPO_SYNC);
        assert_eq!(
            changes.borrow().last().copied(),
            Some((view.params.tempo_sync.as_ptr(), 0.0))
        );
        click(&mut view, &mut cx, target, arp_rate_sync_rect());
        assert_eq!(
            changes.borrow().last().copied(),
            Some((view.arp_params().arp_sync.as_ptr(), 0.0))
        );
        click(&mut view, &mut cx, target, arp_rate_rect());
        click_menu_option(&mut view, &mut cx, target, Menu::ArpRate, 0);
        assert_eq!(
            changes.borrow().last().copied(),
            Some((view.arp_params().rate.as_ptr(), 1.0))
        );
        Arc::get_mut(&mut view.params).unwrap().arp_pages[0].arp_sync =
            BoolParam::new("Sync", false);
        assert!(view
            .base_controls()
            .iter()
            .any(|(c, _)| c.id == "arp_free_ms"));
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

fn string_editor_view() -> ChordboardView {
    let mut view = view(2, false);
    open_arp(&mut view);
    view.snapshot.notes = harmony::Notes {
        values: [60, 64, 67, 71, 0, 0, 0, 0, 0, 0, 0, 0],
        len: 4,
    };
    view
}

fn string_point(index: usize) -> (f32, f32) {
    let g = arpeggiator::graph_rect();
    (g.0 + (index as f32 + 0.5) * g.2 / 4.0, g.1 + g.3 * 0.5)
}

fn select_adjustment(view: &mut ChordboardView, cx: &mut Context, target: Entity, lane: usize) {
    click(view, cx, target, arpeggiator::lane_rect(lane));
    assert!(!view.arp_chooser_open());
}

fn select_arp_page(view: &mut ChordboardView, cx: &mut Context, target: Entity, page: usize) {
    click(view, cx, target, arpeggiator::page_block());
    click(view, cx, target, arpeggiator::page_rect(page));
}

fn release_string(view: &mut ChordboardView, cx: &mut Context, target: Entity, x: f32, y: f32) {
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(!view.arp_editor_dragging());
}

#[test]
fn arp_defaults_to_velocity_and_fast_paint_interpolates_every_string() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    let (x, y) = string_point(0);
    let (end, _) = string_point(3);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y - 32.4,
        WindowEvent::MouseMove(end, y - 32.4),
    );
    release_string(&mut view, &mut cx, target, end, y - 32.4);
    let state = view.params.arpeggiator.snapshot();
    let steps = &state.pages[0].loops[0].steps[..4];
    assert_eq!(
        steps.iter().map(|s| s.velocity).collect::<Vec<_>>(),
        [100, 127, 153, 180]
    );
    assert!(steps.iter().all(|s| s.gate == 65 && s.probability == 100));
    assert!(changes.borrow().is_empty());
}

#[test]
fn every_lane_keeps_left_click_play_mute_skip_buttons_and_modifier_reset() {
    use crate::arpeggiator::StepState;
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    let (x, _) = string_point(0);
    let button = (x - 5.0, ARP_SURFACE.1 + ARP_SURFACE.3 - 39.0, 10.0, 18.0);
    for lane in 0..6 {
        select_adjustment(&mut view, &mut cx, target, lane);
        for expected in [StepState::Mute, StepState::Skip, StepState::Play] {
            click(&mut view, &mut cx, target, button);
            assert_eq!(
                view.params.arpeggiator.snapshot().pages[0].loops[0].steps[0].state,
                expected
            );
        }
        click(&mut view, &mut cx, target, button);
        BackendContext::new(&mut cx)
            .modifiers()
            .set(Modifiers::CTRL, true);
        click(&mut view, &mut cx, target, button);
        BackendContext::new(&mut cx)
            .modifiers()
            .set(Modifiers::CTRL, false);
        assert_eq!(
            view.params.arpeggiator.snapshot().pages[0].loops[0].steps[0].state,
            StepState::Play
        );
    }
}

#[test]
fn modifier_drag_resets_every_crossed_adjustment_to_its_default() {
    let mut view = string_editor_view();
    view.params.arpeggiator.edit(|s| {
        for step in &mut s.pages[0].loops[0].steps[..4] {
            step.velocity = 14;
            step.gate = 90;
        }
    });
    let (mut cx, target, changes) = context();
    let (x, y) = string_point(0);
    let (end, _) = string_point(3);
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::LOGO, true);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y - 30.0,
        WindowEvent::MouseMove(end, y - 30.0),
    );
    release_string(&mut view, &mut cx, target, end, y - 30.0);
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::LOGO, false);
    let state = view.params.arpeggiator.snapshot();
    assert!(state.pages[0].loops[0].steps[..4]
        .iter()
        .all(|s| s.velocity == 100 && s.gate == 90));
    assert!(changes.borrow().is_empty());
}

#[test]
fn shift_paints_a_straight_origin_ramp_without_toggling_latch() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    let (x, y) = string_point(0);
    let (middle, _) = string_point(1);
    let (end, _) = string_point(3);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        middle,
        y - 40.0,
        WindowEvent::MouseMove(middle, y - 40.0),
    );
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, true);
    event(
        &mut view,
        &mut cx,
        target,
        middle,
        y - 40.0,
        WindowEvent::KeyDown(Code::ShiftLeft, None),
    );
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y - 16.2,
        WindowEvent::MouseMove(end, y - 16.2),
    );
    release_string(&mut view, &mut cx, target, end, y - 16.2);
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, false);
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y - 16.2,
        WindowEvent::KeyUp(Code::ShiftLeft, None),
    );
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(
        state.pages[0].loops[0].steps[..4]
            .iter()
            .map(|s| s.velocity)
            .collect::<Vec<_>>(),
        [100, 113, 127, 140]
    );
    assert!(!changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.latch.as_ptr()));
    assert!(view.latch_shift.is_none());
}

#[test]
fn timing_drag_is_horizontal_and_never_paints_neighboring_steps() {
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    select_adjustment(&mut view, &mut cx, target, 3);
    let (x, y) = string_point(0);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y - 45.0,
        WindowEvent::MouseMove(x, y - 45.0),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[0].loops[0].steps[0].micro,
        0
    );
    let (end, _) = string_point(3);
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y,
        WindowEvent::MouseMove(end, y),
    );
    release_string(&mut view, &mut cx, target, end, y);
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.pages[0].loops[0].steps[0].micro, 49);
    assert!(state.pages[0].loops[0].steps[1..4]
        .iter()
        .all(|s| s.micro == 0));
}

#[test]
fn delay_harmony_switches_edit_independently_and_recall_per_page() {
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    click(
        &mut view,
        &mut cx,
        target,
        arpeggiator::delay_leading_rect(),
    );
    let state = view.params.arpeggiator.snapshot();
    assert!(state.pages[0].delay.voice_leading);
    assert!(!state.pages[0].delay.follow_chords);
    click(&mut view, &mut cx, target, arpeggiator::delay_follow_rect());
    assert!(
        view.params.arpeggiator.snapshot().pages[0]
            .delay
            .follow_chords
    );
    select_arp_page(&mut view, &mut cx, target, 1);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let state = view.params.arpeggiator.snapshot();
    assert!(!state.pages[1].delay.voice_leading && !state.pages[1].delay.follow_chords);
    click(&mut view, &mut cx, target, arpeggiator::delay_follow_rect());
    select_arp_page(&mut view, &mut cx, target, 0);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let state = view.params.arpeggiator.snapshot();
    assert!(state.pages[0].delay.voice_leading && state.pages[0].delay.follow_chords);
    assert!(!state.pages[1].delay.voice_leading && state.pages[1].delay.follow_chords);
}

#[test]
fn delay_replaces_keyboard_keeps_arp_editable_and_has_header_power() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
    assert!(view.arp_editor_overlay_visible());
    // Covered keyboard controls cannot receive presses during or after the slide.
    click(&mut view, &mut cx, target, ALWAYS_BASS);
    assert!(changes.borrow().is_empty());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    click(&mut view, &mut cx, target, ALWAYS_BASS);
    assert!(changes.borrow().is_empty());
    click(&mut view, &mut cx, target, DELAY_POWER);
    assert!(view.params.arpeggiator.snapshot().pages[0].delay.enabled);
    assert!(!view.params.arp_enabled.value());
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert!(view.arp_chooser_open());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    click(&mut view, &mut cx, target, arpeggiator::chooser_card(2));
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.arp_params().arp_pattern.as_ptr()));
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    assert!(!view.arp_editor_overlay_visible());
    assert!(view.params.arpeggiator.snapshot().pages[0].delay.enabled);
}

#[test]
fn pages_recall_independent_adjustments_and_delay_sources() {
    use crate::arpeggiator::source;
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let inputs: Vec<_> = view
        .arp_editor_button_rects()
        .into_iter()
        .map(|(r, _)| r)
        .filter(|r| r.1 == PIANO_SURFACE.1 + 10.0 && r.2 == 116.0)
        .collect();
    assert_eq!(inputs.len(), 5);
    click(&mut view, &mut cx, target, inputs[1]); // Manual Strum input
    click(&mut view, &mut cx, target, inputs[2]); // Chords input
    let sources = source::ARP | source::STRUM | source::CHORDS;
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[0].delay.sources,
        sources
    );
    // Closing and selecting another Page preserves the first Page's complete variation.
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    select_arp_page(&mut view, &mut cx, target, 1);
    assert_eq!(view.arp_ui.edit_page, 1);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let p = &view.params.arp_page;
    assert!(changes
        .borrow()
        .contains(&(p.as_ptr(), p.preview_normalized(1))));
    let (x, y) = string_point(2);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y - 16.2,
        WindowEvent::MouseMove(x, y - 16.2),
    );
    release_string(&mut view, &mut cx, target, x, y - 16.2);
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.pages[1].loops[0].steps[2].velocity, 140);
    assert_eq!(state.pages[0].loops[0].steps[2].velocity, 100);
    assert_eq!(state.pages[1].delay.sources, source::ARP);
    select_arp_page(&mut view, &mut cx, target, 0);
    assert_eq!(view.arp_ui.edit_page, 0);
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[0].delay.sources,
        sources
    );
}

#[test]
fn ordering_chooser_captures_strings_and_shift_without_changing_latch() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert!(view.arp_chooser_open());
    let before = view.params.arpeggiator.snapshot();
    let (x, y) = string_point(1);
    // While sliding, covered strings do not start a paint gesture or select a card.
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(!view.arp_editor_dragging());
    assert_eq!(view.params.arpeggiator.snapshot(), before);
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, true);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::ShiftLeft, Some(Key::Shift)),
    );
    assert!(!changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.latch.as_ptr()));
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, false);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::ArrowRight, Some(Key::ArrowRight)),
    );
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::Enter, Some(Key::Enter)),
    );
    assert!(!view.arp_chooser_open());
    assert_eq!(view.params.arpeggiator.snapshot(), before);
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.arp_params().arp_pattern.as_ptr()));
}

#[test]
fn arp_copy_paste_clones_complete_page_and_emits_all_destination_parameters() {
    let mut view = string_editor_view();
    let params = Arc::get_mut(&mut view.params).unwrap();
    params.arp_pages[0].arp_pattern =
        IntParam::new("Ordering", 4, IntRange::Linear { min: 0, max: 6 });
    params.arp_pages[0].arp_octaves =
        IntParam::new("Loop Length", 4, IntRange::Linear { min: 1, max: 4 });
    params.arp_pages[0].rate = FloatParam::new(
        "Rate",
        0.5,
        FloatRange::Linear {
            min: 0.0625,
            max: 2.0,
        },
    );
    view.params.arpeggiator.edit(|s| {
        s.pages[0].loops[3].steps[23].velocity = 31;
        s.pages[0].loops[0].steps[4].state = crate::arpeggiator::StepState::Skip;
        s.pages[0].ensure_loops(4);
        for (l, octave) in s.pages[0].loops.iter_mut().zip([1, 0, -1, 2]) {
            l.octave = octave;
        }
        s.pages[0].delay.enabled = true;
        s.pages[0].delay.taps[4].pitch = 7;
    });
    let expected = view.params.arpeggiator.snapshot().pages[0].clone();
    view.arp_ui.edit_page = 2;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::page_block());
    click(&mut view, &mut cx, target, arpeggiator::page_copy_rect(0));
    click(&mut view, &mut cx, target, arpeggiator::page_paste_rect(5));
    click(
        &mut view,
        &mut cx,
        target,
        arpeggiator::page_paste_option_rect(5, 0),
    );
    assert_eq!(view.arp_ui.edit_page, 2);
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[5].loops,
        expected.loops
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[5].delay,
        expected.delay
    );
    let p = &view.params.arp_pages[5];
    let edits = changes.borrow();
    assert_eq!(edits.len(), 8);
    assert!(edits.contains(&(p.arp_pattern.as_ptr(), p.arp_pattern.preview_normalized(4))));
    assert!(edits.contains(&(p.arp_octaves.as_ptr(), p.arp_octaves.preview_normalized(4))));
    assert!(edits.contains(&(p.rate.as_ptr(), p.rate.preview_normalized(0.5))));
    drop(edits);
    view.params
        .arpeggiator
        .edit(|s| s.pages[5].loops[3].steps[23].velocity = 90);
    assert_eq!(view.params.arpeggiator.snapshot().pages[0], expected);
}

#[test]
fn arp_loop_octave_controls_edit_each_pass_without_touching_step_adjustments() {
    let mut view = string_editor_view();
    Arc::get_mut(&mut view.params).unwrap().arp_pages[0].arp_octaves =
        IntParam::new("Loop Length", 4, IntRange::Linear { min: 1, max: 4 });
    let (mut cx, target, _) = context();
    let width = (ARP_SURFACE.2 - 44.0) / 4.0;
    view.tick_arp_editor(0.0);
    for i in 0..4 {
        let r = (
            ARP_SURFACE.0 + 22.0 + i as f32 * width + width * 0.75,
            ARP_SURFACE.1 + 82.0,
            2.0,
            20.0,
        );
        click(&mut view, &mut cx, target, r);
    }
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.pages[0].octaves(), [1; 4]);
    assert!(state.pages[0].loops[0]
        .steps
        .iter()
        .all(|s| *s == crate::arpeggiator::Step::default()));
}

#[test]
fn root_popover_uses_black_key_priority_preserves_spelling_and_contains_scale() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, KEY_MENU);
    for index in [1, 2, 8, 9, 15, 16] {
        let r = view.menu_option_rect(Menu::Key, index);
        click(&mut view, &mut cx, target, r);
        let (_, note, spelling) = harmony::KEY_CHOICES[index];
        assert_eq!(view.menu, Some(Menu::Key));
        assert!(changes.borrow().contains(&(
            view.params.key.as_ptr(),
            view.params.key.preview_normalized(note as i32)
        )));
        assert!(changes.borrow().contains(&(
            view.params.key_spelling.as_ptr(),
            view.params.key_spelling.preview_normalized(spelling)
        )));
        changes.borrow_mut().clear();
    }
    for index in 0..harmony::SCALE_NAMES.len() {
        assert_eq!(
            view.params
                .scale
                .preview_plain(view.params.scale.preview_normalized(index as i32)),
            index as i32
        );
        view.open_menu(Menu::Key);
        let scale = view.menu_option_rect(Menu::Scale, index);
        click(&mut view, &mut cx, target, scale);
        assert!(view.menu.is_none());
        assert_eq!(
            &*changes.borrow(),
            &[(
                view.params.scale.as_ptr(),
                view.params.scale.preview_normalized(index as i32)
            )]
        );
        changes.borrow_mut().clear();
    }
}

#[test]
fn root_popover_tabs_between_root_and_visible_modes() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, KEY_MENU);
    event(
        &mut view,
        &mut cx,
        target,
        0.0,
        0.0,
        WindowEvent::KeyDown(Code::Tab, None),
    );
    assert_eq!(view.menu, Some(Menu::Scale));
    event(
        &mut view,
        &mut cx,
        target,
        0.0,
        0.0,
        WindowEvent::KeyDown(Code::ArrowRight, None),
    );
    assert_eq!(view.menu_cursor, 2);
    event(
        &mut view,
        &mut cx,
        target,
        0.0,
        0.0,
        WindowEvent::KeyDown(Code::Enter, None),
    );
    assert_eq!(
        &*changes.borrow(),
        &[(
            view.params.scale.as_ptr(),
            view.params.scale.preview_normalized(2)
        )]
    );
}

#[test]
fn delay_middle_tap_removal_requires_confirmation_and_preserves_survivors() {
    for sync in [true, false] {
        let mut view = string_editor_view();
        view.params.arpeggiator.edit(|s| {
            let d = &mut s.pages[0].delay;
            d.sync = sync;
            d.count = 4;
            for (i, t) in d.taps[..4].iter_mut().enumerate() {
                t.time = (i + 1) as f32 * if sync { 0.75 } else { 375.0 };
                t.pitch = i as i8 * 2;
                t.velocity = 0.3 + i as f32 * 0.2;
                t.feedback = 0.1 + i as f32 * 0.1;
            }
        });
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, arpeggiator::delay_rect());
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
        let before = view.params.arpeggiator.snapshot().pages[0].delay;
        let domain = arpeggiator::time_domain(&before);
        let remove = arpeggiator::tap_remove_rect(&before.taps[1], domain);
        click(&mut view, &mut cx, target, remove);
        assert_eq!(view.params.arpeggiator.snapshot().pages[0].delay, before);
        assert!(view.arp_editor_button_rects().contains(&(remove, true)));
        click(&mut view, &mut cx, target, remove);
        let after = view.params.arpeggiator.snapshot().pages[0].delay;
        assert_eq!(after.count, 3);
        assert_eq!(
            &after.taps[..3],
            &[before.taps[0], before.taps[2], before.taps[3]]
        );
        assert_eq!(arpeggiator::time_domain(&after), domain);
    }
}

#[test]
fn delay_remove_confirmation_cancels_on_escape_and_other_controls() {
    let mut view = string_editor_view();
    view.params.arpeggiator.edit(|s| {
        s.pages[0].delay.count = 2;
        s.pages[0].delay.taps[1].time = 1.0;
    });
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let before = view.params.arpeggiator.snapshot().pages[0].delay;
    let remove = arpeggiator::tap_remove_rect(&before.taps[0], arpeggiator::time_domain(&before));
    click(&mut view, &mut cx, target, remove);
    event(
        &mut view,
        &mut cx,
        target,
        remove.0,
        remove.1,
        WindowEvent::KeyDown(Code::Escape, Some(Key::Escape)),
    );
    assert!(view.arp_editor_overlay_visible());
    assert!(view.arp_editor_button_rects().contains(&(remove, false)));
    click(&mut view, &mut cx, target, remove);
    assert_eq!(view.params.arpeggiator.snapshot().pages[0].delay.count, 2);
    click(&mut view, &mut cx, target, arpeggiator::source_rect(1));
    assert!(view.arp_editor_button_rects().contains(&(remove, false)));
    click(&mut view, &mut cx, target, remove);
    assert_eq!(view.params.arpeggiator.snapshot().pages[0].delay.count, 2);
}

#[test]
fn delay_timeline_add_uses_latest_time_and_stops_at_eight_taps() {
    for sync in [true, false] {
        let mut view = string_editor_view();
        view.params.arpeggiator.edit(|s| {
            let d = &mut s.pages[0].delay;
            d.sync = sync;
            d.count = 2;
            d.taps[0].time = if sync { 2.0 } else { 1000.0 };
            d.taps[1].time = if sync { 0.5 } else { 250.0 };
        });
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, arpeggiator::delay_rect());
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
        for count in 2..8 {
            let before = view.params.arpeggiator.snapshot().pages[0].delay;
            let add = arpeggiator::add_rect(&before, arpeggiator::time_domain(&before));
            assert!(view.arp_editor_button_rects().contains(&(add, false)));
            assert!(hit(PIANO_SURFACE, add.0, add.1));
            assert!(hit(PIANO_SURFACE, add.0 + add.2, add.1 + add.3));
            click(&mut view, &mut cx, target, add);
            let after = view.params.arpeggiator.snapshot().pages[0].delay;
            assert_eq!(after.count as usize, count + 1);
            assert_eq!(&after.taps[..count], &before.taps[..count]);
            let latest = before.taps[..count]
                .iter()
                .map(|t| t.time)
                .fold(0.0, f32::max);
            assert_eq!(
                after.taps[count].time,
                latest + if sync { 0.5 } else { 250.0 }
            );
        }
        let full = view.params.arpeggiator.snapshot().pages[0].delay;
        let add = arpeggiator::add_rect(&full, arpeggiator::time_domain(&full));
        assert!(!view.arp_editor_button_rects().contains(&(add, false)));
        click(&mut view, &mut cx, target, add);
        assert_eq!(view.params.arpeggiator.snapshot().pages[0].delay, full);
    }
}

#[test]
fn arp_gate_drag_captures_one_occurrence_across_other_notes_and_resets() {
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    let (x, _) = string_point(0);
    let strip = arpeggiator::gate_rect();
    let y = strip.1 + strip.3 * 0.5;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    let (other, _) = string_point(3);
    event(
        &mut view,
        &mut cx,
        target,
        other,
        y - 100.0,
        WindowEvent::MouseMove(other, y - 100.0),
    );
    release_string(&mut view, &mut cx, target, other, y - 100.0);
    let page = view.params.arpeggiator.snapshot().rows[0].page;
    assert_eq!(page.loops[0].steps[0].gate, 100);
    assert!(page.loops[0].steps[1..4].iter().all(|s| s.gate == 65));
    assert!(page.loops[0].steps.iter().all(|s| s.velocity == 100));
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        x - 200.0,
        y,
        WindowEvent::MouseMove(x - 200.0, y),
    );
    release_string(&mut view, &mut cx, target, x - 200.0, y);
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].page.loops[0].steps[0].gate,
        1
    );
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::CTRL, true);
    click(&mut view, &mut cx, target, (x - 2.0, y - 2.0, 4.0, 4.0));
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].page.loops[0].steps[0].gate,
        65
    );
}

#[test]
fn arp_pattern_paste_keeps_destination_timing_delay_and_global_routes() {
    let mut view = string_editor_view();
    let params = Arc::get_mut(&mut view.params).unwrap();
    params.arp_pages[0].arp_pattern =
        IntParam::new("Ordering", 2, IntRange::Linear { min: 0, max: 6 });
    params.arp_pages[0].arp_octaves =
        IntParam::new("Length", 3, IntRange::Linear { min: 1, max: 4 });
    params.arp_pages[5].rate = FloatParam::new(
        "Rate",
        0.5,
        FloatRange::Linear {
            min: 0.0625,
            max: 2.0,
        },
    );
    view.params.arpeggiator.edit(|s| {
        s.pages[0].ensure_loops(3);
        s.pages[0].loops[1].steps[2].velocity = 31;
        s.pages[0].loops[2].octave = -2;
        s.pages[5].delay.enabled = true;
        s.pages[5].delay.taps[0].pitch = 7;
    });
    let source = view.params.arpeggiator.snapshot().pages[0];
    let destination = view.params.arpeggiator.snapshot().pages[5];
    let routes = view.params.config().routes;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::page_block());
    click(&mut view, &mut cx, target, arpeggiator::page_copy_rect(0));
    view.params
        .arpeggiator
        .edit(|s| s.pages[0].loops[1].steps[2].velocity = 177);
    click(&mut view, &mut cx, target, arpeggiator::page_paste_rect(5));
    // Opening the menu alone must not replace the destination.
    assert_eq!(view.params.arpeggiator.snapshot().pages[5], destination);
    click(
        &mut view,
        &mut cx,
        target,
        arpeggiator::page_paste_option_rect(5, 1),
    );
    assert_eq!(view.arp_ui.edit_page, 0);
    let result = view.params.arpeggiator.snapshot().pages[5];
    assert_eq!(result.loops, source.loops);
    assert_eq!(result.delay, destination.delay);
    let staged = result.replacement.unwrap().settings;
    assert_eq!((staged.pattern, staged.octaves, staged.rate), (2, 3, 0.5));
    assert_eq!(changes.borrow().len(), 2);
    assert_eq!(view.params.config().routes, routes);
}

#[test]
fn arp_horizontal_scroll_keeps_the_whole_row_fitted_and_gate_targets_aligned() {
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    let (x, y) = string_point(3);
    let before = view.params.arpeggiator.snapshot().rows;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseScroll(8.0, 0.0),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows, before);
    let strip = arpeggiator::gate_rect();
    let y = strip.1 + strip.3 * 0.5;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        x + 200.0,
        y,
        WindowEvent::MouseMove(x + 200.0, y),
    );
    release_string(&mut view, &mut cx, target, x + 200.0, y);
    let page = view.params.arpeggiator.snapshot().rows[0].page;
    assert_eq!(page.loops[0].steps[3].gate, 100);
    assert!(page.loops[0].steps[..3].iter().all(|s| s.gate == 65));
}
