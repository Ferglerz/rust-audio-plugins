use super::super::*;
use super::support::*;

fn view(mode: i32, mpe: bool) -> ChordboardView {
    let mut view = super::support::view(mode, mpe);
    view.programming = true;
    view
}

fn blank_note_cell(view: &ChordboardView, pitch: u8) -> Rect {
    let blank = tracker::blank_column(
        &view.params.arpeggiator.snapshot().rows,
        view.tracker_scroll,
    );
    (
        blank.0,
        blank.1 + (11 - pitch) as f32 * tracker::LANE_H,
        blank.2,
        tracker::LANE_H,
    )
}

fn first_chord_cell() -> Rect {
    tracker::row_rect(&[crate::arpeggiator::TrackerRow::default()], 0, 0.0)
}

#[test]
fn row_slide_keeps_tracker_live_and_blocks_moving_controls_until_settled() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|state| {
        state.rows[0].settings.free_ms = 333.0;
        state.rows[0].settings.sync = false;
        state.add_row_after(0);
        state.rows[1].settings.free_ms = 777.0;
    });
    let (mut cx, target, changes) = context();
    let second = tracker_row_rect(&view, 1);
    event(
        &mut view,
        &mut cx,
        target,
        second.0 + 65.0,
        second.1 + 18.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        second.0 + 65.0,
        second.1 + 18.0,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(view.row_slide.active());
    assert_eq!(view.arp_ui.edit_page, 1);
    assert_eq!(view.arp_params().arp_free_ms.value(), 777.0);
    let control = arpeggiator::rate_rect();
    let (x, y) = (control.0 + control.2 * 0.5, control.1 + control.3 * 0.5);
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
    let first = tracker_row_rect(&view, 0);
    event(
        &mut view,
        &mut cx,
        target,
        first.0 + 65.0,
        first.1 + 18.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        &mut view,
        &mut cx,
        target,
        first.0 + 65.0,
        first.1 + 18.0,
        WindowEvent::MouseUp(MouseButton::Left),
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
fn tracker_blank_cell_creates_a_one_loop_chord_and_another_blank_column() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.rows[0].duration = 2;
        s.rows[1].duration = 9;
    });
    let button = blank_note_cell(&view, 7);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, button);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2].duration, 1);
    assert_eq!(rows[2].chord.unwrap().root, 67);
    assert!(rows[2].inherit_arp);
    let blank = tracker::blank_column(&rows, view.tracker_scroll);
    let last = tracker_row_rect(&view, 2);
    assert_eq!(blank.0, last.0 + last.2 + 2.0);
    let next = blank_note_cell(&view, 2);
    click(&mut view, &mut cx, target, next);
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[3]
            .chord
            .unwrap()
            .root,
        62
    );
    assert_eq!(view.arp_ui.edit_page, 3);
}

#[test]
fn tracker_toolbar_delete_removes_selected_row_and_preserves_survivors() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.add_row_after(1);
        s.rows[1].duration = 8;
    });
    let rows = view.params.arpeggiator.snapshot().rows;
    view.arp_ui.edit_page = 1;
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, tracker::action_rect(3));
    assert_eq!(
        view.params.arpeggiator.snapshot().rows,
        vec![rows[0].clone(), rows[2].clone()]
    );
    assert_eq!(view.arp_ui.edit_page, 1);
    click(&mut view, &mut cx, target, tracker::action_rect(3));
    assert_eq!(
        view.params.arpeggiator.snapshot().rows,
        vec![rows[0].clone()]
    );
    click(&mut view, &mut cx, target, tracker::action_rect(3));
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].chord.is_none());
    assert_eq!(rows[0].duration, 1);
    assert_eq!(view.arp_ui.edit_page, 0);
}

#[test]
fn tracker_escape_exits_chord_selection_even_with_pointer_over_tempo() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, first_chord_cell());
    assert!(view.tracker_selection);
    let rows = view.params.arpeggiator.snapshot().rows;
    event(
        &mut view,
        &mut cx,
        target,
        TEMPO_CONTROL.0 + 12.0,
        TEMPO_CONTROL.1 + 12.0,
        WindowEvent::KeyDown(Code::Escape, None),
    );
    assert!(!view.tracker_selection);
    assert_eq!(view.params.arpeggiator.snapshot().rows, rows);
}

