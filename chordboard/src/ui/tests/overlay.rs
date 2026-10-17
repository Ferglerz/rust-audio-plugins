use super::super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use super::support::*;

#[test]
fn destination_chips_open_the_exact_route_and_remain_reachable() {
    let mut view = view(1, false);
    let (mut cx, target, _) = context();
    let source = view.params.routes[0].source.value() as usize - 1;
    let meter = meter_rect(source);
    view.update_route_hover(meter.0 + 5.0, meter.1 + 5.0);
    let (slot, chip) = view.route_chips()[0];
    view.update_route_hover(meter.0 - 4.0, meter.1 + 5.0);
    assert_eq!(view.route_hover, Some(source));
    click(&mut view, &mut cx, target, chip);
    assert_eq!(view.route_slot, slot);
    assert_eq!(view.panel, Some(Panel::Routes));
    assert_eq!(view.panel_rect(Panel::Routes), CHORDS_SURFACE);
    assert!(view
        .placed_controls()
        .iter()
        .any(|(c, _)| c.id == "length_ms"));
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
    assert_eq!(MAIN_W, 1621.0); // Preserve the width of the performance surfaces.
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
    params.arp_pages[0].arp_mode = IntParam::new("Arp repeat", 2, IntRange::Linear { min: 0, max: 3 });
    view.params = Arc::new(params);
    open_arp(&mut view);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arp_rate_rect());
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
    assert_eq!(super::super::performance::meter_fill(0, 0.5), (0.5, 0.5));
    assert_eq!(super::super::performance::meter_fill(0, 0.0), (0.0, 0.5));
    assert_eq!(super::super::performance::meter_fill(0, 1.0), (0.5, 1.0));
    assert_eq!(super::super::performance::meter_fill(5, 0.25), (0.0, 0.25));
    assert_eq!(super::super::performance::meter_fill(5, 0.75), (0.0, 0.75));
}
#[test]
fn current_chord_highlight_follows_midi_latch_and_recall_and_clears_after_silence() {
    let mut view = view(3, false);
    for (root, quality) in [(60, 0), (62, 1), (67, 2)] {
        let chord = SavedChord {
            additions: 0,
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
    let mut once = view(1, false);
    let mut looped = view(3, false);
    open_arp(&mut once);
    open_arp(&mut looped);
    assert_eq!(
        once.base_controls().iter().map(|(c, r)| (c.id, *r)).collect::<Vec<_>>(),
        looped.base_controls().iter().map(|(c, r)| (c.id, *r)).collect::<Vec<_>>()
    );
    for v in [&once, &looped] {
        let ids = v
            .base_controls()
            .into_iter()
            .map(|(c, _)| c.id)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"humanize") && ids.contains(&"swing"));
        assert!(!ids.contains(&"strum_ms"));
        assert!(v
            .route_targets()
            .iter()
            .any(|(i, _)| crate::engine::routing::TARGETS[*i].id == "octaves"));
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
fn output_controls_follow_the_arp_editor_page_and_keep_strumfield_velocity_free() {
    for mode in [1, 2, 3] {
        let v = view(mode, false);
        let ids: Vec<_> = v.base_controls().into_iter().map(|(c, _)| c.id).collect();
        assert!(!ids.contains(&"velocity"));
        assert!(ids.contains(&"length_ms"));
        assert!(!ids.contains(&"strings"));
        let controls = v.base_controls();
        assert!(!controls.iter().any(|(c, _)| c.id == "velocity"));
        assert!(!controls
            .iter()
            .any(|(c, r)| c.id == "velocity" && hit(PAD, r.0, r.1)));
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
    assert_eq!(v.string_count(), 12);
    v.snapshot.notes = harmony::voice(60, 11, None, 0, 0, 0);
    assert_eq!(v.snapshot.notes.len, 2);
    assert_eq!(v.string_count(), 8);
    let (mut cx, target, _) = context();
    let mut snapshot = v.snapshot;
    snapshot.note_strikes[96] = 1;
    snapshot.sounding_notes[96] = false;
    v.bridge.visible.store(true, Ordering::Relaxed);
    v.bridge.publish(snapshot);
    v.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(v.note_anim[96] > 0.0);
}
#[test]
fn arpeggiator_module_sits_above_the_keyboard() {
    let mut view = view(1, false);
    assert!(view.arp_editor_open());
    assert!(ARP_SURFACE.1 + ARP_SURFACE.3 <= PIANO_SURFACE.1);
    assert!(arp_direction_rect().1 < ARP_SURFACE.1 + ARP_SETTINGS_H);
    assert!(arp_power_rect().1 < arp_direction_rect().1 + arp_direction_rect().3);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arp_power_rect());
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.arp_enabled.as_ptr(), 1.0))
    );
}
#[test]
fn arp_repeat_exposes_four_modes_and_sync_loop_default() {
    let params=ChordboardParams::default();
    assert_eq!(params.arp_pages[0].arp_mode.preview_plain(1.0),3);
    assert_eq!(params.arp_pages[0].arp_mode.value(),2);
    assert!(!params.arp_enabled.value());
}

