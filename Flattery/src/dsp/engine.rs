use crate::{
    dsp::{
        constants::MAX_FFT_SIZE,
        fft_analyzer::FftAnalyzer,
        filter_bank::FilterBank,
        leveling::LevelingProcessor,
        ring_buffer::{AnalysisRing, DelayLine},
        telemetry::Shared,
        tilt::{apply_tilt_compensation, calculate_tilt_multiplier_scaled},
    },
    strength::PreparedStrengthCurve,
};
use pleasant_dsp::{
    spectrum::{peak_hold, spectrum_fall_db},
    units::{db_to_linear, linear_to_db},
};
use std::sync::{atomic::Ordering, Arc};

const FFT_SIZES: [usize; 7] = [128, 256, 512, 1024, 2048, 4096, 8192];

#[derive(Clone, Copy, Debug)]
pub struct EngineSettings {
    pub fft_size: usize,
    pub bypassed: bool,
    pub low_cut_hz: f64,
    pub high_cut_hz: f64,
    pub tilt: f64,
    pub tilt_frequency_hz: f64,
    pub max_boost_db: f64,
    pub max_cut_db: f64,
    pub output_gain_db: f64,
    pub mid_side: bool,
    pub input_rms_ms: f64,
    pub minimum_operating_db: f64,
    pub maximum_operating_db: f64,
    pub boost_strength: f64,
    pub cut_strength: f64,
    pub stereo_link: f64,
    pub neighbor_radius: usize,
    pub amplify: bool,
    pub attack_ms: f64,
    pub release_ms: f64,
}

pub struct Engine {
    pub shared: Arc<Shared>,
    pub sample_rate: f64,
    pub fft_size: usize,
    pub hop_size: usize,
    pub hop_counter: usize,
    pub analysis_delay: usize,
    delay_l: DelayLine,
    delay_r: DelayLine,
    ring_l: AnalysisRing,
    ring_r: AnalysisRing,
    analyzer: FftAnalyzer,
    prepared_analyzers: Vec<FftAnalyzer>,
    leveler: LevelingProcessor,
    filter_bank: FilterBank,
    filter_bank_r: FilterBank,
    window_samples_l: Vec<f64>,
    window_samples_r: Vec<f64>,
    target_gains: Vec<f64>,
    target_gains_r: Vec<f64>,
    boost_weights: Vec<f64>,
    cut_weights: Vec<f64>,
    boost_radii: Vec<usize>,
    cut_radii: Vec<usize>,
    boost_nodes: Arc<PreparedStrengthCurve>,
    cut_nodes: Arc<PreparedStrengthCurve>,
    curve_grid: Option<(usize, u64)>,
    curve_default_radius: Option<usize>,
    display_enabled: bool,
    display_fresh_samples_remaining: usize,
    spectrum_reset_pending: bool,
    last_output_gain_db: f64,
    output_gain_linear: f64,
    mid_side: bool,
}

impl Engine {
    pub fn new(shared: Arc<Shared>, sample_rate: f64) -> Self {
        let fft_size = 512;
        let hop_size = fft_size / 2;
        let analysis_delay = hop_size;

        let mut filter_bank = FilterBank::new();
        filter_bank.init_frequencies(fft_size, sample_rate);
        let mut filter_bank_r = FilterBank::new();
        filter_bank_r.init_frequencies(fft_size, sample_rate);
        let prepared_analyzers = FFT_SIZES
            .into_iter()
            .filter(|size| *size != fft_size)
            .map(FftAnalyzer::new)
            .collect();
        let boost_nodes = shared.node_snapshot(true).unwrap_or_default();
        let cut_nodes = shared.node_snapshot(false).unwrap_or_default();

        Self {
            shared,
            sample_rate,
            fft_size,
            hop_size,
            hop_counter: 0,
            analysis_delay,
            delay_l: DelayLine::new(),
            delay_r: DelayLine::new(),
            ring_l: AnalysisRing::new(),
            ring_r: AnalysisRing::new(),
            analyzer: FftAnalyzer::new(fft_size),
            prepared_analyzers,
            leveler: LevelingProcessor::new(),
            filter_bank,
            filter_bank_r,
            window_samples_l: vec![0.0; MAX_FFT_SIZE],
            window_samples_r: vec![0.0; MAX_FFT_SIZE],
            target_gains: vec![1.0; MAX_FFT_SIZE / 2],
            target_gains_r: vec![1.0; MAX_FFT_SIZE / 2],
            boost_weights: vec![1.0; MAX_FFT_SIZE / 2],
            cut_weights: vec![1.0; MAX_FFT_SIZE / 2],
            boost_radii: vec![1; MAX_FFT_SIZE / 2],
            cut_radii: vec![1; MAX_FFT_SIZE / 2],
            boost_nodes,
            cut_nodes,
            curve_grid: None,
            curve_default_radius: None,
            display_enabled: true,
            display_fresh_samples_remaining: 0,
            spectrum_reset_pending: true,
            last_output_gain_db: 0.0,
            output_gain_linear: 1.0,
            mid_side: false,
        }
    }