#[test]
fn tracker_keyboard_assignment_also_auditions_on_performance() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    let row = first_chord_cell();
    click(&mut view, &mut cx, target, row);
    assert!(view.tracker_selection);
    let index = (0..KEY_COUNT)
        .find(|&i| view.keyboard_chord(i).is_some())
        .unwrap();
    let key = view.keyboard_key_rect(index);
    click(&mut view, &mut cx, target, midi_fx::PERFORMANCE_TAB);
    click(&mut view, &mut cx, target, key);
    assert!(view.tracker_selection);
    let saved = view.params.arpeggiator.snapshot().rows[0].chord.unwrap();
    assert_eq!(saved.root, view.params.keyboard_octave.value() as u8);
    assert!(view.tracker_selection);
    let token = index as u8 + POINTER_KEY_OFFSET;
    assert!(matches!(view.bridge.commands.pop(),
        Some(Command::KeyDown(key, root, _)) if key == token && root == saved.root));
    assert!(matches!(view.bridge.commands.pop(), Some(Command::KeyUp(key)) if key == token));
    click(&mut view, &mut cx, target, midi_fx::PROGRAMMING_TAB);
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
    let row = tracker_row_rect(&view, 1);
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
    click(&mut view, &mut cx, target, midi_fx::PERFORMANCE_TAB);
    click(&mut view, &mut cx, target, key);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows[0].chord, original);
    assert_eq!(
        rows[1].chord.unwrap().root,
        view.params.keyboard_octave.value() as u8
    );
    assert!(matches!(view.bridge.commands.pop(), Some(Command::KeyDown(..))));
    assert!(matches!(view.bridge.commands.pop(), Some(Command::KeyUp(..))));
}

#[test]
fn tracker_loop_buttons_change_duration_without_starting_chord_selection() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    let initial = view.params.arpeggiator.snapshot().rows[0].duration;
    click(&mut view, &mut cx, target, tracker::duration_button(true));
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].duration,
        initial + 1
    );
    click(&mut view, &mut cx, target, tracker::duration_button(false));
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
    event(
        &mut view,
        &mut cx,
        target,
        row.0,
        row.1,
        WindowEvent::KeyDown(Code::Backspace, None),
    );
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
    let row = tracker_row_rect(&view, 0);
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
    let blank = blank_note_cell(&view, 7);
    click(&mut view, &mut cx, target, blank);
    assert_eq!(view.params.arpeggiator.snapshot().rows.len(), 2);
    click(&mut view, &mut cx, target, first_chord_cell());
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.tracker_selection);
    assert_eq!(view.expand_target, 1.0);
}

#[test]
fn tracker_block_drag_reorders_the_complete_row_and_keep_its_selection() {
    let mut view = view(2, false);
    view.params
        .arpeggiator
        .edit(|s| s.rows[0].chord = harmony::parse_chord_symbol("C"));
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.add_row_after(1);
        s.rows[0].duration = 3;
        s.rows[0].page.loops[0].octave = -2;
    });
    let id = view.params.arpeggiator.snapshot().rows[0].id;
    let (mut cx, target, _) = context();
    let from = tracker_row_rect(&view, 0);
    let to = tracker_row_rect(&view, 2);
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
    assert!(!view.params.arpeggiator.snapshot().rows[0].enabled);
    assert!(changes.borrow().is_empty());
}

#[test]
fn tracker_replaces_performance_modules_and_removes_the_bottom_row() {
    let r = TRACKER_SURFACE;
    assert_eq!(r, EXPANDED_SURFACE);
    assert_eq!(r.0, CHORDS_SURFACE.0);
    assert_eq!(r.0 + r.2, PERF_SURFACE.0 + PERF_SURFACE.2);
    assert_eq!(H, MODULE_BOTTOM + 22.0);
    assert!(r.1 + r.3 < ARP_SURFACE.1);
    for i in 0..tracker::ACTIONS.len() {
        let button = tracker::action_rect(i);
        assert!(button.0 >= r.0 && button.0 + button.2 <= r.0 + r.2);
        assert!(button.1 + button.3 < tracker::grid_rect().1);
    }
}

