#[allow(dead_code)]
mod support {
    include!("../tests/support.rs");
}
use super::*;

#[test]
fn visual_one_rotates_hit_targets_and_edits_the_original_step() {
    use self::support::*;
    let mut view = view(2, false);
    open_arp(&mut view);
    view.params.arpeggiator.edit(|s| {
        s.rows[0].chord = Some(harmony::parse_chord_symbol("C").unwrap());
        s.rows[0].settings.lock_length = true;
        s.rows[0].settings.steps = 4;
    });
    view.refresh_arp_editor_params();
    let (mut cx, target, _) = context();
    let before = view.params.arpeggiator.initial_prepared().rows.clone();
    let layout = view.arp_string_layout(0, 4);
    event(
        &mut view,
        &mut cx,
        target,
        layout.x[2],
        graph_rect().1 + 30.0,
        WindowEvent::MouseDown(MouseButton::Right),
    );
    let layout = view.arp_string_layout(0, 4);
    assert_eq!(layout.origin, 2);
    assert_eq!(layout.at(layout.x[2]), 2);
    assert!(layout.x[2] < layout.x[3] && layout.x[3] < layout.x[0] && layout.x[0] < layout.x[1]);
    assert_eq!(view.params.arpeggiator.initial_prepared().rows, before);
    click(&mut view, &mut cx, target, layout.status(2));
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.rows[0].page.step(2, 4).state, StepState::Mute);
    assert_eq!(state.rows[0].page.step(0, 4).state, StepState::Play);
    // A drag across the visual wrap edits adjacent displayed steps, not the long canonical range.
    let mut painted = Vec::new();
    paint_between(layout.visual(3), layout.visual(0), 0.0, 100.0, |i, _| {
        painted.push(layout.source(i))
    });
    assert_eq!(painted, vec![3, 0]);
}

#[test]
fn visual_one_wraps_loop_headers_and_keeps_their_edit_identity() {
    use self::support::*;
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        let row = &mut s.performance;
        row.settings.lock_length = true;
        row.settings.steps = 4;
        row.settings.octaves = 2;
        row.page.ensure_loops(2);
    });
    view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
    view.refresh_arp_editor_params();
    view.params.arpeggiator.set_view_step_origin(0, true, 5);
    let g = graph_rect();
    let headers = view.loop_headers(0, 2);
    assert_eq!(headers.iter().filter(|(i, _)| *i == 1).count(), 2);
    let r = headers
        .into_iter()
        .find(|(i, r)| *i == 1 && r.0 < g.0)
        .unwrap()
        .1;
    let plus = loop_header_button(r, 4);
    assert!(plus.0 >= g.0 && plus.0 + plus.2 <= g.0 + g.2);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, plus);
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.performance.page.loops[1].octave, 1);
    assert_eq!(state.performance.page.loops[0].octave, 0);
}

#[test]
fn preset_restore_reconciles_selected_tracker_row_before_layout() {
    use self::support::*;
    // Selection, animation source, and a pending reorder can each reference
    // a row that disappears, even when the other two references are valid.
    for (selected, from, drag) in [(3, 0, None), (0, 3, None), (0, 0, Some((3, 3)))] {
        let mut view = view(2, false);
        open_arp(&mut view);
        let saved = view.params.serialize_fields();
        view.params.arpeggiator.edit(|state| {
            for i in 0..3 {
                state.add_row_after(i);
            }
        });
        view.arp_ui.edit_page = selected;
        view.arp_ui.page_from = from;
        view.arp_ui.page_elapsed = 0.0;
        view.tracker_drag = drag.map(|(from, _)| {
            tracker::TrackerDrag::block(
                &view.params.arpeggiator.snapshot(),
                from,
                (0.0, 0.0),
                0.0,
                false,
            )
        });
        view.tracker_symbol_edit = true;
        view.tracker_symbol = "Cm7".into();
        // Host preset restore replaces persisted rows, but keeps this editor alive.
        view.params.deserialize_fields(&saved);
        let (mut cx, target, _) = context();
        view.event(
            &mut EventContext::new_with_current(&mut cx, target),
            &mut Event::new(RawParamEvent::ParametersChanged),
        );
        // Parameter notification must repair state even before the next UI tick.
        view.arp_string_layout(view.arp_ui.edit_page, 1);
        view.arp_string_layout(view.arp_ui.page_from, 1);
        view.tick_arp_editor(0.016);
        assert_eq!(view.arp_ui.edit_page, 0);
        assert_eq!(view.arp_ui.page_from, 0);
        assert_eq!(view.arp_ui.page_elapsed, pleasant_ui::page_slide::DURATION);
        assert!(view.tracker_drag.is_none());
        assert!(!view.tracker_symbol_edit);
        assert!(view.tracker_symbol.is_empty());
    }
}

