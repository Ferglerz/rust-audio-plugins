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
    let mut view = ChordboardView::new(
        params,
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    view.page_position = 1.0;
    view
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
    assert!(!view.arp_page());
    assert!(arp_direction_rect().1 < step_rect(0, 0).1);
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
    assert!(!view.arp_page());
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
fn reserved_modulators_have_header_buttons() {
    let mut view = view();
    view.sequencer_ui.open = true;
    for lane in VISIBLE_LANES {
        view.sequencer_ui.lane = lane;
        let state = view.params.sequencer.snapshot();
        let items = view.seq_items(&state);
        for index in 0..2 {
            assert!(items.iter().any(|item| item.label == format!("Mod {}", index + 1)
                && matches!(item.action, Action::Modifier(Some(Field::Mod(i))) if i == index)));
        }
    }
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
    // 0.75 cell heights of travel covers 0.75 / SLIDER_FRICTION of the 0..200 range.
    assert_eq!(state.lanes[0][2].steps[19].velocity, 40);
    assert!(state.lanes[0][2].steps[19].enabled);
    assert_eq!(state.lanes[0][0].steps[19].velocity, 100);
    assert_eq!(view.sequencer_ui.arp_step, 19);
    assert!(view.sequencer_ui.slider_drag.is_none());
}
#[test]
fn arp_page_clicks_pass_through_the_sequencer() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    let press = WindowEvent::MouseDown(MouseButton::Left);
    let direction = arp_direction_rect();
    assert!(!view.sequencer_event(
        &mut cx,
        &press,
        direction.0 + 5.0,
        direction.1 + 5.0
    ));
    let octave = octave_rect(0);
    assert!(!view.sequencer_event(&mut cx, &press, octave.0 + 5.0, octave.1 + 5.0));
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
    let y = a.1 + a.3 * 0.5;
    // Half of the friction travel lifts pressure from 0 to half of its 254-wide range.
    let up = y - a.3 * SLIDER_FRICTION * 0.5;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Left),
        a.0 + 4.0,
        y,
    );
    view.sequencer_event(&mut cx, &WindowEvent::MouseMove(0.0, 0.0), b.0 + 4.0, up);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseUp(MouseButton::Left),
        b.0 + 4.0,
        up,
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
fn cell_slider_drag_is_relative_with_friction() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.sequencer_ui.modifier = Some(Field::Velocity);
    let r = step_rect(0, 2);
    let (x, y) = (r.0 + 4.0, r.1 + 4.0);
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    let velocity =
        |view: &ChordboardView| view.params.sequencer.snapshot().lanes[0][0].steps[2].velocity;
    // Pressing near the top of the cell must not jump the value.
    view.sequencer_event(&mut cx, &WindowEvent::MouseDown(MouseButton::Left), x, y);
    assert_eq!(velocity(&view), 100);
    view.sequencer_event(&mut cx, &WindowEvent::MouseUp(MouseButton::Left), x, y);
    assert_eq!(velocity(&view), 100);
    view.sequencer_event(&mut cx, &WindowEvent::MouseDown(MouseButton::Left), x, y);
    let travel = 14.0;
    view.sequencer_event(&mut cx, &WindowEvent::MouseMove(0.0, 0.0), x, y - travel);
    let expected = 100.0 + travel * 200.0 / (STEP_H * SLIDER_FRICTION);
    assert_eq!(velocity(&view), expected.round() as u8);
    view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseMove(0.0, 0.0),
        x,
        y - travel + 7.0,
    );
    let expected = expected - 7.0 * 200.0 / (STEP_H * SLIDER_FRICTION);
    assert_eq!(velocity(&view), expected.round() as u8);
    view.sequencer_event(&mut cx, &WindowEvent::MouseUp(MouseButton::Left), x, y);
    let mut drag = SliderDrag {
        reset: false,
        lane: 0,
        page: 0,
        step: 2,
        field: Field::Velocity,
        last_y: 0.0,
        value: 100.0,
    };
    view.drag_step_slider(&mut drag, 14.0, true);
    assert!(
        (drag.value - (100.0 - 14.0 * 200.0 / (STEP_H * SLIDER_FRICTION) * SLIDER_FINE)).abs()
            < 1e-4
    );
    view.drag_step_slider(&mut drag, 10_000.0, false);
    assert_eq!(drag.value, 0.0);
}
#[test]
fn speed_and_fine_fields_round_trip() {
    let ui = SequencerUi::default();
    let mut p = Page::default();
    for speed in 0..SPEED_STEPS.len() as u8 {
        p.steps[0].speed = speed;
        let text = field_value(Field::Speed, &p, &p.steps[0], &ui);
        assert_eq!(text, SPEED_NAMES[speed as usize]);
        p.steps[0].speed = 0;
        assert!(set_field(Field::Speed, &text, &mut p, 0, 0), "{text}");
        assert_eq!(p.steps[0].speed, speed);
    }
    assert!(set_field(Field::Speed, "x3", &mut p, 0, 0));
    assert_eq!(SPEED_STEPS[p.steps[0].speed as usize], 3.0);
    assert!(set_field(Field::Speed, "0.5", &mut p, 0, 0));
    assert_eq!(SPEED_STEPS[p.steps[0].speed as usize], 0.5);
    assert!(!set_field(Field::Speed, "fast", &mut p, 0, 0));
    set_slider_value(Field::Speed, 5.6, &mut p, 0);
    assert_eq!(p.steps[0].speed as usize, SPEED_STEPS.len() - 1);
    assert_eq!(
        slider_range(Field::Speed),
        (0.0, (SPEED_STEPS.len() - 1) as f32)
    );
    for fine in [-FINE_RANGE, -12, 0, 12, FINE_RANGE] {
        p.steps[0].fine = fine;
        let text = field_value(Field::Fine, &p, &p.steps[0], &ui);
        p.steps[0].fine = 1;
        assert!(set_field(Field::Fine, &text, &mut p, 0, 0), "{text}");
        assert_eq!(p.steps[0].fine, fine);
    }
    p.steps[0].fine = 12;
    assert_eq!(field_value(Field::Fine, &p, &p.steps[0], &ui), "+12%");
    assert!(set_field(Field::Fine, "+80%", &mut p, 0, 0));
    assert_eq!(p.steps[0].fine, FINE_RANGE);
    let (min, max) = slider_range(Field::Fine);
    assert!(min < 0.0 && max == -min);
    for field in [Field::Speed, Field::Fine] {
        assert!(is_step_value(field));
        assert!(MODIFIERS.iter().any(|(_, f)| *f == Some(field)));
    }
}
#[test]
fn modulator_meters_no_longer_capture_step_edit_clicks() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let meter = meter_rect(0);
    assert!(!view.sequencer_event(&mut EventContext::new_with_current(&mut context, target),
        &WindowEvent::MouseDown(MouseButton::Left), meter.0 + meter.2 - 10.0, meter.1 + 10.0));
    assert_eq!(view.sequencer_ui.modifier, None);
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
        assert!(!view.arp_page());
        assert_eq!(arp_direction_rect().1, seq_rate_rect().1);
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
fn arp_controls_stay_inside_the_pad_without_overlap() {
    for looping in [false, true] {
        let rects = arp_controls(looping).map(|(_, r)| r).to_vec();
        for (i, a) in rects.iter().enumerate() {
            assert!(hit(SEQ_SURFACE, a.0, a.1) && hit(SEQ_SURFACE, a.0 + a.2, a.1 + a.3));
            for b in &rects[i + 1..] {
                assert!(
                    a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1
                );
            }
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
        let targets: Vec<_> = view
            .sequencer_route_targets()
            .into_iter()
            .filter(|(t, _)| {
                crate::engine::routing::TARGETS[*t]
                    .id
                    .starts_with("seq_page_")
            })
            .collect();
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
                if page % PAGE_COLUMNS < PAGE_COLUMNS - 1 {
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
                assert_eq!(r.3, STEP_H);
                assert!(r.3 > r.2, "steps are taller than wide");
                let length = length_rect(lane);
                assert!(length.1 >= r.1 + r.3);
                assert!(hit(s, length.0 + length.2, length.1 + length.3));
                if step > 0 {
                    let prev = step_rect(lane, step - 1);
                    assert!(prev.0 + prev.2 < r.0);
                }
            }
        }
        let lane0 = length_rect(VISIBLE_LANES[0]);
        assert!(lane0.1 + lane0.3 < step_rect(VISIBLE_LANES[1], 0).1);
        let left = page_block_rect(VISIBLE_LANES[0]);
        assert!(left.0 + left.2 < step_rect(0, 0).0);
    }
}
#[test]
fn page_buttons_form_two_rows_of_four_matching_one_step() {
    for lane in VISIBLE_LANES {
        let step = step_rect(lane, 0);
        let first = page_rect(lane, 0);
        let last = page_rect(lane, 7);
        assert_eq!(last.1 + last.3 - first.1, step.3);
        assert_eq!(first.1, step.1);
        for page in 0..8 {
            let r = page_rect(lane, page);
            assert_eq!(r.1, if page < 4 { first.1 } else { last.1 });
            assert!(hit(page_block_rect(lane), r.0, r.1));
            assert!(hit(page_block_rect(lane), r.0 + r.2, r.1 + r.3));
            for other in page + 1..8 {
                let o = page_rect(lane, other);
                assert!(
                    r.0 + r.2 <= o.0 || o.0 + o.2 <= r.0 || r.1 + r.3 <= o.1 || o.1 + o.3 <= r.1
                );
            }
        }
        let label = lane_label_rect(lane);
        assert!(label.0 + label.2 < first.0);
    }
}
#[test]
fn step_selectors_fit_below_patterns_without_overlap() {
    let mut view = view();
    view.sequencer_ui.open = true;
    for lane in VISIBLE_LANES {
        view.sequencer_ui.lane = lane;
        let state = view.params.sequencer.snapshot();
        let items = view.seq_inspector_items(&state, 0, inspector_rect(), true);
        let rects: Vec<_> = items.iter().map(|i| i.rect).collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(hit(SEQ_SURFACE, a.0, a.1) && hit(SEQ_SURFACE, a.0 + a.2, a.1 + a.3));
            assert!(a.1 >= SEQ_SURFACE.1 + SEQ_SETTINGS_H);
            assert!(a.1 + a.3 <= step_rect(0, 0).1);
            for b in &rects[i + 1..] {
                assert!(a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1, "{a:?} overlaps {b:?}");
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
    let view = view();
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
    assert!(!items.iter().any(|item| item.label.starts_with("Gate %:")));
    assert!(items
        .iter()
        .any(|item| matches!(item.action, Action::Modifier(Some(Field::Gate)))));
    assert!(items
        .iter()
        .any(|item| item.label == "Step on" || item.label == "Step off"));
    let surface = SEQ_SURFACE;
    for item in &items {
        assert!(item.rect.1 + item.rect.3 <= step_rect(0, 0).1);
        assert!(hit(surface, item.rect.0, item.rect.1));
    }
}
#[test]
fn raw_controller_offsets_and_finite_values() {
    let mut p = Page::default();
    assert!(set_field(Field::Cc(0), "-32", &mut p, 0, 0));
    assert_eq!(p.steps[0].cc[0], -32);
    assert!(!set_field(Field::Velocity, "NaN", &mut p, 0, 0));
}
#[test]
fn inspector_exposes_core_sections_without_switching_tabs() {
    let mut view = view();
    view.sequencer_ui.open = true;
    let items = view.seq_items(&view.params.sequencer.snapshot());
    for field in [Field::Tone] {
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
    assert!(!items
        .iter()
        .any(|item| item.label == "Tie" || item.label == "Ghosts"));
    assert!(!view
        .seq_items(&view.params.sequencer.snapshot())
        .iter()
        .any(|item| matches!(
            item.action,
            Action::LaneEnabled(3) | Action::SelectStep(3, _)
        )));
}
#[test]
fn inspector_controls_fit_in_both_sizes() {
    let mut view = view();
    view.sequencer_ui.inspector = true;
    let state = view.params.sequencer.snapshot();
    {
        let s = SEQ_SURFACE;
        for item in view.seq_inspector_items(&state, 0, inspector_rect(), true) {
            assert!(hit(s, item.rect.0, item.rect.1));
            assert!(hit(
                s,
                item.rect.0 + item.rect.2 - 1.0,
                item.rect.1 + item.rect.3 - 1.0
            ));
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
                let rect = view
                    .seq_items(&state)
                    .into_iter()
                    .find(|c| matches!(c.action, Action::EditPage(l, p) if l == lane && p == page))
                    .unwrap()
                    .rect;
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
        .find(|c| matches!(c.action, Action::Field(Field::Tone)))
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
fn cell_faces_start_light_and_alternate_in_groups_of_the_meter_numerator() {
    for (num, size) in [(4, 4), (3, 3), (6, 6), (7, 7)] {
        let group = cells_per_group(num);
        assert_eq!(group, size);
        for step in 0..32 {
            assert_eq!(light_bar(step, group), step / size % 2 == 0);
        }
    }
    assert_eq!(cells_per_group(0), 1);
    assert_eq!(cells_per_group(255), 32);
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

#[test]
fn locked_page_inherits_longest_current_length() {
    let view = view();
    view.params.sequencer.edit(|s| {
        s.lanes[0][0].length = 24;
        s.lanes[0][0].end = 23;
    });
    view.inherit_sequencer_length(0, 1);
    let state = view.params.sequencer.snapshot();
    assert_eq!(state.lanes[0][1].length, 24);
    assert_eq!(state.lanes[1][0].length, 24);
    assert_eq!(state.lanes[0][2].length, 16);
}

#[test]
fn control_and_command_drag_reset_every_crossed_cell() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    for modifier in [Modifiers::CTRL, Modifiers::LOGO] {
        let mut view = view();
        view.sequencer_ui.open = true;
        view.sequencer_ui.progress = 1.0;
        view.sequencer_ui.modifier = Some(Field::Velocity);
        view.params.sequencer.edit(|s| {
            for step in &mut s.lanes[0][0].steps { step.velocity = 40; }
        });
        let mut context = Context::default();
        let target = Element::new(&mut context).entity();
        BackendContext::new(&mut context).modifiers().set(modifier, true);
        let mut cx = EventContext::new_with_current(&mut context, target);
        let a = step_rect(0, 1);
        let b = step_rect(0, 5);
        view.sequencer_event(&mut cx, &WindowEvent::MouseDown(MouseButton::Left), a.0 + 4.0, a.1 + 8.0);
        view.sequencer_event(&mut cx, &WindowEvent::MouseMove(0.0, 0.0), b.0 + 4.0, b.1 + 2.0);
        view.sequencer_event(&mut cx, &WindowEvent::MouseUp(MouseButton::Left), b.0 + 4.0, b.1 + 2.0);
        let state = view.params.sequencer.snapshot();
        for step in 1..=5 { assert_eq!(state.lanes[0][0].steps[step].velocity, Step::default().velocity); }
        assert_eq!(state.lanes[0][0].steps[6].velocity, 40);
        assert!(view.sequencer_ui.slider_drag.is_none());
    }
}

#[test]
fn meter_uses_host_when_available_and_manual_otherwise() {
    let mut view = view();
    view.snapshot.time_sig_num = 7;
    view.snapshot.time_sig_den = 8;
    view.snapshot.time_sig_host = true;
    assert_eq!(view.meter(), (7, 8));
    view.snapshot.time_sig_host = false;
    assert_eq!(view.meter(), (4, 4));
    for mode in [1, 2, 3] { assert!(!output_controls(mode).iter().any(|(id, _)| *id == "velocity")); }
}

#[test]
fn accent_is_absent_from_sequencer_and_note_label_is_compact() {
    let mut view = view();
    view.sequencer_ui.open = true;
    let state = view.params.sequencer.snapshot();
    let items = view.seq_items(&state);
    assert!(items.iter().any(|item| item.label == "Note"));
    assert!(!items.iter().any(|item| item.label.contains("Accent") || item.label == "Use global"));
}

#[test]
fn chords_page_rate_is_editable_without_changing_arp_rate() {
    let mut view = view();
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.sequencer_ui.lane = 1;
    let mut context = Context::default();
    let target = Element::new(&mut context).entity();
    let mut cx = EventContext::new_with_current(&mut context, target);
    let state = view.params.sequencer.snapshot();
    let rate = view.params.rate.value();
    let item = view
        .seq_items(&state)
        .into_iter()
        .find(|item| matches!(item.action, Action::PageRate))
        .unwrap();
    assert!(view.sequencer_event(
        &mut cx,
        &WindowEvent::MouseDown(MouseButton::Left),
        item.rect.0 + 2.0,
        item.rect.1 + 2.0
    ));
    assert_eq!(view.params.sequencer.snapshot().lanes[1][0].rate, 0.125);
    assert_eq!(view.params.rate.value(), rate);
}

#[test]
fn accepted_sequencer_value_press_focuses_and_window_focus_loss_clears_capture() {
    use nih_plug_vizia::vizia::backend::BackendContext;
    use std::{cell::RefCell, rc::Rc};
    fn event(view: &mut ChordboardView, cx: &mut Context, target: Entity, x: f32, y: f32, event: WindowEvent) {
        let mut backend = BackendContext::new_with_event_manager(cx);
        backend.emit_origin(WindowEvent::MouseMove(x, y));
        backend.process_events();
        view.window_event(&mut EventContext::new_with_current(cx, target), &event);
        BackendContext::new_with_event_manager(cx).process_events();
    }
    for length in [false, true] {
        let mut view = view();
        view.sequencer_ui.open = true;
        view.sequencer_ui.progress = 1.0;
        view.sequencer_ui.modifier = Some(Field::Velocity);
        let r = if length { length_rect(0) } else { step_rect(0, 0) };
        let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
        let mut cx = Context::default();
        let target = Element::new(&mut cx).entity();
        BackendContext::new(&mut cx).cache().set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, W, H));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let observed = changes.clone();
        cx.add_global_listener(move |_, event| {
            event.map(|event: &RawParamEvent, _| {
                if let RawParamEvent::SetParameterNormalized(ptr, value) = event {
                    observed.borrow_mut().push((*ptr, *value));
                }
            });
        });
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        assert!(view.focused);
        assert!(view.sequencer_dragging());
        event(&mut view, &mut cx, target, x, y, WindowEvent::KeyDown(Code::ShiftLeft, None));
        assert!(!changes.borrow().iter().any(|(ptr, _)| *ptr == view.params.latch.as_ptr()));
        // Focus cleanup must also run while the host has removed view geometry.
        BackendContext::new(&mut cx).cache().set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, 0.0, 0.0));
        event(&mut view, &mut cx, target, x, y, WindowEvent::FocusOut);
        assert!(!view.focused);
        assert!(!view.sequencer_dragging());
        assert!(!view.sequencer_editing());
        let state = view.params.sequencer.snapshot();
        BackendContext::new(&mut cx).cache().set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, W, H));
        event(&mut view, &mut cx, target, x, y - 20.0, WindowEvent::MouseMove(x, y - 20.0));
        assert_eq!(view.params.sequencer.snapshot(), state);
    }
}

