use super::super::*;
use super::support::*;

fn first_chord_cell() -> Rect {
    let r = tracker::row_rect(0, 0.0);
    (r.0 + 50.0, r.1, r.2 - 202.0, r.3)
}

#[test]
fn row_slide_keeps_tracker_live_and_blocks_moving_controls_until_settled() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|state| {
        state.rows[0].settings.free_ms = 333.0;
        state.add_row_after(0);
        state.rows[1].settings.free_ms = 777.0;
    });
    let (mut cx, target, changes) = context();
    let second = tracker::row_rect(1, 0.0);
    event(
        &mut view,
        &mut cx,
        target,
        second.0 + 65.0,
        second.1 + 18.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(view.row_slide.active());
    assert_eq!(view.arp_ui.edit_page, 1);
    assert_eq!(view.arp_params().arp_free_ms.value(), 777.0);
    let control = globals::global_rect("velocity_scale");
    let (x, y) = (control.0 + control.2 * 0.5, control.1 + 84.0);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(view.drag.is_none());
    assert!(changes.borrow().is_empty());

    // A second selection can interrupt the slide and immediately binds its row.
    let first = tracker::row_rect(0, 0.0);
    event(
        &mut view,
        &mut cx,
        target,
        first.0 + 65.0,
        first.1 + 18.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(view.row_slide.active());
    assert_eq!(view.arp_ui.edit_page, 0);
    assert_eq!(view.arp_params().arp_free_ms.value(), 333.0);
    view.row_slide.tick(pleasant_ui::page_slide::DURATION);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::Control(_))));
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[1].settings.free_ms,
        777.0
    );
}

#[test]
fn tracker_duplicate_inserts_the_complete_selected_row_with_a_new_identity() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.rows[0].duration = 7;
        s.rows[0].chord = Some(harmony::parse_chord_symbol("Dm9").unwrap());
        s.rows[0].settings.pattern = 4;
        s.rows[0].page.loops[0].steps[3].velocity = 41;
    });
    let before = view.params.arpeggiator.snapshot().rows;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, tracker::action_rect(0));
    let after = view.params.arpeggiator.snapshot().rows;
    assert_eq!(after.len(), 3);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[2], before[1]);
    assert!(before.iter().all(|row| row.id != after[1].id));
    let mut expected = before[0].clone();
    expected.id = after[1].id;
    assert_eq!(after[1], expected);
    assert_eq!(view.arp_ui.edit_page, 1);
    assert!(changes.borrow().is_empty());
}

#[test]
fn tracker_add_is_centered_below_rows_and_inherits_the_last_rows_loops() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.rows[0].duration = 2;
        s.rows[1].duration = 9;
    });
    let button = tracker::add_row_rect(2, 0.0);
    let last = tracker::row_rect(1, 0.0);
    assert!(button.1 > last.1 + last.3);
    assert_eq!(
        button.0 + button.2 * 0.5,
        TRACKER_SURFACE.0 + TRACKER_SURFACE.2 * 0.5
    );
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, button);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2].duration, 9);
    assert_eq!(view.arp_ui.edit_page, 2);
}

#[test]
fn tracker_delete_targets_its_row_and_keeps_the_other_selected_row() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.add_row_after(1);
        s.rows[1].duration = 8;
    });
    let selected = view.params.arpeggiator.snapshot().rows[1].clone();
    view.arp_ui.edit_page = 1;
    let (mut cx, target, _) = context();
    let first = tracker::row_rect(0, 0.0);
    let delete = tracker::delete_button(first);
    let plus = tracker::duration_button(first, true);
    assert!(delete.0 > plus.0 + plus.2);
    click(&mut view, &mut cx, target, delete);
    assert_eq!(view.params.arpeggiator.snapshot().rows[0], selected);
    assert_eq!(view.arp_ui.edit_page, 0);
    assert!(!view.tracker_selection);
    click(
        &mut view,
        &mut cx,
        target,
        tracker::delete_button(tracker::row_rect(1, 0.0)),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows[0], selected);
    click(&mut view, &mut cx, target, delete);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].chord.is_none());
    assert_eq!(rows[0].duration, 1);
    assert_eq!(view.arp_ui.edit_page, 0);
}

#[test]
fn tracker_done_exits_chord_selection_without_closing_or_changing_the_row() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, first_chord_cell());
    assert!(view.tracker_selection);
    let rows = view.params.arpeggiator.snapshot().rows;
    click(&mut view, &mut cx, target, tracker::action_rect(4));
    assert!(!view.tracker_selection);
    assert_eq!(view.params.arpeggiator.snapshot().rows, rows);
}

