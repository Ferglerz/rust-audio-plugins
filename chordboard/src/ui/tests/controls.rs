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
    let command = if cfg!(target_os = "macos") {
        Modifiers::LOGO
    } else {
        Modifiers::CTRL
    };
    BackendContext::new(&mut cx).modifiers().set(command, true);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::KeyDown(Code::KeyA, None),
    );
    BackendContext::new(&mut cx).modifiers().set(command, false);
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
    for double in [false] {
        for id in ["length_ms", "swing", "humanize", "arp_free_ms"] {
            let mut view = view(2, false);
            view.params.arpeggiator.edit(|s| {
                s.editor_row_mut(0, !view.programming).settings.swing = 0.5;
                s.editor_row_mut(0, !view.programming).settings.humanize = 0.8;
                s.editor_row_mut(0, !view.programming).settings.sync = false;
                s.editor_row_mut(0, !view.programming).settings.free_ms = 500.0;
            });
            view.refresh_arp_editor_params();
            let (mut cx, target, changes) = context();
            let (control, rect) = view
                .placed_controls()
                .into_iter()
                .find(|(c, _)| c.id == id)
                .unwrap();
            let value = if matches!(id, "swing" | "humanize") { arpeggiator::adjustment_value_rect(rect) } else { pleasant_ui::slider_value_rect(rect) };
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
        for id in ["velocity_scale", "base_inversion", "transpose"] {
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
fn arp_modes_and_octaves_edit_the_selected_tracker_row_independently() {
    let mut view = view(0, false);
    open_arp(&mut view);
    ensure_tracker_rows(&view, 4);
    view.arp_ui.edit_page = 3;
    view.refresh_arp_editor_params();
    let untouched = view.params.arpeggiator.snapshot().rows[..3].to_vec();
    let (mut cx, target, changes) = context();
    for mode in [0, 2] {
        let r = arpeggiator::mode_rect();
        click(&mut view, &mut cx, target, r);
        let p = &view.arp_params().arp_mode;
        let expected = (p.as_ptr(), p.preview_normalized(mode));
        assert_eq!(changes.borrow().last().copied(), Some(expected));
        assert!(
            view.arp_row_param_event(&RawParamEvent::SetParameterNormalized(
                expected.0, expected.1
            ))
        );
        assert_eq!(
            view.params.arpeggiator.snapshot().editor_row(3, !view.programming).settings.mode,
            crate::arpeggiator::ArpMode::from_index(mode as u8)
        );
    }
    changes.borrow_mut().clear();
    click(&mut view, &mut cx, target, arpeggiator::tracker_voicing_button(false, true));
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[3].arp_octave,
        1
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows[..3], untouched);
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
fn direct_learn_and_global_transpose_knob() {
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
    let r = globals::global_rect("transpose");
    let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(changes.borrow().is_empty(), "knob press must not jump");
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y - 36.0,
        WindowEvent::MouseMove(x, y - 36.0),
    );
    let (ptr, norm) = changes.borrow().last().copied().unwrap();
    assert_eq!(ptr, view.params.transpose.as_ptr());
    assert!(view.params.transpose.preview_plain(norm) > 0);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y - 36.0,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(view.drag.is_none());
    assert!(view.edit.is_none());
    click(&mut view, &mut cx, target, globals::value_rect("transpose"));
    assert_eq!(
        view.edit.as_ref().map(|edit| edit.target),
        Some("transpose")
    );
}

#[test]
fn global_learn_mode_uses_whole_knob_and_blocks_value_edits() {
    let mut view = view(0, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, midi_learn::switch_rect());
    view.tick_quick_midi(
        &mut EventContext::new_with_current(&mut cx, target),
        pleasant_ui::page_slide::DURATION,
    );
    let r = globals::global_rect("transpose");
    click(&mut view, &mut cx, target, r);
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 4);
    assert!(view.drag.is_none());
    event(
        &mut view,
        &mut cx,
        target,
        r.0 + 20.0,
        r.1 + 20.0,
        WindowEvent::MouseScroll(0.0, 1.0),
    );
    click(&mut view, &mut cx, target, ORDER);
    click(&mut view, &mut cx, target, arpeggiator::sync_rect());
    assert!(
        changes.borrow().is_empty(),
        "dimmed controls must not change parameters"
    );
    click(&mut view, &mut cx, target, globals::value_rect("transpose"));
    assert!(view.edit.is_none());
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
    event(
        &mut view,
        &mut cx,
        target,
        r.0,
        r.1,
        WindowEvent::KeyDown(Code::Escape, None),
    );
    assert!(!view.quick_midi.show);
    click(&mut view, &mut cx, target, globals::value_rect("transpose"));
    assert_eq!(
        view.edit.as_ref().map(|edit| edit.target),
        Some("transpose")
    );
}

#[test]
fn native_repeated_clicks_reach_all_chord_and_parameter_buttons() {
    for button in 0..69 {
        // Transpose now uses a knob tested above; arp rate and ordering use dropdowns.
        // 47..=53 were the direction buttons; direction is a dropdown now.
        if (36..=40).contains(&button)
            || (42..=44).contains(&button)
            || (47..=53).contains(&button)
            || (54..=61).contains(&button)
            || button == 66
        {
            continue;
        }
        let mut cx = Context::default();
        let params = Arc::new(ChordboardParams {
            scale_layout: IntParam::new("Layout", 0, IntRange::Linear { min: 0, max: 2 }),
            ..ChordboardParams::default()
        });
        let bridge = Arc::new(Bridge::default());
        let mut view =
            ChordboardView::new(params.clone(), bridge.clone(), Arc::new(PreviewContext));
        if button == 41 {
            open_arp(&mut view);
        }
        let target = view.build(&mut cx, |_| {}).focusable(true).entity();
        BackendContext::new(&mut cx)
            .cache()
            .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, EDITOR_W, H));
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
            backend.emit_origin(WindowEvent::MouseMove(
                CONTENT_X + r.0 + r.2 / 2.0,
                r.1 + r.3 / 2.0,
            ));
            backend.emit_origin(WindowEvent::MouseDown(MouseButton::Left));
            backend.process_events();
            if expected.is_none() {
                assert!(
                    std::iter::from_fn(|| bridge.commands.pop())
                        .any(|c| matches!(c, Command::KeyDown(key, _, _) if key == button as u8 + POINTER_KEY_OFFSET)),
                    "chord click {press} was lost by native dispatch"
                );
            } else if button == 41 {
                assert_eq!(params.arpeggiator.snapshot().rows[0].enabled, press % 2 == 0);
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
        view.params
            .arpeggiator
            .edit(|s| s.editor_row_mut(0, !view.programming).settings.sync = false);
        view.refresh_arp_editor_params();
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
    view.params
        .arpeggiator
        .edit(|s| s.editor_row_mut(0, !view.programming).settings.sync = false);
    view.refresh_arp_editor_params();
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
        scale_layout: IntParam::new("Layout", 0, IntRange::Linear { min: 0, max: 2 }),
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
fn tempo_is_in_tracker_and_arp_rate_sync_is_page_specific() {
    for mode in [1, 3] {
        let mut view = view(mode, false);
        view.programming = true;
        view.expand_progress = if mode == 3 { 1.0 } else { 0.0 };
        view.expand_target = view.expand_progress;
        view.refresh_arp_editor_params();
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
        view.params
            .arpeggiator
            .edit(|s| s.editor_row_mut(0, !view.programming).settings.sync = false);
        view.refresh_arp_editor_params();
        assert!(view
            .base_controls()
            .iter()
            .any(|(c, _)| c.id == "arp_free_ms"));
    }
}

#[test]
fn tracker_tempo_rail_drags_only_in_manual_mode() {
    for (sync, expand) in [(false, 0.0), (false, 1.0), (true, 0.0)] {
        let mut view = view(0, false);
        Arc::get_mut(&mut view.params).unwrap().tempo_sync = BoolParam::new("Host sync", sync);
        view.programming = true;
        view.expand_progress = expand;
        view.expand_target = expand;
        let (mut cx, target, changes) = context();
        let (left, top, width, _) = TEMPO_CONTROL;
        let (x, y) = (left + width * 0.5, top + 12.0);
        event(
            &mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(changes.borrow().is_empty(), "press must not jump or edit an octave");
        event(
            &mut view, &mut cx, target, x + 36.0, y, WindowEvent::MouseMove(x + 36.0, y),
        );
        event(
            &mut view, &mut cx, target, x + 36.0, y, WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(view.drag.is_none());
        if sync {
            assert!(changes.borrow().is_empty());
        } else {
            let tempo = &view.params.tempo;
            assert!(!changes.borrow().is_empty(), "manual tempo drag must emit a change");
            assert!(changes.borrow().iter().all(|(ptr, norm)| *ptr == tempo.as_ptr()
                && *norm > tempo.unmodulated_normalized_value()));
            click(&mut view, &mut cx, target, pleasant_ui::slider_value_rect(TEMPO_CONTROL));
            assert_eq!(view.edit.as_ref().map(|edit| edit.target), Some("tempo"));
        }
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

fn ensure_tracker_rows(view: &ChordboardView, count: usize) {
    view.params.arpeggiator.edit(|state| {
        while state.rows.len() < count {
            state.rows.push(crate::arpeggiator::TrackerRow {
                id: state.rows.len() as u64 + 1,
                ..crate::arpeggiator::TrackerRow::default()
            });
        }
    });
}

fn select_tracker_row(view: &mut ChordboardView, cx: &mut Context, target: Entity, row: usize) {
    let was_fx = view.fx_ui.open;
    ensure_tracker_rows(view, row + 1);
    click(view, cx, target, midi_fx::PROGRAMMING_TAB);
    let r = tracker_row_rect(view, row);
    let header = tracker::column_header_rect();
    click(view, cx, target, (r.0 + 10.0, header.1, 40.0, header.3));
    assert_eq!(view.arp_ui.edit_page, row);
    event(view, cx, target, r.0, header.1, WindowEvent::KeyDown(Code::Escape, None));
    assert!(!view.tracker_selection);
    view.refresh_arp_editor_params();
    view.row_slide.tick(pleasant_ui::page_slide::DURATION);
    if was_fx { click(view, cx, target, midi_fx::FX_TAB); }
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
    let velocity_rise = arpeggiator::graph_rect().3 * 0.4;
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
        y - velocity_rise,
        WindowEvent::MouseMove(end, y - velocity_rise),
    );
    release_string(&mut view, &mut cx, target, end, y - velocity_rise);
    let state = view.params.arpeggiator.snapshot();
    let steps = &state.editor_row(0, !view.programming).page.loops[0].steps[..4];
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
                view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page.loops[0].steps[0].state,
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
            view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page.loops[0].steps[0].state,
            StepState::Play
        );
    }
}

#[test]
fn modifier_drag_resets_every_crossed_adjustment_to_its_default() {
    let mut view = string_editor_view();
    view.params.arpeggiator.edit(|s| {
        for step in &mut s.editor_row_mut(0, !view.programming).page.loops[0].steps[..4] {
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
    assert!(state.editor_row(0, !view.programming).page.loops[0].steps[..4]
        .iter()
        .all(|s| s.velocity == 100 && s.gate == 90));
    assert!(changes.borrow().is_empty());
}

#[test]
fn shift_paints_a_straight_origin_ramp_without_toggling_latch() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    let velocity_rise = arpeggiator::graph_rect().3 * 0.2;
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
        y - velocity_rise,
        WindowEvent::MouseMove(end, y - velocity_rise),
    );
    release_string(&mut view, &mut cx, target, end, y - velocity_rise);
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, false);
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y - velocity_rise,
        WindowEvent::KeyUp(Code::ShiftLeft, None),
    );
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(
        state.editor_row(0, !view.programming).page.loops[0].steps[..4]
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
    for lane in 0..6 {
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    select_adjustment(&mut view, &mut cx, target, lane);
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
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y - 45.0,
        WindowEvent::MouseMove(x, y - 45.0),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page.loops[0].steps[0].micro,
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
    assert_eq!(state.editor_row(0, !view.programming).page.loops[0].steps[0].micro, 49);
    assert!(state.editor_row(0, !view.programming).page.loops[0].steps[1..4]
        .iter()
        .all(|s| s.micro == 0));
}
}

#[test]
fn delay_harmony_switches_are_shared_by_all_tracker_rows() {
    let mut view = string_editor_view();
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, midi_fx::FX_TAB);
    click(
        &mut view,
        &mut cx,
        target,
        arpeggiator::delay_leading_rect(),
    );
    click(&mut view, &mut cx, target, arpeggiator::delay_follow_rect());
    select_tracker_row(&mut view, &mut cx, target, 1);
    let delay = view.params.arpeggiator.snapshot().midi_fx.delay;
    assert!(delay.voice_leading && delay.follow_chords);
    click(&mut view, &mut cx, target, arpeggiator::delay_follow_rect());
    select_tracker_row(&mut view, &mut cx, target, 0);
    let delay = view.params.arpeggiator.snapshot().midi_fx.delay;
    assert!(delay.voice_leading && !delay.follow_chords);
    assert!(view.fx_ui.open);
}

#[test]
fn delay_fx_page_blocks_hidden_controls_and_has_module_power() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    assert!(!view.fx_ui.open);
    click(&mut view, &mut cx, target, midi_fx::FX_TAB);
    click(&mut view, &mut cx, target, ALWAYS_BASS);
    assert!(changes.borrow().is_empty());
    click(&mut view, &mut cx, target, arpeggiator::delay_power_rect());
    assert!(view.params.arpeggiator.snapshot().midi_fx.delay.enabled);
    assert!(!view.params.arp_enabled.value());
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert!(!view.arp_chooser_open());
    assert!(changes.borrow().is_empty());
    click(&mut view, &mut cx, target, midi_fx::PERFORMANCE_TAB);
    assert!(!view.fx_ui.open);
    assert!(view.params.arpeggiator.snapshot().midi_fx.delay.enabled);
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert_eq!(view.menu, Some(Menu::ArpPattern));
}

#[test]
fn tracker_rows_recall_independent_adjustments_but_share_delay_sources() {
    use crate::midi_fx::source;
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, midi_fx::FX_TAB);
    click(&mut view, &mut cx, target, arpeggiator::source_rect(1));
    click(&mut view, &mut cx, target, arpeggiator::source_rect(2));
    let sources = source::ARP | source::STRUM | source::CHORDS;
    click(&mut view, &mut cx, target, midi_fx::PERFORMANCE_TAB);
    select_tracker_row(&mut view, &mut cx, target, 1);
    assert_eq!(view.arp_ui.edit_page, 1);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let (x, y) = string_point(2);
    let velocity_rise = arpeggiator::graph_rect().3 * 0.2;
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
        y - velocity_rise,
        WindowEvent::MouseMove(x, y - velocity_rise),
    );
    release_string(&mut view, &mut cx, target, x, y - velocity_rise);
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.editor_row(1, !view.programming).page.loops[0].steps[2].velocity, 140);
    assert_eq!(state.editor_row(0, !view.programming).page.loops[0].steps[2].velocity, 100);
    assert_eq!(state.midi_fx.delay.sources, sources);
    select_tracker_row(&mut view, &mut cx, target, 0);
    assert_eq!(
        view.params.arpeggiator.snapshot().midi_fx.delay.sources,
        sources
    );
    assert!(!changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.arp_page.as_ptr()));
}

