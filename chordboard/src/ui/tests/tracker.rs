use super::super::*;
use super::support::*;

fn first_chord_cell() -> Rect {
    let r = tracker::row_rect(0, 0.0);
    (r.0 + 50.0, r.1, r.2 - 198.0, r.3)
}

#[test]
fn tracker_done_exits_chord_selection_without_closing_or_changing_the_row() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, first_chord_cell());
    assert!(view.tracker_selection);
    let rows = view.params.arpeggiator.snapshot().rows;
    click(&mut view, &mut cx, target, tracker::action_rect(4));
    assert!(!view.tracker_selection);
    assert!(view.tracker_open);
    assert_eq!(view.params.arpeggiator.snapshot().rows, rows);
}

#[test]
fn tracker_done_accepts_a_valid_typed_chord_and_ends_editing() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    view.tracker_symbol_edit = true;
    view.tracker_symbol = "Dm7".into();
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, tracker::action_rect(4));
    assert!(!view.tracker_symbol_edit);
    assert!(!view.tracker_selection);
    assert!(view.tracker_symbol.is_empty());
    assert!(view.tracker_open);
    let chord = harmony::parse_chord_symbol("Dm7").unwrap();
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].chord,
        Some(SavedChord {
            root: 60 + chord.root,
            ..chord
        })
    );
}

#[test]
fn tracker_keyboard_assignment_keeps_selection_mode_open_without_audition() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    let (mut cx, target, _) = context();
    let row = first_chord_cell();
    click(&mut view, &mut cx, target, row);
    assert!(view.tracker_selection);
    let index = (0..KEY_COUNT)
        .find(|&i| view.keyboard_chord(i).is_some())
        .unwrap();
    let key = view.keyboard_key_rect(index);
    click(&mut view, &mut cx, target, key);
    assert!(view.tracker_selection);
    let saved = view.params.arpeggiator.snapshot().rows[0].chord.unwrap();
    assert_eq!(saved.root, view.params.keyboard_octave.value() as u8);
    assert!(view.tracker_selection);
    assert!(view.bridge.commands.is_empty());
    click(&mut view, &mut cx, target, row);
    assert!(view.tracker_selection);
    event(
        &mut view,
        &mut cx,
        target,
        row.0,
        row.1,
        WindowEvent::KeyDown(Code::Escape, None),
    );
    assert!(!view.tracker_selection);
}

#[test]
fn tracker_row_selection_allows_chord_changes_from_the_row_edge() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
    });
    let original = view.params.arpeggiator.snapshot().rows[0].chord;
    let (mut cx, target, _) = context();
    let row = tracker::row_rect(1, 0.0);
    click(
        &mut view,
        &mut cx,
        target,
        (row.0 + row.2 - 10.0, row.1, 8.0, row.3),
    );
    assert_eq!(view.arp_ui.edit_page, 1);
    assert!(view.tracker_selection);
    let index = (0..KEY_COUNT)
        .find(|&i| view.keyboard_chord(i).is_some())
        .unwrap();
    let key = view.keyboard_key_rect(index);
    click(&mut view, &mut cx, target, key);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows[0].chord, original);
    assert_eq!(
        rows[1].chord.unwrap().root,
        view.params.keyboard_octave.value() as u8
    );
    assert!(view.bridge.commands.is_empty());
}

#[test]
fn tracker_loop_buttons_change_duration_without_starting_chord_selection() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    let (mut cx, target, _) = context();
    let row = tracker::row_rect(0, 0.0);
    let initial = view.params.arpeggiator.snapshot().rows[0].duration;
    click(
        &mut view,
        &mut cx,
        target,
        (row.0 + row.2 - 40.0, row.1 + 4.0, 28.0, 28.0),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].duration,
        initial + 1
    );
    click(
        &mut view,
        &mut cx,
        target,
        (row.0 + row.2 - 120.0, row.1 + 4.0, 28.0, 28.0),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows[0].duration, initial);
    assert!(!view.tracker_selection);
}

