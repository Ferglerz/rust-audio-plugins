//! Linked, pre-reduction spectra and a stereo-linked overlap-add processor.
//! Plans and buffers are allocated on construction; audio never locks or waits.
use crate::{
    network::{Network, Slot, CAPACITY},
    params::SharedParams,
};
use atomic_float::AtomicF32;
use rustfft::{num_complex::Complex32, Fft, FftPlanner};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering::SeqCst},
    Arc,
};

pub const BANDS: usize = 32;
pub const FFT_SIZE: usize = 2048;
const HOP: usize = FFT_SIZE / 4;
fn band_position(hz: f32) -> f32 {
    ((hz.max(40.0) / 40.0).ln() / 500.0_f32.ln() * (BANDS - 1) as f32)
        .clamp(0.0, (BANDS - 1) as f32)
}

/// Center frequency of each log band, from 40 Hz to 20 kHz.
/// Computed once so each hop does not repeat the same `powf`.
fn band_centers() -> &'static [f32; BANDS] {
    static CENTERS: std::sync::OnceLock<[f32; BANDS]> = std::sync::OnceLock::new();
    CENTERS.get_or_init(|| {
        std::array::from_fn(|band| 40.0 * 500.0_f32.powf(band as f32 / (BANDS - 1) as f32))
    })
}

pub struct SpectrumBus {
    sequence: AtomicU64,
    valid: AtomicBool,
    power: [AtomicF32; BANDS],
    cut: [AtomicF32; BANDS],
}
impl Default for SpectrumBus {
    fn default() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            valid: AtomicBool::new(false),
            power: std::array::from_fn(|_| AtomicF32::new(0.0)),
            cut: std::array::from_fn(|_| AtomicF32::new(0.0)),
        }
    }
}
impl SpectrumBus {
    pub fn clear(&self) {
        self.valid.store(false, SeqCst);
    }
    fn publish(&self, power: &[f32; BANDS], cut: &[f32; BANDS]) {
        self.sequence.fetch_add(1, SeqCst);
        for i in 0..BANDS {
            self.power[i].store(power[i], SeqCst);
            self.cut[i].store(cut[i], SeqCst);
        }
        self.sequence.fetch_add(1, SeqCst);
        self.valid.store(true, SeqCst);
    }
    // A single bounded attempt: skip a concurrently written frame rather than spin.
    pub fn read(&self) -> Option<(u64, [f32; BANDS], [f32; BANDS])> {
        if !self.valid.load(SeqCst) {
            return None;
        }
        let sequence = self.sequence.load(SeqCst);
        if sequence & 1 != 0 {
            return None;
        }
        let power = std::array::from_fn(|i| self.power[i].load(SeqCst));
        let cut = std::array::from_fn(|i| self.cut[i].load(SeqCst));
        (sequence == self.sequence.load(SeqCst) && self.valid.load(SeqCst))
            .then_some((sequence, power, cut))
    }
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub amount: f32,
    pub gain: f32,
    pub blend: f32,
    pub cap: f32,
    pub low: f32,
    pub high: f32,
    pub attack: f32,
    pub release: f32,
    pub smooth: f32,
}
impl Settings {
    pub fn from_params(p: &SharedParams, mic: usize) -> Self {
        Self {
            amount: p.spectral_amount.value() * 0.01,
            gain: 1.0,
            blend: p.mics[mic].eq_blend.value() * 0.01,
            cap: p.spectral_cut.value(),
            low: p.spectral_low.value(),
            high: p.spectral_high.value(),
            attack: p.spectral_attack.value(),
            release: p.spectral_release.value(),
            smooth: p.spectral_smooth.value() * 0.01,
        }
    }
}

// Strict priority; equal or lower weights can never initiate reduction.
fn influence(own: f32, peer: f32, own_peak: f32, peer_peak: f32, gap: f32) -> f32 {
    if gap <= 0.0 || own < 1e-10 || peer < 1e-10 {
        return 0.0;
    }
    let overlap = (own / (own_peak * 0.001).max(1e-10)).min(1.0)
        * (peer / (peer_peak * 0.001).max(1e-10)).min(1.0);
    let priority = 1.0 - 10.0_f32.powf(-gap / 20.0);
    overlap * priority * peer / (own + peer).max(1e-20)
}