#[test]
fn direction_dropdown_captures_strings_and_shift_without_changing_latch() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert_eq!(view.menu, Some(Menu::ArpPattern));
    let before = view.params.arpeggiator.snapshot();
    let (x, y) = string_point(1);
    // An outside click dismisses the dropdown without starting a string gesture.
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
    view.open_menu(Menu::ArpPattern);
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
fn tracker_copy_paste_clones_complete_row_without_host_parameters() {
    let mut view = string_editor_view();
    ensure_tracker_rows(&view, 6);
    view.params.arpeggiator.edit(|s| {
        let row = &mut s.editor_row_mut(0, !view.programming);
        row.settings.pattern = 4;
        row.settings.octaves = 4;
        row.settings.rate = 0.5;
        row.duration = 3;
        row.chord = Some(harmony::parse_chord_symbol("Dm9").unwrap());
        row.page.ensure_loops(4);
        row.page.loops[3].steps[23].velocity = 31;
        row.page.loops[0].steps[4].state = crate::arpeggiator::StepState::Skip;
        for (variation, octave) in row.page.loops.iter_mut().zip([1, 0, -1, 2]) {
            variation.octave = octave;
        }
        s.midi_fx.delay.enabled = true;
        s.midi_fx.delay.taps[4].pitch = 7;
    });
    let expected = view.params.arpeggiator.snapshot().editor_row(0, !view.programming).clone();
    let destination_id = view.params.arpeggiator.snapshot().editor_row(5, !view.programming).id;
    let (mut cx, target, changes) = context();
    select_tracker_row(&mut view, &mut cx, target, 0);
    click(&mut view, &mut cx, target, tracker::action_rect(1));
    select_tracker_row(&mut view, &mut cx, target, 5);
    click(&mut view, &mut cx, target, tracker::action_rect(2));
    assert_eq!(view.arp_ui.edit_page, 5);
    let copied = view.params.arpeggiator.snapshot().editor_row(5, !view.programming).clone();
    assert_eq!(copied.id, destination_id);
    assert_eq!(
        (copied.chord, copied.duration),
        (expected.chord, expected.duration)
    );
    assert_eq!(copied.settings, expected.settings);
    assert_eq!(copied.page, expected.page);
    assert!(
        changes.borrow().is_empty(),
        "row paste persists directly, without host parameter automation"
    );
    view.params
        .arpeggiator
        .edit(|s| s.editor_row_mut(5, !view.programming).page.loops[3].steps[23].velocity = 90);
    assert_eq!(view.params.arpeggiator.snapshot().editor_row(0, !view.programming), &expected);
}