#[test]
fn tracker_double_click_enters_and_validates_chord_symbols() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    let (mut cx, target, _) = context();
    let row = first_chord_cell();
    click(&mut view, &mut cx, target, row);
    let previous = view.params.arpeggiator.snapshot().rows[0].chord;
    event(
        &mut view,
        &mut cx,
        target,
        row.0 + 12.0,
        row.1 + 12.0,
        WindowEvent::MouseDoubleClick(MouseButton::Left),
    );
    assert!(view.tracker_symbol_edit);
    for ch in ['D', 'm', '7'] {
        event(
            &mut view,
            &mut cx,
            target,
            row.0,
            row.1,
            WindowEvent::CharInput(ch),
        );
    }
    event(
        &mut view,
        &mut cx,
        target,
        row.0,
        row.1,
        WindowEvent::KeyDown(Code::Enter, None),
    );
    let saved = view.params.arpeggiator.snapshot().rows[0].chord.unwrap();
    assert_eq!((saved.root, saved.quality), (62, 4));
    event(
        &mut view,
        &mut cx,
        target,
        row.0 + 12.0,
        row.1 + 12.0,
        WindowEvent::MouseDoubleClick(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        row.0,
        row.1,
        WindowEvent::CharInput('H'),
    );
    event(
        &mut view,
        &mut cx,
        target,
        row.0,
        row.1,
        WindowEvent::KeyDown(Code::Enter, None),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].chord,
        Some(saved)
    );
    assert_ne!(previous, Some(saved));
}

#[test]
fn tracker_toggle_cannot_open_from_a_bypassed_arp_but_can_close() {
    let mut view = view(2, false);
    assert!(!view.params.arp_enabled.value());
    view.expand_progress = 1.0;
    view.expand_start = 1.0;
    view.expand_target = 1.0;
    let (mut cx, target, changes) = context();
    let toggle = tracker::toggle_rect();
    for press in [
        WindowEvent::MouseDown(MouseButton::Left),
        WindowEvent::MouseDoubleClick(MouseButton::Left),
        WindowEvent::MouseTripleClick(MouseButton::Left),
    ] {
        event(
            &mut view,
            &mut cx,
            target,
            toggle.0 + toggle.2 / 2.0,
            toggle.1 + toggle.3 / 2.0,
            press,
        );
        assert!(!view.tracker_open);
        assert!(!view.tracker_restore_expand);
        assert_eq!(view.expand_target, 1.0);
        assert!(changes.borrow().is_empty());
    }
    view.tracker_open = true;
    click(&mut view, &mut cx, target, toggle);
    assert!(!view.tracker_open);
}

#[test]
fn tracker_bypass_button_slides_back_and_cancels_row_interactions() {
    let mut view = view(2, false);
    Arc::get_mut(&mut view.params).unwrap().arp_enabled = BoolParam::new("Arpeggiator", true);
    view.tracker_open = true;
    view.tracker_progress = 0.7;
    view.tracker_selection = true;
    view.tracker_symbol_edit = true;
    view.tracker_symbol = "Dm".into();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::power_rect());
    assert!(!view.tracker_open);
    assert_eq!(view.tracker_start, 0.7);
    assert_eq!(view.tracker_elapsed, 0.0);
    assert!(!view.tracker_selection);
    assert!(!view.tracker_symbol_edit);
    assert!(view.tracker_symbol.is_empty());
    assert!(changes
        .borrow()
        .contains(&(view.params.arp_enabled.as_ptr(), 0.0)));
}

