use super::super::*;
use super::support::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};

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
    assert_eq!(view.page_position, 2.0);
    assert!(view.arp_editor_open());
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.page_position, 2.0);
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
    (
        ARP_SURFACE.0 + 22.0 + (index as f32 + 0.5) * (ARP_SURFACE.2 - 44.0) / 4.0,
        ARP_SURFACE.1 + 150.0,
    )
}

fn select_adjustment(view: &mut ChordboardView, cx: &mut Context, target: Entity, lane: usize) {
    let r = view
        .arp_editor_button_rects()
        .into_iter()
        .map(|(r, _)| r)
        .find(|r| (r.1 - (ARP_SURFACE.1 + 49.0)).abs() < 0.1 && r.2 > 100.0)
        .unwrap();
    click(view, cx, target, r);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    click(view, cx, target, arpeggiator::chooser_card(lane));
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
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
        y - 42.0,
        WindowEvent::MouseMove(end, y - 42.0),
    );
    release_string(&mut view, &mut cx, target, end, y - 42.0);
    let state = view.params.arpeggiator.snapshot();
    let steps = &state.pages[0].steps[..4];
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
    let button = (x - 5.0, ARP_SURFACE.1 + ARP_SURFACE.3 - 27.0, 10.0, 18.0);
    for lane in 0..7 {
        select_adjustment(&mut view, &mut cx, target, lane);
        for expected in [StepState::Mute, StepState::Skip, StepState::Play] {
            click(&mut view, &mut cx, target, button);
            assert_eq!(
                view.params.arpeggiator.snapshot().pages[0].steps[0].state,
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
            view.params.arpeggiator.snapshot().pages[0].steps[0].state,
            StepState::Play
        );
    }
}

#[test]
fn modifier_drag_resets_every_crossed_adjustment_to_its_default() {
    let mut view = string_editor_view();
    view.params.arpeggiator.edit(|s| {
        for step in &mut s.pages[0].steps[..4] {
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
    assert!(state.pages[0].steps[..4]
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
        y - 21.0,
        WindowEvent::MouseMove(end, y - 21.0),
    );
    release_string(&mut view, &mut cx, target, end, y - 21.0);
    BackendContext::new(&mut cx)
        .modifiers()
        .set(Modifiers::SHIFT, false);
    event(
        &mut view,
        &mut cx,
        target,
        end,
        y - 21.0,
        WindowEvent::KeyUp(Code::ShiftLeft, None),
    );
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(
        state.pages[0].steps[..4]
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
    select_adjustment(&mut view, &mut cx, target, 4);
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
        view.params.arpeggiator.snapshot().pages[0].steps[0].micro,
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
    assert_eq!(state.pages[0].steps[0].micro, 49);
    assert!(state.pages[0].steps[1..4].iter().all(|s| s.micro == 0));
}

#[test]
fn delay_slide_over_blocks_arp_controls_and_has_independent_power() {
    let mut view = string_editor_view();
    assert!(!view.params.arp_enabled.value());
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    // Until the slide settles, an underlying arp control remains covered.
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert!(view.menu.is_none());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    click(&mut view, &mut cx, target, arpeggiator::power_rect());
    assert!(view.params.arpeggiator.snapshot().pages[0].delay.enabled);
    assert!(!view.params.arp_enabled.value());
    click(&mut view, &mut cx, target, arpeggiator::humanize_rect());
    click(&mut view, &mut cx, target, arpeggiator::swing_rect());
    assert!(changes.borrow().is_empty());
    assert!(view.menu.is_none() && view.drag.is_none());
    // Closing the panel does not bypass its independent effect.
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    assert!(view.params.arpeggiator.snapshot().pages[0].delay.enabled);
}

#[test]
fn pages_recall_independent_adjustments_and_delay_sources() {
    use crate::arpeggiator::source;
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let controls = view.arp_editor_button_rects();
    click(&mut view, &mut cx, target, controls[2].0); // Advanced
    let controls = view.arp_editor_button_rects();
    assert_eq!(controls.len(), 16);
    click(&mut view, &mut cx, target, controls[12].0); // Manual Strum input
    click(&mut view, &mut cx, target, controls[13].0); // Chords input
    let sources = source::ARP | source::STRUM | source::CHORDS;
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[0].delay.sources,
        sources
    );
    // Closing and selecting another Page preserves the first Page's complete variation.
    click(&mut view, &mut cx, target, arpeggiator::delay_rect());
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    let page_two = (ARP_SURFACE.0 + 92.0, ARP_SURFACE.1 + 49.0, 28.0, 25.0);
    click(&mut view, &mut cx, target, page_two);
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
        y - 21.0,
        WindowEvent::MouseMove(x, y - 21.0),
    );
    release_string(&mut view, &mut cx, target, x, y - 21.0);
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.pages[1].steps[2].velocity, 140);
    assert_eq!(state.pages[0].steps[2].velocity, 100);
    assert_eq!(state.pages[1].delay.sources, source::ARP);
    click(
        &mut view,
        &mut cx,
        target,
        (ARP_SURFACE.0 + 58.0, ARP_SURFACE.1 + 49.0, 28.0, 25.0),
    );
    assert_eq!(view.arp_ui.edit_page, 0);
    assert_eq!(
        view.params.arpeggiator.snapshot().pages[0].delay.sources,
        sources
    );
}

#[test]
fn large_arp_chooser_captures_strings_and_shift_without_changing_latch() {
    let mut view = string_editor_view();
    let (mut cx, target, changes) = context();
    let r = view.arp_editor_button_rects().into_iter().map(|(r, _)| r)
        .find(|r| (r.1 - (ARP_SURFACE.1 + 49.0)).abs() < 0.1 && r.2 > 100.0).unwrap();
    click(&mut view, &mut cx, target, r);
    assert!(view.arp_chooser_open());
    let before = view.params.arpeggiator.snapshot();
    let (x, y) = string_point(1);
    // While sliding, covered strings do not start a paint gesture or select a card.
    event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
    assert!(!view.arp_editor_dragging());
    assert_eq!(view.params.arpeggiator.snapshot(), before);
    BackendContext::new(&mut cx).modifiers().set(Modifiers::SHIFT, true);
    event(&mut view, &mut cx, target, x, y, WindowEvent::KeyDown(Code::ShiftLeft, Some(Key::Shift)));
    assert!(!changes.borrow().iter().any(|(ptr, _)| *ptr == view.params.latch.as_ptr()));
    BackendContext::new(&mut cx).modifiers().set(Modifiers::SHIFT, false);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    event(&mut view, &mut cx, target, x, y, WindowEvent::KeyDown(Code::ArrowRight, Some(Key::ArrowRight)));
    event(&mut view, &mut cx, target, x, y, WindowEvent::KeyDown(Code::Enter, Some(Key::Enter)));
    assert!(!view.arp_chooser_open());
    assert_eq!(view.params.arpeggiator.snapshot(), before);
}
