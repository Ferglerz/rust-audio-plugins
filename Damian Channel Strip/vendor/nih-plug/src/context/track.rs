//! Optional track metadata supplied by the host on its control thread.

use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareInput {
    /// Zero-based first channel of the track's physical audio input.
    pub first_channel: usize,
    /// Display label for the configured input or input range.
    pub label: String,
}

pub enum HostTrackUpdate {
    Name(Option<String>),
    HardwareInput(Option<HardwareInput>),
    /// The host is preparing this instance to start processing.
    Prepare,
}

/// Receives owned values outside audio processing. The handler may outlive the
/// plugin object, so it must use shared state instead of borrowing the plugin.
pub type HostTrackHandler = Arc<dyn Fn(HostTrackUpdate) + Send + Sync>;
