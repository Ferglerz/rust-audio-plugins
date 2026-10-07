use super::super::*;
use nih_plug_vizia::vizia::backend::BackendContext;
use std::{cell::RefCell, rc::Rc};

pub(super) type Changes = Rc<RefCell<Vec<(ParamPtr, f32)>>>;

pub(super) fn view(mode: i32, mpe: bool) -> ChordboardView {
    let view = ChordboardView::new(
        Arc::new(ChordboardParams {
            mode: IntParam::new(
                "Play mode",
                mode.min(3),
                IntRange::Linear { min: 0, max: 3 },
            ),
            output_mode: IntParam::new(
                "Output protocol",
                if mpe { 1 } else { 2 },
                IntRange::Linear { min: 0, max: 2 },
            ),
            ..ChordboardParams::default()
        }),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    view
}
/// The Arpeggiator lives on the Sequencer page; settle both slides there.
pub(super) fn open_arp(view: &mut ChordboardView) {
    view.sequencer_ui.open = true;
    view.sequencer_ui.progress = 1.0;
    view.page_target = 1;
    view.page_start = 1.0;
    view.page_position = 1.0;
    view.page_elapsed = pleasant_ui::page_slide::DURATION;
}
pub(super) fn context() -> (Context, Entity, Changes) {
    let mut cx = Context::default();
    let target = Element::new(&mut cx).entity();
    BackendContext::new(&mut cx)
        .cache()
        .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, W, H));
    let changes: Changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    cx.add_global_listener(move |_, event| {
        event.map(|event: &RawParamEvent, _| {
            if let RawParamEvent::SetParameterNormalized(ptr, value) = event {
                observed.borrow_mut().push((*ptr, *value));
            }
        });
    });
    (cx, target, changes)
}
pub(super) fn event(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    x: f32,
    y: f32,
    event: WindowEvent,
) {
    let mut backend = BackendContext::new_with_event_manager(cx);
    backend.emit_origin(WindowEvent::MouseMove(x, y));
    backend.process_events();
    view.window_event(&mut EventContext::new_with_current(cx, target), &event);
    BackendContext::new_with_event_manager(cx).process_events();
}
pub(super) fn click(view: &mut ChordboardView, cx: &mut Context, target: Entity, r: Rect) {
    // Ordinary interaction tests operate after the quarter-second page slide.
    view.route_elapsed = pleasant_ui::page_slide::DURATION;
    view.route_progress = if view.panel == Some(Panel::Routes) {
        1.0
    } else {
        0.0
    };
    let x = r.0 + r.2 / 2.0;
    let y = r.1 + r.3 / 2.0;
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    event(
        view,
        cx,
        target,
        x,
        y,
        WindowEvent::MouseUp(MouseButton::Left),
    );
}
pub(super) fn click_menu_trigger(view: &mut ChordboardView, cx: &mut Context, target: Entity, menu: Menu) {
    let r = view.menu_trigger_rect(menu);
    click(view, cx, target, r);
}
pub(super) fn click_menu_option(
    view: &mut ChordboardView,
    cx: &mut Context,
    target: Entity,
    menu: Menu,
    index: usize,
) {
    let r = view.menu_option_rect(menu, index);
    click(view, cx, target, r);
}
