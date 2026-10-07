use super::*;
fn view() -> ChordboardView {
    let params = Arc::new(ChordboardParams::default());
    params.sequencer.edit(|state| {
        for lane in &mut state.lanes {
            for page in lane {
                page.length = 16;
                page.end = 15;
            }
        }
    });
    ChordboardView::new(
        params,
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    )
}
#[test]
fn sequencer_length_slider_resizes_edited_pages_and_keeps_other_pages() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.edit_pages = [1, 2, 3, 4];
    let r = length_rect(0);
    let first = step_rect(0, 0);
    let last = step_rect(0, 31);
    assert_eq!(r.0, first.0);
    assert_eq!(r.0 + r.2, last.0 + last.2);
    assert!(r.1 >= first.1 + first.3);
    view.set_sequencer_length(0, r.0);
    let state = view.params.sequencer.snapshot();
    for lane in VISIBLE_LANES {
        let p = &state.lanes[lane][view.sequencer_ui.edit_pages[lane]];
        assert_eq!((p.length, p.start, p.end), (1, 0, 0));
        assert_eq!(state.lanes[lane][0].length, 16);
    }
    view.set_sequencer_length(0, r.0 + r.2);
    let state = view.params.sequencer.snapshot();
    for lane in VISIBLE_LANES {
        let p = &state.lanes[lane][view.sequencer_ui.edit_pages[lane]];
        assert_eq!((p.length, p.end), (32, 31));
    }
}
#[test]
fn unlocked_lengths_edit_only_the_target_row() {
    let mut view = view();
    assert!(view.params.seq_length_lock.value());
    view.params = Arc::new(ChordboardParams {
        seq_length_lock: BoolParam::new("Lock lengths", false),
        sequencer: view.params.sequencer.clone(),
        ..ChordboardParams::default()
    });
    view.sequencer_ui.step = 12;
    let r = length_rect(1);
    view.set_sequencer_length(1, r.0 + r.2 * 0.25);
    let state = view.params.sequencer.snapshot();
    assert_eq!(state.lanes[0][0].length, 16);
    assert_eq!(state.lanes[1][0].length, 8);
    assert_eq!(state.lanes[3][0].length, 16);
    assert_eq!(view.sequencer_ui.step, 12);
}
#[test]
fn sequencer_page_shows_arp_and_bass_has_no_grid_controls() {
    let mut view = view();
    assert!(!view.arp_page());
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    assert!(view.arp_page());
    let items = view.seq_items(&view.params.sequencer.snapshot());
    assert!(!items.iter().any(|item| matches!(
        item.action,
        Action::LaneEnabled(2) | Action::EditPage(2, _) | Action::SelectStep(2, _)
    )));
    let r = step_rect(1, 2);
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    assert!(view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Right),
        r.0 + 2.0,
        r.1 + 2.0
    ));
    assert!(view.arp_page());
    assert_eq!((view.sequencer_ui.lane, view.sequencer_ui.step), (1, 2));
}
#[test]
fn step_click_selects_one_step_and_selection_cannot_be_cleared() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    let a = step_rect(0, 3);
    let b = step_rect(0, 5);
    let press = WindowEvent::MouseDown(MouseButton::Left);
    assert!(view.sequencer_event(&mut cx, &press, a.0 + 2.0, a.1 + 2.0));
    assert_eq!((view.sequencer_ui.lane, view.sequencer_ui.step), (0, 3));
    assert!(view.sequencer_event(&mut cx, &press, b.0 + 2.0, b.1 + 2.0));
    assert_eq!((view.sequencer_ui.lane, view.sequencer_ui.step), (0, 5));
    let items = view.seq_items(&view.params.sequencer.snapshot());
    assert!(!items.iter().any(|item| item.label == "Loop all"));
    assert!(view.sequencer_ui.step == 5);
    view.set_sequencer_open(&mut cx, false);
    assert_eq!(view.sequencer_ui.step, 5);
}
#[test]
fn expression_modes_follow_open_sequencer_independent_of_playback() {
    let mut view = view();
    assert!(view.expression_mode_items().is_empty());
    view.params = Arc::new(ChordboardParams {
        seq_enabled: BoolParam::new("Sequencer enabled", true),
        ..ChordboardParams::default()
    });
    assert!(view.expression_mode_items().is_empty());
    view.params = Arc::new(ChordboardParams::default());
    view.sequencer_ui.open = true;
    let expected = [
        (0, Field::Bend),
        (4, Field::Pressure),
        (5, Field::Timbre),
        (6, Field::Velocity),
        (1, Field::Cc(1)),
        (2, Field::Cc(0)),
        (3, Field::Cc(2)),
    ];
    let items = view.expression_mode_items();
    assert_eq!(items.len(), expected.len());
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    for (item, (source, field)) in items.iter().zip(expected) {
        let meter = meter_rect(source);
        assert_eq!((item.rect.0, item.rect.2), (meter.0, meter.2));
        assert!(item.rect.1 + item.rect.3 < meter.1);
        let mut cx = EventContext::new_with_current(&mut context, target);
        let press = WindowEvent::MouseDown(MouseButton::Left);
        assert!(view.sequencer_event(&mut cx, &press, item.rect.0 + 2.0, item.rect.1 + 2.0));
        assert_eq!(view.sequencer_ui.modifier, Some(field));
        assert!(view.sequencer_open());
        assert!(view.sequencer_event(&mut cx, &press, item.rect.0 + 2.0, item.rect.1 + 2.0));
        assert_eq!(view.sequencer_ui.modifier, None);
    }
    // CC buttons follow the selected pattern's assignments, including reordered tracks.
    view.params
        .sequencer
        .edit(|state| state.lanes[1][2].cc_numbers = [1, 4, 11, 2]);
    view.sequencer_ui.lane = 1;
    view.sequencer_ui.edit_pages[1] = 2;
    let items = view.expression_mode_items();
    for (source, field) in [(1, Field::Cc(0)), (2, Field::Cc(2)), (3, Field::Cc(3))] {
        assert!(items.iter().any(|item| item.rect.0 == meter_rect(source).0
            && matches!(item.action, Action::Modifier(Some(f)) if f == field)));
    }
    view.sequencer_ui.open = false;
    assert!(view.expression_mode_items().is_empty());
}
#[test]
fn cell_numbers_hold_then_fade_and_hover_stays_on_its_row() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.params.sequencer.edit(|state| {
        state.lanes[1][0].length = 32;
    });
    let rect = step_rect(1, 15);
    view.pointer = Some((rect.0 + 4.0, rect.1 + 4.0));
    view.tick_sequencer(0.0);
    assert_eq!(view.sequencer_ui.number_age[1][14], 0.0);
    assert_eq!(view.sequencer_ui.number_age[1][15], 0.0);
    assert_eq!(number_alpha(view.sequencer_ui.number_age[1][13]), 0.0);
    assert_eq!(number_alpha(view.sequencer_ui.number_age[1][16]), 1.0);
    assert_eq!(number_alpha(view.sequencer_ui.number_age[0][15]), 0.0);
    view.pointer = None;
    view.tick_sequencer(0.5);
    assert_eq!(number_alpha(view.sequencer_ui.number_age[1][15]), 1.0);
    view.tick_sequencer(0.175);
    assert!((number_alpha(view.sequencer_ui.number_age[1][15]) - 0.5).abs() < 0.0001);
    view.tick_sequencer(0.175);
    assert_eq!(number_alpha(view.sequencer_ui.number_age[1][15]), 0.0);
    view.pointer = Some((rect.0 + 4.0, rect.1 + 4.0));
    view.tick_sequencer(0.0);
    assert_eq!(number_alpha(view.sequencer_ui.number_age[1][15]), 1.0);
}
#[test]
fn row_page_slides_are_independent_and_reverse_from_current_position() {
    let mut view = view();
    view.sequencer_ui.edit_pages[1] = 1;
    view.sequencer_ui.page_elapsed[1] = 0.0;
    view.tick_sequencer(pleasant_ui::page_slide::DURATION / 2.0);
    assert_eq!(view.sequencer_ui.page_positions, [0.0, 0.5, 0.0, 0.0]);
    view.sequencer_ui.page_starts[1] = view.sequencer_ui.page_positions[1];
    view.sequencer_ui.edit_pages[1] = 0;
    view.sequencer_ui.page_elapsed[1] = 0.0;
    view.tick_sequencer(0.0);
    assert_eq!(view.sequencer_ui.page_positions[1], 0.5);
    view.tick_sequencer(pleasant_ui::page_slide::DURATION);
    assert_eq!(view.sequencer_ui.page_positions, [0.0; 4]);
}
#[test]
fn velocity_drag_edits_selected_cell_without_toggling_or_changing_other_patterns() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.params.sequencer.edit(|state| {
        state.lanes[0][2].length = 32;
        state.lanes[0][2].end = 31;
    });
    view.sequencer_ui.edit_pages[0] = 2;
    view.sequencer_ui.modifier = Some(Field::Velocity);
    let r = step_rect(0, 19);
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Left),
        r.0 + 8.0,
        r.1,
    );
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseMove(0.0, 0.0),
        r.0 + 8.0,
        r.1 + r.3 * 0.75,
    );
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseUp(MouseButton::Left),
        r.0 + 8.0,
        r.1 + r.3,
    );
    let state = view.params.sequencer.snapshot();
    assert_eq!(state.lanes[0][2].steps[19].velocity, 50);
    assert!(state.lanes[0][2].steps[19].enabled);
    assert_eq!(state.lanes[0][0].steps[19].velocity, 100);
    assert_eq!(view.sequencer_ui.arp_step, 19);
    assert!(view.sequencer_ui.slider_drag.is_none());
}
#[test]
fn arp_pattern_and_rate_stay_global_while_interlock_edits_the_arp_page() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.sequencer_ui.edit_pages[0] = 5;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    let press = WindowEvent::MouseDown(MouseButton::Left);
    for r in [pattern_rect(0), rate_rect(0)] {
        assert!(!view.sequencer_event(&mut cx, &press, r.0 + 5.0, r.1 + 5.0));
    }
    let r = arp_interlock_rect();
    assert!(view.sequencer_event(&mut cx, &press, r.0 + 5.0, r.1 + 5.0));
    let state = view.params.sequencer.snapshot();
    assert_eq!(state.lanes[0][5].interlock_override, Some(0));
    assert_eq!(state.lanes[0][0].interlock_override, None);
}
#[test]
fn swipe_and_wheel_edit_values_without_enabling_rests() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.sequencer_ui.modifier = Some(Field::Pressure);
    view.params
        .sequencer
        .edit(|state| state.lanes[0][0].steps[2].enabled = false);
    let a = step_rect(0, 1);
    let b = step_rect(0, 4);
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Left),
        a.0 + 4.0,
        a.1,
    );
    view.sequencer_event(&mut cx, &WindowEvent::MouseMove(0.0, 0.0), b.0 + 4.0, b.1);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseUp(MouseButton::Left),
        b.0 + 4.0,
        b.1,
    );
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseScroll(0.0, -1.0),
        b.0 + 4.0,
        b.1,
    );
    let state = view.params.sequencer.snapshot();
    assert_eq!(state.lanes[0][0].steps[1].pressure, 127);
    assert_eq!(state.lanes[0][0].steps[2].pressure, 127);
    assert!(!state.lanes[0][0].steps[2].enabled);
    assert_eq!(state.lanes[0][0].steps[4].pressure, 126);
    assert_eq!(state.lanes[0][0].steps[5].pressure, 0);
}
#[test]
fn selecting_sequencer_steps_preserves_the_arp_layout() {
    let mut view = view();
    view.sequencer_ui.open = true;
    let before = view
        .base_controls()
        .iter()
        .map(|(c, r)| (c.id, *r))
        .collect::<Vec<_>>();
    view.sequencer_ui.open = true;
    view.sequencer_ui.inspector = true;
    for lane in VISIBLE_LANES {
        view.sequencer_ui.lane = lane;
        assert!(view.arp_page());
        assert_eq!(
            view.base_controls()
                .iter()
                .map(|(c, r)| (c.id, *r))
                .collect::<Vec<_>>(),
            before
        );
    }
}
#[test]
fn strumfield_controls_have_no_velocity_scalar() {
    assert!(!output_controls(2).iter().any(|(id, _)| *id == "velocity"));
}
#[test]
fn selected_arp_targets_do_not_overlap_shared_controls() {
    let view = view();
    let mut rects = view
        .arp_items(&view.params.sequencer.snapshot())
        .into_iter()
        .map(|i| i.rect)
        .collect::<Vec<_>>();
    rects.extend(arp_controls().map(|(_, r)| r));
    for (i, a) in rects.iter().enumerate() {
        assert!(hit(ARP_PAD, a.0, a.1) && hit(ARP_PAD, a.0 + a.2, a.1 + a.3));
        for b in &rects[i + 1..] {
            assert!(
                a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1
            );
        }
    }
}
#[test]
fn page_modulation_targets_cover_number_buttons_and_gaps_in_both_sizes() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    let state = view.params.sequencer.snapshot();
    {
        let targets = view.sequencer_route_targets();
        assert_eq!(targets.len(), VISIBLE_LANES.len());
        let items = view.seq_items(&state);
        assert!(!items
            .iter()
            .any(|item| item.label == "Play" || item.label == "Manual strum"));
        for (row, (target, rect)) in targets.iter().enumerate() {
            let lane = VISIBLE_LANES[row];
            assert_eq!(
                crate::engine::routing::TARGETS[*target].id,
                ["seq_page_0", "seq_page_1", "seq_page_2", "seq_page_3"][lane]
            );
            for page in 0..8 {
                let button = items.iter().find(|item|
                    matches!(item.action, Action::EditPage(l, p) if l == lane && p == page)
                ).unwrap().rect;
                assert!(hit(*rect, button.0, button.1));
                assert!(hit(*rect, button.0 + button.2, button.1 + button.3));
                assert_eq!(
                    view.route_target_at(button.0 + button.2 / 2.0, button.1 + 2.0),
                    Some(*target)
                );
                if page < 7 {
                    assert_eq!(
                        view.route_target_at(button.0 + button.2 + 1.0, button.1 + 2.0),
                        Some(*target)
                    );
                }
            }
            let step = step_rect(lane, 0);
            assert!(rect.0 + rect.2 < step.0 || rect.1 + rect.3 < step.1);
        }
    }
}
#[test]
fn geometry_keeps_all_steps_inside_surface_without_overlap() {
    {
        let s = surface();
        for lane in VISIBLE_LANES {
            for step in 0..visible_steps() {
                let r = step_rect(lane, step);
                assert!(hit(s, r.0, r.1) && hit(s, r.0 + r.2, r.1 + r.3));
                assert!(hit(r, r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
                {
                    assert_eq!(r.2, r.3);
                }
                let length = length_rect(lane);
                assert!(length.1 >= r.1 + r.3);
                assert!(hit(s, length.0 + length.2, length.1 + length.3));
                if step > 0 {
                    let prev = step_rect(lane, step - 1);
                    assert!(prev.0 + prev.2 < r.0);
                }
            }
        }
    }
}
#[test]
fn editing_page_is_independent_and_slide_reversal_is_continuous() {
    let mut ui = SequencerUi::default();
    ui.edit_pages[0] = 6;
    assert_eq!(ui.edit_pages, [6, 0, 0, 0]);
    let midpoint = pleasant_ui::page_slide::position(0.0, 1.0, 0.125);
    assert_eq!(
        pleasant_ui::page_slide::position(midpoint, 0.0, 0.0),
        midpoint
    );
}
#[test]
fn arp_inspector_omits_duplicate_and_removed_controls() {
    let mut view = view();
    let state = view.params.sequencer.snapshot();
    let items = view.seq_inspector_items(&state, 0, inspector_rect(), true);
    for removed in [
        "Name:",
        "Steps:",
        "Loop last:",
        "Loop first:",
        "Hits:",
        "Rotation:",
        "Order:",
        "Interlock:",
        "Euclidean",
        "Accents",
        "Tone pool",
        "Tie",
        "Ghosts",
        "Pattern",
        "Delete box",
    ] {
        assert!(
            !items.iter().any(|item| item.label.starts_with(removed)),
            "removed control remained: {removed}"
        );
    }
    assert!(items.iter().any(|item| item.label.starts_with("Gate %:")));
    assert!(items.iter().any(|item| item.label == "Step on" || item.label == "Step off"));
    let surface = surface();
    for item in &items {
        assert!(item.rect.1 + item.rect.3 <= step_rect(0, 0).1);
        assert!(hit(surface, item.rect.0, item.rect.1));
    }
    view.sequencer_ui.open = true;
    let arp = view.arp_items(&state);
    assert!(!arp.iter().any(|item| item.label == "Live pattern"));
    assert!(arp.iter().any(|item| item.label.starts_with("Interlock:")));
    assert!(!arp.iter().any(|item| matches!(
        item.action,
        Action::Field(
            Field::Gate | Field::Octave | Field::Probability | Field::Ratchets | Field::Micro
        )
    )));
}
#[test]
fn raw_controller_offsets_and_finite_values() {
    let mut p = Page::default();
    assert!(set_field(Field::Cc(0), "-32", &mut p, 0, 0));
    assert_eq!(p.steps[0].cc[0], -32);
    assert!(!set_field(Field::Rate, "NaN", &mut p, 0, 0));
}
#[test]
fn inspector_exposes_core_sections_without_switching_tabs() {
    let view = view();
    let items = view.seq_items(&view.params.sequencer.snapshot());
    for field in [Field::Gate, Field::Tone] {
        assert!(items
            .iter()
            .any(|item| matches!(item.action, Action::Field(f) if f == field)));
    }
    assert!(!items.iter().any(|item| matches!(
        item.action,
        Action::Field(
            Field::Velocity | Field::Pressure | Field::Timbre | Field::Bend | Field::Cc(_)
        )
    )));
    assert!(!items.iter().any(|item| item.label == "Tie" || item.label == "Ghosts"));
    assert!(!view
        .seq_items(&view.params.sequencer.snapshot())
        .iter()
        .any(|item| matches!(item.action, Action::LaneEnabled(3) | Action::SelectStep(3, _))));
}
#[test]
fn inspector_controls_fit_in_both_sizes() {
    let mut view = view();
    view.sequencer_ui.inspector = true;
    let state = view.params.sequencer.snapshot();
    {
        let s = surface();
        for item in view.seq_inspector_items(&state, 0, inspector_rect(), true) {
            assert!(hit(s, item.rect.0, item.rect.1));
            assert!(hit(s, item.rect.0 + item.rect.2 - 1.0, item.rect.1 + item.rect.3 - 1.0));
            assert!(item.rect.1 + item.rect.3 <= step_rect(0, 0).1);
        }
    }
}
#[test]
fn focus_loss_discards_draft_and_leaves_pattern_unchanged() {
    let mut view = view();
    let before = view.params.sequencer.snapshot();
    let mut edit = ValueEdit::new(Field::Velocity, (0.0, 0.0, 120.0, 28.0), "100".into());
    edit.insert("32");
    view.sequencer_ui.edit = Some(edit);
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    assert!(!view.sequencer_event(&mut cx, &WindowEvent::FocusOut, 0.0, 0.0));
    assert!(!view.sequencer_editing());
    assert_eq!(view.params.sequencer.snapshot(), before);
}
#[test]
fn selecting_a_row_page_requests_playback_for_only_that_lane() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    use std::{cell::RefCell, rc::Rc};

    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    context.add_global_listener(move |_, event| {
        event.map(|event: &RawParamEvent, _| {
            if let RawParamEvent::SetParameterNormalized(ptr, value) = event {
                observed.borrow_mut().push((*ptr, *value));
            }
        });
    });
    {
        for lane in VISIBLE_LANES {
            for page in (0..8).chain(std::iter::once(7)) {
                let before = view.sequencer_ui.edit_pages;
                let state = view.params.sequencer.snapshot();
                let rect = view.seq_items(&state).into_iter()
                    .find(|c| matches!(c.action, Action::EditPage(l, p) if l == lane && p == page))
                    .unwrap().rect;
                let mut cx = EventContext::new_with_current(&mut context, target);
                assert!(view.sequencer_event(
                    &mut cx,
                    &WindowEvent::MouseDown(MouseButton::Left),
                    rect.0 + 2.0,
                    rect.1 + 2.0
                ));
                BackendContext::new_with_event_manager(&mut context).process_events();
                let param = &view.params.seq_pages[lane].page;
                assert_eq!(
                    &*changes.borrow(),
                    &[(param.as_ptr(), param.preview_normalized(page as i32))]
                );
                changes.borrow_mut().clear();
                let mut expected = before;
                expected[lane] = page;
                assert_eq!(view.sequencer_ui.edit_pages, expected);
            }
        }
    }
}
#[test]
fn text_editor_releases_performance_keys_before_capturing_typing() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.sequencer_ui.inspector = true;
    view.pressed[0] = true;
    view.memory_held[0] = true;
    let state = view.params.sequencer.snapshot();
    let rect = view
        .seq_items(&state)
        .into_iter()
        .find(|c| matches!(c.action, Action::Field(Field::Gate)))
        .unwrap()
        .rect;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    assert!(view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDoubleClick(MouseButton::Left),
        rect.0 + 2.0,
        rect.1 + 2.0
    ));
    assert!(view.sequencer_editing());
    assert!(view.pressed.iter().all(|pressed| !pressed));
    assert!(view.memory_held.iter().all(|held| !held));
    assert!(view.sequencer_event(&mut cx, &WindowEvent::CharInput('q'), 0.0, 0.0));
    assert!(view.pressed.iter().all(|pressed| !pressed));
}
#[test]
fn host_meter_groups_quarter_and_sixteenth_steps_into_bars() {
    assert_eq!(steps_per_bar(0.25, 4, 4), 16);
    assert_eq!(steps_per_bar(1.0, 4, 4), 4);
    assert_eq!(steps_per_bar(1.0, 3, 4), 3);
    assert_eq!(steps_per_bar(1.0, 6, 8), 3);
}
#[test]
fn harmony_row_is_absent_from_the_sequencer() {
    let view = view();
    let items = view.seq_items(&view.params.sequencer.snapshot());
    assert!(!items.iter().any(|item| matches!(
        item.action,
        Action::LaneEnabled(3) | Action::EditPage(3, _) | Action::SelectStep(3, _)
    )));
    assert_eq!(VISIBLE_LANES, [0, 1]);
}
#[test]
fn note_click_toggles_and_right_click_only_selects() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    let r = step_rect(1, 3);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Left),
        r.0 + 4.0,
        r.1 + 4.0,
    );
    assert!(view.params.sequencer.snapshot().lanes[1][0].steps[3].enabled);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Right),
        r.0 + 4.0,
        r.1 + 4.0,
    );
    assert!(view.params.sequencer.snapshot().lanes[1][0].steps[3].enabled);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDoubleClick(MouseButton::Left),
        r.0 + 4.0,
        r.1 + 4.0,
    );
    assert!(!view.params.sequencer.snapshot().lanes[1][0].steps[3].enabled);
}