#[test]
fn dedicated_modulator_cells_edit_and_sanitize_saved_values() {
    let mut page = Page::default();
    for index in 0..2 {
        assert!(set_field(Field::Mod(index), "75", &mut page, 0, 0));
        assert_eq!(slider_value(Field::Mod(index), &page.steps[0]), 75.0);
        assert_eq!(slider_range(Field::Mod(index)), (-100.0, 100.0));
        set_slider_value(Field::Mod(index), 150.0, &mut page, 0);
        assert_eq!(page.steps[0].mods[index], 100);
        assert!(set_field(Field::Mod(index), "-75", &mut page, 0, 0));
        assert_eq!(page.steps[0].mods[index], -75);
        assert_eq!(field_value(Field::Mod(index), &page, &page.steps[0], &SequencerUi::default()), "-75%");
        set_slider_value(Field::Mod(index), -150.0, &mut page, 0);
        assert_eq!(page.steps[0].mods[index], -100);
    }
    let legacy: Step = serde_json::from_str("{\"velocity\":90}").unwrap();
    assert_eq!(legacy.mods, [0; 2]);
    let restored: Page = serde_json::from_str(&serde_json::to_string(&page).unwrap()).unwrap();
    assert_eq!(restored.steps[0].mods, [-100; 2]);
}

#[test]
fn bipolar_cell_fills_start_at_zero_and_extend_in_the_value_direction() {
    let mut step = Step::default();
    for field in [Field::Fine, Field::Mod(0), Field::Mod(1), Field::Bend, Field::Pressure, Field::Timbre, Field::Cc(0)] {
        assert_eq!(slider_fill(field, &step), (0.5, 0.5));
    }
    step.fine = FINE_RANGE;
    step.mods = [-100, 100];
    assert_eq!(slider_fill(Field::Fine, &step), (0.0, 0.5));
    assert_eq!(slider_fill(Field::Mod(0), &step), (0.5, 1.0));
    assert_eq!(slider_fill(Field::Mod(1), &step), (0.0, 0.5));
    assert_eq!(slider_fill(Field::Velocity, &step), (0.5, 1.0));
}
