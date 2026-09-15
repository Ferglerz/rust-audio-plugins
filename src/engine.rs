use crate::{
    band::Band,
    dsp::{db_gain, gain_db, BandRuntime, CompSettings, VocalComp},
};
use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub struct Bank {
    pub bands: Vec<BandRuntime>,
    pub sr: f64,
}
pub struct Shared {
    bands: Arc<Mutex<Vec<Band>>>,
    pub input: AtomicF32,
    pub output: AtomicF32,
    pub gr: AtomicF32,
    pub sample_rate: AtomicF32,
    pub spectrum: [AtomicF32; 128],
    pub selected_id: std::sync::atomic::AtomicU64,
    pub band_gr: AtomicF32,
    pub pending: ArrayQueue<Box<Bank>>,
    pub retired: ArrayQueue<Box<Bank>>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl Shared {
    pub fn new(bands: Arc<Mutex<Vec<Band>>>) -> Arc<Self> {
        let shared = Arc::new(Self {
            bands: bands.clone(),
            input: AtomicF32::new(-90.0),
            output: AtomicF32::new(-90.0),
            gr: AtomicF32::new(0.0),
            sample_rate: AtomicF32::new(44100.0),
            spectrum: std::array::from_fn(|_| AtomicF32::new(-90.0)),
            selected_id: std::sync::atomic::AtomicU64::new(0),
            band_gr: AtomicF32::new(0.0),
            pending: ArrayQueue::new(1),
            retired: ArrayQueue::new(2),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        });
        let weak = Arc::downgrade(&shared);
        let stop = shared.stop.clone();
        let handle = std::thread::spawn(move || {
            let mut last: Option<(Vec<Band>, f64)> = None;
            while !stop.load(Ordering::Relaxed) {
                if let Some(s) = weak.upgrade() {
                    while s.retired.pop().is_some() {}
                    if !s.pending.is_full() {
                        let sr = s.sample_rate.load(Ordering::Relaxed) as f64;
                        let snapshot = bands.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if last
                            .as_ref()
                            .is_none_or(|(old, rate)| *old != snapshot || *rate != sr)
                        {
                            let bank = Box::new(Bank {
                                bands: snapshot
                                    .iter()
                                    .cloned()
                                    .map(|b| BandRuntime::new(b, sr))
                                    .collect(),
                                sr,
                            });
                            if s.pending.push(bank).is_ok() {
                                last = Some((snapshot, sr));
                            }
                        }
                    }
                } else {
                    break;
                }
                std::thread::sleep(Duration::from_millis(8));
            }
        });
        *shared.worker.lock().unwrap() = Some(handle);
        shared
    }
}
impl Drop for Shared {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker.get_mut().unwrap().take() {
            if handle.thread().id() != std::thread::current().id() {
                let _ = handle.join();
            }
        }
    }
}
pub struct Engine {
    shared: Arc<Shared>,
    bank: Box<Bank>,
    previous: Option<Box<Bank>>,
    transition: usize,
    sr: f64,
    comp: VocalComp,
    fft: Arc<dyn Fft<f32>>,
    samples: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: [f32; 2048],
    position: usize,
    in_peak: f64,
    out_peak: f64,
    eq_mix: f64,
    comp_mix: f64,
    bypass_mix: f64,
}
impl Engine {
    pub fn new(shared: Arc<Shared>, sr: f64) -> Self {
        shared.sample_rate.store(sr as f32, Ordering::Relaxed);
        let fft = FftPlanner::new().plan_fft_forward(2048);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let bank = Box::new(Bank {
            bands: shared
                .bands
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .cloned()
                .map(|b| BandRuntime::new(b, sr))
                .collect(),
            sr,
        });
        Self {
            shared,
            bank,
            previous: None,
            transition: 0,
            sr,
            comp: VocalComp::new(),
            fft,
            samples: vec![Complex::default(); 2048],
            scratch,
            window: std::array::from_fn(|i| {
                0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / 2048.0).cos()
            }),
            position: 0,
            in_peak: 0.0,
            out_peak: 0.0,
            eq_mix: 1.0,
            comp_mix: 1.0,
            bypass_mix: 0.0,
        }
    }
    pub fn reset(&mut self) {
        for b in &mut self.bank.bands {
            b.reset();
        }
        self.comp = VocalComp::new();
        self.position = 0;
        self.samples.fill(Complex::default());
        self.in_peak = 0.0;
        self.out_peak = 0.0;
    }
    pub fn sync(&mut self) {
        if self.transition == 0 && !self.shared.retired.is_full() {
            if let Some(old) = self.previous.take() {
                let result = self.shared.retired.push(old);
                debug_assert!(result.is_ok());
            }
        }
        if self.previous.is_none() && !self.shared.retired.is_full() {
            if let Some(mut next) = self.shared.pending.pop() {
                if next.sr == self.sr {
                    let mut j = 0;
                    for b in &mut next.bands {
                        while j < self.bank.bands.len() && self.bank.bands[j].band.id < b.band.id {
                            j += 1;
                        }
                        if let Some(old) = self.bank.bands.get(j) {
                            if old.band.id == b.band.id {
                                b.inherit(old);
                            }
                        }
                    }
                    std::mem::swap(&mut self.bank, &mut next);
                    self.previous = Some(next);
                    self.transition = (self.sr * 0.010) as usize;
                } else {
                    let result = self.shared.retired.push(next);
                    debug_assert!(result.is_ok());
                }
            }
        }
    }
    pub fn tick(
        &mut self,
        input: [f64; 2],
        settings: CompSettings,
        output: f64,
        eq_on: bool,
        comp_on: bool,
        bypass: bool,
    ) -> [f64; 2] {
        let k = 1.0 - (-1.0 / (self.sr * 0.010)).exp();
        self.eq_mix += k * (f64::from(eq_on) - self.eq_mix);
        self.comp_mix += k * (f64::from(comp_on) - self.comp_mix);
        self.bypass_mix += k * (f64::from(bypass) - self.bypass_mix);
        let mut x = input;
        let selected = self.shared.selected_id.load(Ordering::Relaxed);
        let mut band_gr = 0.0;
        for b in &mut self.bank.bands {
            x = b.tick(x, self.sr);
            if b.band.id == selected {
                band_gr = b.reduction;
            }
        }
        if self.transition > 0 {
            if let Some(previous) = &mut self.previous {
                let mut old = input;
                for b in &mut previous.bands {
                    old = b.tick(old, self.sr);
                }
                let wet = 1.0 - self.transition as f64 / (self.sr * 0.010).floor();
                for i in 0..2 {
                    x[i] = old[i] + wet * (x[i] - old[i]);
                }
            }
            self.transition -= 1;
        }
        for i in 0..2 {
            x[i] = input[i] + self.eq_mix * (x[i] - input[i]);
        }
        let (compressed, gr) = self.comp.tick(x, settings, self.sr);
        let gain = db_gain(output);
        for i in 0..2 {
            x[i] = (x[i] + self.comp_mix * (compressed[i] - x[i])) * gain;
            x[i] += self.bypass_mix * (input[i] - x[i]);
            if !x[i].is_finite() {
                x[i] = 0.0;
            }
        }
        self.in_peak = self.in_peak.max(input[0].abs()).max(input[1].abs());
        self.out_peak = self.out_peak.max(x[0].abs()).max(x[1].abs());
        // Input spectrum. Pick the louder channel to avoid mono cancellation.
        let sample = if input[0].abs() > input[1].abs() {
            input[0]
        } else {
            input[1]
        };
        self.samples[self.position] = Complex::new(sample as f32 * self.window[self.position], 0.0);
        self.position += 1;
        if self.position == 2048 {
            self.position = 0;
            self.fft
                .process_with_scratch(&mut self.samples, &mut self.scratch);
            for i in 0..128 {
                let lo = 20.0_f64 * 1000.0_f64.powf(i as f64 / 128.0);
                let hi = 20.0_f64 * 1000.0_f64.powf((i + 1) as f64 / 128.0);
                let start = (lo / self.sr * 2048.0).floor() as usize;
                let end = ((hi / self.sr * 2048.0).ceil() as usize).min(1023);
                let mut mag = 0.0_f32;
                if start <= end {
                    for bin in start.max(1)..=end {
                        mag = mag.max(self.samples[bin].norm() / 512.0);
                    }
                }
                let db = (20.0 * mag.max(0.0000316).log10()).max(-90.0);
                let old = self.shared.spectrum[i].load(Ordering::Relaxed);
                self.shared.spectrum[i].store(db.max(old - 3.0), Ordering::Relaxed);
            }
            self.shared
                .input
                .store(gain_db(self.in_peak) as f32, Ordering::Relaxed);
            self.shared
                .output
                .store(gain_db(self.out_peak) as f32, Ordering::Relaxed);
            self.shared.gr.store(
                (gr * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
                Ordering::Relaxed,
            );
            self.shared.band_gr.store(band_gr as f32, Ordering::Relaxed);
            self.in_peak = 0.0;
            self.out_peak = 0.0;
        }
        x
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unlimited_band_state_and_neutral_audio() {
        let bands = Arc::new(Mutex::new(
            (1..=200)
                .map(|id| Band {
                    id,
                    ..Band::default()
                })
                .collect(),
        ));
        let shared = Shared::new(bands);
        let mut engine = Engine::new(shared, 48000.0);
        assert_eq!(engine.bank.bands.len(), 200);
        let settings = CompSettings {
            amount: 0.0,
            gate: -80.0,
            hpf: 80.0,
            mix: 1.0,
        };
        for i in 0..4096 {
            let x = (i as f64 * 0.1).sin() * 0.5;
            let out = engine.tick([x, x], settings, 0.0, true, true, false);
            assert!((out[0] - x).abs() < 1e-9);
        }
    }
    #[test]
    fn bypass_preserves_input_after_ramp() {
        let shared = Shared::new(Arc::new(Mutex::new(vec![Band {
            gain: 18.0,
            ..Band::default()
        }])));
        let mut engine = Engine::new(shared, 48000.0);
        let settings = CompSettings {
            amount: 30.0,
            gate: -80.0,
            hpf: 80.0,
            mix: 1.0,
        };
        for i in 0..24000 {
            let x = (i as f64 * 0.13).sin() * 0.1;
            let out = engine.tick([x, x], settings, 6.0, true, true, true);
            if i > 23000 {
                assert!((out[0] - x).abs() < 1e-10);
            }
        }
    }
}