#[test]
fn shift_tap_toggles_and_hold_restores_both_latch_states() {
    for base in [false, true] {
        for held in [false, true] {
            let mut view = view(0, false);
            Arc::get_mut(&mut view.params).unwrap().latch = BoolParam::new("Latch", base);
            view.focused = true;
            let (mut cx, target, changes) = context();
            assert!(view.shift_latch_event(
                &mut EventContext::new_with_current(&mut cx, target),
                &WindowEvent::KeyDown(Code::ShiftLeft, None)
            ));
            assert!(view.shift_latch_event(
                &mut EventContext::new_with_current(&mut cx, target),
                &WindowEvent::KeyDown(Code::ShiftLeft, None)
            ));
            if held {
                view.latch_shift.as_mut().unwrap().1 -= Duration::from_millis(300);
            }
            assert!(view.shift_latch_event(
                &mut EventContext::new_with_current(&mut cx, target),
                &WindowEvent::KeyUp(Code::ShiftLeft, None)
            ));
            BackendContext::new_with_event_manager(&mut cx).process_events();
            let events = changes.borrow();
            assert_eq!(events.len(), if held { 2 } else { 1 });
            assert_eq!(
                events[0],
                (view.params.latch.as_ptr(), if base { 0.0 } else { 1.0 })
            );
            if held {
                assert_eq!(
                    events[1],
                    (view.params.latch.as_ptr(), if base { 1.0 } else { 0.0 })
                );
            }
        }
    }
}
#[test]
fn shift_chord_and_focus_loss_restore_latch_and_typing_does_not_toggle_it() {
    let mut view = view(0, false);
    view.focused = true;
    let (mut cx, target, changes) = context();
    view.shift_latch_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::ShiftLeft, None),
    );
    view.shift_latch_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::Digit1, None),
    );
    view.shift_latch_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyUp(Code::ShiftLeft, None),
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert_eq!(changes.borrow().len(), 2);
    view.shift_latch_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::ShiftRight, None),
    );
    view.restore_shift_latch(&mut EventContext::new_with_current(&mut cx, target));
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert_eq!(changes.borrow().len(), 4);
    view.edit = Some(ValueEdit::new("tempo", TEMPO_CONTROL, "120".into()));
    assert!(!view.shift_latch_event(
        &mut EventContext::new_with_current(&mut cx, target),
        &WindowEvent::KeyDown(Code::ShiftLeft, None)
    ));
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert_eq!(changes.borrow().len(), 4);
}

#[test]
fn escape_cancels_value_draft_before_closing_its_panel() {
    let mut view = view(1, false);
    view.set_panel(Some(Panel::Routes));
    let mut edit = ValueEdit::new("tempo", TEMPO_CONTROL, "120".into());
    edit.insert("bad");
    view.edit = Some(edit);
    let (mut cx, target, changes) = context();
    event(&mut view, &mut cx, target, 0.0, 0.0, WindowEvent::KeyDown(Code::Escape, None));
    assert!(view.edit.is_none());
    assert_eq!(view.panel, Some(Panel::Routes));
    assert!(changes.borrow().is_empty());
}

#[test]
fn fine_adjustment_shift_during_or_before_value_press_does_not_persist_latch() {
    for before in [false, true] {
        let mut view = view(1, false);
        view.focused = true;
        let (mut cx, target, changes) = context();
        let r = view.placed_controls().into_iter().find(|(c, _)| c.id == "length_ms").unwrap().1;
        let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
        if before {
            // The pointer already indicates a parameter gesture before Shift is pressed.
            event(&mut view, &mut cx, target, x, y, WindowEvent::MouseMove(x, y));
            event(&mut view, &mut cx, target, x, y, WindowEvent::KeyDown(Code::ShiftLeft, None));
        }
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        assert!(matches!(view.drag, Some(Drag::Control(_))));
        if !before {
            event(&mut view, &mut cx, target, x, y, WindowEvent::KeyDown(Code::ShiftLeft, None));
        }
        event(&mut view, &mut cx, target, x, y - 12.0, WindowEvent::MouseMove(x, y - 12.0));
        event(&mut view, &mut cx, target, x, y - 12.0, WindowEvent::KeyUp(Code::ShiftLeft, None));
        assert!(view.latch_shift.is_none());
        assert!(!changes.borrow().iter().any(|(ptr, _)| *ptr == view.params.latch.as_ptr()));
    }
}