#[test]
fn explicit_loop_controls_add_remove_and_preserve_other_variations() {
    use self::support::*;
    let mut view = view(2, false);
    view.params.arpeggiator.edit(|s| {
        s.performance.settings.octaves = 1;
        s.performance.page.initialized = 1;
        s.performance.page.loops[0].steps[0].velocity = 76;
    });
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, loop_add_rect());
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.performance.settings.octaves, 2);
    assert_eq!(state.performance.page.loops[1].steps[0].velocity, 76);
    let r = view.loop_button_rect(1, 4);
    click(&mut view, &mut cx, target, r);
    assert_eq!(
        view.params.arpeggiator.snapshot().performance.page.loops[1].octave,
        1
    );
    let r = view.loop_button_rect(0, 3);
    click(&mut view, &mut cx, target, r);
    assert_eq!(
        view.params.arpeggiator.snapshot().performance.page.loops[0].octave,
        -1
    );
    let header = loop_rect(0, 2);
    click(
        &mut view,
        &mut cx,
        target,
        // Click the loop label; the old broad rectangle's center lands on OCT +.
        (header.0 + 4.0, header.1, 40.0, header.3),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().performance.page.loops[0].octave,
        -1
    );
    view.pointer = None;
    assert!(!view.loop_remove_visible(0, 2));
    let r = view.loop_button_rect(0, 2);
    view.pointer = Some((r.0 + r.2 * 0.5, r.1 + r.3 * 0.5));
    assert!(view.loop_remove_visible(0, 2));
    assert!(!view.loop_remove_visible(1, 2));
    click(&mut view, &mut cx, target, r);
    let state = view.params.arpeggiator.snapshot();
    assert_eq!(state.performance.settings.octaves, 1);
    assert_eq!(state.performance.page.loops[0].octave, 1);
    let r = view.loop_button_rect(0, 2);
    click(&mut view, &mut cx, target, r);
    assert_eq!(
        view.params.arpeggiator.snapshot().performance.settings.octaves,
        1
    );
    assert!(!view.loop_remove_visible(0, 1));
}

#[test]
fn unlocked_length_controls_ignore_clicks_and_scrolling() {
    use self::support::*;
    let mut view = view(2, false);
    open_arp(&mut view);
    let (mut cx, target, _) = context();
    let before = view.params.arpeggiator.snapshot().rows[0].settings;
    assert!(!before.lock_length);
    for r in [fixed_steps_rect(), fill_rect()] {
        click(&mut view, &mut cx, target, r);
        event(
            &mut view,
            &mut cx,
            target,
            r.0 + r.2 * 0.5,
            r.1 + r.3 * 0.5,
            WindowEvent::MouseScroll(0.0, 1.0),
        );
    }
    let after = view.params.arpeggiator.snapshot().rows[0].settings;
    assert_eq!(after.steps, before.steps);
    assert_eq!(after.fill, before.fill);
    click(&mut view, &mut cx, target, lock_length_rect());
    click(&mut view, &mut cx, target, fixed_steps_rect());
    click(&mut view, &mut cx, target, fill_rect());
    let locked = view.params.arpeggiator.snapshot().rows[0].settings;
    assert_eq!(locked.steps, before.steps + 1);
    assert_eq!(before.fill, Fill::Evolve);
    assert_eq!(locked.fill, Fill::Repeat);
    click(&mut view, &mut cx, target, lock_length_rect());
    click(&mut view, &mut cx, target, fixed_steps_rect());
    click(&mut view, &mut cx, target, fill_rect());
    let unlocked = view.params.arpeggiator.snapshot().rows[0].settings;
    assert_eq!(unlocked.steps, locked.steps);
    assert_eq!(unlocked.fill, locked.fill);
}

