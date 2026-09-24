//! Live display state — meters (atomic) + histogram / trail buffers.

use std::sync::atomic::Ordering;
use std::sync::RwLock;
use std::time::Instant;

use atomic_float::AtomicF32 as AtomicF32Crate;
use crossbeam_queue::ArrayQueue;

/// Ring buffer size: `HISTOGRAM_WINDOW_SECONDS * 60` from JSFX.
pub const HISTOGRAM_LEN: usize = 180;

/// ~2.5s of meter trail lines.
pub const TRAIL_MAX_AGE_SECS: f32 = 2.5;
pub const TRAIL_MAX_LINES: usize = 96;

pub const DOT_TRAIL_MAX: usize = 48;
pub const DOT_TRAIL_MAX_AGE_SECS: f32 = 2.0;
pub const DOT_TRAIL_FADE_PEAK: f32 = 0.6;
pub const METER_TRAIL_FADE_PEAK: f32 = 0.75;

/// Fade alpha for trail/history dots and lines.
#[inline]
pub fn fade_by_age(age: f32, max_age: f32, peak: f32, quadratic: bool) -> f32 {
    if age >= max_age {
        return 0.0;
    }
    let t = 1.0 - age / max_age;
    if quadratic {
        (t * t).min(peak)
    } else {
        t * peak
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DotTrail {
    pub input_db: f32,
    pub age_secs: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TrailLine {
    pub gr_db: f32,
    pub age_secs: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct RingSlot<T> {
    payload: T,
    age_secs: f32,
    active: bool,
}

struct RingBuffer<T, const N: usize> {
    data: [RingSlot<T>; N],
    len: usize,
    tail: usize,
}

impl<T: Default + Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self {
            data: [RingSlot::default(); N],
            len: 0,
            tail: 0,
        }
    }
}

impl<T: Default + Copy, const N: usize> RingBuffer<T, N> {
    fn push(&mut self, payload: T) {
        let slot = RingSlot {
            payload,
            age_secs: 0.0,
            active: true,
        };
        if self.len < N {
            let idx = (self.tail + self.len) % N;
            self.data[idx] = slot;
            self.len += 1;
        } else {
            self.data[self.tail] = slot;
            self.tail = (self.tail + 1) % N;
        }
    }

    fn age_time(&mut self, dt_secs: f32, max_age: f32) {
        let mut kept = 0usize;
        let mut new_tail = self.tail;
        for i in 0..self.len {
            let idx = (self.tail + i) % N;
            if !self.data[idx].active {
                continue;
            }
            self.data[idx].age_secs += dt_secs;
            if self.data[idx].age_secs < max_age {
                if kept == 0 {
                    new_tail = idx;
                } else {
                    let dst = (new_tail + kept) % N;
                    self.data[dst] = self.data[idx];
                }
                kept += 1;
            }
        }
        self.tail = new_tail;
        self.len = kept;
    }

    fn collect_active<R>(&self, out: &mut Vec<R>, map_fn: impl Fn(T, f32) -> R) {
        out.clear();
        for i in 0..self.len {
            let idx = (self.tail + i) % N;
            let slot = &self.data[idx];
            if slot.active {
                out.push(map_fn(slot.payload, slot.age_secs));
            }
        }
    }
}

struct ReadoutInner {
    graph: String,
    param: String,
}

#[derive(Debug)]
pub struct HistogramBuffer {
    data: [f32; HISTOGRAM_LEN],
    pos: usize,
    len: usize,
}

impl Default for HistogramBuffer {
    fn default() -> Self {
        Self {
            data: [0.0; HISTOGRAM_LEN],
            pos: 0,
            len: 0,
        }
    }
}

impl HistogramBuffer {
    pub fn push(&mut self, value: f32) {
        self.data[self.pos] = value;
        self.pos = (self.pos + 1) % HISTOGRAM_LEN;
        self.len = (self.len + 1).min(HISTOGRAM_LEN);
    }

    pub fn sample_at_age(&self, age: usize) -> f32 {
        if age >= self.len {
            return f32::NEG_INFINITY;
        }
        let idx = (self.pos + HISTOGRAM_LEN - 1 - age) % HISTOGRAM_LEN;
        self.data[idx]
    }
}

#[cfg(test)]
mod histogram_tests {
    use super::HistogramBuffer;

    #[test]
    fn empty_history_is_not_drawable_audio() {
        let mut history = HistogramBuffer::default();
        assert_eq!(history.sample_at_age(0), f32::NEG_INFINITY);
        history.push(-18.0);
        assert_eq!(history.sample_at_age(0), -18.0);
        assert_eq!(history.sample_at_age(1), f32::NEG_INFINITY);
    }
}

struct BlockVisualInner {
    gr_hist: HistogramBuffer,
    input_hist: HistogramBuffer,
    trails: RingBuffer<f32, TRAIL_MAX_LINES>,
    dots: RingBuffer<f32, DOT_TRAIL_MAX>,
    last_update: Instant,
}

/// Shared between audio thread and editor.
pub struct UiDisplay {
    pub sample_rate: AtomicF32Crate,
    pub detector_db: AtomicF32Crate,
    pub gr_db: AtomicF32Crate,
    /// Interior graph point count for overlay label.
    pub graph_interior_points: std::sync::atomic::AtomicU32,
    /// Bumped when graph point count changes so labels refresh.
    pub graph_points_version: std::sync::atomic::AtomicU32,
    pub graph_hint_visible: std::sync::atomic::AtomicBool,
    /// JSFX `menu_debug_enabled` — toggled by invisible 30×30 top-left hit area.
    pub debug_enabled: std::sync::atomic::AtomicBool,
    filter_preview_drag: std::sync::atomic::AtomicBool,
    readout_version: std::sync::atomic::AtomicU32,
    debug_version: std::sync::atomic::AtomicU32,
    debug_lut_threshold: AtomicF32Crate,
    debug_target_gr: AtomicF32Crate,
    debug_latency: std::sync::atomic::AtomicU32,
    block_visuals_queue: ArrayQueue<(f32, f32)>,
    block_visuals: RwLock<BlockVisualInner>,
    readout: RwLock<ReadoutInner>,
    trail_scratch: RwLock<Vec<TrailLine>>,
    dot_scratch: RwLock<Vec<DotTrail>>,
}

impl Default for UiDisplay {
    fn default() -> Self {
        Self {
            sample_rate: AtomicF32Crate::new(48000.0),
            detector_db: AtomicF32Crate::new(0.0),
            gr_db: AtomicF32Crate::new(0.0),
            graph_interior_points: std::sync::atomic::AtomicU32::new(4),
            graph_points_version: std::sync::atomic::AtomicU32::new(0),
            graph_hint_visible: std::sync::atomic::AtomicBool::new(false),
            debug_enabled: std::sync::atomic::AtomicBool::new(false),
            filter_preview_drag: std::sync::atomic::AtomicBool::new(false),
            debug_version: std::sync::atomic::AtomicU32::new(0),
            debug_lut_threshold: AtomicF32Crate::new(1000.0),
            debug_target_gr: AtomicF32Crate::new(0.0),
            debug_latency: std::sync::atomic::AtomicU32::new(0),
            block_visuals_queue: ArrayQueue::new(2048),
            block_visuals: RwLock::new(BlockVisualInner {
                gr_hist: HistogramBuffer::default(),
                input_hist: HistogramBuffer::default(),
                trails: RingBuffer::default(),
                dots: RingBuffer::default(),
                last_update: Instant::now(),
            }),
            readout_version: std::sync::atomic::AtomicU32::new(0),
            readout: RwLock::new(ReadoutInner {
                graph: String::new(),
                param: String::new(),
            }),
            trail_scratch: RwLock::new(Vec::with_capacity(TRAIL_MAX_LINES)),
            dot_scratch: RwLock::new(Vec::with_capacity(DOT_TRAIL_MAX)),
        }
    }
}

impl UiDisplay {
    pub fn set_graph_hint(&self, visible: bool) {
        self.graph_hint_visible.store(visible, Ordering::Relaxed);
    }

    pub fn toggle_debug(&self) {
        let on = !self.debug_enabled.load(Ordering::Relaxed);
        self.debug_enabled.store(on, Ordering::Relaxed);
        self.debug_version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn is_debug_enabled(&self) -> bool {
        self.debug_enabled.load(Ordering::Relaxed)
    }

    pub fn debug_revision(&self) -> u32 {
        self.debug_version.load(Ordering::Relaxed)
    }

    pub fn update_debug_telemetry(
        &self,
        _detector_db: f32,
        _gr_db: f32,
        target_gr_db: f32,
        lut_threshold_db: f32,
        latency_samples: u32,
    ) {
        self.debug_target_gr.store(target_gr_db, Ordering::Relaxed);
        self.debug_lut_threshold
            .store(lut_threshold_db, Ordering::Relaxed);
        self.debug_latency.store(latency_samples, Ordering::Relaxed);
        self.debug_version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn debug_overlay_text(&self, detector_db: f32, gr_db: f32) -> String {
        if !self.is_debug_enabled() {
            return String::new();
        }
        let _v = self.debug_version.load(Ordering::Relaxed);
        let target = self.debug_target_gr.load(Ordering::Relaxed);
        let lut_thr = self.debug_lut_threshold.load(Ordering::Relaxed);
        let latency = self.debug_latency.load(Ordering::Relaxed);
        format!(
            "Runtime\n\
             det {detector_db:.2} dB\n\
             gr {gr_db:.2} dB\n\
             target {target:.2} dB\n\
             lut thr {lut_thr:.1} dB\n\
             latency {latency} smp"
        )
    }

    pub fn set_filter_preview_drag(&self, active: bool) {
        self.filter_preview_drag.store(active, Ordering::Relaxed);
    }

    pub fn filter_preview_active(&self) -> bool {
        self.filter_preview_drag.load(Ordering::Relaxed)
    }

    pub fn sync_graph_points(&self, num_points: usize) {
        self.graph_interior_points
            .store(num_points.saturating_sub(2) as u32, Ordering::Relaxed);
        self.graph_points_version.fetch_add(1, Ordering::Relaxed);
    }

    fn bump_readout(&self) {
        self.readout_version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_graph_readout(&self, text: String) {
        if let Ok(mut r) = self.readout.write() {
            r.graph = text;
        }
        self.bump_readout();
    }

    pub fn clear_graph_readout(&self) {
        if let Ok(mut r) = self.readout.write() {
            r.graph.clear();
        }
        self.bump_readout();
    }

    pub fn set_param_readout(&self, text: String) {
        if let Ok(mut r) = self.readout.write() {
            r.param = text;
        }
        self.bump_readout();
    }

    pub fn clear_param_readout(&self) {
        if let Ok(mut r) = self.readout.write() {
            r.param.clear();
        }
        self.bump_readout();
    }

    pub fn active_readout(&self) -> String {
        let _v = self.readout_version.load(Ordering::Relaxed);
        if let Ok(r) = self.readout.read() {
            if !r.param.is_empty() {
                r.param.clone()
            } else {
                r.graph.clone()
            }
        } else {
            String::new()
        }
    }

    pub fn drain_queue(&self) {
        if let Ok(mut v) = self.block_visuals.write() {
            while let Some((detector_db, gr_db)) = self.block_visuals_queue.pop() {
                v.gr_hist.push(gr_db);
                v.input_hist.push(detector_db);
                v.dots.push(detector_db);
                if gr_db.abs() > 0.01 {
                    v.trails.push(gr_db);
                }
            }
        }
    }

    pub fn age_dot_trails_one_frame(&self) {
        self.drain_queue();
        if let Ok(mut v) = self.block_visuals.write() {
            let now = Instant::now();
            let dt = now.duration_since(v.last_update).as_secs_f32();
            v.last_update = now;
            v.dots.age_time(dt, DOT_TRAIL_MAX_AGE_SECS);
            v.trails.age_time(dt, TRAIL_MAX_AGE_SECS);
        }
    }

    pub fn read_dot_trails<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[DotTrail]) -> R,
    {
        if let (Ok(mut scratch), Ok(visuals)) =
            (self.dot_scratch.write(), self.block_visuals.read())
        {
            visuals
                .dots
                .collect_active(&mut scratch, |val, age| DotTrail {
                    input_db: val,
                    age_secs: age,
                });
            f(&scratch)
        } else {
            f(&[])
        }
    }

    pub fn update_block(&self, detector_db: f32, gr_db: f32) {
        self.detector_db.store(detector_db, Ordering::Relaxed);
        self.gr_db.store(gr_db, Ordering::Relaxed);
        let _ = self.block_visuals_queue.push((detector_db, gr_db));
    }

    pub fn read_trails<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[TrailLine]) -> R,
    {
        if let (Ok(mut scratch), Ok(visuals)) =
            (self.trail_scratch.write(), self.block_visuals.read())
        {
            visuals
                .trails
                .collect_active(&mut scratch, |val, age| TrailLine {
                    gr_db: val,
                    age_secs: age,
                });
            f(&scratch)
        } else {
            f(&[])
        }
    }

    pub fn read_histograms<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&HistogramBuffer, &HistogramBuffer) -> R,
    {
        if let Ok(v) = self.block_visuals.read() {
            f(&v.gr_hist, &v.input_hist)
        } else {
            let dummy_gr = HistogramBuffer::default();
            let dummy_input = HistogramBuffer::default();
            f(&dummy_gr, &dummy_input)
        }
    }
}
