use super::*;

mod modal_input {
    use super::*;
    use nih_plug_vizia::vizia::backend::BackendContext;
    use std::rc::Rc;

    fn editor(modal: usize) -> (Context, Entity, Rc<Cell<usize>>) {
        let mut cx = Context::default();
        let writes = Rc::new(Cell::new(0));
        let observed = writes.clone();
        cx.add_global_listener(move |_, event| {
            event.map(|event: &RawParamEvent, _| {
                if matches!(event, RawParamEvent::SetParameterNormalized(..)) {
                    observed.set(observed.get() + 1);
                }
            });
        });
        let target = ScdEditorView::new(&mut cx, Arc::new(ScdParams::default()))
            .modify(|view| match modal {
                0 => view.sub_kick_open = true,
                1 => view.vel_map_open = Some(KitPieceId::Tom1),
                _ => view.add_preset_open = true,
            })
            .entity();
        EventContext::new_with_current(&mut cx, target)
            .set_bounds(BoundingBox::from_min_max(0.0, 0.0, WINDOW_W, WINDOW_H));
        (cx, target, writes)
    }

    fn send(cx: &mut Context, target: Entity, x: f32, y: f32, event: WindowEvent) {
        let mut backend = BackendContext::new_with_event_manager(cx);
        backend.send_event(Event::new(WindowEvent::MouseMove(x, y)).origin(Entity::root()));
        backend.process_events();
        // Deliver to the view after updating the real cursor state, without
        // synthesizing a double-click when the test sends adjacent clicks.
        backend.send_event(Event::new(event).origin(target).direct(target));
        backend.process_events();
    }

    #[test]
    fn modal_clicks_do_not_reach_background_controls() {
        let (sx, sy, sw, sh) = ScdEditorView::sof_rect(0);
        let lock = ScdEditorView::lock_rect();
        for modal in 0..3 {
            for (x, y) in [
                (sx + sw * 0.5, sy + sh * 0.5),
                (lock.0 + lock.2 * 0.5, lock.1 + lock.3 * 0.5),
                (PRESET_X + 10.0, PRESET_Y + 10.0),
                (SAMPLES_X + 10.0, SAMPLES_Y + 10.0),
                (CC_INVERT.0 + 10.0, CC_INVERT.1 + 10.0),
                (CC_SELECT.0 + 10.0, CC_SELECT.1 + 10.0),
                (SUB_KICK.0 + 10.0, SUB_KICK.1 + 10.0),
            ] {
                let (mut cx, target, writes) = editor(modal);
                send(
                    &mut cx,
                    target,
                    x,
                    y,
                    WindowEvent::MouseDown(MouseButton::Left),
                );
                send(
                    &mut cx,
                    target,
                    x,
                    y,
                    WindowEvent::MouseUp(MouseButton::Left),
                );
                let event_cx = EventContext::new_with_current(&mut cx, target);
                let view = event_cx.get_view::<ScdEditorView>().unwrap();
                assert_eq!(writes.get(), 0, "modal {modal}, click at ({x}, {y})");
                assert!(
                    view.active_sof.is_none(),
                    "background mic selection changed"
                );
                assert!(!view.preset_open && !view.samples_open && !view.cc_menu_open);
                assert!(!view.sub_kick_open, "background Sub Kick button activated");
            }
        }
    }

    #[test]
    fn modal_blocks_cc_scroll_and_background_hover() {
        for modal in 0..3 {
            let (mut cx, target, writes) = editor(modal);
            send(
                &mut cx,
                target,
                CC_SELECT.0 + 10.0,
                CC_SELECT.1 + 10.0,
                WindowEvent::MouseScroll(0.0, 1.0),
            );
            assert_eq!(
                writes.get(),
                0,
                "modal {modal} allowed a background CC change"
            );
            let lock = ScdEditorView::lock_rect();
            send(
                &mut cx,
                target,
                lock.0 + lock.2 * 0.5,
                lock.1 + lock.3 * 0.5,
                WindowEvent::MouseMove(lock.0, lock.1),
            );
            let event_cx = EventContext::new_with_current(&mut cx, target);
            assert!(!event_cx.get_view::<ScdEditorView>().unwrap().hover_lock);
        }
    }