#[test]
fn tracker_host_bypass_closes_and_restores_the_expanded_strumfield() {
    let mut view = view(2, false);
    Arc::get_mut(&mut view.params).unwrap().arp_enabled = BoolParam::new("Arpeggiator", true);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    view.tracker_start = 1.0;
    view.tracker_selection = true;
    view.tracker_drag = Some((0, 0));
    view.tracker_restore_expand = true;
    let (mut cx, target, _) = context();
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.tracker_open);

    Arc::get_mut(&mut view.params).unwrap().arp_enabled = BoolParam::new("Arpeggiator", false);
    for frame in 0..3 {
        view.last_frame = Instant::now() - std::time::Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
        assert!(!view.tracker_open);
        assert!(!view.tracker_selection);
        assert!(view.tracker_drag.is_none());
        if frame == 0 {
            assert!(view.tracker_progress > 0.0 && view.tracker_progress < 1.0);
        }
    }
    assert_eq!(view.tracker_progress, 0.0);
    assert!(!view.tracker_restore_expand);
    assert_eq!(view.expand_target, 1.0);
}

#[test]
fn tracker_open_collapses_and_remembers_an_expanded_strumfield() {
    let mut view = view(2, false);
    Arc::get_mut(&mut view.params).unwrap().arp_enabled = BoolParam::new("Arpeggiator", true);
    view.expand_progress = 1.0;
    view.expand_start = 1.0;
    view.expand_target = 1.0;
    let (mut cx, target, _) = context();
    let toggle = (ARP_SURFACE.0 + ARP_SURFACE.2 - 58.0, ARP_SURFACE.1 + 20.0);
    event(
        &mut view,
        &mut cx,
        target,
        toggle.0,
        toggle.1,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(view.tracker_open);
    assert!(view.tracker_restore_expand);
    assert_eq!(view.expand_target, 0.0);
}

#[test]
fn tracker_drag_dots_reorder_the_complete_row_and_keep_its_selection() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.add_row_after(1);
        s.rows[0].duration = 3;
        s.rows[0].page.loops[0].octave = -2;
    });
    let id = view.params.arpeggiator.snapshot().rows[0].id;
    let (mut cx, target, _) = context();
    let from = tracker::row_rect(0, 0.0);
    let to = tracker::row_rect(2, 0.0);
    event(
        &mut view,
        &mut cx,
        target,
        from.0 + 14.0,
        from.1 + 18.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        to.0 + 14.0,
        to.1 + 18.0,
        WindowEvent::MouseMove(to.0 + 14.0, to.1 + 18.0),
    );
    event(
        &mut view,
        &mut cx,
        target,
        to.0 + 14.0,
        to.1 + 18.0,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(
        (
            state.rows[2].id,
            state.rows[2].duration,
            state.rows[2].page.loops[0].octave
        ),
        (id, 3, -2)
    );
    assert_eq!(view.arp_ui.edit_page, 2);
    assert!(view.tracker_drag.is_none());
}

#[test]
fn tracker_text_edit_does_not_swallow_arpeggiator_clicks() {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    view.tracker_symbol_edit = true;
    view.tracker_symbol = "Dm7".into();
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arpeggiator::sync_rect());
    assert!(!view.tracker_symbol_edit);
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0]
            .chord
            .unwrap()
            .quality,
        4
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.arp_params().arp_sync.as_ptr()));
    changes.borrow_mut().clear();
    click(&mut view, &mut cx, target, arpeggiator::power_rect());
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.arp_enabled.as_ptr()));
}

#[test]
fn tracker_toggle_has_its_own_target_beside_all_six_lanes() {
    let r = tracker::toggle_rect();
    for i in 0..6 {
        let lane = arpeggiator::lane_rect(i);
        assert!(lane.0 + lane.2 <= r.0 || lane.1 >= r.1 + r.3);
    }
}