#[test]
fn tracker_inherit_hides_controls_and_restores_the_saved_pattern() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.rows[1].settings.sync = false;
        s.rows[1].page.loops[0].steps[0].velocity = 73;
    });
    let (mut cx, target, _) = context();
    let inherit = arpeggiator::inherit_rect();
    click(&mut view, &mut cx, target, inherit);
    assert!(!view.params.arpeggiator.snapshot().rows[0].inherit_arp);

    view.arp_ui.edit_page = 1;
    click(&mut view, &mut cx, target, inherit);
    let controls = view.arp_editor_button_rects();
    assert!(controls.contains(&(inherit, true)));
    assert_eq!(controls.len(), 5);
    click(&mut view, &mut cx, target, arpeggiator::tracker_voicing_button(false, true));
    click(&mut view, &mut cx, target, arpeggiator::tracker_voicing_button(true, false));
    let state = view.params.arpeggiator.snapshot();
    assert_eq!((state.rows[1].arp_octave, state.rows[1].arp_inversion), (1, -1));
    assert_eq!((state.rows[0].arp_octave, state.rows[0].arp_inversion), (0, 0));
    assert!(!view.placed_controls().iter().any(|(c, _)| c.id == "arp_free_ms"));
    click(&mut view, &mut cx, target, arpeggiator::power_rect());
    click(&mut view, &mut cx, target, arpeggiator::order_rect());
    assert!(view.menu.is_none());
    assert!(view.params.arpeggiator.snapshot().rows[1].enabled);

    click(&mut view, &mut cx, target, inherit);
    assert!(view.arp_editor_button_rects().contains(&(inherit, false)));
    assert!(view.placed_controls().iter().any(|(c, _)| c.id == "arp_free_ms"));
    assert_eq!(view.params.arpeggiator.snapshot().rows[1].page.loops[0].steps[0].velocity, 73);
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
        s.rows[0].chord = Some(SavedChord {
            root: 60,
            ..harmony::parse_chord_symbol("C").unwrap()
        });
        for i in 0..63 {
            s.add_row_after(i);
        }
    });
    view
}

#[test]
fn tracker_wheel_scroll_eases_and_accepts_both_axes() {
    let mut view = scrolling_tracker();
    let (mut cx, target, _) = context();
    let row = tracker_row_rect(&view, 0);
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
    assert_eq!(view.tracker_scroll_target, 0.0);
}

#[test]
fn tracker_partial_rows_hit_the_visible_row_and_do_not_cover_the_header() {
    let mut view = scrolling_tracker();
    let (mut cx, target, _) = context();
    let row = tracker_row_rect(&view, 0);
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
    let last = tracker_row_rect(&view, 63);
    let grid = tracker::grid_rect();
    assert!((last.0 + last.2 + 2.0 + tracker::loop_width(&view.params.arpeggiator.snapshot().rows) - (grid.0 + grid.2)).abs() < 0.001);
    let add = blank_note_cell(&view, 7);
    assert!(add.0 >= grid.0 && add.0 + add.2 <= grid.0 + grid.2);
    view.params.arpeggiator.edit(|s| s.rows.truncate(1));
    view.tick_tracker_scroll(0.016);
    assert_eq!(view.tracker_scroll, 0.0);
    assert_eq!(view.tracker_scroll_target, 0.0);
}

