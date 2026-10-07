use super::super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use super::support::*;

#[test]
fn routing_panel_owns_covered_targets_and_edits_the_selected_slot() {
    let mut view = view(2, false);
    let mut params = ChordboardParams::default();
    params.routes[7].source = IntParam::new("Source", 8, IntRange::Linear { min: 0, max: 9 });
    params.mode = IntParam::new("Mode", 2, IntRange::Linear { min: 0, max: 3 });
    view.params = Arc::new(params);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, meter_rect(7));
    assert_eq!(view.panel, Some(Panel::Routes));
    let r = view
        .editor_route_chips()
        .into_iter()
        .find(|(slot, _)| *slot == 7)
        .unwrap()
        .1;
    click(&mut view, &mut cx, target, r);
    assert_eq!(view.route_slot, 7);
    click(&mut view, &mut cx, target, ROUTE_SOURCE);
    assert_eq!(view.menu, Some(Menu::RouteSource));
    let choice = view.menu_option_rect(Menu::RouteSource, 2);
    click(&mut view, &mut cx, target, choice);
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.routes[7].source.as_ptr()));
    assert!(!changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.routes[0].source.as_ptr()));
    // Covered pad space cannot start a strum through the smaller editor.
    click(&mut view, &mut cx, target, (900.0, 392.0, 16.0, 16.0));
    assert_eq!(view.panel, Some(Panel::Routes));
}
#[test]
fn route_ranges_use_destination_units_and_route_slots_fit() {
    let mut view = view(2, false);
    view.route_slot = 1;
    let c = view.control("route_min").unwrap();
    assert_eq!(view.display_value(&c), "3");
    assert!((view.parse_display_value(&c, "8").unwrap() - 5.0 / 9.0).abs() < 1e-6);
    for i in 0..crate::engine::routing::ROUTE_COUNT {
        let r = route_slot_rect(i);
        assert!(hit(Panel::Routes.rect(), r.0, r.1));
        assert!(hit(Panel::Routes.rect(), r.0 + r.2 - 1.0, r.1 + r.3 - 1.0));
    }
    for menu in [Menu::RouteSource, Menu::RouteTarget] {
        let r = menu.bounds();
        assert!(r.0 >= 0.0 && r.1 >= 0.0 && r.0 + r.2 <= W && r.1 + r.3 <= H);
    }
}
#[test]
fn source_meter_click_opens_routes_and_selection_mode_follows_the_strumfield() {
    for mode in 1..=3 {
        let mut view = view(mode, false);
        let (mut cx, target, changes) = context();
        click(&mut view, &mut cx, target, ROOT_ON_SELECT);
        assert!(changes
            .borrow()
            .iter()
            .any(|(p, v)| *p == view.params.root_on_select.as_ptr() && *v == 1.0));
        changes.borrow_mut().clear();
        click(&mut view, &mut cx, target, meter_rect(3));
        assert_eq!(view.panel, Some(Panel::Routes));
        assert_eq!(view.selected_modulator, Some(3));
        assert!(view.drag.is_none());
        assert!(changes.borrow().is_empty());
    }
}
fn route_drag_to(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    source: usize,
    x: f32,
    y: f32,
) {
    let r = meter_rect(source);
    event(
        view,
        cx,
        target,
        r.0 + r.2 / 2.0,
        r.1 + r.3 / 2.0,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(view, cx, target, x, y, WindowEvent::MouseMove(x, y));
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
}
#[test]
fn route_drag_links_only_on_drop_and_never_moves_the_destination_control() {
    for (source, id) in [(0, "strings"), (3, "spread"), (8, "velocity")] {
        let mut view = view(2, false);
        if id == "velocity" {
            open_arp(&mut view);
        }
        let (mut cx, entity, changes) = context();
        let r = meter_rect(source);
        let (_, destination) = view
            .route_targets()
            .into_iter()
            .find(|(i, _)| crate::engine::routing::TARGETS[*i].id == id)
            .unwrap();
        let (x, y) = (
            destination.0 + destination.2 / 2.0,
            destination.1 + destination.3 / 2.0,
        );
        event(
            &mut view,
            &mut cx,
            entity,
            r.0 + 20.0,
            r.1 + 20.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        assert!(changes.borrow().is_empty());
        assert!(matches!(view.drag, Some(Drag::Route(drag)) if drag.active));
        assert!(view.route_drag_hint().unwrap().starts_with("Link "));
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert_eq!(view.panel, Some(Panel::Routes));
        let route = &view.params.routes[view.route_slot];
        let destination_index = crate::engine::routing::TARGETS
            .iter()
            .position(|t| t.id == id)
            .unwrap();
        assert!(changes.borrow().contains(&(
            route.source.as_ptr(),
            route.source.preview_normalized(source as i32 + 1)
        )));
        assert!(changes.borrow().contains(&(
            route.target.as_ptr(),
            route.target.preview_normalized(destination_index as i32)
        )));
        assert!(!changes
            .borrow()
            .iter()
            .any(|(p, _)| *p == view.control(id).unwrap().ptr));
        assert!(view.drag.is_none() && view.edit.is_none() && view.menu.is_none());
    }
}
#[test]
fn route_drag_cancel_jitter_and_focus_loss_never_assign_or_strum() {
    for cancel in [0, 1, 2, 3] {
        let mut view = view(2, false);
        Arc::get_mut(&mut view.params).unwrap().strum_latch =
            BoolParam::new("Trackpad latch", true);
        let (mut cx, entity, changes) = context();
        let r = meter_rect(1);
        event(
            &mut view,
            &mut cx,
            entity,
            r.0 + 20.0,
            r.1 + 20.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        let (x, y) = if cancel == 0 {
            (r.0 + 22.0, r.1 + 21.0)
        } else if cancel == 1 {
            (730.0, 538.0)
        } else {
            (850.0, 400.0)
        };
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        if cancel == 2 {
            event(
                &mut view,
                &mut cx,
                entity,
                x,
                y,
                WindowEvent::KeyDown(Code::Escape, None),
            );
        } else if cancel == 3 {
            event(&mut view, &mut cx, entity, x, y, WindowEvent::FocusOut);
        }
        event(
            &mut view,
            &mut cx,
            entity,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        assert!(changes.borrow().is_empty());
        assert!(view.drag.is_none() && !view.pad_hover);
        assert!(
            !std::iter::from_fn(|| view.bridge.commands.pop()).any(|c| matches!(
                c,
                Command::BeginGesture(_, _) | Command::X(_) | Command::Y(_)
            ))
        );
    }
}
#[test]
fn route_drag_reuses_existing_destination_and_preserves_range_and_curve() {
    for source in [0, 3] {
        let mut view = view(1, false);
        let route = &mut Arc::get_mut(&mut view.params).unwrap().routes[6];
        route.source = IntParam::new("Source", 1, IntRange::Linear { min: 0, max: 9 });
        route.min = FloatParam::new("Minimum", 0.25, FloatRange::Linear { min: 0.0, max: 1.0 });
        route.curve = FloatParam::new(
            "Curve",
            -0.4,
            FloatRange::Linear {
                min: -1.0,
                max: 1.0,
            },
        );
        open_arp(&mut view);
        let (mut cx, entity, changes) = context();
        let rect = output_controls(1)
            .into_iter()
            .find(|(id, _)| *id == "strings")
            .unwrap()
            .1;
        route_drag_to(
            &mut view,
            &mut cx,
            entity,
            source,
            rect.0 + rect.2 / 2.0,
            rect.1 + 18.0,
        );
        assert_eq!(view.route_slot, 6);
        assert_eq!(view.panel, Some(Panel::Routes));
        let expected = if source == 0 {
            vec![]
        } else {
            vec![(view.params.routes[6].source.as_ptr(), 4.0 / 9.0)]
        };
        assert_eq!(*changes.borrow(), expected);
    }
}
#[test]
fn route_drag_exposes_keyboard_output_destinations() {
    let mut view = view(1, false);
    let (mut cx, entity, changes) = context();
    let (_, rect) = view
        .keyboard_controls()
        .into_iter()
        .find(|(c, _)| c.id == "output_channel")
        .unwrap();
    route_drag_to(&mut view, &mut cx, entity, 1, rect.0 + 25.0, rect.1 + 20.0);
    assert_eq!(view.panel, Some(Panel::Routes));
    assert!(changes
        .borrow()
        .iter()
        .any(|(p, _)| *p == view.params.routes[view.route_slot].source.as_ptr()));
    changes.borrow_mut().clear();
    // Starting a new drag folds the route editor away so the strumfield Strings is reachable.
    let rect = output_controls(2)
        .into_iter()
        .find(|(id, _)| *id == "strings")
        .unwrap()
        .1;
    route_drag_to(
        &mut view,
        &mut cx,
        entity,
        2,
        rect.0 + rect.2 / 2.0,
        rect.1 + 18.0,
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|(p, _)| *p == view.params.routes[view.route_slot].target.as_ptr()));
}
#[test]
fn route_drag_with_no_free_slots_does_not_overwrite_an_unrelated_link() {
    let mut view = view(2, false);
    for route in &mut Arc::get_mut(&mut view.params).unwrap().routes {
        route.source = IntParam::new("Source", 1, IntRange::Linear { min: 0, max: 9 });
    }
    let (mut cx, entity, changes) = context();
    route_drag_to(&mut view, &mut cx, entity, 1, 200.0, PIANO_KEYS.1 + 20.0); // Spread; every slot is assigned.
    assert!(changes.borrow().is_empty());
    assert!(view.status.starts_with("All 16 routes"));
    assert!(view.panel.is_none());
}
#[test]
fn auto_strum_cannot_expand_and_collapses_the_manual_field() {
    let mut view = view(1, false);
    let (mut cx, target, _) = context();
    assert!(!view.can_expand_strum());
    click(&mut view, &mut cx, target, expand_rect(0.0));
    assert_eq!(view.expand_target, 0.0);
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.expand_target, 0.0);
}
#[test]
fn modulator_drop_targets_include_auto_amount_and_manual_sweep() {
    for (mode, id) in [(1, "strings_played"), (2, "x")] {
        let mut view = view(mode, false);
        if id == "strings_played" {
            open_arp(&mut view);
        }
        let (mut cx, entity, changes) = context();
        let target = crate::engine::routing::TARGETS
            .iter()
            .position(|t| t.id == id)
            .unwrap();
        let (_, r) = view
            .route_targets()
            .into_iter()
            .find(|(i, _)| *i == target)
            .unwrap();
        view.finish_route_drag(
            &mut EventContext::new_with_current(&mut cx, entity),
            1,
            r.0 + r.2 / 2.0,
            r.1 + r.3 / 2.0,
        );
        BackendContext::new_with_event_manager(&mut cx).process_events();
        let route = &view.params.routes[view.route_slot];
        if id != "x" {
            assert!(changes.borrow().contains(&(
                route.target.as_ptr(),
                route.target.preview_normalized(target as i32)
            )));
        }
        assert!(changes
            .borrow()
            .contains(&(route.source.as_ptr(), route.source.preview_normalized(2))));
    }
}
#[test]
fn route_graph_nodes_edit_selected_parameters_and_release_capture() {
    for node in 0..3 {
        let mut view = view(2, false);
        view.panel = Some(Panel::Routes);
        view.route_slot = 7;
        Arc::get_mut(&mut view.params).unwrap().routes[7].source =
            IntParam::new("Source", 2, IntRange::Linear { min: 0, max: 9 });
        let (mut cx, target, changes) = context();
        let (x, y) = view.route_nodes()[node];
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(matches!(view.drag, Some(Drag::RouteNode(..))));
        let y = ROUTE_GRAPH.1 + ROUTE_GRAPH.3 * 0.75;
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseMove(x, y),
        );
        event(
            &mut view,
            &mut cx,
            target,
            x,
            y,
            WindowEvent::MouseUp(MouseButton::Left),
        );
        let r = &view.params.routes[7];
        let (ptr, expected) = match node {
            0 => (r.min.as_ptr(), 0.25),
            2 => (r.max.as_ptr(), 0.25),
            _ => (r.curve.as_ptr(), r.curve.preview_normalized(1.0 / 3.0)),
        };
        assert!(changes
            .borrow()
            .iter()
            .any(|(p, v)| *p == ptr && (v - expected).abs() < 0.0001));
        assert!(view.drag.is_none());
        assert!(!std::iter::from_fn(|| view.bridge.commands.pop())
            .any(|c| matches!(c, Command::BeginGesture(..))));
    }
}
#[test]
fn route_graph_curve_handles_reversed_and_flat_ranges() {
    let mut view = view(2, false);
    let r = &mut Arc::get_mut(&mut view.params).unwrap().routes[0];
    r.min = FloatParam::new("Minimum", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    r.max = FloatParam::new("Maximum", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    let value = view
        .route_node_value(1, ROUTE_GRAPH.1 + ROUTE_GRAPH.3 * 0.25)
        .unwrap();
    assert!((value - view.params.routes[0].curve.preview_normalized(1.0 / 3.0)).abs() < 0.0001);
    Arc::get_mut(&mut view.params).unwrap().routes[0].max =
        FloatParam::new("Maximum", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 });
    assert_eq!(view.route_node_value(1, ROUTE_GRAPH.1), None);
}
#[test]
fn route_destination_menu_skips_touch_bounds_without_shifting_saved_ids() {
    let mut view = view(2, false);
    view.route_slot = 1;
    let (mut cx, entity, changes) = context();
    for (menu_index, (saved_index, target)) in
        crate::engine::routing::available_targets().enumerate()
    {
        assert_eq!(Menu::RouteTarget.items()[menu_index], target.name);
        view.select_menu(
            &mut EventContext::new_with_current(&mut cx, entity),
            Menu::RouteTarget,
            menu_index,
        );
        BackendContext::new_with_event_manager(&mut cx).process_events();
        let p = &view.params.routes[1].target;
        assert!(changes
            .borrow()
            .contains(&(p.as_ptr(), p.preview_normalized(saved_index as i32))));
    }
    assert!(view
        .route_targets()
        .iter()
        .all(|(i, _)| crate::engine::routing::TARGETS[*i].available()));
}
#[test]
fn leading_animation_restarts_when_a_new_transition_arrives() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    view.bridge
        .snapshots
        .push(Snapshot {
            leading_serial: 1,
            ..Snapshot::default()
        })
        .unwrap();
    view.last_frame = Instant::now() - Duration::from_millis(50);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.leading_progress < 0.2);
    for _ in 0..8 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.leading_progress, 1.0);
    view.bridge
        .snapshots
        .push(Snapshot {
            leading_serial: 2,
            ..Snapshot::default()
        })
        .unwrap();
    view.last_frame = Instant::now() - Duration::from_millis(50);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.leading_progress < 0.2);
}
