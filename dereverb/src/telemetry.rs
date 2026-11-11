use atomic_float::AtomicF32;
use std::sync::{
    atomic::{AtomicU32, AtomicUsize, Ordering},
    Mutex,
};

pub const POINTS: usize = 160;
pub const INACTIVE: u32 = 0;
pub const ACTIVE: u32 = 1;
pub const RECOVERING: u32 = 2;
pub const FAILED: u32 = 3;

pub struct Telemetry {
    pub status: AtomicU32,
    pub sample_rate: AtomicF32,
    pub latency: AtomicU32,
    pub quality: AtomicU32,
    pub engine: AtomicU32,
    pub errors: Mutex<[String; crate::MODES]>, // prepare/UI only; never locked by processing or reset
    pub absolute_db: AtomicF32,
    pub relative_db: AtomicF32,
    pub weight: AtomicF32,
    pub effective: AtomicF32,
    pub active_level: AtomicU32,
    input: [AtomicF32; POINTS],
    output: [AtomicF32; POINTS],
    cursor: AtomicUsize,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            status: AtomicU32::new(INACTIVE),
            sample_rate: AtomicF32::new(0.0),
            latency: AtomicU32::new(0),
            quality: AtomicU32::new(0),
            engine: AtomicU32::new(0),
            errors: Mutex::default(),
            absolute_db: AtomicF32::new(-120.0),
            relative_db: AtomicF32::new(-36.0),
            weight: AtomicF32::new(1.0),
            effective: AtomicF32::new(0.0),
            active_level: AtomicU32::new(0),
            input: std::array::from_fn(|_| AtomicF32::new(0.0)),
            output: std::array::from_fn(|_| AtomicF32::new(0.0)),
            cursor: AtomicUsize::new(0),
        }
    }
}
impl Telemetry {
    pub fn clear(&self) {
        self.set_activity(crate::weighting::FrameLevel::default(), 1.0, 0.0);
        for value in self.input.iter().chain(&self.output) {
            value.store(0.0, Ordering::Relaxed);
        }
        self.cursor.store(0, Ordering::Release);
    }
    pub fn push(&self, input: f32, output: f32) {
        let slot = self.cursor.load(Ordering::Relaxed);
        self.input[slot].store(input, Ordering::Relaxed);
        self.output[slot].store(output, Ordering::Relaxed);
        self.cursor.store((slot + 1) % POINTS, Ordering::Release);
    }
    pub fn set_activity(&self, level: crate::weighting::FrameLevel, weight: f32, effective: f32) {
        self.absolute_db.store(level.absolute_db, Ordering::Relaxed);
        self.relative_db.store(level.relative_db, Ordering::Relaxed);
        self.weight.store(weight, Ordering::Relaxed);
        self.effective.store(effective, Ordering::Relaxed);
        self.active_level
            .store(u32::from(level.active), Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> ([f32; POINTS], [f32; POINTS]) {
        let first = self.cursor.load(Ordering::Acquire);
        (
            std::array::from_fn(|i| self.input[(first + i) % POINTS].load(Ordering::Relaxed)),
            std::array::from_fn(|i| self.output[(first + i) % POINTS].load(Ordering::Relaxed)),
        )
    }
}