#[test]
fn fixed_length_controls_and_tracker_inversion_edit_separate_state() {
    use self::support::*;
    let mut view = view(2, false);
    open_arp(&mut view);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, lock_length_rect());
    click(&mut view, &mut cx, target, fixed_steps_rect());
    click(&mut view, &mut cx, target, fill_rect());
    let plus = tracker_voicing_button(true, true);
    click(&mut view, &mut cx, target, plus);
    let state = view.params.arpeggiator.snapshot();
    assert!(state.rows[0].settings.lock_length);
    assert_eq!(state.rows[0].settings.steps, 5);
    assert_eq!(state.rows[0].settings.fill, Fill::Repeat);
    assert_eq!(state.rows[0].arp_inversion, 1);
    assert_eq!(state.rows[0].arp_octave, 0);
    for _ in 0..30 {
        view.edit_tracker_voicing(true, -1, false);
    }
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].arp_inversion,
        -12
    );
    view.edit_tracker_voicing(true, 0, true);
    assert_eq!(view.params.arpeggiator.snapshot().rows[0].arp_inversion, 0);
    view.edit_fixed_steps(-24, false);
    assert_eq!(view.params.arpeggiator.snapshot().rows[0].settings.steps, 1);
    view.edit_fixed_steps(24, false);
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].settings.steps,
        24
    );
}

#[test]
fn loop_header_actions_fit_four_loops_and_custom_keeps_chord_board() {
    use self::support::*;
    for count in 1..=4 {
        for index in 0..count {
            let header = loop_rect(index, count);
            let buttons: Vec<_> = [0, 1, 2, 3, 4]
                .into_iter()
                .map(|i| loop_header_button(header, i))
                .collect();
            for (i, button) in buttons.iter().enumerate() {
                assert!(button.0 >= header.0 && button.0 + button.2 <= header.0 + header.2);
                for other in &buttons[..i] {
                    assert!(button.0 >= other.0 + other.2 || other.0 >= button.0 + button.2);
                }
            }
        }
    }
    let mut view = view(2, false);
    open_arp(&mut view);
    let (mut cx, target, _) = context();
    view.set_arp_chooser(Some(Chooser::Order));
    view.choose_arp_option(&mut EventContext::new_with_current(&mut cx, target), 7);
    assert_eq!(view.panel, None);
    assert!(view.arp_ui.chooser.is_none());
}

#[test]
fn locked_skip_preview_excludes_swing_clock_and_pattern_paste_keeps_program() {
    use self::support::*;
    let mut view = view(2, false);
    open_arp(&mut view);
    view.params.arpeggiator.edit(|s| {
        let row = &mut s.rows[0];
        row.settings.lock_length = true;
        row.settings.steps = 4;
        row.settings.swing = 0.5;
        row.settings.pattern = 7;
        row.settings.custom[0].degree = 5;
        row.page.loops[0].steps[1].state = StepState::Skip;
    });
    let layout = view.arp_string_layout(0, 4);
    assert_eq!(layout.interval[1], layout.interval[2]);
    view.copy_arp_page(0);
    view.params.arpeggiator.edit(|s| {
        s.add_row_after(0);
        s.rows[1].settings = Default::default();
    });
    let (mut cx, target, _) = context();
    view.paste_arp_page(
        &mut EventContext::new_with_current(&mut cx, target),
        1,
        true,
    );
    let state = view.params.arpeggiator.snapshot();
    assert!(state.rows[1].settings.lock_length);
    assert_eq!(state.rows[1].settings.pattern, 7);
    assert_eq!(state.rows[1].settings.custom[0].degree, 5);
    assert_eq!(
        state.rows[1].settings.swing, 0.0,
        "pattern-only paste keeps timing rate controls"
    );
}

