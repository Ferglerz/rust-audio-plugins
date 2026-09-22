# NIH-plug: sliders dead when the DAW engine is disconnected

Copy this into the SCD-rust repo. The bug is in NIH-plug, not in plugin UI.

## Symptom

Host parameters (sliders, knobs, typed values) stop updating when the DAW playback engine is disconnected or not responding. Mouse events still fire. Node graphs / other state that is **not** a host parameter still move.

SCD will show this on every control that goes through `ParamSetter` / `RawParamEvent::SetParameterNormalized`.

## Cause

NIH-plug VST3 skips the local parameter write while `is_processing` is true, and waits for the host to echo `perform_edit` back in `process()`.

A healthy host:

1. `IAudioProcessor::setProcessing(true)` sets `is_processing`
2. GUI calls `begin_edit` / `perform_edit` / `end_edit`
3. Host sends the change back as a ParameterChange in `process()`
4. That callback writes the atomic; the UI reads `unmodulated_normalized_value()` and redraws

A disconnected engine often leaves `is_processing == true` (never calls `setProcessing(false)`) and never calls `process()`. Local write is skipped. Echo never arrives. The slider looks dead.

If `component_handler` is `None`, the old code skipped the entire arm, including the local write. Debug builds assert. Release builds silently no-op.

This is the same class as the REAPER-bypass FIXME already in that function, and as [nih-plug#110](https://github.com/robbert-vdh/nih-plug/issues/110).

Do **not** change plugin UI, hit-testing, or pleasant-ui. Graphs that still work are mutating plugin `Mutex` state directly and never go through `ParamSetter`.

Leave `IEditController::set_param_normalized` alone. That is the host-to-plugin automation path. `process()` should still apply those.

## VST3 fix (required)

File (vendored or git checkout of nih-plug):

`src/wrapper/vst3/context.rs` — `GuiContext::raw_set_parameter_normalized`

**Before** (upstream):

```rust
unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, normalized: f32) {
    match &*self.inner.component_handler.borrow() {
        Some(handler) => match self.inner.param_ptr_to_hash.get(&param) {
            Some(hash) => {
                // Only update locally if the host is not processing audio.
                if !self.inner.is_processing.load(Ordering::SeqCst) {
                    self.inner.set_normalized_value_by_hash(
                        *hash,
                        normalized,
                        self.inner
                            .current_buffer_config
                            .load()
                            .map(|c| c.sample_rate),
                    );
                }

                handler.perform_edit(*hash, normalized as f64);
            }
            None => nih_debug_assert_failure!("Unknown parameter: {:?}", param),
        },
        None => nih_debug_assert_failure!("Component handler not yet set"),
    }
    // ... debug gesture checker unchanged ...
}
```

**After:**

```rust
unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, normalized: f32) {
    match self.inner.param_ptr_to_hash.get(&param) {
        Some(hash) => {
            // Always apply locally. Params are atomics, so a GUI write during
            // process is sound. A live host that echoes perform_edit into
            // process() will set the same value again. Skipping the local
            // write while is_processing is true leaves sliders dead when the
            // engine disconnects without setProcessing(false).
            self.inner.set_normalized_value_by_hash(
                *hash,
                normalized,
                self.inner
                    .current_buffer_config
                    .load()
                    .map(|c| c.sample_rate),
            );

            match &*self.inner.component_handler.borrow() {
                Some(handler) => {
                    handler.perform_edit(*hash, normalized as f64);
                }
                None => nih_debug_assert_failure!("Component handler not yet set"),
            }
        }
        None => nih_debug_assert_failure!("Unknown parameter: {:?}", param),
    }
    // ... debug gesture checker unchanged ...
}
```

If `Ordering` is unused after the change, drop `use std::sync::atomic::Ordering;`.

Still call `perform_edit` when a handler exists so a live host can record automation. Host echo of the same atomic value is idempotent.

## CLAP fix (same class, do this too if SCD ships CLAP)

File: `src/wrapper/clap/wrapper.rs` — `Wrapper::queue_parameter_event`

Upstream only applies the plain value when the host later flushes or `process()` writes output events. A dead engine that ignores `request_flush` stalls the same way.

Apply `SetValue` locally when queueing:

```rust
pub fn queue_parameter_event(&self, event: OutputParamEvent) -> bool {
    if let OutputParamEvent::SetValue {
        param_hash,
        clap_plain_value,
    } = &event
    {
        let sample_rate = self.current_buffer_config.load().map(|c| c.sample_rate);
        self.update_plain_value_by_hash(
            *param_hash,
            ClapParamUpdate::PlainValueSet(*clap_plain_value),
            sample_rate,
        );
    }

    let result = self.output_parameter_events.push(event).is_ok();
    // existing request_flush stays as-is
    result
}
```

Update the comment in `src/wrapper/clap/context.rs` `raw_set_parameter_normalized` if it still says the value only changes when the output event is written.

Leaving the process-time apply in place is fine. `set_normalized_value` returns false when the value did not change.

## How to land it in SCD

SCD likely depends on nih-plug via git or a vendor copy.

- **Vendored nih-plug:** edit the two files above in place.
- **Git dependency:** path-patch to a local checkout that contains the change, for example:

```toml
[patch."https://github.com/robbert-vdh/nih-plug.git"]
nih_plug = { path = "vendor/nih-plug" }
nih_plug_vizia = { path = "vendor/nih-plug/nih_plug_vizia" }
```

If SCD is a Cargo workspace, put `[patch]` on the workspace root. Member-level patches are ignored.

## Verify

1. Disconnect / stop the DAW playback engine so `process()` is not running.
2. Drag a host slider. Value and fill must move immediately.
3. Reconnect the engine. Slider automation / `perform_edit` must still reach the host.