#[test]
fn tracker_done_accepts_a_valid_typed_chord_and_ends_editing() {
    let mut view = view(2, false);
    view.tracker_symbol_edit = true;
    view.tracker_symbol = "Dm7".into();
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, tracker::action_rect(4));
    assert!(!view.tracker_symbol_edit);
    assert!(!view.tracker_selection);
    assert!(view.tracker_symbol.is_empty());
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
    let (mut cx, target, _) = context();
    let row = tracker::row_rect(0, 0.0);
    let initial = view.params.arpeggiator.snapshot().rows[0].duration;
    click(
        &mut view,
        &mut cx,
        target,
        tracker::duration_button(row, true),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].duration,
        initial + 1
    );
    click(
        &mut view,
        &mut cx,
        target,
        tracker::duration_button(row, false),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows[0].duration, initial);
    assert!(!view.tracker_selection);
}

#[test]
fn tracker_double_click_enters_and_validates_chord_symbols() {
    let mut view = view(2, false);
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
fn tracker_remains_editable_when_bypassed_and_strumfield_expanded() {
    let mut view = view(2, false);
    assert!(!view.params.arp_enabled.value());
    view.expand_progress = 1.0;
    view.expand_start = 1.0;
    view.expand_target = 1.0;
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, tracker::add_row_rect(1, 0.0));
    assert_eq!(view.params.arpeggiator.snapshot().rows.len(), 2);
    click(&mut view, &mut cx, target, first_chord_cell());
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.tracker_selection);
    assert_eq!(view.expand_target, 1.0);
}

#[test]
fn tracker_drag_dots_reorder_the_complete_row_and_keep_its_selection() {
    let mut view = view(2, false);
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
fn tracker_is_a_separate_full_height_module_below_the_header() {
    let r = TRACKER_SURFACE;
    assert!(r.1 > HEADER_H);
    assert!(r.0 + r.2 < CHORDS_SURFACE.0);
    assert_eq!(r.1 + r.3, PIANO_SURFACE.1 + PIANO_SURFACE.3);
    for i in [0, 1, 2, 4] {
        let button = tracker::action_rect(i);
        assert!(button.0 >= r.0 && button.0 + button.2 <= r.0 + r.2);
        assert!(button.1 + button.3 < tracker::row_rect(0, 0.0).1);
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
    let view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        for i in 0..63 {
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
    let last = tracker::row_rect(63, view.tracker_scroll);
    assert!((last.1 + last.3 - (TRACKER_SURFACE.1 + TRACKER_SURFACE.3 - 64.0)).abs() < 0.001);
    let add = tracker::add_row_rect(64, view.tracker_scroll);
    assert!(add.1 > last.1 + last.3);
    assert!(add.1 + add.3 < TRACKER_SURFACE.1 + TRACKER_SURFACE.3 - 20.0);
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
        BoundingBox::from_min_max(0.0, 0.0, EDITOR_W * 2.0, H * 2.0),
    );
    let row = tracker::row_rect(0, 0.0);
    {
        let mut backend = BackendContext::new_with_event_manager(&mut cx);
        backend.emit_origin(WindowEvent::MouseMove(
            (row.0 + CONTENT_X + 60.0) * 2.0,
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

#[test]
fn tracker_chord_selection_keeps_header_buttons_and_route_panel_usable() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, first_chord_cell());
    assert!(view.tracker_selection);
    click(&mut view, &mut cx, target, ORDER);
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.scale_layout.as_ptr()));
    click(&mut view, &mut cx, target, KEY_MENU);
    assert_eq!(view.menu, Some(Menu::Key));
    view.menu = None;
    view.set_panel(Some(Panel::Routes));
    view.route_elapsed = pleasant_ui::page_slide::DURATION;
    view.route_progress = 1.0;
    click(&mut view, &mut cx, target, ROUTE_TARGET);
    assert_eq!(view.menu, Some(Menu::RouteTarget));
}

#[test]
fn degrees_is_the_initial_keyboard_layout() {
    let view = ChordboardView::new(
        Arc::new(ChordboardParams::default()),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    assert_eq!(view.keyboard_layout(), 2);
}

#[test]
fn tracker_buttons_hover_without_motion_registration_and_hold_until_release() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    let r = tracker::action_rect(1);
    let (x, y) = (r.0 + 10.0, r.1 + 10.0);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert_eq!(view.hover_amount(r), 1.0);
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    view.tick_motion(1.0);
    assert_eq!(
        view.button_press_amount(r),
        1.0,
        "held state must outlive the click flash"
    );
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert_eq!(view.button_press_amount(r), 0.0);
}

#[test]
fn narrow_tracker_keeps_actions_and_row_controls_separate() {
    let panel = TRACKER_SURFACE;
    for i in 0..6 {
        let r = tracker::action_rect(i);
        assert!(r.0 >= panel.0 && r.0 + r.2 <= panel.0 + panel.2);
    }
    let row = tracker::row_rect(0, 0.0);
    let chord = (row.0 + 50.0, row.1, row.2 - 202.0, row.3);
    let minus = tracker::duration_button(row, false);
    let plus = tracker::duration_button(row, true);
    let delete = tracker::delete_button(row);
    assert!(chord.0 + chord.2 < minus.0);
    assert!(minus.0 + minus.2 < plus.0);
    assert!(plus.0 + plus.2 < delete.0);
}
