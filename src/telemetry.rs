use atomic_float::AtomicF32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct TapeStopTelemetry {
    pub speed_l: AtomicF32,
    pub speed_r: AtomicF32,
    pub is_braking: AtomicBool,
    pub is_crossfading: AtomicBool,
    pub brake_progress: AtomicF32,
    pub input_peak: AtomicF32,
    pub output_peak: AtomicF32,
    pub live_envelope: AtomicF32,
    pub transient_flash: AtomicF32,
    pub manual_trigger: AtomicBool,
}

impl TapeStopTelemetry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            speed_l: AtomicF32::new(1.0),
            speed_r: AtomicF32::new(1.0),
            is_braking: AtomicBool::new(false),
            is_crossfading: AtomicBool::new(false),
            brake_progress: AtomicF32::new(0.0),
            input_peak: AtomicF32::new(0.0),
            output_peak: AtomicF32::new(0.0),
            live_envelope: AtomicF32::new(0.0),
            transient_flash: AtomicF32::new(0.0),
            manual_trigger: AtomicBool::new(false),
        })
    }

    #[inline(always)]
    pub fn update(
        &self,
        speed_l: f32,
        speed_r: f32,
        is_braking: bool,
        is_crossfading: bool,
        brake_progress: f32,
        in_peak: f32,
        out_peak: f32,
        live_envelope: f32,
        transient_flash: f32,
    ) {
        self.speed_l.store(speed_l, Ordering::Relaxed);
        self.speed_r.store(speed_r, Ordering::Relaxed);
        self.is_braking.store(is_braking, Ordering::Relaxed);
        self.is_crossfading.store(is_crossfading, Ordering::Relaxed);
        self.brake_progress.store(brake_progress, Ordering::Relaxed);
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
