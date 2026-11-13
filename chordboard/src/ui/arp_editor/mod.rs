//! String-based arp adjustments and the independently powered MIDI delay.
//! All pointer geometry is shared with drawing; edits publish only from the UI thread.
use super::midi_fx::DELAY_SURFACE;
use super::*;
use crate::arpeggiator::{Fill, Step, StepState, LOOPS, STEPS};
use crate::midi_fx::{DelaySettings, DelayTap, TAPS};

pub(super) mod delay_editor;
mod drag;
mod draw;
mod lanes;
mod layout;
#[cfg(test)]
mod tests;

use lanes::{AdjustLane, Chooser, Drag, DragKind, Dropdown, Lane, TapLane, LANES, TAP_LANES};
pub(in crate::ui) use delay_editor::*;
pub(in crate::ui) use lanes::ArpUi;
pub(in crate::ui) use layout::*;