    #[test]
    fn sub_kick_blank_space_does_not_start_background_fader_drag() {
        let (mut cx, target, writes) = editor(0);
        let (mx, my, _, _) = ScdEditorView::sub_kick_modal();
        send(
            &mut cx,
            target,
            mx + 10.0,
            my + 110.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        let event_cx = EventContext::new_with_current(&mut cx, target);
        assert!(event_cx.get_view::<ScdEditorView>().unwrap().drag.is_none());
        assert_eq!(writes.get(), 0);
    }

    #[test]
    fn modal_controls_still_receive_clicks() {
        let (mut cx, target, _) = editor(0);
        let (mx, my, _, _) = ScdEditorView::sub_kick_modal();
        send(
            &mut cx,
            target,
            ScdEditorView::sub_kick_knob_x(mx, 0),
            my + 55.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        let event_cx = EventContext::new_with_current(&mut cx, target);
        assert_eq!(
            event_cx.get_view::<ScdEditorView>().unwrap().drag,
            Some(DragTarget::SubKickVol)
        );

        let (mut cx, target, _) = editor(1);
        let (x, y, _, _) = ScdEditorView::vel_art_dropdown_rect();
        send(
            &mut cx,
            target,
            x + 10.0,
            y + 10.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        let event_cx = EventContext::new_with_current(&mut cx, target);
        assert!(
            event_cx
                .get_view::<ScdEditorView>()
                .unwrap()
                .vel_art_menu_open
        );
    }
}

#[test]
fn mapping_dropdown_keeps_all_articulation_names() {
    for piece in KitPieceId::ALL {
        if piece == KitPieceId::Hihat {
            continue;
        }
        let options = ScdEditorView::vel_art_options(piece);
        assert_eq!(options[0], (ALL_ART, "ALL".into()));
        assert_eq!(options.len(), stonehouse().arts(piece).len() + 1);
        for ((index, label), art) in options[1..].iter().zip(stonehouse().arts(piece)) {
            assert_eq!(label, &art.name.replace('_', " "));
            assert_eq!(&stonehouse().arts(piece)[*index].name, &art.name);
        }
    }
}

#[test]
fn mapping_delete_is_near_node_and_perpendicular_to_tangent() {
    let mut curve = VelCurve::identity();
    let i = curve.insert_at(0.5).unwrap();
    let node = &curve.nodes[i];
    let (nx, ny) = ScdEditorView::norm_to_graph(node.x, node.y);
    let (ix, iy) = ScdEditorView::norm_to_graph(node.in_handle.x, node.in_handle.y);
    let (ox, oy) = ScdEditorView::norm_to_graph(node.out_handle.x, node.out_handle.y);
    let (dx, dy) = ScdEditorView::node_delete_pos(&curve, i);
    assert!(((dx - nx).hypot(dy - ny) - 20.0).abs() < 0.001);
    assert!(((dx - nx) * (ox - ix) + (dy - ny) * (oy - iy)).abs() < 0.02);
}

#[test]
fn mapping_midi_holds_then_fades_for_two_seconds() {
    assert_eq!(midi_hit_alpha(0.0), 1.0);
    assert_eq!(midi_hit_alpha(0.5), 1.0);
    assert_eq!(midi_hit_alpha(1.5), 0.5);
    assert_eq!(midi_hit_alpha(2.5), 0.0);
    assert_eq!(midi_hit_alpha(3.0), 0.0);
}

#[test]
fn note_digit_replaces_selection_and_caps_at_three() {
    let target = NoteTarget {
        kit_piece: KitPieceId::Kick,
        art: 0,
        note2: false,
    };
    let mut edit = ValueEdit::new(target, (0.0, 0.0, 36.0, 28.0), "36".into());
    assert!(insert_note_digit(&mut edit, '4'));
    assert_eq!(edit.text, "4");
    assert!(insert_note_digit(&mut edit, '2'));
    assert!(insert_note_digit(&mut edit, '0'));
    assert!(!insert_note_digit(&mut edit, '1'));
    assert_eq!(edit.text, "420");
    assert!(!insert_note_digit(&mut edit, 'a'));
}

#[test]
fn five_stage_covers_hise_filmstrip() {
    assert_eq!(five_stage_fill(false, false, false), None);
    assert_eq!(five_stage_fill(false, true, false), Some(STAGE_HOVER));
    assert_eq!(five_stage_fill(false, true, true), Some(STAGE_DOWN));
    assert_eq!(five_stage_fill(true, false, false), Some(STAGE_ON));
    assert_eq!(five_stage_fill(true, true, false), Some(STAGE_ON_HOVER));
    assert_eq!(five_stage_fill(true, false, true), Some(STAGE_DOWN));
}

#[test]
fn reset_hit_targets_follow_visible_controls() {
    let piece = KitPieceId::Kick;
    let x = ScdEditorView::strip_x(piece) + STRIP_W * 0.5;
    assert_eq!(
        ScdEditorView::reset_target_at(x, PITCH_Y + 5.0, false),
        Some(DragTarget::Pitch(piece))
    );
    assert_eq!(
        ScdEditorView::reset_target_at(x, PAN_Y + 5.0, false),
        Some(DragTarget::Pan(piece))
    );
    assert_eq!(
        ScdEditorView::reset_target_at(x, FADER_Y + 5.0, false),
        Some(DragTarget::Fader(piece))
    );
    assert_eq!(
        ScdEditorView::reset_target_at(x, PUNCH_Y + 5.0, false),
        Some(DragTarget::Punch(piece))
    );

    let (mx, my, _, _) = ScdEditorView::sub_kick_modal();
    for (i, target) in [
        DragTarget::SubKickVol,
        DragTarget::SubKickLength,
        DragTarget::SubKickDive,
        DragTarget::SubKickSpeed,
        DragTarget::SubKickOffset,
    ]
    .into_iter()
    .enumerate()
    {
        let knob_x = ScdEditorView::sub_kick_knob_x(mx, i);
        assert_eq!(
            ScdEditorView::reset_target_at(knob_x, my + 55.0, true),
            Some(target)
        );
    }
    assert_eq!(ScdEditorView::reset_target_at(x, PITCH_Y + 5.0, true), None);
}

#[test]
fn mapping_helper_sits_inside_menu_below_last_note() {
    let (_, last_y, _, last_h) = ScdEditorView::samples_item_rect(KitPieceId::COUNT - 1);
    let (hx, hy, hw, hh) = ScdEditorView::mapping_helper_rect();
    let (mx, my, mw, mh) = ScdEditorView::mapping_menu_rect();
    assert_eq!(hy, last_y + last_h);
    assert!(hx >= mx && hx + hw <= mx + mw);
    assert!(hy >= my && hy + hh <= my + mh);
}

#[test]
fn hihat_mapping_groups_route_to_all_underlying_articulations() {
    let options = ScdEditorView::vel_art_options(KitPieceId::Hihat);
    assert_eq!(
        options.iter().map(|(art, _)| *art).collect::<Vec<_>>(),
        [ALL_ART, 0, 1, 2, 7]
    );
    assert_eq!(
        ScdEditorView::vel_art_members(KitPieceId::Hihat, 2),
        vec![2, 3, 4, 5, 6]
    );
    assert_eq!(
        ScdEditorView::vel_art_members(KitPieceId::Hihat, 7),
        vec![7, 8, 9, 10, 11]
    );
    assert_eq!(
        ScdEditorView::vel_art_members(KitPieceId::Hihat, 1),
        vec![1]
    );
}

#[test]
fn hihat_mapping_group_edit_and_reset_fan_out() {
    let state = VelMapState::identity();
    let mut curve = VelCurve::identity();
    curve.move_node(0, 0.0, 1.0);
    ScdEditorView::set_vel_curve_group(&state, KitPieceId::Hihat, 2, curve);
    for art in 2..7 {
        assert_eq!(state.lookup(KitPieceId::Hihat, art, 64), 127);
    }
    assert_eq!(state.lookup(KitPieceId::Hihat, 1, 64), 64);
    ScdEditorView::reset_vel_curve_group(&state, KitPieceId::Hihat, 2);
    for art in 2..7 {
        assert_eq!(state.curve(KitPieceId::Hihat, art), VelCurve::identity());
    }
    assert_eq!(state.lookup(KitPieceId::Hihat, 1, 64), 64);
}
