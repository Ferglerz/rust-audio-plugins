use atomic_float::AtomicF32;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering, Ordering::SeqCst};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum TransportStage {
    #[default]
    Idle,
    Live,
    Braking,
    Returning,
    Crossfading,
    Position,
}

impl TransportStage {
    fn from_byte(value: u8) -> Self {
        match value {
            1 => Self::Live,
            2 => Self::Braking,
            3 => Self::Returning,
            4 => Self::Crossfading,
            5 => Self::Position,
            _ => Self::Idle,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct TransportFrame {
    pub stage: TransportStage,
    pub speed: [f32; 2],
    pub brake_progress: [f32; 2],
}

pub struct TapeStopTelemetry {
    pub tempo: AtomicF32,
    pub effective_drop: AtomicF32,
    pub effective_return: AtomicF32,
    transport_sequence: AtomicU64,
    stage: AtomicU8,
    speed: [AtomicF32; 2],
    brake_progress: [AtomicF32; 2],
    pub input_peak: AtomicF32,
    pub output_peak: AtomicF32,
    pub live_envelope: AtomicF32,
    pub transient_flash: AtomicF32,
    pub manual_trigger: AtomicBool,
}

impl TapeStopTelemetry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tempo: AtomicF32::new(120.0),
            effective_drop: AtomicF32::new(0.5),
            effective_return: AtomicF32::new(0.5),
            transport_sequence: AtomicU64::new(0),
            stage: AtomicU8::new(TransportStage::Idle as u8),
            speed: std::array::from_fn(|_| AtomicF32::new(0.0)),
            brake_progress: std::array::from_fn(|_| AtomicF32::new(0.0)),
            input_peak: AtomicF32::new(0.0),
            output_peak: AtomicF32::new(0.0),
            live_envelope: AtomicF32::new(0.0),
            transient_flash: AtomicF32::new(0.0),
            manual_trigger: AtomicBool::new(false),
        })
    }

    pub(crate) fn publish_transport(&self, frame: TransportFrame) {
        self.transport_sequence.fetch_add(1, SeqCst);
        for channel in 0..2 {
            self.speed[channel].store(frame.speed[channel], SeqCst);
            self.brake_progress[channel].store(frame.brake_progress[channel], SeqCst);
        }
        self.stage.store(frame.stage as u8, SeqCst);
        self.transport_sequence.fetch_add(1, SeqCst);
    }

    pub(crate) fn transport(&self) -> Option<(u64, TransportFrame)> {
        let sequence = self.transport_sequence.load(SeqCst);
        if sequence & 1 != 0 {
            return None;
        }
        let frame = TransportFrame {
            stage: TransportStage::from_byte(self.stage.load(SeqCst)),
            speed: self.speed.each_ref().map(|speed| speed.load(SeqCst)),
            brake_progress: self
                .brake_progress
                .each_ref()
                .map(|progress| progress.load(SeqCst)),
        };
        (self.transport_sequence.load(SeqCst) == sequence).then_some((sequence, frame))
    }

    pub(crate) fn is_braking(&self) -> bool {
        self.stage.load(Ordering::Relaxed) == TransportStage::Braking as u8
    }

    pub fn update_levels(
        &self,
        in_peak: f32,
        out_peak: f32,
        live_envelope: f32,
        transient_flash: f32,
    ) {
        self.input_peak.store(in_peak, Ordering::Relaxed);
        self.output_peak.store(out_peak, Ordering::Relaxed);
        self.live_envelope.store(live_envelope, Ordering::Relaxed);
        self.transient_flash
            .store(transient_flash, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn get_manual_trigger(&self) -> bool {
        self.manual_trigger.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn set_manual_trigger(&self, active: bool) {
        self.manual_trigger.store(active, Ordering::Relaxed);
    }
}