    pub fn reset(&mut self) {
        self.delay_l.reset();
        self.delay_r.reset();
        self.ring_l.reset();
        self.ring_r.reset();
        self.analyzer.reset();
        self.leveler.reset();
        self.filter_bank.reset();
        self.filter_bank_r.reset();
        self.hop_counter = 0;
        self.spectrum_reset_pending = true;
        self.display_fresh_samples_remaining = self.fft_size;
        if let Ok(mut lock) = self.shared.spectrum_mags_db.try_write() {
            lock.fill(-120.0);
        }
    }

    pub fn latency(&self) -> u32 {
        self.analysis_delay as u32
    }

    /// Called once per host block; spectral processing continues with the editor closed.
    pub fn set_display_enabled(&mut self, enabled: bool) {
        if enabled && !self.display_enabled {
            self.spectrum_reset_pending = true;
            self.display_fresh_samples_remaining = self.fft_size;
            if let Ok(mut spectrum) = self.shared.spectrum_mags_db.try_write() {
                spectrum.fill(-120.0);
                self.spectrum_reset_pending = false;
            }
        }
        self.display_enabled = enabled;
    }

    fn update_strength_curves(&mut self, default_radius: usize) {
        let boost_changed = self
            .shared
            .refresh_node_snapshot(true, &mut self.boost_nodes);
        let cut_changed = self
            .shared
            .refresh_node_snapshot(false, &mut self.cut_nodes);
        let grid = (self.fft_size, self.sample_rate.to_bits());
        let grid_changed = self.curve_grid != Some(grid);
        let radius_changed = self.curve_default_radius != Some(default_radius);
        let half = self.fft_size / 2;
        let bin_hz = self.sample_rate / self.fft_size as f64;
        for (curve, changed, weights, radii) in [
            (
                &self.boost_nodes,
                boost_changed,
                &mut self.boost_weights,
                &mut self.boost_radii,
            ),
            (
                &self.cut_nodes,
                cut_changed,
                &mut self.cut_weights,
                &mut self.cut_radii,
            ),
        ] {
            if changed || grid_changed {
                curve.fill_bin_weights(half, bin_hz, weights);
            }
            if changed || grid_changed || radius_changed {
                curve.fill_bin_radii(half, bin_hz, default_radius, radii);
            }
        }
        self.curve_grid = Some(grid);
        self.curve_default_radius = Some(default_radius);
    }

    pub fn set_fft_size(&mut self, new_size: usize) {
        if self.fft_size == new_size {
            return;
        }
        let Some(index) = self
            .prepared_analyzers
            .iter()
            .position(|analyzer| analyzer.fft_size == new_size)
        else {
            return;
        };
        std::mem::swap(&mut self.analyzer, &mut self.prepared_analyzers[index]);
        self.analyzer.reset();
        self.fft_size = new_size;
        self.spectrum_reset_pending = true;
        self.display_fresh_samples_remaining = new_size;
        self.hop_size = new_size / 2;
        self.analysis_delay = self.hop_size;
        self.hop_counter = 0;
        self.filter_bank
            .init_frequencies(new_size, self.sample_rate);
        self.filter_bank_r
            .init_frequencies(new_size, self.sample_rate);
        self.shared.fft_size.store(new_size, Ordering::Relaxed);
    }

    pub fn set_sample_rate(&mut self, srate: f64) {
        if self.sample_rate != srate {
            self.spectrum_reset_pending = true;
            self.display_fresh_samples_remaining = self.fft_size;
        }
        self.sample_rate = srate;
        self.filter_bank.init_frequencies(self.fft_size, srate);
        self.filter_bank_r.init_frequencies(self.fft_size, srate);
        self.shared
            .sample_rate
            .store(srate as f32, Ordering::Relaxed);
    }