#[test]
fn tracker_arp_controls_edit_row_settings_without_host_parameter_registration() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        for i in 0..10 {
            s.add_row_after(i);
        }
    });
    view.arp_ui.edit_page = 9;
    view.refresh_arp_editor_params();
    let initial = view.params.arpeggiator.snapshot().rows[0].settings;
    let (mut cx, target, _) = context();
    let p = view.arp_params();
    let edits = [
        (p.arp_mode.as_ptr(), p.arp_mode.preview_normalized(1)),
        (p.arp_pattern.as_ptr(), p.arp_pattern.preview_normalized(6)),
        (p.arp_octaves.as_ptr(), p.arp_octaves.preview_normalized(3)),
        (p.arp_sync.as_ptr(), p.arp_sync.preview_normalized(false)),
        (
            p.arp_free_ms.as_ptr(),
            p.arp_free_ms.preview_normalized(350.0),
        ),
        (p.rate.as_ptr(), p.rate.preview_normalized(0.5)),
        (p.swing.as_ptr(), p.swing.preview_normalized(-0.25)),
        (p.humanize.as_ptr(), p.humanize.preview_normalized(0.4)),
    ];
    let host_parameters = view.params.param_map();
    for (ptr, value) in edits {
        assert!(!host_parameters
            .iter()
            .any(|(_, host_ptr, _)| *host_ptr == ptr));
        assert!(view.arp_row_param_event(&RawParamEvent::BeginSetParameter(ptr)));
        let mut event = Event::new(RawParamEvent::SetParameterNormalized(ptr, value));
        View::event(
            &mut view,
            &mut EventContext::new_with_current(&mut cx, target),
            &mut event,
        );
        assert!(view.arp_row_param_event(&RawParamEvent::EndSetParameter(ptr)));
    }
    let settings = view.params.arpeggiator.snapshot().rows[9].settings;
    assert_eq!(settings.mode, crate::arpeggiator::ArpMode::Loop);
    assert_eq!(
        (settings.pattern, settings.octaves, settings.sync),
        (6, 3, false)
    );
    assert!((settings.free_ms - 350.0).abs() < 0.001);
    assert!((settings.rate - 0.5).abs() < 0.001);
    assert!((settings.swing + 0.25).abs() < 0.001);
    assert!((settings.humanize - 0.4).abs() < 0.001);
    assert_eq!(view.arp_params().settings(), settings);
    assert_eq!(view.params.arpeggiator.snapshot().rows[0].settings, initial);
    let saved = view.params.serialize_fields();
    let restored = ChordboardParams::default();
    restored.deserialize_fields(&saved);
    assert_eq!(restored.arpeggiator.snapshot().rows[9].settings, settings);
    assert!(!view.arp_row_param_event(&RawParamEvent::BeginSetParameter(
        view.params.arp_enabled.as_ptr()
    )));
}

#[test]
fn tracker_row_events_stop_before_the_host_model() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    use std::rc::Rc;
    struct HostSpy(Rc<Cell<usize>>);
    impl Model for HostSpy {
        fn event(&mut self, _: &mut EventContext, event: &mut Event) {
            event.map(|_: &RawParamEvent, _| self.0.set(self.0.get() + 1));
        }
    }
    let ui = view(2, false);
    let params = ui.params.clone();
    let ptr = ui.arp_params().arp_sync.as_ptr();
    let mut cx = Context::default();
    let seen = Rc::new(Cell::new(0));
    HostSpy(seen.clone()).build(&mut cx);
    let entity = ui.build(&mut cx, |_| {}).entity();
    for event in [
        RawParamEvent::BeginSetParameter(ptr),
        RawParamEvent::SetParameterNormalized(ptr, 0.0),
        RawParamEvent::EndSetParameter(ptr),
    ] {
        cx.emit_custom(Event::new(event).target(entity).propagate(Propagation::Up));
    }
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(!params.arpeggiator.snapshot().rows[0].settings.sync);
    assert_eq!(seen.get(), 0, "row edits reached the host parameter model");
    cx.emit_custom(
        Event::new(RawParamEvent::BeginSetParameter(
            params.arp_enabled.as_ptr(),
        ))
        .target(entity)
        .propagate(Propagation::Up),
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert_eq!(
        seen.get(),
        1,
        "registered plugin controls must still reach the host"
    );
}

fn scrolling_tracker() -> ChordboardView {
    let mut view = view(2, false);
    view.tracker_open = true;
    view.tracker_progress = 1.0;
    view.params.arpeggiator.edit(|s| {
        for i in 0..11 {
            s.add_row_after(i);
        }
    });
    view
}