#[test]
fn tracker_native_pixels_reach_the_view_and_the_legacy_event_is_not_applied_twice() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    let ui = scrolling_tracker();
    let row = tracker_row_rect(&ui, 0);
    let mut cx = Context::default();
    let entity = ui.build(&mut cx, |_| {}).entity();
    BackendContext::new(&mut cx).cache().set_bounds(
        entity,
        BoundingBox::from_min_max(0.0, 0.0, EDITOR_W * 2.0, H * 2.0),
    );
    {
        let mut backend = BackendContext::new_with_event_manager(&mut cx);
        backend.emit_origin(WindowEvent::MouseMove(
            (row.0 + CONTENT_X + 60.0) * 2.0,
            (row.1 + 18.0) * 2.0,
        ));
        backend.send_event(
            Event::new(nih_plug_vizia::PixelScroll { x: -15.5, y: 0.0 })
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
    click(&mut view, &mut cx, target, midi_fx::PERFORMANCE_TAB);
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
fn tracker_toolbar_controls_fit_without_overlapping() {
    let panel = TRACKER_SURFACE;
    let mut controls: Vec<_> = (0..tracker::ACTIONS.len()).map(tracker::action_rect).collect();
    controls.extend([
        tracker::duration_button(false),
        tracker::duration_button(true),
        tracker::transport_rect(),
    ]);
    for (i, r) in controls.iter().enumerate() {
        assert!(r.0 >= panel.0 && r.0 + r.2 <= panel.0 + panel.2);
        assert!(r.1 + r.3 < tracker::grid_rect().1);
        for other in controls.iter().skip(i + 1) {
            assert!(r.0 + r.2 < other.0 || other.0 + other.2 < r.0);
        }
    }
}

fn pointer_event(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    p: (f32, f32),
    e: WindowEvent,
) {
    event(view, cx, target, p.0, p.1, e);
}
fn block_center(view: &ChordboardView, index: usize) -> (f32, f32) {
    let r = tracker_row_rect(view, index);
    (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5)
}

#[test]
fn tracker_pitch_drag_previews_then_commits_without_changing_the_recipe() {
    let mut view = view(2, false);
    let chord = SavedChord {
        root: 60,
        second: Some(64),
        quality: 0,
        additions: 1 << 10,
        inversion: 1,
        spread: 2,
        transpose: 2,
    };
    view.params.arpeggiator.edit(|s| {
        s.rows[0].chord = Some(chord);
        s.rows[0].settings.free_ms = 317.0;
        s.rows[0].page.loops[0].octave = -2;
    });
    let original = view.params.arpeggiator.snapshot().rows[0].clone();
    let (mut cx, target, changes) = context();
    let from = block_center(&view, 0);
    let to = (from.0, from.1 - 3.0 * tracker::LANE_H);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseMove(to.0, to.1),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0],
        original,
        "preview must not publish to playback"
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    let after = view.params.arpeggiator.snapshot().rows[0].clone();
    let expected = SavedChord {
        root: 63,
        second: Some(67),
        ..chord
    };
    assert_eq!(
        after,
        crate::arpeggiator::TrackerRow {
            chord: Some(expected),
            ..original
        }
    );
    assert_eq!(
        harmony::recipe_intervals(expected).as_slice(),
        harmony::recipe_intervals(chord).as_slice()
    );
    assert!(changes.borrow().is_empty());
    let saved = view.params.serialize_fields();
    let restored = ChordboardParams::default();
    restored.deserialize_fields(&saved);
    assert_eq!(restored.arpeggiator.snapshot().rows[0], after);
}

#[test]
fn tracker_pitch_drag_clamps_to_lanes_and_both_midi_inputs() {
    for (root, second, transpose, delta, expected) in [
        (60, 64, 0, 20, 71),   // C up past B stops at B.
        (71, 75, 0, -20, 60),  // B down past C stops at C.
        (126, 127, 0, 4, 126), // The second input is already at the MIDI ceiling.
        (1, 0, 0, -1, 1),      // The second input is already at the MIDI floor.
        (120, 120, 7, 4, 120), // The transposed root is at the MIDI ceiling.
    ] {
        let mut view = view(2, false);
        view.params.arpeggiator.edit(|s| {
            s.rows[0].chord = Some(SavedChord {
                root,
                second: Some(second),
                transpose,
                quality: 0,
                additions: 0,
                inversion: 0,
                spread: 0,
            })
        });
        let (mut cx, target, _) = context();
        let from = block_center(&view, 0);
        let to = (from.0, from.1 - delta as f32 * tracker::LANE_H);
        pointer_event(
            &mut view,
            &mut cx,
            target,
            from,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        pointer_event(
            &mut view,
            &mut cx,
            target,
            to,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        let chord = view.params.arpeggiator.snapshot().rows[0].chord.unwrap();
        assert_eq!(chord.root, expected);
        assert_eq!(
            chord.second.unwrap() as i16 - chord.root as i16,
            second as i16 - root as i16
        );
    }
}

#[test]
fn tracker_click_threshold_and_escape_preserve_sequence() {
    let mut view = scrolling_tracker();
    view.params
        .arpeggiator
        .edit(|s| s.rows[0].chord = Some(harmony::parse_chord_symbol("Dm7").unwrap()));
    let original = view.params.arpeggiator.snapshot().rows;
    let (mut cx, target, _) = context();
    let from = block_center(&view, 0);
    let near = (from.0 + 2.0, from.1 - 2.0);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        near,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows, original);
    assert!(view.tracker_selection);
    let to = (from.0 + 3.0 * tracker::LOOP_W, from.1 - tracker::LANE_H);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseMove(to.0, to.1),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::KeyDown(Code::Escape, None),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows, original);
    assert!(view.tracker_drag.is_none());
}

#[test]
fn tracker_resize_snaps_loops_on_release_and_preserves_following_rows() {
    let mut view = view(2, false);
    view.params
        .arpeggiator
        .edit(|s| s.rows[0].chord = harmony::parse_chord_symbol("C"));
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
    });
    let original = view.params.arpeggiator.snapshot().rows;
    let (mut cx, target, _) = context();
    let r = tracker_row_rect(&view, 0);
    let from = (r.0 + r.2 - 3.0, r.1 + 10.0);
    let to = (from.0 + 1.6 * tracker::LOOP_W, from.1);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseMove(to.0, to.1),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows, original);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(
        rows[0],
        crate::arpeggiator::TrackerRow {
            duration: 3,
            ..original[0].clone()
        }
    );
    assert_eq!(rows[1], original[1]);
    assert_eq!(tracker_row_rect(&view, 1).0 - r.0, 3.0 * tracker::LOOP_W);
    let r = tracker_row_rect(&view, 0);
    let from = (r.0 + r.2 - 3.0, r.1 + 10.0);
    let to = (from.0 - 5.0 * tracker::LOOP_W, from.1);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows[0].duration, 1);
}

