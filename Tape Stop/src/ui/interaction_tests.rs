use super::{controls::*, *};
use nih_plug_vizia::{vizia::backend::BackendContext, widgets::RawParamEvent};
use std::{cell::RefCell, rc::Rc};

fn view() -> TapeStopView {
    TapeStopView {
        params: Arc::new(TapeStopParams::default()),
        telemetry: TapeStopTelemetry::new(),
        font: Cell::new(None),
        drag: None,
        edit: None,
        value_press: None,
        hover_stop: Cell::new(false),
        stop_click: Cell::new(0.0),
        hover: None,
        curve_hover_anim: Cell::new(0.0),
        auto_restart_anim: ButtonAnim::new(),
        show_axis_controls: false,
        axis_controls_progress: 0.0,
        gesture_params: [None; 2],
        pad_binding: None,
    }
}

type Changes = Rc<RefCell<Vec<(ParamPtr, u8, f32)>>>;
fn context() -> (Context, Entity, Changes) {
    let mut cx = Context::default();
    let target = Element::new(&mut cx).entity();
    BackendContext::new(&mut cx)
        .cache()
        .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, UI_W, UI_H));
    let changes: Changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    cx.add_global_listener(move |_, event| {
        event.map(|event: &RawParamEvent, _| {
            let entry = match event {
                RawParamEvent::BeginSetParameter(p) => (*p, 0, 0.0),
                RawParamEvent::SetParameterNormalized(p, v) => (*p, 1, *v),
                RawParamEvent::EndSetParameter(p) => (*p, 2, 0.0),
                _ => return,
            };
            observed.borrow_mut().push(entry);
        });
    });
    (cx, target, changes)
}

fn event(
    view: &mut TapeStopView,
    cx: &mut Context,
    target: Entity,
    pointer: (f32, f32),
    event: WindowEvent,
) {
    let mut backend = BackendContext::new_with_event_manager(cx);
    backend.emit_origin(WindowEvent::MouseMove(pointer.0, pointer.1));
    backend.process_events();
    view.event(
        &mut EventContext::new_with_current(cx, target),
        &mut Event::new(event),
    );
    BackendContext::new_with_event_manager(cx).process_events();
}

#[test]
fn pad_drags_have_two_complete_gestures_on_release_escape_and_focus_loss() {
    for ending in [
        WindowEvent::MouseUp(MouseButton::Left),
        WindowEvent::KeyDown(Code::Escape, None),
        WindowEvent::FocusOut,
        WindowEvent::MouseDown(MouseButton::Right),
    ] {
        let mut view = view();
        let (mut cx, target, changes) = context();
        let pointer = (XY_PAD.0 + 60.0, XY_PAD.1 + 100.0);
        event(
            &mut view,
            &mut cx,
            target,
            pointer,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        event(
            &mut view,
            &mut cx,
            target,
            pointer,
            WindowEvent::MouseMove(pointer.0, pointer.1),
        );
        event(&mut view, &mut cx, target, pointer, ending);
        assert!(view.drag.is_none());
        assert!(view.gesture_params.iter().all(Option::is_none));
        for ptr in [
            view.params.drop_curve.as_ptr(),
            view.params.drop_time.as_ptr(),
        ] {
            let changes = changes.borrow();
            let phases: Vec<_> = changes.iter().filter(|e| e.0 == ptr).map(|e| e.1).collect();
            assert_eq!(phases.first(), Some(&0));
            assert_eq!(phases.last(), Some(&2));
            assert_eq!(phases.iter().filter(|&&p| p == 0).count(), 1);
            assert_eq!(phases.iter().filter(|&&p| p == 2).count(), 1);
        }
    }
}

#[test]
fn mapping_changes_finish_existing_pad_without_writing_time_values() {
    let mut view = view();
    let (mut cx, target, changes) = context();
    let pointer = (XY_PAD.0 + 60.0, XY_PAD.1 + 100.0);
    event(
        &mut view,
        &mut cx,
        target,
        pointer,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    let time = view.params.drop_time.value();
    let curve_ptr = view.params.drop_curve.as_ptr();
    Arc::get_mut(&mut view.params).unwrap().xy_x_axis = EnumParam::new("X axis", XAxis::Return);
    changes.borrow_mut().clear();
    view.event(
        &mut EventContext::new_with_current(&mut cx, target),
        &mut Event::new(PerformanceTick),
    );
    BackendContext::new_with_event_manager(&mut cx).process_events();
    assert!(view.drag.is_none());
    assert_eq!(view.params.drop_time.value(), time);
    assert!(changes.borrow().iter().all(|e| e.1 == 2));
    assert!(changes.borrow().iter().any(|e| e.0 == curve_ptr));
}

#[test]
fn return_is_always_editable_and_synced_pad_targets_divisions() {
    let mut view = view();
    assert_eq!(
        view.readout_at(RETURN_TIME_READOUT.0 + 5.0, RETURN_TIME_READOUT.1 + 5.0),
        Some(KnobId::Return)
    );
    assert!(!view.show_axis_controls);
    let params = Arc::get_mut(&mut view.params).unwrap();
    params.time_sync = BoolParam::new("Sync", true);
    params.xy_x_axis = EnumParam::new("X", XAxis::Return);
    let (mut cx, target, changes) = context();
    let pointer = (XY_PAD.0 + 70.0, XY_PAD.1 + 100.0);
    event(
        &mut view,
        &mut cx,
        target,
        pointer,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    let changes = changes.borrow();
    assert!(changes
        .iter()
        .any(|e| e.0 == view.params.drop_division.as_ptr() && e.1 == 1));
    assert!(changes
        .iter()
        .any(|e| e.0 == view.params.return_division.as_ptr() && e.1 == 1));
    assert!(changes
        .iter()
        .all(|e| e.0 != view.params.drop_time.as_ptr() && e.0 != view.params.return_sec.as_ptr()));
}

#[test]
fn relative_pad_and_external_parameter_changes_use_current_values() {
    let mut view = view();
    Arc::get_mut(&mut view.params).unwrap().xy_mode = EnumParam::new("Mode", PadMode::Relative);
    let (mut cx, target, changes) = context();
    let current = view.pad_values();
    let pointer = (XY_PAD.0, XY_PAD.1);
    event(
        &mut view,
        &mut cx,
        target,
        pointer,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|e| e.0 == view.params.drop_time.as_ptr() && e.1 == 1 && e.2 == current.1));
    event(
        &mut view,
        &mut cx,
        target,
        pointer,
        WindowEvent::MouseUp(MouseButton::Left),
    );
    let params = Arc::get_mut(&mut view.params).unwrap();
    params.drop_curve = FloatParam::new("Curve", 3.0, FloatRange::Linear { min: 0.1, max: 4.0 });
    assert!((view.pad_values().0 - (3.0 - 0.1) / 3.9).abs() < 1e-6);
}

#[test]
fn typed_and_dragged_times_do_not_snap() {
    let mut view = view();
    let (mut cx, target, changes) = context();
    for id in [KnobId::DropTime, KnobId::Return] {
        view.edit = Some(ValueEdit::new(id, DROP_TIME_READOUT, "137.4ms".to_owned()));
        view.commit_edit(&mut EventContext::new_with_current(&mut cx, target));
        BackendContext::new_with_event_manager(&mut cx).process_events();
        let normalized = changes.borrow().iter().rev().find(|e| e.1 == 1).unwrap().2;
        assert!((view.param(id).preview_plain(normalized) - 0.1374).abs() < 1e-6);
        assert_eq!(view.param(id).step_count(), None);
    }
}