#[test]
fn arp_loop_octave_controls_edit_each_pass_without_touching_step_adjustments() {
    let mut view = string_editor_view();
    view.programming = false;
    view.params.arpeggiator.edit(|s| {
        s.editor_row_mut(0, !view.programming).settings.octaves = 4;
        s.editor_row_mut(0, !view.programming).page.ensure_loops(4);
    });
    view.refresh_arp_editor_params();
    let (mut cx, target, _) = context();
    view.tick_arp_editor(0.0);
    for i in 0..4 {
        let r = view.loop_button_rect(i, 4);
        click(&mut view, &mut cx, target, r);
    }
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.editor_row(0, !view.programming).page.octaves(), [1; 4]);
    assert!(state.editor_row(0, !view.programming).page.loops.iter().all(|variation| variation
        .steps
        .iter()
        .all(|step| *step == crate::arpeggiator::Step::default())));
}

#[test]
fn root_popover_uses_black_key_priority_preserves_spelling_and_contains_scale() {
    let mut view = string_editor_view();
    view.programming = false;
    view.refresh_arp_editor_params();
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
    view.programming = false;
    view.refresh_arp_editor_params();
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
            let d = &mut s.midi_fx.delay;
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
        let before = view.params.arpeggiator.snapshot().midi_fx.delay;
        let domain = arpeggiator::time_domain(&before);
        let remove = arpeggiator::tap_remove_rect(&before.taps[1], domain);
        click(&mut view, &mut cx, target, remove);
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.delay, before);
        assert!(view.arp_editor_button_rects().contains(&(remove, true)));
        click(&mut view, &mut cx, target, remove);
        let after = view.params.arpeggiator.snapshot().midi_fx.delay;
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
        s.midi_fx.delay.count = 2;
        s.midi_fx.delay.taps[1].time = 1.0;
    });
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let before = view.params.arpeggiator.snapshot().midi_fx.delay;
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
    assert!(view.fx_ui.open);
    assert!(view.arp_editor_button_rects().contains(&(remove, false)));
    click(&mut view, &mut cx, target, remove);
    assert_eq!(view.params.arpeggiator.snapshot().midi_fx.delay.count, 2);
    click(&mut view, &mut cx, target, arpeggiator::source_rect(1));
    assert!(view.arp_editor_button_rects().contains(&(remove, false)));
    click(&mut view, &mut cx, target, remove);
    assert_eq!(view.params.arpeggiator.snapshot().midi_fx.delay.count, 2);
}

