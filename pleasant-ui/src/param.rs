//! Host parameter events shared by custom canvas controls.

use nih_plug::prelude::ParamPtr;
use nih_plug_vizia::{
    vizia::prelude::{EmitContext, EventContext},
    widgets::RawParamEvent,
};

pub fn begin(cx: &mut EventContext, ptr: ParamPtr) {
    cx.emit(RawParamEvent::BeginSetParameter(ptr));
}

pub fn set_normalized(cx: &mut EventContext, ptr: ParamPtr, normalized: f32) {
    cx.emit(RawParamEvent::SetParameterNormalized(
        ptr,
        normalized.clamp(0.0, 1.0),
    ));
}

pub fn end(cx: &mut EventContext, ptr: ParamPtr) {
    cx.emit(RawParamEvent::EndSetParameter(ptr));
}

/// Send a complete gesture for a discrete adjustment, such as a wheel tick.
pub fn set_normalized_once(cx: &mut EventContext, ptr: ParamPtr, normalized: f32) {
    begin(cx, ptr);
    set_normalized(cx, ptr, normalized);
    end(cx, ptr);
}

/// Finish and clear every parameter tracked by a multi-parameter gesture.
pub fn end_all(cx: &mut EventContext, parameters: &mut [Option<ParamPtr>]) {
    for ptr in parameters.iter_mut().filter_map(Option::take) {
        end(cx, ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::prelude::{FloatParam, FloatRange, Param};
    use nih_plug_vizia::vizia::{backend::BackendContext, prelude::*};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn discrete_and_live_adjustments_preserve_gesture_order_and_clamp_values() {
        let param = FloatParam::new("Test", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 });
        let ptr = param.as_ptr();
        let mut cx = Context::default();
        let target = Element::new(&mut cx).entity();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        cx.add_global_listener(move |_, event| {
            event.map(|event: &RawParamEvent, _| {
                let phase = match event {
                    RawParamEvent::BeginSetParameter(ptr) => Some((*ptr, 0, 0.0)),
                    RawParamEvent::SetParameterNormalized(ptr, value) => Some((*ptr, 1, *value)),
                    RawParamEvent::EndSetParameter(ptr) => Some((*ptr, 2, 0.0)),
                    _ => None,
                };
                if let Some(phase) = phase {
                    observed.borrow_mut().push(phase);
                }
            });
        });
        {
            let mut event_cx = EventContext::new_with_current(&mut cx, target);
            set_normalized_once(&mut event_cx, ptr, 2.0);
            begin(&mut event_cx, ptr);
            set_normalized(&mut event_cx, ptr, -1.0);
            set_normalized(&mut event_cx, ptr, 0.25);
            let mut tracked = [None, Some(ptr)];
            end_all(&mut event_cx, &mut tracked);
            end_all(&mut event_cx, &mut tracked);
            assert!(tracked.iter().all(Option::is_none));
        }
        BackendContext::new_with_event_manager(&mut cx).process_events();
        assert_eq!(
            &*events.borrow(),
            &[
                (ptr, 0, 0.0),
                (ptr, 1, 1.0),
                (ptr, 2, 0.0),
                (ptr, 0, 0.0),
                (ptr, 1, 0.0),
                (ptr, 1, 0.25),
                (ptr, 2, 0.0)
            ]
        );
    }
}