#[test]
fn tracker_resize_escape_and_focus_loss_cancel_without_publishing() {
    for end in [
        WindowEvent::KeyDown(Code::Escape, None),
        WindowEvent::FocusOut,
    ] {
        let mut view = view(2, false);
        view.params
            .arpeggiator
            .edit(|s| s.rows[0].chord = harmony::parse_chord_symbol("C"));
        let original = view.params.arpeggiator.snapshot().rows;
        let (mut cx, target, _) = context();
        let r = tracker_row_rect(&view, 0);
        let from = (r.0 + r.2 - 3.0, r.1 + 10.0);
        let to = (from.0 + tracker::LOOP_W * 4.0, from.1);
        pointer_event(
            &mut view,
            &mut cx,
            target,
            from,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        pointer_event(
            &mut view,
            &mut cx,
            target,
            to,
            WindowEvent::MouseMove(to.0, to.1),
        );
        pointer_event(&mut view, &mut cx, target, to, end);
        assert_eq!(view.params.arpeggiator.snapshot().rows, original);
        assert!(view.tracker_drag.is_none());
    }
}

#[test]
fn tracker_empty_cell_creates_c_without_starting_a_drag() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    let from = block_center(&view, 0);
    let to = (from.0, from.1 - tracker::LANE_H * 8.0);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0]
            .chord
            .unwrap()
            .root,
        60
    );
    assert!(view.tracker_drag.is_none());
}