#[test]
fn delay_timeline_add_uses_latest_time_and_stops_at_eight_taps() {
    for sync in [true, false] {
        let mut view = string_editor_view();
        view.params.arpeggiator.edit(|s| {
            let d = &mut s.midi_fx.delay;
            d.sync = sync;
            d.count = 2;
            d.taps[0].time = if sync { 2.0 } else { 1000.0 };
            d.taps[1].time = if sync { 0.5 } else { 250.0 };
        });
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, arpeggiator::delay_rect());
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
        for count in 2..8 {
            let before = view.params.arpeggiator.snapshot().midi_fx.delay;
            let add = arpeggiator::add_rect(&before, arpeggiator::time_domain(&before));
            assert!(view.arp_editor_button_rects().contains(&(add, false)));
            assert!(hit(midi_fx::DELAY_SURFACE, add.0, add.1));
            assert!(hit(midi_fx::DELAY_SURFACE, add.0 + add.2, add.1 + add.3));
            click(&mut view, &mut cx, target, add);
            let after = view.params.arpeggiator.snapshot().midi_fx.delay;
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
        let full = view.params.arpeggiator.snapshot().midi_fx.delay;
        let add = arpeggiator::add_rect(&full, arpeggiator::time_domain(&full));
        assert!(!view.arp_editor_button_rects().contains(&(add, false)));
        click(&mut view, &mut cx, target, add);
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.delay, full);
    }
}