// Normalize away track level before comparing tone. Floor each band at -40 dB
// relative power so absent frequencies cannot demand unbounded boosts.
fn tonal_shape(power: &[f32; BANDS]) -> Option<[f32; BANDS]> {
    let total: f32 = power.iter().sum();
    (total > 1e-8).then(|| std::array::from_fn(|i| 10.0 * (power[i] / total).max(1e-4).log10()))
}

#[derive(Clone, Copy, Default)]
struct Peer {
    owner: u64,
    sequence: u64,
    age: u64,
}
pub struct Spectral {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex32>,
    bins: [Vec<Complex32>; 2],
    input: [Vec<f32>; 2],
    output: [Vec<f32>; 2],
    gain_delay: Vec<f32>,
    gain_reduction: f32,
    window: Vec<f32>,
    positions: Vec<f32>,
    cursor: usize,
    hop: usize,
    rate: f32,
    stale_after: u64,
    peers: [Peer; CAPACITY],
    pub power: [f32; BANDS],
    pub cut: [f32; BANDS],
    eq: [f32; BANDS],
    regular_cut: [f32; BANDS],
    drum_cut: [f32; BANDS],
    peak_cut: f32,
    input_energy: f64,
    output_energy: f64,
}
impl Spectral {
    pub fn new(rate: f32, max_buffer: u32) -> Self {
        let _ = band_centers();
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(FFT_SIZE);
        let inverse = planner.plan_fft_inverse(FFT_SIZE);
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        Self {
            forward,
            inverse,
            scratch: vec![Complex32::default(); scratch_len],
            bins: std::array::from_fn(|_| vec![Complex32::default(); FFT_SIZE]),
            input: std::array::from_fn(|_| vec![0.0; FFT_SIZE]),
            output: std::array::from_fn(|_| vec![0.0; FFT_SIZE]),
            gain_delay: vec![1.0; FFT_SIZE],
            gain_reduction: 0.0,
            window: (0..FFT_SIZE)
                .map(|i| {
                    (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FFT_SIZE as f32).cos()).sqrt()
                })
                .collect(),
            positions: (0..=FFT_SIZE / 2)
                .map(|i| band_position(i as f32 * rate / FFT_SIZE as f32))
                .collect(),
            cursor: 0,
            hop: 0,
            rate,
            stale_after: (rate * 0.5) as u64 + 2 * max_buffer as u64,
            peers: [Peer::default(); CAPACITY],
            power: [0.0; BANDS],
            cut: [0.0; BANDS],
            eq: [0.0; BANDS],
            regular_cut: [0.0; BANDS],
            drum_cut: [0.0; BANDS],
            peak_cut: 0.0,
            input_energy: 0.0,
            output_energy: 0.0,
        }
    }
    pub fn reset(&mut self) {
        for channel in &mut self.input {
            channel.fill(0.0);
        }
        for channel in &mut self.output {
            channel.fill(0.0);
        }
        self.gain_delay.fill(1.0);
        self.gain_reduction = 0.0;
        self.cursor = 0;
        self.hop = 0;
        self.power.fill(0.0);
        self.cut.fill(0.0);
        self.eq.fill(0.0);
        self.regular_cut.fill(0.0);
        self.drum_cut.fill(0.0);
        self.peak_cut = 0.0;
        self.peers.fill(Peer::default());
        self.input_energy = 0.0;
        self.output_energy = 0.0;
    }
    pub fn process(
        &mut self,
        input: [f32; 2],
        network: &Network,
        slot: Option<&Slot>,
        mic: usize,
        bypass: bool,
        settings: Settings,
    ) -> [f32; 2] {
        // Match broadband gain to the input sample emerging after the FFT delay.
        // Analysis still sees the original signal, keeping linked detection independent.
        let gain = self.gain_delay[self.cursor];
        self.gain_delay[self.cursor] = settings.gain;
        self.gain_reduction = self.gain_reduction.max(-20.0 * gain.max(1e-6).log10());
        let output = std::array::from_fn(|ch| {
            let out = self.output[ch][self.cursor] * gain;
            self.output[ch][self.cursor] = 0.0;
            self.input[ch][self.cursor] = input[ch];
            out
        });
        self.input_energy += input.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() * 0.5;
        self.output_energy += output.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() * 0.5;
        self.cursor = (self.cursor + 1) % FFT_SIZE;
        self.hop += 1;
        if self.hop == HOP {
            self.hop = 0;
            self.frame(network, slot, mic, bypass, settings);
            if let Some(slot) = slot {
                let db = |energy: f64| (10.0 * (energy / HOP as f64).max(1e-12).log10()) as f32;
                slot.input_db.store(db(self.input_energy), SeqCst);
                slot.output_db.store(db(self.output_energy), SeqCst);
                slot.reduction_db
                    .store(self.peak_cut + self.gain_reduction, SeqCst);
            }
            self.gain_reduction = 0.0;
            self.input_energy = 0.0;
            self.output_energy = 0.0;
        }
        output
    }
    fn frame(
        &mut self,
        network: &Network,
        slot: Option<&Slot>,
        mic: usize,
        bypass: bool,
        settings: Settings,
    ) {
        for ch in 0..2 {
            for i in 0..FFT_SIZE {
                self.bins[ch][i] = Complex32::new(
                    self.input[ch][(self.cursor + i) % FFT_SIZE] * self.window[i],
                    0.0,
                );
            }
            self.forward
                .process_with_scratch(&mut self.bins[ch], &mut self.scratch);
        }
        self.power.fill(0.0);
        let scale = 2.0 / (FFT_SIZE * FFT_SIZE) as f32;
        for bin in 1..FFT_SIZE / 2 {
            let pos = self.positions[bin];
            let lo = pos as usize;
            let hi = (lo + 1).min(BANDS - 1);
            let mix = pos - lo as f32;
            let power = (self.bins[0][bin].norm_sqr() + self.bins[1][bin].norm_sqr()) * scale;
            self.power[lo] += power * (1.0 - mix);
            self.power[hi] += power * mix;
        }
        let conflicts = network.conflict_mask();
        let enabled = !bypass && slot.is_some() && conflicts & (1 << mic) == 0;
        if !enabled {
            if let Some(slot) = slot {
                slot.spectrum.clear();
            }
        }
        let own_weight = network.params.mics[mic].spectral_weight.value();
        let centers = band_centers();
        let own_peak = self.power.iter().copied().fold(0.0, f32::max);
        let mut target = [0.0_f32; BANDS];
        let mut drum_target = [0.0_f32; BANDS];
        let own_drums = network.params.mics[mic].is_drums();
        let own_shape = tonal_shape(&self.power);
        let mut shape_sum = own_shape.unwrap_or([0.0; BANDS]);
        let mut shape_count = 1;

        for (i, peer_slot) in network.slots.iter().enumerate() {
            let peer = &mut self.peers[i];
            peer.age = peer.age.saturating_add(HOP as u64);
            let owner = peer_slot.owner.load(SeqCst);
            let peer_mic = peer_slot.mic.load(SeqCst).min(CAPACITY - 1);
            if !enabled
                || owner == 0
                || peer_mic == mic
                || conflicts & (1 << peer_mic) != 0
                || peer_slot.signal.load(SeqCst) == 0
            {
                continue;
            }
            let Some((sequence, power, _)) = peer_slot.spectrum.read() else {
                continue;
            };
            if owner != peer.owner || sequence != peer.sequence {
                *peer = Peer {
                    owner,
                    sequence,
                    age: 0,
                };
            }
            if peer.age > self.stale_after || owner != peer_slot.owner.load(SeqCst) {
                continue;
            }
            if let Some(shape) = tonal_shape(&power) {
                for band in 0..BANDS {
                    shape_sum[band] += shape[band];
                }
                shape_count += 1;
            }
            let gap = network.params.mics[peer_mic].spectral_weight.value() - own_weight;
            let peer_drums = network.params.mics[peer_mic].is_drums();
            let peak = power.iter().copied().fold(0.0, f32::max);
            for band in 0..BANDS {
                let hz = centers[band];
                let destination = if hz >= 3000.0 && (own_drums || peer_drums) {
                    &mut drum_target[band]
                } else {
                    &mut target[band]
                };
                *destination = destination.max(
                    influence(self.power[band], power[band], own_peak, peak, gap)
                        * settings.cap
                        * settings.amount,
                );
            }
        }
        let eq_target: [f32; BANDS] = std::array::from_fn(|band| {
            if enabled && shape_count > 1 {
                own_shape.map_or(0.0, |own| {
                    ((shape_sum[band] / shape_count as f32 - own[band]) * settings.blend)
                        .clamp(-6.0, 6.0)
                })
            } else {
                0.0
            }
        });
        let dt = HOP as f32 / self.rate * 1000.0;
        let drum_eq_band = band_position(3000.0);
        for band in 0..BANDS {
            let neighbor =
                (target[band.saturating_sub(1)] + target[(band + 1).min(BANDS - 1)]) * 0.5;
            let desired = target[band] + (neighbor - target[band]) * settings.smooth;
            let ms = if !enabled {
                20.0
            } else if desired > self.regular_cut[band] {
                settings.attack
            } else {
                settings.release
            };
            self.regular_cut[band] +=
                (desired - self.regular_cut[band]) * (1.0 - (-dt / ms.max(1.0)).exp());
            let drum_neighbor = (drum_target[band.saturating_sub(1)]
                + drum_target[(band + 1).min(BANDS - 1)])
                * 0.5;
            let drum_desired =
                drum_target[band] + (drum_neighbor - drum_target[band]) * settings.smooth;
            let drum_ms = if drum_desired > self.drum_cut[band] {
                settings.attack.min(5.0)
            } else {
                settings.release.min(40.0)
            };
            self.drum_cut[band] +=
                (drum_desired - self.drum_cut[band]) * (1.0 - (-dt / drum_ms.max(1.0)).exp());
            self.cut[band] = self.regular_cut[band]
                .max(self.drum_cut[band])
                .clamp(0.0, settings.cap);
            let neighbor_eq =
                (eq_target[band.saturating_sub(1)] + eq_target[(band + 1).min(BANDS - 1)]) * 0.5;
            let desired_eq = eq_target[band] + (neighbor_eq - eq_target[band]) * settings.smooth;
            let eq_ms = if !enabled {
                20.0
            } else if own_drums && band as f32 >= drum_eq_band {
                if desired_eq.abs() > self.eq[band].abs() {
                    5.0
                } else {
                    40.0
                }
            } else {
                settings.release.max(100.0)
            };
            self.eq[band] += (desired_eq - self.eq[band]) * (1.0 - (-dt / eq_ms).exp());
        }
        self.peak_cut = 0.0;
        for bin in 0..=FFT_SIZE / 2 {
            let hz = bin as f32 * self.rate / FFT_SIZE as f32;
            let pos = self.positions[bin];
            let lo = pos as usize;
            let hi = (lo + 1).min(BANDS - 1);
            // A reversed range intentionally means no affected frequencies.
            let in_range = hz >= settings.low && hz <= settings.high;
            let interpolate = |bands: &[f32; BANDS]| {
                if in_range {
                    bands[lo] + (bands[hi] - bands[lo]) * (pos - lo as f32)
                } else {
                    0.0
                }
            };
            let cut = interpolate(&self.cut);
            self.peak_cut = self.peak_cut.max(cut);
            let mixer_gain = 10.0_f32.powf(-cut / 20.0);
            let post_eq_gain = 10.0_f32.powf(interpolate(&self.eq) / 20.0);
            // Mixer reduction is measured before EQ Blend. Apply EQ downstream
            // in the same FFT frame, without feeding it into linked detectors.
            for ch in 0..2 {
                self.bins[ch][bin] *= mixer_gain;
                self.bins[ch][bin] *= post_eq_gain;
                if bin > 0 && bin < FFT_SIZE / 2 {
                    self.bins[ch][FFT_SIZE - bin] *= mixer_gain;
                    self.bins[ch][FFT_SIZE - bin] *= post_eq_gain;
                }
            }
        }
        if enabled {
            let displayed = std::array::from_fn(|i| {
                let hz = centers[i];
                if hz >= settings.low && hz <= settings.high && hz < self.rate * 0.5 {
                    self.cut[i]
                } else {
                    0.0
                }
            });
            if let Some(slot) = slot {
                slot.spectrum.publish(&self.power, &displayed);
            }
        }
        for ch in 0..2 {
            self.inverse
                .process_with_scratch(&mut self.bins[ch], &mut self.scratch);
            for i in 0..FFT_SIZE {
                self.output[ch][(self.cursor + i) % FFT_SIZE] +=
                    self.bins[ch][i].re * self.window[i] * (0.5 / FFT_SIZE as f32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> Settings {
        static SETTINGS: std::sync::OnceLock<Settings> = std::sync::OnceLock::new();
        *SETTINGS.get_or_init(|| Settings::from_params(&SharedParams::default(), 0))
    }
    fn weighted_network(first: f32, second: f32) -> Arc<Network> {
        let mut network = Network::default();
        let params = Arc::get_mut(&mut network.params).unwrap();
        params.mics[0].spectral_weight =
            crate::params::float("Spectral weight", first, -12.0, 12.0, " dB");
        params.mics[1].spectral_weight =
            crate::params::float("Spectral weight", second, -12.0, 12.0, " dB");
        Arc::new(network)
    }
    fn tone(n: usize, bin: usize) -> f32 {
        (std::f32::consts::TAU * bin as f32 * n as f32 / FFT_SIZE as f32).sin() * 0.1
    }
    fn max_cut(engine: &Spectral) -> f32 {
        engine.cut.iter().copied().fold(0.0, f32::max)
    }

    #[test]
    fn broadband_gain_follows_its_sample_through_fft_delay() {
        let network = Arc::new(Network::default());
        let mut engine = Spectral::new(48000.0, 128);
        let mut expected = Vec::new();
        for n in 0..FFT_SIZE * 4 {
            let input = tone(n, 43);
            let gain = if n % 113 < 57 { 0.25 } else { 0.75 };
            expected.push(input * gain);
            let output = engine.process(
                [input; 2],
                &network,
                None,
                0,
                false,
                Settings { gain, ..settings() },
            );
            if n >= FFT_SIZE {
                assert!(
                    (output[0] - expected[n - FFT_SIZE]).abs() < 1e-5,
                    "gain must follow the delayed sample"
                );
            }
        }
        engine.reset();
        assert!(engine.gain_delay.iter().all(|gain| *gain == 1.0));
    }

    #[test]
    fn post_mixer_eq_changes_output_without_changing_detection_or_gr() {
        fn render(blend: f32) -> (Vec<([f32; BANDS], [f32; BANDS], f32)>, f64) {
            let network = weighted_network(12.0, 0.0);
            let a = network.register();
            let b = network.register();
            let sa = a.slot().unwrap();
            let sb = b.slot().unwrap();
            sa.publish(0.1, true, true);
            sb.publish(0.1, true, true);
            let mut source = Spectral::new(48000.0, 128);
            let mut receiver = Spectral::new(48000.0, 128);
            let config = Settings {
                blend,
                ..settings()
            };
            let mut snapshots = Vec::new();
            let mut energy = 0.0;
            for n in 0..48000 {
                source.process(
                    [tone(n, 43) * 0.25 + tone(n, 256); 2],
                    &network,
                    Some(sa),
                    0,
                    false,
                    config,
                );
                let output = receiver.process(
                    [tone(n, 43) + tone(n, 256) * 0.25; 2],
                    &network,
                    Some(sb),
                    1,
                    false,
                    config,
                );
                if n % HOP == HOP - 1 {
                    let (_, power, cut) = sb.spectrum.read().unwrap();
                    snapshots.push((power, cut, sb.reduction_db.load(SeqCst)));
                }
                if n > 24000 {
                    energy += (output[0] as f64).powi(2);
                }
            }
            assert!(
                receiver.peak_cut > 0.1,
                "test must exercise active mixer reduction"
            );
            (snapshots, energy)
        }
        let (neutral, neutral_energy) = render(0.0);
        for blend in [-1.0, 1.0] {
            let (eq, energy) = render(blend);
            assert_eq!(
                neutral, eq,
                "EQ must not change input spectra, mixer cuts or GR meters"
            );
            assert!(
                (energy / neutral_energy - 1.0).abs() > 0.1,
                "post-mixer EQ must still affect output"
            );
        }
    }

    #[test]
    fn drums_speed_up_high_band_influence_but_preserve_low_band_timing() {
        fn response(drums: bool, bin: usize) -> (f32, f32) {
            let mut network = Network::default();
            Arc::get_mut(&mut network.params).unwrap().mics[0].group =
                nih_plug::prelude::EnumParam::new(
                    "Group",
                    if drums {
                        crate::params::MicGroup::Drums
                    } else {
                        crate::params::MicGroup::Other
                    },
                );
            Arc::get_mut(&mut network.params).unwrap().mics[0].spectral_weight =
                crate::params::float("Spectral weight", 12.0, -12.0, 12.0, " dB");
            let network = Arc::new(network);
            let a = network.register();
            let b = network.register();
            let sa = a.slot().unwrap();
            let sb = b.slot().unwrap();
            sa.publish(0.1, true, false);
            sb.publish(0.1, true, false);
            let mut receiver = Spectral::new(48000.0, 128);
            for n in 0..FFT_SIZE * 4 {
                receiver.process([tone(n, bin); 2], &network, Some(sb), 1, false, settings());
            }
            let power = receiver.power;
            let mut attack = 0.0;
            for n in 0..HOP * 10 {
                sa.spectrum.publish(
                    if n < HOP * 2 { &power } else { &[0.0; BANDS] },
                    &[0.0; BANDS],
                );
                receiver.process([tone(n, bin); 2], &network, Some(sb), 1, false, settings());
                if n == HOP * 2 - 1 {
                    attack = max_cut(&receiver);
                }
            }
            (attack, max_cut(&receiver))
        }
        let normal = response(false, 256);
        let drums = response(true, 256);
        assert!(
            drums.0 > normal.0 * 1.5,
            "high band attack: drums={drums:?}, normal={normal:?}"
        );
        assert!(
            drums.1 < normal.1 * 0.5,
            "high band release: drums={drums:?}, normal={normal:?}"
        );
        let normal_low = response(false, 43);
        let drum_low = response(true, 43);
        assert!((normal_low.0 - drum_low.0).abs() < 1e-5);
        assert!((normal_low.1 - drum_low.1).abs() < 1e-5);
    }

    #[test]
    fn eq_blend_matches_or_separates_tone_independently_and_releases() {
        for blend in [-1.0, 0.0, 1.0] {
            let network = Arc::new(Network::default());
            let a = network.register();
            let b = network.register();
            let sa = a.slot().unwrap();
            let sb = b.slot().unwrap();
            sa.publish(0.1, true, true);
            sb.publish(0.1, true, true);
            let mut source = Spectral::new(48000.0, 128);
            let mut receiver = Spectral::new(48000.0, 128);
            let config = Settings {
                blend,
                amount: 0.0,
                cap: 0.0,
                ..settings()
            };
            let mut input_bins = [Complex32::default(); 2];
            let mut output_bins = input_bins;
            for n in 0..96000 {
                source.process(
                    [tone(n, 43) * 0.25 + tone(n, 256); 2],
                    &network,
                    Some(sa),
                    0,
                    false,
                    config,
                );
                let input = tone(n, 43) + tone(n, 256) * 0.25;
                let output =
                    receiver.process([input, -input], &network, Some(sb), 1, false, config);
                assert!((output[0] + output[1]).abs() < 2e-5);
                if n >= 96000 - FFT_SIZE {
                    for (i, bin) in [43, 256].into_iter().enumerate() {
                        let oscillator = Complex32::from_polar(
                            1.0,
                            -std::f32::consts::TAU * bin as f32 * n as f32 / FFT_SIZE as f32,
                        );
                        input_bins[i] += oscillator * input;
                        output_bins[i] += oscillator * output[0];
                    }
                }
            }
            let gains: [f32; 2] =
                std::array::from_fn(|i| output_bins[i].norm() / input_bins[i].norm());
            if blend > 0.0 {
                assert!(
                    gains[0] < 0.8 && gains[1] > 1.2,
                    "matching gains: {gains:?}"
                );
            } else if blend < 0.0 {
                assert!(
                    gains[0] > 1.2 && gains[1] < 0.8,
                    "separating gains: {gains:?}"
                );
            } else {
                assert!(gains.iter().all(|gain| (gain - 1.0).abs() < 0.01));
            }
            assert_eq!(receiver.peak_cut, 0.0, "EQ alone must report zero GR");
            assert_eq!(sb.reduction_db.load(SeqCst), 0.0);
            assert_eq!(
                max_cut(&receiver),
                0.0,
                "EQ must not engage automix ducking"
            );
            assert!(receiver.eq.iter().all(|eq| eq.abs() <= 6.0));
            sa.withdraw();
            for n in 0..96000 {
                receiver.process([tone(n, 43); 2], &network, Some(sb), 1, false, config);
            }
            assert!(
                receiver.eq.iter().all(|eq| eq.abs() < 0.01),
                "withdrawn peer must release EQ"
            );
        }
    }

    #[test]
    fn tonal_comparison_ignores_level_and_silence() {
        let power = std::array::from_fn(|i| (i + 1) as f32 * 0.001);
        let quieter = power.map(|p| p * 0.01);
        let a = tonal_shape(&power).unwrap();
        let b = tonal_shape(&quieter).unwrap();
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4));
        assert!(tonal_shape(&[0.0; BANDS]).is_none());
    }

    #[test]
    fn unity_reconstructs_stereo_with_exact_reported_delay_at_supported_rates() {
        for rate in [44100.0, 48000.0, 96000.0] {
            let network = Arc::new(Network::default());
            let registration = network.register();
            let slot = registration.slot().unwrap();
            slot.publish(0.1, true, true);
            let mut engine = Spectral::new(rate, 512);
            for n in 0..FFT_SIZE * 5 {
                let input = [tone(n, 43), -tone(n, 43) * 0.5];
                let output = engine.process(input, &network, Some(slot), 0, false, settings());
                let expected = if n >= FFT_SIZE {
                    [tone(n - FFT_SIZE, 43), -tone(n - FFT_SIZE, 43) * 0.5]
                } else {
                    [0.0; 2]
                };
                for ch in 0..2 {
                    assert!(
                        (output[ch] - expected[ch]).abs() < 2e-5,
                        "unity reconstruction or delay: rate={rate}, sample={n}"
                    );
                }
            }
            assert_eq!(max_cut(&engine), 0.0);
        }
    }

    #[test]
    fn equal_weights_are_unity_and_only_lower_priority_yields_at_overlapping_frequencies() {
        let network = Arc::new(Network::default());
        let a = network.register();
        let b = network.register();
        let sa = a.slot().unwrap();
        let sb = b.slot().unwrap();
        sa.publish(0.1, true, true);
        sb.publish(0.1, true, false);
        let mut high = Spectral::new(48000.0, 128);
        let mut low = Spectral::new(48000.0, 128);
        for n in 0..12000 {
            high.process([tone(n, 43); 2], &network, Some(sa), 0, false, settings());
            low.process([tone(n, 43); 2], &network, Some(sb), 1, false, settings());
        }
        assert_eq!(max_cut(&high), 0.0);
        assert_eq!(max_cut(&low), 0.0);
        let network = weighted_network(12.0, 0.0);
        let a = network.register();
        let b = network.register();
        let sa = a.slot().unwrap();
        let sb = b.slot().unwrap();
        sa.publish(0.1, true, true);
        sb.publish(0.1, true, false);
        let mut input_bin = [Complex32::default(); 2];
        let mut output_bin = input_bin;
        for n in 0..48000 {
            high.process([tone(n, 43); 2], &network, Some(sa), 0, false, settings());
            let input = tone(n, 43) + tone(n, 256);
            let out = low.process([input, -input], &network, Some(sb), 1, false, settings());
            if n >= FFT_SIZE * 2 {
                assert!(
                    (out[0] + out[1]).abs() < 2e-5,
                    "stereo must share the same reduction"
                );
            }
            if n >= 48000 - FFT_SIZE {
                for (i, bin) in [43, 256].into_iter().enumerate() {
                    let phase = -std::f32::consts::TAU * bin as f32 * n as f32 / FFT_SIZE as f32;
                    let oscillator = Complex32::from_polar(1.0, phase);
                    input_bin[i] += oscillator * input;
                    output_bin[i] += oscillator * out[0];
                }
            }
        }
        let overlap_gain = output_bin[0].norm() / input_bin[0].norm();
        let separate_gain = output_bin[1].norm() / input_bin[1].norm();
        assert!(
            overlap_gain < 0.85,
            "overlapping tone must yield: {overlap_gain}"
        );
        assert!(
            (separate_gain - 1.0).abs() < 0.01,
            "unrelated tone must stay intact: {separate_gain}"
        );
        assert_eq!(
            max_cut(&high),
            0.0,
            "higher priority must never yield to lower"
        );
        assert!(max_cut(&low) <= settings().cap);
        // Equalizing priority releases all cuts rather than creating mutual ducking.
        let network = weighted_network(12.0, 12.0);
        let a = network.register();
        let b = network.register();
        let sa = a.slot().unwrap();
        let sb = b.slot().unwrap();
        sa.publish(0.1, true, true);
        sb.publish(0.1, true, false);
        for n in 0..96000 {
            high.process([tone(n, 43); 2], &network, Some(sa), 0, false, settings());
            low.process([tone(n, 43); 2], &network, Some(sb), 1, false, settings());
        }
        assert!(max_cut(&low) < 0.01);
    }

    #[test]
    fn silent_withdrawn_conflicting_and_stale_sources_cannot_keep_ducking() {
        for reason in 0..4 {
            let network = weighted_network(12.0, 0.0);
            let source = network.register();
            let receiver = network.register();
            let sa = source.slot().unwrap();
            let sb = receiver.slot().unwrap();
            sa.publish(0.1, true, true);
            sb.publish(0.1, true, true);
            let mut a = Spectral::new(48000.0, 128);
            let mut b = Spectral::new(48000.0, 128);
            for n in 0..24000 {
                a.process([tone(n, 43); 2], &network, Some(sa), 0, false, settings());
                b.process([tone(n, 43); 2], &network, Some(sb), 1, false, settings());
            }
            assert!(max_cut(&b) > 1.0);
            if reason == 1 {
                sa.withdraw();
            }
            if reason == 2 {
                sa.mic.store(1, SeqCst);
            }
            for n in 0..144000 {
                if reason == 0 {
                    a.process([0.0; 2], &network, Some(sa), 0, false, settings());
                }
                b.process([tone(n, 43); 2], &network, Some(sb), 1, false, settings());
            }
            assert!(max_cut(&b) < 0.01, "source exclusion failed: {reason}");
        }
    }

    #[test]
    fn bypass_amount_and_range_restore_delayed_unity_and_cap_bounds_cuts() {
        let network = weighted_network(12.0, 0.0);
        let a = network.register();
        let b = network.register();
        let sa = a.slot().unwrap();
        let sb = b.slot().unwrap();
        sa.publish(0.1, true, true);
        sb.publish(0.1, true, true);
        let mut source = Spectral::new(48000.0, 128);
        let mut receiver = Spectral::new(48000.0, 128);
        for scenario in 0..4 {
            let mut config = settings();
            if scenario == 1 {
                config.amount = 0.0;
            }
            if scenario == 2 {
                config.low = 4000.0;
                config.high = 8000.0;
            }
            config.cap = 3.0;
            let mut error = 0.0_f32;
            for n in 0..96000 {
                source.process([tone(n, 43); 2], &network, Some(sa), 0, false, config);
                let output = receiver.process(
                    [tone(n, 43); 2],
                    &network,
                    Some(sb),
                    1,
                    scenario == 3,
                    config,
                );
                assert!(max_cut(&receiver) <= 3.0);
                if n >= 96000 - FFT_SIZE {
                    error = error.max((output[0] - tone(n - FFT_SIZE, 43)).abs());
                }
            }
            if scenario > 0 {
                assert!(
                    error < 1e-4,
                    "unity failed for scenario {scenario}: {error}"
                );
            } else {
                assert!(error > 0.005);
            }
        }
        assert!(
            sb.spectrum.read().is_none(),
            "bypassed receiver must withdraw its spectrum"
        );
    }
}