#[test]
fn tracker_scrollbar_release_keeps_scroll_position_and_drag_autoscrolls_on_ticks() {
    let mut view = scrolling_tracker();
    let (mut cx, target, _) = context();
    let track = tracker::scrollbar_rect();
    let from = (track.0 + 10.0, track.1 + 4.0);
    let to = (from.0 + 300.0, from.1);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    let scrolled = view.tracker_scroll;
    assert!(scrolled > 500.0);
    view.tick_tracker_scroll(1.0);
    assert_eq!(
        view.tracker_scroll, scrolled,
        "scrollbar release must not jump to selection"
    );
    view.tracker_scroll = 0.0;
    view.tracker_scroll_target = 0.0;
    let from = block_center(&view, 0);
    let grid = tracker::grid_rect();
    let to = (grid.0 + grid.2 - 5.0, from.1);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseMove(to.0, to.1),
    );
    assert!(view.tick_tracker_scroll(0.016));
    let first = view.tracker_scroll;
    assert!(view.tick_tracker_scroll(0.016));
    assert!(view.tracker_scroll > first);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(view.arp_ui.edit_page > 0);
}

#[test]
fn tracker_same_size_preset_restore_cancels_in_flight_edit() {
    let mut view = view(2, false);
    view.params
        .arpeggiator
        .edit(|s| s.rows[0].chord = Some(harmony::parse_chord_symbol("Dm").unwrap()));
    let saved = view.params.serialize_fields();
    let (mut cx, target, _) = context();
    let from = block_center(&view, 0);
    let to = (from.0, from.1 - tracker::LANE_H);
    pointer_event(
        &mut view,
        &mut cx,
        target,
        from,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseMove(to.0, to.1),
    );
    view.params.deserialize_fields(&saved);
    view.event(
        &mut EventContext::new_with_current(&mut cx, target),
        &mut Event::new(RawParamEvent::ParametersChanged),
    );
    pointer_event(
        &mut view,
        &mut cx,
        target,
        to,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    assert!(view.tracker_drag.is_none());
    assert_eq!(
        harmony::chord_name(view.params.arpeggiator.snapshot().rows[0].chord.unwrap()),
        "Dm"
    );
}

#[test]
fn tracker_is_hidden_on_midi_fx_and_custom_pages() {
    for custom in [false, true] {
        let mut view = view(2, false);
        if custom {
            view.set_panel(Some(Panel::Custom));
        } else {
            view.fx_ui.open = true;
        }
        let (mut cx, target, _) = context();
        let blank = blank_note_cell(&view, 7);
        click(&mut view, &mut cx, target, blank);
        assert_eq!(view.params.arpeggiator.snapshot().rows.len(), 1);
        assert!(!view.tracker_visible());
        let r = tracker_row_rect(&view, 0);
        click(&mut view, &mut cx, target, r);
        assert_eq!(view.arp_ui.edit_page, 0);
        assert!(!view.tracker_selection);
    }
}

#[test]
fn tracker_pitch_drag_uses_fitted_coordinates_in_restored_and_narrow_windows() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    for (width, height) in [(2339.0, 1338.0), (1200.0, 900.0), (700.0, 1500.0)] {
        let mut view = view(2, false);
        view.params.arpeggiator.edit(|s| {
            s.rows[0].chord = Some(SavedChord {
                root: 60,
                ..harmony::parse_chord_symbol("C").unwrap()
            })
        });
        let (mut cx, target, _) = context();
        BackendContext::new(&mut cx)
            .cache()
            .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, width, height));
        let scale = editor_scale(width, height);
        assert!(EDITOR_W * scale <= width && H * scale <= height);
        let logical = block_center(&view, 0);
        let from = (logical.0 * scale, logical.1 * scale);
        let to = (from.0, from.1 - tracker::LANE_H * 2.0 * scale);
        pointer_event(
            &mut view,
            &mut cx,
            target,
            from,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        pointer_event(
            &mut view,
            &mut cx,
            target,
            to,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert_eq!(
            view.params.arpeggiator.snapshot().rows[0]
                .chord
                .unwrap()
                .root,
            62
        );
    }
}

#[test]
fn tracker_grid_has_twelve_lanes_and_header_readout_clears_the_title() {
    let mut rows = Vec::new();
    for pc in 0..12 {
        rows.push(crate::arpeggiator::TrackerRow {
            chord: Some(SavedChord {
                root: 60 + pc,
                ..harmony::parse_chord_symbol("C").unwrap()
            }),
            ..crate::arpeggiator::TrackerRow::default()
        });
    }
    let grid = tracker::grid_rect();
    assert_eq!(grid.3, 12.0 * tracker::LANE_H);
    for i in 0..12 {
        let r = tracker::row_rect(&rows, i, 0.0);
        assert_eq!(r.1, grid.1 + (11 - i) as f32 * tracker::LANE_H + 2.0);
        assert!(hit(grid, r.0, r.1) && hit(grid, r.0 + r.2, r.1 + r.3));
    }
    assert!(chord_readout_rect().0 >= 250.0);
}

