// Each copy must resolve the private evaluator in its own generated solver.
#![allow(clippy::duplicate_mod)]

// Compile the self-contained generated modules under the wrapper test target.
// This exercises their private helpers without directly testing the vendor crate.
#[path = "../vendor/openwurli-dsp/src/gen_power_amp.rs"]
mod power_amp;
#[path = "../vendor/openwurli-dsp/src/gen_preamp.rs"]
mod preamp;
#[path = "../vendor/openwurli-dsp/src/gen_tremolo.rs"]
mod tremolo;