#[test]
fn step_click_and_scroll_finish_without_reentering_the_pattern_lock() {
    use self::support::*;
    let mut view = view(2, false);
    open_arp(&mut view);
    view.programming = true;
    view.row_slide.tick(pleasant_ui::page_slide::DURATION);
    view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
    view.params
        .arpeggiator
        .edit(|s| s.rows[0].settings.octaves = 1);
    let (mut cx, target, _) = context();
    let (_, len) = view.arp_cycle_for_editor();
    let layout = view.arp_string_layout(0, len);
    click(&mut view, &mut cx, target, layout.status(0));
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].page.loops[0].steps[0].state,
        StepState::Mute
    );
    let g = graph_rect();
    event(
        &mut view,
        &mut cx,
        target,
        layout.x[0],
        g.1 + 20.0,
        WindowEvent::MouseScroll(0.0, 1.0),
    );
    assert_eq!(
        view.params.arpeggiator.snapshot().rows[0].page.loops[0].steps[0].velocity,
        101
    );
}

#[test]
fn chooser_cards_fill_the_string_area_with_separate_large_targets() {
    let area = chooser_area();
    for i in 0..8 {
        let r = chooser_card(i);
        assert!(r.2 > 140.0 && r.3 > 100.0);
        assert_eq!(r.1, area.1);
        assert_eq!(r.3, area.3);
        if i > 0 {
            let previous = chooser_card(i - 1);
            assert!((r.0 - previous.0 - previous.2 - 8.0).abs() < 0.001);
        }
    }
    let last = chooser_card(7);
    assert!((last.0 + last.2 - area.0 - area.2).abs() < 0.001);
}

#[test]
fn swing_positions_and_microtiming_follow_the_effective_clock_inside_the_graph() {
    let mut steps = [Step::default(); STEPS];
    for len in [2, 3, 4, 95, 96, STEPS] {
        for swing in [-0.75, 0.0, 0.75] {
            for phase in [0, 1] {
                let layout = StringLayout::new(&steps, len, swing, phase);
                for i in 0..len {
                    assert_eq!(layout.at(layout.x[i]), i);
                    for micro in [-0.49, 0.49] {
                        let x = layout.x[i] + micro * layout.interval[i];
                        assert!(
                            x >= graph_rect().0 - 0.001
                                && x <= graph_rect().0 + graph_rect().2 + 0.001
                        );
                    }
                    if i > 0 {
                        assert!(
                            (layout.x[i] - layout.x[i - 1] - layout.interval[i - 1]).abs() < 0.001
                        );
                    }
                }
            }
        }
    }
    steps[1].state = StepState::Skip;
    let layout = StringLayout::new(&steps, 4, 0.5, 0);
    assert_eq!(layout.interval[1], layout.interval[2]);
    steps[1].state = StepState::Mute;
    let muted = StringLayout::new(&steps, 4, 0.5, 0);
    assert_ne!(muted.interval[1], muted.interval[2]);
}