#[test]
fn tracker_wheel_scroll_eases_without_quantizing_to_rows_and_ignores_horizontal_motion() {
    let mut view = scrolling_tracker();
    let (mut cx, target, _) = context();
    let row = tracker::row_rect(0, 0.0);
    event(
        &mut view,
        &mut cx,
        target,
        row.0 + 60.0,
        row.1 + 18.0,
        WindowEvent::MouseScroll(0.0, -0.25),
    );
    assert_eq!(view.tracker_scroll, 0.0);
    assert_eq!(view.tracker_scroll_target, 10.0);
    assert!(view.tick_tracker_scroll(0.016));
    assert!(view.tracker_scroll > 0.0 && view.tracker_scroll < 10.0);
    for _ in 0..40 {
        view.tick_tracker_scroll(0.016);
    }
    assert_eq!(view.tracker_scroll, 10.0);
    event(
        &mut view,
        &mut cx,
        target,
        row.0 + 60.0,
        row.1 + 18.0,
        WindowEvent::MouseScroll(1.0, 0.0),
    );
    assert_eq!(view.tracker_scroll_target, 10.0);
}

#[test]
fn tracker_partial_rows_hit_the_visible_row_and_do_not_cover_the_header() {
    let mut view = scrolling_tracker();
    let (mut cx, target, _) = context();
    let row = tracker::row_rect(0, 0.0);
    assert!(view.scroll_tracker(row.0 + 60.0, row.1, -55.5, true));
    assert_eq!(view.tracker_scroll, 55.5);
    click(
        &mut view,
        &mut cx,
        target,
        (row.0 + 60.0, row.1 + 2.0, 10.0, 8.0),
    );
    assert_eq!(view.arp_ui.edit_page, 1);
    click(
        &mut view,
        &mut cx,
        target,
        (row.0 + 60.0, row.1 - 10.0, 10.0, 4.0),
    );
    assert_eq!(view.arp_ui.edit_page, 1);
    assert!(view.scroll_tracker(row.0, row.1, -10000.0, true));
    let last = tracker::row_rect(11, view.tracker_scroll);
    assert!((last.1 + last.3 - (PERF_SURFACE.1 + PERF_SURFACE.3 - 28.0)).abs() < 0.001);
    view.params.arpeggiator.edit(|s| s.rows.truncate(1));
    view.tick_tracker_scroll(0.016);
    assert_eq!(view.tracker_scroll, 0.0);
    assert_eq!(view.tracker_scroll_target, 0.0);
}

#[test]
fn tracker_native_pixels_reach_the_view_and_the_legacy_event_is_not_applied_twice() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    let ui = scrolling_tracker();
    let mut cx = Context::default();
    let entity = ui.build(&mut cx, |_| {}).entity();
    BackendContext::new(&mut cx).cache().set_bounds(
        entity,
        BoundingBox::from_min_max(0.0, 0.0, W * 2.0, H * 2.0),
    );
    let row = tracker::row_rect(0, 0.0);
    {
        let mut backend = BackendContext::new_with_event_manager(&mut cx);
        backend.emit_origin(WindowEvent::MouseMove(
            (row.0 + 60.0) * 2.0,
            (row.1 + 18.0) * 2.0,
        ));
        backend.send_event(
            Event::new(nih_plug_vizia::PixelScroll { x: 0.0, y: -15.5 })
                .target(Entity::root())
                .origin(Entity::root())
                .propagate(Propagation::Subtree),
        );
        backend.emit_origin(WindowEvent::MouseScroll(0.0, -1.0));
        backend.process_events();
    }
    let ecx = EventContext::new_with_current(&mut cx, entity);
    let ui = ecx.get_view::<ChordboardView>().unwrap();
    assert_eq!(
        ui.tracker_scroll, 7.75,
        "precise pixels must survive routing and editor scaling"
    );
    assert_eq!(
        ui.tracker_scroll_target, 7.75,
        "legacy line event applied the gesture twice"
    );
    assert!(!ui.tracker_skip_line_scroll);
}