#[test]
fn programming_navigation_slides_and_blocks_hidden_performance_controls() {
    let mut view = super::support::view(2, false);
    let (mut cx, target, changes) = context();
    assert!(midi_fx::PROGRAMMING_TAB.0 + midi_fx::PROGRAMMING_TAB.2 <= midi_fx::PERFORMANCE_TAB.0);
    click(&mut view, &mut cx, target, midi_fx::PROGRAMMING_TAB);
    assert!(view.programming && view.top_slide.active());
    let blank = blank_note_cell(&view, 7);
    event(
        &mut view,
        &mut cx,
        target,
        blank.0 + 30.0,
        blank.1 + 10.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows.len(), 1);
    view.top_slide.tick(pleasant_ui::page_slide::DURATION);
    click(&mut view, &mut cx, target, ORDER);
    assert!(changes.borrow().is_empty());
    assert!(view.menu.is_none());
    click(&mut view, &mut cx, target, midi_fx::FX_TAB);
    assert!(!view.tracker_visible());
    click(&mut view, &mut cx, target, blank);
    assert_eq!(view.params.arpeggiator.snapshot().rows.len(), 1);
    click(&mut view, &mut cx, target, midi_fx::PROGRAMMING_TAB);
    click(&mut view, &mut cx, target, blank);
    assert_eq!(view.params.arpeggiator.snapshot().rows.len(), 2);
}

#[test]
fn tracker_insert_triangle_preserves_neighbors_and_creates_an_editable_column() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.rows[0].chord = harmony::parse_chord_symbol("Cm7");
        s.rows[0].duration = 3;
        s.add_row_after(0);
        s.rows[1].chord = harmony::parse_chord_symbol("D");
    });
    let before = view.params.arpeggiator.snapshot().rows;
    let (mut cx, target, _) = context();
    let insert = tracker::insert_rect(&before, 1, 0.0);
    event(
        &mut view,
        &mut cx,
        target,
        insert.0 + 10.0,
        insert.1 + 10.0,
        WindowEvent::MouseMove(insert.0 + 10.0, insert.1 + 10.0),
    );
    assert_eq!(view.params.arpeggiator.snapshot().rows, before);
    click(&mut view, &mut cx, target, insert);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0], before[0]);
    assert_eq!(rows[2], before[1]);
    assert!(rows[1].chord.is_none());
    assert!(rows[1].inherit_arp);
    assert_eq!(rows[1].duration, 1);
    assert_eq!(view.arp_ui.edit_page, 1);
    let grid = tracker::grid_rect();
    let cell = (
        insert.0 + 10.0,
        grid.1 + 11.0 * tracker::LANE_H,
        tracker::LOOP_W,
        tracker::LANE_H,
    );
    click(&mut view, &mut cx, target, cell);
    let rows = view.params.arpeggiator.snapshot().rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].chord.unwrap().root, 60);
    assert_eq!(rows[0], before[0]);
    assert_eq!(rows[2], before[1]);
}

#[test]
fn tracker_transport_buttons_enable_playback_and_pause_without_changing_the_sequence() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
    });
    view.arp_ui.edit_page = 1;
    let original = view.params.arpeggiator.snapshot().rows;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, tracker::transport_rect());
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::TrackerPlay(1))
    ));
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, value)| *ptr == view.params.arp_enabled.as_ptr() && *value == 1.0));
    changes.borrow_mut().clear();
    view.snapshot.arp_running = true;
    click(&mut view, &mut cx, target, tracker::transport_rect());
    assert!(matches!(
        view.bridge.commands.pop(),
        Some(Command::TrackerPause)
    ));
    assert!(changes.borrow().is_empty());
    assert_eq!(view.params.arpeggiator.snapshot().rows, original);
}