#[test]
fn chooser_and_page_changes_use_the_shared_slide_and_keep_their_destination() {
    let mut view = ChordboardView::new(
        Arc::new(ChordboardParams::default()),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    view.set_arp_chooser(Some(Chooser::Order));
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
    assert!((view.arp_ui.chooser_position - 0.5).abs() < 0.001);
    view.set_arp_chooser(None);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    assert_eq!(view.arp_ui.chooser_position, 0.0);
    view.set_arp_editor_page(1);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
    assert_eq!(view.arp_ui.page_from, 0);
    assert_eq!(view.arp_ui.edit_page, 1);
    assert_eq!(
        view.arp_ui.page_elapsed,
        pleasant_ui::page_slide::DURATION * 0.5
    );
    view.set_arp_editor_page(2);
    view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
    assert_eq!(view.arp_ui.page_from, 1);
    assert_eq!(view.arp_ui.edit_page, 2);
}

#[test]
fn full_cycles_stay_even_and_inside_the_module() {
    for len in [1, 4, 32, 48, 94, 96, STEPS] {
        let layout = StringLayout::new(&[Step::default(); STEPS], len, 0.0, 0);
        for i in 0..len {
            let r = layout.status(i);
            assert!(hit(ARP_SURFACE, r.0, r.1));
            assert!(hit(ARP_SURFACE, r.0 + r.2, r.1 + r.3));
            if i > 0 {
                assert!((layout.x[i] - layout.x[i - 1] - layout.spacing).abs() < 0.001)
            }
        }
    }
}

#[test]
fn tracker_loop_geometry_ignores_obsolete_modulation_and_live_playhead() {
    use crate::engine::routing::TARGETS;
    let mut view = ChordboardView::new(
        Arc::new(ChordboardParams::default()),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    view.programming = true;
    view.row_slide.tick(pleasant_ui::page_slide::DURATION);
    view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
    let target = TARGETS.iter().position(|t| t.id == "arp_octaves").unwrap();
    let route = &mut Arc::get_mut(&mut view.params).unwrap().routes[1];
    route.source = IntParam::new(
        "Source",
        1,
        IntRange::Linear {
            min: 0,
            max: crate::engine::routing::SOURCE_COUNT as i32,
        },
    );
    route.target = IntParam::new(
        "Target",
        target as i32,
        IntRange::Linear {
            min: 0,
            max: TARGETS.len() as i32 - 1,
        },
    );
    assert!(!TARGETS[target].available());
    view.params.arpeggiator.edit(|state| {
        state.rows[0].settings.octaves = 4;
        state.rows[0].settings.swing = 0.5;
        state.rows[0].page.ensure_loops(4);
    });
    let (_, count) = view.arp_cycle_for_editor();
    assert_eq!(count, 12);
    assert_eq!(view.visible_loops(), 4);
    view.snapshot.arp_running = true;
    view.snapshot.arp_phase = 0;
    let before = view.arp_string_layout(0, count);
    view.snapshot.arp_phase = 25;
    view.snapshot.arp_step = 1;
    view.snapshot.routed[target] = Some(1.0);
    view.snapshot.arp_len = 24;
    let (_, after_count) = view.arp_cycle_for_editor();
    let after = view.arp_string_layout(0, after_count);
    assert_eq!(before.x, after.x);
    assert_eq!(before.status(11), after.status(11));
    view.snapshot.routed[target] = Some(0.0);
    view.snapshot.arp_len = 3;
    assert_eq!(view.arp_cycle_for_editor().1, count);
}

#[test]
fn fast_paint_and_reverse_ramp_fill_every_crossed_string() {
    let mut values = [0.0; 8];
    paint_between(1, 6, 20.0, 120.0, |i, v| values[i] = v);
    assert_eq!(values, [0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0, 0.0]);
    paint_between(6, 2, 100.0, 20.0, |i, v| values[i] = v);
    assert_eq!(&values[2..=6], &[20.0, 40.0, 60.0, 80.0, 100.0]);
}

#[test]
fn adjustment_defaults_and_bounds_match_the_engine() {
    let mut s = Step::default();
    assert_eq!(Lane::Velocity.get(&s), 100.0);
    assert_eq!(s.gate, 65);
    Lane::Gate.set(&mut s, 500.0);
    assert_eq!(s.gate, 100);
    Lane::Mod2.set(&mut s, -200.0);
    assert_eq!(s.mods[1], -100);
    let mut t = DelayTap::default();
    TapLane::Feedback.set(&mut t, 1.0);
    assert_eq!(t.feedback, 0.95);
    TapLane::Channel.set(&mut t, 20.0);
    assert_eq!(t.channel, 16);
}

#[test]
fn slider_reset_gestures_restore_each_step_lane_and_gate() {
    use self::support::*;
    use nih_plug_vizia::vizia::backend::BackendContext;
    for double in [false] {
        for lane in LANES {
            let mut view = view(2, false);
            open_arp(&mut view);
            view.programming = true;
            view.row_slide.tick(pleasant_ui::page_slide::DURATION);
            view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
            view.arp_ui.lane = lane;
            view.params.arpeggiator.edit(|s| {
                lane.set(&mut s.rows[0].page.loops[0].steps[0], lane.range().1);
                s.rows[0].page.loops[0].steps[0].gate = 99;
            });
            let (mut cx, target, _) = context();
            BackendContext::new(&mut cx)
                .modifiers()
                .set(Modifiers::LOGO, !double);
            let (_, len) = view.arp_cycle_for_editor();
            let x = view.arp_string_layout(0, len).x[0];
            let g = graph_rect();
            let press = || {
                if double {
                    WindowEvent::MouseDoubleClick(MouseButton::Left)
                } else {
                    WindowEvent::MouseDown(MouseButton::Left)
                }
            };
            event(&mut view, &mut cx, target, x, g.1 + g.3 * 0.5, press());
            event(
                &mut view,
                &mut cx,
                target,
                x,
                g.1 + g.3 * 0.5,
                WindowEvent::MouseUp(MouseButton::Left),
            );
            assert_eq!(
                lane.get(&view.params.arpeggiator.snapshot().rows[0].page.loops[0].steps[0]),
                lane.get(&Step::default()),
                "{lane:?}, double={double}"
            );
            {
                let gate = gate_rect();
                event(
                    &mut view,
                    &mut cx,
                    target,
                    x,
                    gate.1 + gate.3 * 0.5,
                    press(),
                );
                assert_eq!(
                    view.params.arpeggiator.snapshot().rows[0].page.loops[0].steps[0].micro,
                    Step::default().micro
                );
            }
        }
    }
}

#[test]
fn slider_reset_gestures_restore_delay_controls_and_tap_sliders() {
    use self::support::*;
    use nih_plug_vizia::vizia::backend::BackendContext;
    for double in [false] {
        for sync in [false, true] {
            let mut view = view(2, false);
            open_arp(&mut view);
            view.fx_ui.open = true;
            view.params.arpeggiator.edit(|s| {
                let delay = &mut s.midi_fx.delay;
                delay.sync = sync;
                delay.taps[0].time = if sync { 1.5 } else { 750.0 };
                delay.taps[0].velocity = 1.5;
                delay.taps[0].feedback = 0.8;
            });
            let (mut cx, target, _) = context();
            BackendContext::new(&mut cx)
                .modifiers()
                .set(Modifiers::LOGO, !double);
            for r in [
                delay_time_rect(),
                delay_gain_rect(),
                delay_feedback_rect(),
                tap_graph(),
            ] {
                event(
                    &mut view,
                    &mut cx,
                    target,
                    r.0 + r.2 * 0.5,
                    r.1 + r.3 * 0.5,
                    if double {
                        WindowEvent::MouseDoubleClick(MouseButton::Left)
                    } else {
                        WindowEvent::MouseDown(MouseButton::Left)
                    },
                );
                event(
                    &mut view,
                    &mut cx,
                    target,
                    r.0 + r.2 * 0.5,
                    r.1 + r.3 * 0.5,
                    WindowEvent::MouseUp(MouseButton::Left),
                );
            }
            let tap = view.params.arpeggiator.snapshot().midi_fx.delay.taps[0];
            assert_eq!(tap.time, if sync { 0.5 } else { 250.0 });
            assert_eq!(tap.velocity, DelayTap::default().velocity);
            assert_eq!(tap.feedback, DelayTap::default().feedback);
        }
    }
}

#[test]
fn lane_grid_selects_each_adjustment_without_editing_the_row() {
    use self::support::*;
    let mut view = view(2, false);
    open_arp(&mut view);
    view.programming = true;
    view.row_slide.tick(pleasant_ui::page_slide::DURATION);
    view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
    let (mut cx, target, changes) = context();
    let rows = view.params.arpeggiator.snapshot().rows;
    for (i, lane) in LANES.iter().enumerate() {
        click(&mut view, &mut cx, target, lane_rect(i));
        assert_eq!(view.arp_ui.lane, *lane);
        assert!(view.arp_ui.drag.is_none());
        assert_eq!(view.params.arpeggiator.snapshot().rows, rows);
    }
    assert!(changes.borrow().is_empty());
}

#[test]
fn page_menu_and_direct_adjustments_have_separate_targets() {
    let mut controls = vec![
        mode_rect(),
        order_rect(),
        loop_add_rect(),
        sync_rect(),
        rate_rect(),
        humanize_rect(),
        swing_rect(),
        lock_length_rect(),
        fixed_steps_rect(),
        fill_rect(),
        custom_edit_rect(),
    ];
    controls.extend((0..LANES.len()).map(lane_rect));
    for (i, r) in controls.iter().enumerate() {
        assert!(
            r.1 + r.3 <= content_rect().1
                || r.0 + r.2 < content_rect().0
                || r.0 >= content_rect().0 + content_rect().2
        );
        for other in &controls[..i] {
            assert!(
                r.0 >= other.0 + other.2
                    || other.0 >= r.0 + r.2
                    || r.1 >= other.1 + other.3
                    || other.1 >= r.1 + r.3,
                "{r:?} overlaps {other:?}"
            );
        }
    }
    for page in 0..8 {
        let r = page_rect(page);
        let copy = page_copy_rect(page);
        let paste = page_paste_rect(page);
        assert!(r.0 + r.2 < copy.0 && copy.0 + copy.2 < paste.0);
        assert!(paste.1 + paste.3 < ARP_SURFACE.1 + ARP_SURFACE.3);
    }
    let g = tap_graph();
    assert!(g.3 >= 88.0);
    let remove = tap_remove_rect(&DelayTap::default(), 2.5);
    assert_eq!((remove.2, remove.3), (18.0, 18.0));
    assert!(remove.1 > g.1 + g.3 + 14.0);
    assert!(remove.1 + remove.3 <= DELAY_SURFACE.1 + DELAY_SURFACE.3);
}

#[test]
fn dropdowns_and_delay_graph_fit_and_handles_are_separate() {
    for (which, n) in [
        (Dropdown::TapLane, 6),
        (Dropdown::Paste(0), 2),
        (Dropdown::Paste(7), 2),
    ] {
        for i in 0..n {
            let r = dropdown_rect(which, i);
            let surface = if which == Dropdown::TapLane {
                DELAY_SURFACE
            } else {
                ARP_SURFACE
            };
            assert!(hit(surface, r.0, r.1));
            assert!(hit(surface, r.0 + r.2, r.1 + r.3));
        }
    }
    let g = tap_graph();
    assert!(g.1 > delay_time_rect().1 + delay_time_rect().3);
    assert!(g.1 + g.3 < DELAY_SURFACE.1 + DELAY_SURFACE.3 - 27.0);
}

#[test]
fn delay_controls_and_timeline_reserve_shared_source_column() {
    let boundary = super::super::midi_fx::source_rect(0).0 - 12.0;
    for rect in [
        delay_power_rect(),
        delay_sync_rect(),
        delay_time_rect(),
        delay_gain_rect(),
        delay_feedback_rect(),
        tap_lane_rect(),
        ping_rect(),
        delay_leading_rect(),
        delay_follow_rect(),
        tap_graph(),
    ] {
        assert!(
            rect.0 + rect.2 < boundary,
            "Delay control overlaps Sources: {rect:?}"
        );
    }
    for i in 0..6 {
        let rect = dropdown_rect(Dropdown::TapLane, i);
        assert!(rect.0 + rect.2 < boundary);
    }
    for sync in [true, false] {
        let mut delay = DelaySettings {
            sync,
            count: 8,
            ..DelaySettings::default()
        };
        for (i, tap) in delay.taps.iter_mut().enumerate() {
            tap.time = time_range(sync).1 * (i + 1) as f32 / 8.0;
        }
        let domain = time_domain(&delay);
        for tap in &delay.taps {
            let remove = tap_remove_rect(tap, domain);
            assert!(remove.0 + remove.2 < boundary);
        }
        let add = add_rect(&delay, domain);
        assert!(add.0 + add.2 < boundary);
    }
}