#[test]
fn grouped_menu_arrows_follow_visual_rows_and_stop_at_edges() {
    let view = view(1, false);
    let menu = Menu::RouteSource;
    assert_eq!(view.menu_neighbor(menu, 0, Code::ArrowLeft), 0);
    assert_eq!(view.menu_neighbor(menu, 0, Code::ArrowUp), 0);
    assert_eq!(view.menu_neighbor(menu, 0, Code::ArrowRight), 0);
    assert_eq!(view.menu_neighbor(menu, 0, Code::ArrowDown), 1);
    assert_eq!(view.menu_neighbor(menu, 2, Code::ArrowDown), 4);
    assert_eq!(view.menu_neighbor(menu, 6, Code::ArrowDown), 8);
    assert_eq!(view.menu_neighbor(menu, 8, Code::ArrowRight), 8);
    assert_eq!(view.menu_neighbor(menu, 9, Code::ArrowDown), 10);
    assert_eq!(view.menu_neighbor(menu, 10, Code::ArrowDown), 12);
    assert_eq!(view.menu_neighbor(menu, 12, Code::ArrowDown), 12);
    for menu in [Menu::RouteSource, Menu::RouteTarget] {
        for i in 0..menu.items().len() {
            for code in [Code::ArrowLeft, Code::ArrowRight, Code::ArrowUp, Code::ArrowDown] {
                let next = view.menu_neighbor(menu, i, code);
                if next != i {
                    assert!(view.route_menu_allowed(menu, next));
                    let a = view.menu_option_rect(menu, i);
                    let b = view.menu_option_rect(menu, next);
                    if matches!(code, Code::ArrowLeft | Code::ArrowRight) {
                        assert_eq!(a.1, b.1);
                    }
                }
            }
        }
    }
}

#[test]
fn pending_route_removal_cancels_on_escape_focus_and_source_change() {
    for exit in [WindowEvent::KeyDown(Code::Escape, None), WindowEvent::FocusOut] {
        let mut view = view(3, false);
        view.toggle_modulator(7);
        let (slot, chip) = view.route_chips()[0];
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, ChordboardView::route_remove_rect(chip));
        assert_eq!(view.pending_route_remove, Some(slot));
        event(&mut view, &mut cx, target, chip.0 + 3.0, chip.1 + 3.0, exit);
        assert_eq!(view.pending_route_remove, None);
        assert!(changes.borrow().is_empty());
    }
    let mut view = view(3, false);
    view.pending_route_remove = Some(0);
    view.toggle_modulator(2);
    assert_eq!(view.pending_route_remove, None);
}

#[test]
fn route_menu_arrows_skip_destinations_unavailable_for_selected_source() {
    let mut view = view(1, false);
    Arc::get_mut(&mut view.params).unwrap().routes[0].source =
        IntParam::new("Source", 8, IntRange::Linear { min: 0, max: 9 });
    view.route_slot = 0;
    let menu = Menu::RouteTarget;
    let disabled = crate::engine::routing::available_targets()
        .position(|(_, target)| target.id == "x_reverse").unwrap();
    assert!(!view.route_menu_allowed(menu, disabled));
    for i in 0..menu.items().len() {
        if !view.route_menu_allowed(menu, i) { continue; }
        for code in [Code::ArrowLeft, Code::ArrowRight, Code::ArrowUp, Code::ArrowDown] {
            assert_ne!(view.menu_neighbor(menu, i, code), disabled);
        }
    }
}

#[test]
fn moving_from_performance_shift_to_value_gesture_restores_latch_immediately() {
    for base in [false, true] {
        let mut view = view(1, false);
        view.focused = true;
        Arc::get_mut(&mut view.params).unwrap().latch = BoolParam::new("Latch", base);
        let (mut cx, target, changes) = context();
        event(&mut view, &mut cx, target, 0.0, 0.0, WindowEvent::KeyDown(Code::ShiftLeft, None));
        assert!(view.latch_shift.is_some());
        let r = view.placed_controls().into_iter().find(|(c, _)| c.id == "length_ms").unwrap().1;
        event(&mut view, &mut cx, target, r.0 + r.2 * 0.5, r.1 + r.3 * 0.5, WindowEvent::MouseDown(MouseButton::Left));
        assert!(view.latch_shift.is_none());
        let latch_changes: Vec<_> = changes.borrow().iter().copied()
            .filter(|(ptr, _)| *ptr == view.params.latch.as_ptr()).collect();
        assert_eq!(latch_changes, vec![
            (view.params.latch.as_ptr(), if base { 0.0 } else { 1.0 }),
            (view.params.latch.as_ptr(), if base { 1.0 } else { 0.0 }),
        ]);
    }
}

#[test]
fn dedicated_modulator_meters_fill_from_center_in_both_directions() {
    for source in [9, 10] {
        assert_eq!(super::super::performance::meter_fill(source, 0.0), (0.5, 0.5));
        assert_eq!(super::super::performance::meter_fill(source, -1.0), (0.0, 0.5));
        assert_eq!(super::super::performance::meter_fill(source, 1.0), (0.5, 1.0));
    }
}

#[test]
fn arpeggiator_settings_work_while_strumfield_is_expanded() {
    let mut view = view(2, false);
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, arp_power_rect());
    assert_eq!(changes.borrow().last().copied(), Some((view.params.arp_enabled.as_ptr(), 1.0)));
    click(&mut view, &mut cx, target, arp_direction_rect());
    assert!(view.arp_chooser_open());
    view.menu = None;
    let humanize = arp_controls()[0].1;
    event(&mut view, &mut cx, target, humanize.0 + 5.0, humanize.1 + 5.0,
        WindowEvent::MouseDown(MouseButton::Left));
    assert!(matches!(view.drag, Some(Drag::Control(_))));
}