#[test]
fn programming_transport_toggles_from_every_page_without_navigating() {
    for (programming, fx_open) in [(true, false), (false, false), (false, true)] {
        let mut view = view(2, false);
        view.programming = programming;
        view.fx_ui.open = fx_open;
        let original = view.params.arpeggiator.snapshot().rows;
        let (mut cx, target, _) = context();
        for (running, paused) in [(false, false), (true, false), (false, true)] {
            view.snapshot.arp_running = running;
            view.snapshot.tracker_paused = paused;
            click(&mut view, &mut cx, target, midi_fx::PROGRAMMING_TRANSPORT);
            let command = view.bridge.commands.pop();
            if running {
                assert!(matches!(command, Some(Command::TrackerPause)));
            } else {
                assert!(matches!(command, Some(Command::TrackerPlay(0))));
            }
            assert_eq!((view.programming, view.fx_ui.open), (programming, fx_open));
            assert_eq!(view.params.arpeggiator.snapshot().rows, original);
        }
    }
}

#[test]
fn tracker_compresses_twenty_four_four_loop_entries_before_scrolling() {
    let mut rows = vec![crate::arpeggiator::TrackerRow { duration: 4, ..Default::default() }; 24];
    let grid = tracker::grid_rect();
    let last = tracker::row_rect(&rows, 23, 0.0);
    assert!(last.2 >= 60.0);
    assert!(last.0 + last.2 + tracker::loop_width(&rows) <= grid.0 + grid.2 + 0.01);
    rows.push(crate::arpeggiator::TrackerRow { duration: 8, ..Default::default() });
    let short = tracker::row_rect(&rows, 0, 0.0);
    let long = tracker::row_rect(&rows, 24, 0.0);
    assert!(((short.2 + 4.0) * 2.0 - (long.2 + 4.0)).abs() < 0.01);
    rows.resize(64, rows[0].clone());
    assert!(tracker::row_rect(&rows, 63, 0.0).0 > grid.0 + grid.2);
}

#[test]
fn keyboard_and_modulator_controls_remain_usable_during_tracker_row_slide() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    view.row_slide.select(1, 0);
    view.row_slide.select(2, 1);
    for (r, ptr) in [
        (output_protocol_rect(false, 1), view.params.output_mode.as_ptr()),
        (modulator_bypass_rect(0), view.params.modulators[0].bypass.as_ptr()),
    ] {
        let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseUp(MouseButton::Left));
        assert!(view.row_slide.active());
        assert!(changes.borrow().iter().any(|(changed, _)| *changed == ptr));
    }
}

#[test]
fn tracker_visual_one_rotates_columns_preserving_selection_audio_and_edit_targets() {
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.rows[0].chord = Some(harmony::parse_chord_symbol("C").unwrap());
        s.add_row_after(0);
        s.rows[1].chord = Some(harmony::parse_chord_symbol("Dm").unwrap());
        s.rows[1].duration = 3;
        s.add_row_after(1);
        s.rows[2].duration = 2;
    });
    view.refresh_arp_editor_params();
    let before = view.params.arpeggiator.snapshot().rows;
    let (mut cx, target, _) = context();
    let r = tracker::row_rect(&before, 1, 0.0);
    pointer_event(&mut view, &mut cx, target, (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5), WindowEvent::MouseDown(MouseButton::Right));
    assert_eq!(view.params.arpeggiator.snapshot().view_row_start(), 1);
    assert_eq!(view.arp_ui.edit_page, 0);
    assert_eq!(view.params.arpeggiator.initial_prepared().rows, before);
    assert!(view.bridge.commands.pop().is_none());
    let grid = tracker::grid_rect();
    // The same Dm block is now the leftmost column, including its 3-loop width.
    let first = (grid.0 + 10.0, r.1 + r.3 * 0.5, 12.0, 4.0);
    click(&mut view, &mut cx, target, first);
    assert_eq!(view.arp_ui.edit_page, 1);
    click(&mut view, &mut cx, target, tracker::duration_button(true));
    let after = view.params.arpeggiator.snapshot();
    assert_eq!(after.rows[1].duration, 4);
    assert_eq!(after.rows[0], before[0]);
}
