use super::super::*;
use super::support::*;

fn open(view: &mut ChordboardView, cx: &mut Context, target: Entity) {
    click(view, cx, target, midi_learn::switch_rect());
    view.tick_quick_midi(
        &mut EventContext::new_with_current(cx, target),
        pleasant_ui::page_slide::DURATION,
    );
}

#[test]
fn midi_learn_slide_blocks_hidden_modulators_and_supports_cancel_and_clear() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, midi_learn::switch_rect());
    assert!(view.quick_midi.show);
    click(&mut view, &mut cx, target, meter_rect(0));
    assert!(view.drag.is_none());
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
    view.tick_quick_midi(
        &mut EventContext::new_with_current(&mut cx, target),
        pleasant_ui::page_slide::DURATION,
    );
    for i in 0..crate::midi_learn::COUNT {
        let r = midi_learn::chip_rect(i);
        assert!(r.1 >= MOD_SURFACE.1 + MODULE_HEADER_H);
        assert!(r.1 + r.3 <= MOD_SURFACE.1 + MOD_SURFACE.3);
        click(&mut view, &mut cx, target, r);
        assert_eq!(
            view.bridge.quick_learn.load(Ordering::Acquire),
            i as u32 + 1
        );
        click(&mut view, &mut cx, target, r);
        assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
    }
    assert!(changes.borrow().is_empty());
    view.params.quick_midi[0].mapping.store(
        crate::engine::Mapping::cc(21, 9).encode(),
        Ordering::Release,
    );
    click(&mut view, &mut cx, target, midi_learn::mode_rect(0));
    assert!(view.params.quick_midi[0].step_cc.load(Ordering::Relaxed));
    click(&mut view, &mut cx, target, midi_learn::clear_rect(0));
    assert_eq!(view.params.quick_midi[0].mapping.load(Ordering::Acquire), 0);
    click(&mut view, &mut cx, target, midi_learn::chip_rect(1));
    event(
        &mut view,
        &mut cx,
        target,
        1.0,
        1.0,
        WindowEvent::KeyDown(Code::Escape, None),
    );
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
    assert!(view.quick_midi.show);
    click(&mut view, &mut cx, target, midi_learn::chip_rect(1));
    event(&mut view, &mut cx, target, 1.0, 1.0, WindowEvent::FocusOut);
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
}

#[test]
fn midi_learn_works_when_strumfield_is_expanded_and_return_restores_routes() {
    let mut view = view(1, false);
    let (mut cx, target, _) = context();
    view.expand_progress = 1.0;
    open(&mut view, &mut cx, target);
    click(&mut view, &mut cx, target, midi_learn::chip_rect(7));
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 8);
    click(&mut view, &mut cx, target, midi_learn::switch_rect());
    assert!(!view.quick_midi.show);
    assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
    view.tick_quick_midi(
        &mut EventContext::new_with_current(&mut cx, target),
        pleasant_ui::page_slide::DURATION,
    );
    view.expand_progress = 0.0;
    click(&mut view, &mut cx, target, meter_rect(0));
    assert_eq!(view.selected_modulator, Some(0));
}

#[test]
fn midi_learn_values_reach_host_controls_on_the_gui_thread() {
    let mut view = view(1, false);
    let (mut cx, target, changes) = context();
    let mut input = crate::midi_learn::Input::default();
    view.bridge.quick_learn.store(4, Ordering::Release);
    let cc = |value| NoteEvent::MidiCC {
        timing: 0,
        channel: 9,
        cc: 21,
        value,
    };
    assert!(input.consume(cc(0.0), &view.params, &view.bridge.quick_learn, [0; 2]));
    assert!(input.consume(cc(1.0), &view.params, &view.bridge.quick_learn, [0; 2]));
    assert_eq!(view.params.config().transpose, 24);
    view.tick_quick_midi(&mut EventContext::new_with_current(&mut cx, target), 0.016);
    nih_plug_vizia::vizia::backend::BackendContext::new_with_event_manager(&mut cx)
        .process_events();
    assert!(changes
        .borrow()
        .contains(&(view.params.transpose.as_ptr(), 1.0)));
}