#[test]
fn arp_timing_drag_captures_one_occurrence_across_other_notes_and_resets() {
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
    let page = view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page;
    assert_eq!(page.loops[0].steps[0].micro, 49);
    assert!(page.loops[0].steps[1..4].iter().all(|s| s.micro == 0));
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
        x - 1000.0,
        y,
        WindowEvent::MouseMove(x - 1000.0, y),
    );
    release_string(&mut view, &mut cx, target, x - 1000.0, y);
    assert_eq!(
        view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page.loops[0].steps[0].micro,
        -49
    );
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::CTRL, true);
    click(&mut view, &mut cx, target, (x - 2.0, y - 2.0, 4.0, 4.0));
    assert_eq!(
        view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page.loops[0].steps[0].micro,
        0
    );
}

#[test]
fn arp_pattern_paste_keeps_destination_timing_delay_and_global_routes() {
    let mut view = string_editor_view();
    ensure_tracker_rows(&view, 6);
    view.params.arpeggiator.edit(|s| {
        s.editor_row_mut(0, !view.programming).settings.pattern = 2;
        s.editor_row_mut(0, !view.programming).settings.octaves = 3;
        s.editor_row_mut(0, !view.programming).settings.custom[0].degree = 5;
        s.editor_row_mut(0, !view.programming).page.ensure_loops(3);
        s.editor_row_mut(0, !view.programming).page.loops[1].steps[2].velocity = 31;
        s.editor_row_mut(0, !view.programming).page.loops[2].octave = -2;
        s.editor_row_mut(5, !view.programming).settings.rate = 0.5;
        s.editor_row_mut(5, !view.programming).settings.swing = 0.2;
        s.midi_fx.delay.enabled = true;
        s.midi_fx.delay.taps[0].pitch = 7;
    });
    let source = view.params.arpeggiator.snapshot().editor_row(0, !view.programming).clone();
    let destination = view.params.arpeggiator.snapshot().editor_row(5, !view.programming).clone();
    let routes = view.params.config().routes;
    let (mut cx, target, changes) = context();
    view.copy_arp_page(0);
    view.params
        .arpeggiator
        .edit(|s| s.editor_row_mut(0, !view.programming).page.loops[1].steps[2].velocity = 177);
    assert_eq!(view.params.arpeggiator.snapshot().editor_row(5, !view.programming), &destination);
    view.paste_arp_page(
        &mut EventContext::new_with_current(&mut cx, target),
        5,
        true,
    );
    assert_eq!(view.arp_ui.edit_page, 0);
    let result = view.params.arpeggiator.snapshot().editor_row(5, !view.programming).clone();
    assert_eq!(result.page.loops, source.page.loops);
    assert_eq!(
        view.params.arpeggiator.snapshot().midi_fx.delay.taps[0].pitch,
        7
    );
    let expected_settings = crate::arpeggiator::Settings {
        pattern: source.settings.pattern,
        octaves: source.settings.octaves,
        lock_length: source.settings.lock_length,
        steps: source.settings.steps,
        fill: source.settings.fill,
        custom: source.settings.custom,
        ..destination.settings
    };
    assert_eq!(result.settings, expected_settings);
    assert_eq!(
        (result.id, result.chord, result.duration),
        (destination.id, destination.chord, destination.duration)
    );
    assert!(changes.borrow().is_empty());
    assert_eq!(view.params.config().routes, routes);
}