    pub fn tick(&mut self, in_l: f64, in_r: f64, settings: &EngineSettings) -> (f64, f64) {
        if settings.fft_size != self.fft_size {
            self.set_fft_size(settings.fft_size);
        }
        if settings.mid_side != self.mid_side {
            self.mid_side = settings.mid_side;
            self.analyzer.reset();
            self.leveler.reset();
            self.filter_bank.reset();
            self.filter_bank_r.reset();
        }

        let bypassed = settings.bypassed;
        self.shared.is_bypassed.store(bypassed, Ordering::Relaxed);

        let low_cut_hz = settings.low_cut_hz;
        let high_cut_hz = settings.high_cut_hz;
        let tilt = settings.tilt;
        let tilt_freq_hz = settings.tilt_frequency_hz;
        let max_boost_db = settings.max_boost_db;
        let max_cut_db = settings.max_cut_db;
        let output_gain_db = settings.output_gain_db;

        // Push to analysis rings
        self.ring_l.push(in_l);
        self.ring_r.push(in_r);
        if self.display_enabled {
            self.display_fresh_samples_remaining =
                self.display_fresh_samples_remaining.saturating_sub(1);
        }

        // Delayed dry audio aligned to analysis window
        let delayed_l = self.delay_l.write_and_read(in_l, self.analysis_delay);
        let delayed_r = self.delay_r.write_and_read(in_r, self.analysis_delay);

        self.hop_counter += 1;
        if self.hop_counter >= self.hop_size {
            self.hop_counter = 0;
            let n = self.fft_size;
            let half = n / 2;
            let bin_hz = self.sample_rate / n as f64;
            let frame_dt = self.hop_size as f64 / self.sample_rate;

            self.ring_l.read_window(n, &mut self.window_samples_l[..n]);
            self.ring_r.read_window(n, &mut self.window_samples_r[..n]);

            let is_ms = settings.mid_side;
            let input_rms_ms = settings.input_rms_ms;

            self.analyzer.analyze(
                &self.window_samples_l[..n],
                &self.window_samples_r[..n],
                is_ms,
                input_rms_ms,
                frame_dt,
            );

            let low_cut_bin = (low_cut_hz / bin_hz.max(1e-6)).floor() as usize;
            let high_cut_bin = (high_cut_hz / bin_hz.max(1e-6)).floor() as usize;

            let min_operate_lin = db_to_linear(settings.minimum_operating_db);
            let max_operate_lin = db_to_linear(settings.maximum_operating_db);

            let str_boost = settings.boost_strength;
            let str_cut = settings.cut_strength;
            let stereo_link = settings.stereo_link;
            let default_radius = settings.neighbor_radius;
            let amplify = settings.amplify;
            let att_ms = settings.attack_ms;
            let rel_ms = settings.release_ms;

            self.update_strength_curves(default_radius);

            self.leveler.process(
                &self.analyzer.mag_l,
                &self.analyzer.mag_r,
                half,
                &self.boost_radii[..half],
                &self.cut_radii[..half],
                amplify,
                stereo_link,
                str_boost,
                str_cut,
                max_boost_db,
                max_cut_db,
                low_cut_bin,
                high_cut_bin,
                min_operate_lin,
                max_operate_lin,
                att_ms,
                rel_ms,
                frame_dt,
                &self.boost_weights[..half],
                &self.cut_weights[..half],
            );

            // Map leveler gains to filter bank
            let filter_count = self.filter_bank.filters.len();

            let tilt_amount = tilt * 0.01;
            for i in 0..filter_count {
                let center_hz = self.filter_bank.filters[i].center_hz;
                let bin_idx = ((center_hz / bin_hz.max(1e-6)).floor() as usize).min(half - 1);

                let tilt_mult_scaled = calculate_tilt_multiplier_scaled(
                    center_hz,
                    tilt_freq_hz,
                    tilt_amount,
                    self.sample_rate,
                );
                // The leveler already blends each channel toward the linked target.
                for (target, raw_gain_db) in [
                    (
                        &mut self.target_gains[i],
                        self.leveler.smoothed_gain_db_l[bin_idx],
                    ),
                    (
                        &mut self.target_gains_r[i],
                        self.leveler.smoothed_gain_db_r[bin_idx],
                    ),
                ] {
                    let tilted_gain_db =
                        (raw_gain_db * tilt_mult_scaled).clamp(-max_cut_db, max_boost_db);
                    *target = apply_tilt_compensation(
                        db_to_linear(tilted_gain_db),
                        tilt_mult_scaled,
                        tilt_amount,
                    );
                }
            }

            if str_boost == 0.0 && str_cut == 0.0 {
                self.target_gains[..filter_count].fill(1.0);
                self.target_gains_r[..filter_count].fill(1.0);
            }

            self.filter_bank.set_filter_gains(
                &self.target_gains[..filter_count],
                self.sample_rate,
                low_cut_hz,
                high_cut_hz,
            );
            self.filter_bank_r.set_filter_gains(
                &self.target_gains_r[..filter_count],
                self.sample_rate,
                low_cut_hz,
                high_cut_hz,
            );

            // Only display publication is optional: the FFT above drives the audio.
            if self.display_enabled {
                if let Ok(mut lock) = self.shared.spectrum_mags_db.try_write() {
                    if self.spectrum_reset_pending {
                        lock.fill(-120.0);
                        self.spectrum_reset_pending = false;
                    }
                    if self.display_fresh_samples_remaining == 0 {
                        if lock.len() != half {
                            lock.clear();
                            lock.resize(half, -120.0);
                        }
                        let fall = spectrum_fall_db(self.hop_size as f32);
                        for k in 0..half {
                            let avg = 0.5 * (self.analyzer.mag_l[k] + self.analyzer.mag_r[k]);
                            let db = linear_to_db(avg) as f32;
                            lock[k] = peak_hold(lock[k], db, fall);
                        }
                    }
                }
                if let Ok(mut lock) = self.shared.filter_display.try_write() {
                    lock.clear();
                    for (l, r) in self
                        .filter_bank
                        .filters
                        .iter()
                        .zip(&self.filter_bank_r.filters)
                    {
                        let db = 0.5 * (linear_to_db(l.gain_linear) + linear_to_db(r.gain_linear));
                        lock.push((l.center_hz as f32, db as f32));
                    }
                }
            }
        }

        if bypassed {
            return (delayed_l, delayed_r);
        }

        let (domain_l, domain_r) = if self.mid_side {
            (0.5 * (delayed_l + delayed_r), 0.5 * (delayed_l - delayed_r))
        } else {
            (delayed_l, delayed_r)
        };
        let filtered_l = self.filter_bank.process_mono(domain_l);
        let filtered_r = self.filter_bank_r.process_mono(domain_r);
        let (wet_l, wet_r) = if self.mid_side {
            (filtered_l + filtered_r, filtered_l - filtered_r)
        } else {
            (filtered_l, filtered_r)
        };
        if output_gain_db != self.last_output_gain_db {
            self.output_gain_linear = db_to_linear(output_gain_db);
            self.last_output_gain_db = output_gain_db;
        }
        (
            wet_l * self.output_gain_linear,
            wet_r * self.output_gain_linear,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strength::{radius_at, weight_at, StrengthNode};

    fn assert_cached_curves(engine: &Engine, default_radius: usize) {
        let bin_hz = engine.sample_rate / engine.fft_size as f64;
        for (curve, weights, radii) in [
            (
                &engine.boost_nodes,
                &engine.boost_weights,
                &engine.boost_radii,
            ),
            (&engine.cut_nodes, &engine.cut_weights, &engine.cut_radii),
        ] {
            for k in 0..engine.fft_size / 2 {
                let freq = (k as f64 + 0.5) * bin_hz;
                assert_eq!(weights[k], weight_at(curve.nodes(), freq, 10.0, 22050.0));
                assert_eq!(
                    radii[k],
                    (radius_at(curve.nodes(), freq, default_radius).round() as usize).clamp(1, 12)
                );
            }
        }
    }

    #[test]
    fn cached_curves_follow_nodes_fft_sample_rate_and_default_radius() {
        let shared = Arc::new(Shared::new());
        let mut engine = Engine::new(Arc::clone(&shared), 44100.0);
        engine.update_strength_curves(3);
        assert_cached_curves(&engine, 3);
        let mut high = StrengthNode::new(1, 6000.0, 2.0);
        high.radius = 11;
        let mut low = StrengthNode::new(2, 500.0, 0.5);
        low.radius = 2;
        shared.publish_nodes(true, Arc::from([high.clone(), low.clone()]));
        for fft_size in FFT_SIZES {
            engine.set_fft_size(fft_size);
            for sample_rate in [44100.0, 96000.0] {
                engine.set_sample_rate(sample_rate);
                for default_radius in [1, 7, 12] {
                    engine.update_strength_curves(default_radius);
                    assert_cached_curves(&engine, default_radius);
                    engine.update_strength_curves(default_radius);
                    assert_cached_curves(&engine, default_radius);
                }
            }
        }
        low.weight = 0.0;
        low.radius = 9;
        shared.publish_nodes(true, Arc::from([high, low]));
        shared.publish_nodes(false, Arc::from([StrengthNode::new(3, 1200.0, 3.0)]));
        engine.update_strength_curves(4);
        assert_cached_curves(&engine, 4);
        shared.publish_nodes(true, Arc::from([]));
        engine.update_strength_curves(4);
        assert_cached_curves(&engine, 4);
    }
}