#[test]
fn arp_horizontal_scroll_keeps_the_whole_row_fitted_and_timing_targets_aligned() {
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
    let page = view.params.arpeggiator.snapshot().editor_row(0, !view.programming).page;
    assert_eq!(page.loops[0].steps[3].micro, 49);
    assert!(page.loops[0].steps[..3].iter().all(|s| s.micro == 0));
}

#[test]
fn relocated_performance_knobs_keep_drag_scroll_and_value_entry() {
    for id in globals::GLOBAL_IDS.into_iter().chain(["gliss_ms"]) {
        let mut view = view(0, false);
        let (mut cx, target, changes) = context();
        let control = view.control(id).unwrap();
        let r = if id == "gliss_ms" { GLISS_RATE } else { globals::global_rect(id) };
        let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        assert!(changes.borrow().is_empty(), "{id}: press must not jump");
        event(&mut view, &mut cx, target, x, y - 40.0, WindowEvent::MouseMove(x, y - 40.0));
        assert!(changes.borrow().iter().any(|(ptr, norm)| *ptr == control.ptr && *norm > control.norm), "{id}: drag must edit its parameter");
        event(&mut view, &mut cx, target, x, y - 40.0, WindowEvent::MouseUp(MouseButton::Left));
        assert!(view.drag.is_none());
        changes.borrow_mut().clear();
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseScroll(0.0, 1.0));
        assert!(changes.borrow().iter().any(|(ptr, _)| *ptr == control.ptr), "{id}: wheel must reach knob");
        let value = if id == "gliss_ms" { GLISS_VALUE } else { globals::value_rect(id) };
        click(&mut view, &mut cx, target, value);
        assert_eq!(view.edit.as_ref().map(|edit| edit.target), Some(id));
    }
}

#[test]
fn global_knobs_stay_available_during_row_slides_but_not_on_fx() {
    for fx in [false, true] {
        let mut view = view(0, false);
        let (mut cx, target, changes) = context();
        view.fx_ui.open = fx;
        if !fx {
            view.row_slide.select(1, 0);
            view.row_slide.select(2, 1);
        }
        for (_, r) in globals::global_controls() {
            let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
            event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
            assert_eq!(view.drag.is_none(), fx);
            event(&mut view, &mut cx, target, x, y - 40.0, WindowEvent::MouseMove(x, y - 40.0));
            event(&mut view, &mut cx, target, x, y, WindowEvent::MouseUp(MouseButton::Left));
            assert_eq!(changes.borrow().is_empty(), fx);
            changes.borrow_mut().clear();
        }
        if !fx {
            let r = globals::global_rect("complexity");
            view.pointer = Some((r.0 + r.2 * 0.5, r.1 + r.3 * 0.5));
            assert!(view.hover_pointer().is_some());
        }
    }
}
