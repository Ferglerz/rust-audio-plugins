use std::sync::Arc;

use crate::dsp::constants::{clamp_gr_db, MIN_DETECTOR_DB};
use crate::dsp::core_math::db_to_linear;
use crate::dsp::envelope::{EnvelopeEngine, EnvelopeParams};
use crate::dsp::filters::DetectionFilters;
use crate::dsp::gain_reduction::calculate_gain_reduction_from_db;
use crate::dsp::graph_snapshot::GraphSnapshot;
use crate::dsp::harmonics::{
    apply_harmonic_stereo, compute_harmonic_modulation, HarmonicParams, HarmonicProcessor,
};
use crate::dsp::limiter::SoftClipLimiter;
use crate::dsp::lookahead::LookaheadBuffer;
use crate::dsp::param_smooth::{
    smooth_envelope_params, smooth_one_minus, smooth_step, PARAM_SMOOTH_MS,
};
use crate::dsp::param_sync::BlockParamState;
use crate::dsp::rms::RmsDetector;

fn denormal_flush_hold_samples(srate: f64) -> usize {
    256.max((srate * 0.05).floor() as usize)
}

/// Runtime processing state derived from plugin parameters.
#[derive(Debug, Clone, Copy)]
pub struct ChainParams {
    pub strength_multiplier: f64,
    pub makeup_gain_linear: f64,
    pub input_offset_db: f64,
    pub detection_feedback: bool,
    pub rms_normalization: bool,
    pub rms_size_ms: f64,
    pub lookahead_ms: f64,
    pub brickwall_limiter: bool,
    pub mid_side_mode: bool,
    pub sc_adjust_preview: bool,
    pub use_sidechain: bool,
    pub harmonic_params: HarmonicParams,
    pub harmonics_on: bool,
    pub hold_ms: f64,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            strength_multiplier: 1.0,
            makeup_gain_linear: 1.0,
            input_offset_db: 0.0,
            detection_feedback: false,
            rms_normalization: false,
            rms_size_ms: 0.0,
            lookahead_ms: 0.0,
            brickwall_limiter: true,
            mid_side_mode: false,
            sc_adjust_preview: false,
            use_sidechain: false,
            harmonic_params: HarmonicParams {
                harmonic_type: 0,
                drive: 0.0,
                mix: 0.0,
                even_boost: 0.0,
                odd_boost: 0.0,
                harmonic_amount: 1.0,
            },
            harmonics_on: true,
            hold_ms: 0.0,
        }
    }
}

pub struct ProcessingChain {
    srate: f64,
    filters: DetectionFilters,
    detector_eq: super::detector_eq::DetectorEq,
    rms: RmsDetector,
    snapshot: Arc<GraphSnapshot>,
    envelope: EnvelopeEngine,
    lookahead: LookaheadBuffer,
    limiter: SoftClipLimiter,
    harmonics: HarmonicProcessor,
    params: ChainParams,
    final_prev_l: f64,
    final_prev_r: f64,
    block_detector_max_db: f64,
    block_gr_max_db: f64,
    curve_input_db: f64,
    listen_crossfade: f64,
    listen_fade_coeff: f64,
    preview_detect_l: f64,
    preview_detect_r: f64,
    listen_drag_active: bool,
    denormal_quiet_i: usize,
    denormal_flush_done: bool,
    denormal_flush_hold_samples: usize,
    last_target_gr_db: f64,
    cached_gr_db: f64,
    cached_gr_lin: f64,
    target_envelope_params: EnvelopeParams,
    smoothed_envelope_params: EnvelopeParams,
    param_smooth_one_minus: f64,
    target_makeup_gain_linear: f64,
    target_strength_multiplier: f64,
    target_input_offset_db: f64,
    target_lookahead_ms: f64,
    smooth_lookahead_ms: f64,
    cached_block_params: Option<BlockParamState>,
    reported_latency_samples: u32,
    param_smooth_counter: usize,
}

impl ProcessingChain {
    pub fn new(srate: f64) -> Self {
        let snapshot = Arc::new(GraphSnapshot::default());
        let chain = Self {
            srate,
            filters: DetectionFilters::new(),
            detector_eq: super::detector_eq::DetectorEq::new(srate),
            rms: RmsDetector::new(srate),
            snapshot,
            envelope: EnvelopeEngine::new(),
            lookahead: LookaheadBuffer::new(srate),
            limiter: SoftClipLimiter::new(),
            harmonics: HarmonicProcessor::new(srate),
            params: ChainParams::default(),
            final_prev_l: 0.0,
            final_prev_r: 0.0,
            block_detector_max_db: MIN_DETECTOR_DB,
            block_gr_max_db: 0.0,
            curve_input_db: -20.0,
            listen_crossfade: 0.0,
            listen_fade_coeff: 1.0 - (-1.0 / (0.025 * srate)).exp(),
            preview_detect_l: 0.0,
            preview_detect_r: 0.0,
            listen_drag_active: false,
            denormal_quiet_i: 0,
            denormal_flush_done: false,
            denormal_flush_hold_samples: denormal_flush_hold_samples(srate),
            last_target_gr_db: 0.0,
            cached_gr_db: 0.0,
            cached_gr_lin: 1.0,
            target_envelope_params: EnvelopeParams::default(),
            smoothed_envelope_params: EnvelopeParams::default(),
            param_smooth_one_minus: smooth_one_minus(srate, PARAM_SMOOTH_MS),
            target_makeup_gain_linear: 1.0,
            target_strength_multiplier: 1.0,
            target_input_offset_db: 0.0,
            target_lookahead_ms: 0.0,
            smooth_lookahead_ms: 0.0,
            cached_block_params: None,
            reported_latency_samples: 0,
            param_smooth_counter: 0,
        };
        chain
    }

    pub fn graph(&self) -> &crate::dsp::graph::CompressionGraph {
        &self.snapshot.graph
    }

    #[cfg(test)]
    pub fn set_test_snapshot(&mut self, snapshot: GraphSnapshot) {
        self.snapshot = Arc::new(snapshot);
    }

    #[cfg(test)]
    pub fn rebuild_lut(&mut self) {
        let graph = self.snapshot.graph;
        self.snapshot = Arc::new(GraphSnapshot::from_graph(graph));
    }

    pub fn set_envelope_params(&mut self, ep: EnvelopeParams) {
        self.target_envelope_params = ep;
        self.snap_smoothed_params();
    }

    pub fn set_chain_params(&mut self, p: ChainParams) {
        self.target_makeup_gain_linear = p.makeup_gain_linear;
        self.target_strength_multiplier = p.strength_multiplier;
        self.target_input_offset_db = p.input_offset_db;
        self.target_lookahead_ms = p.lookahead_ms;
        self.params = p;
        self.rms.update_rms_targets(p.rms_size_ms);
        self.snap_smoothed_params();
    }

    /// Snap smoothed runtime params to targets (e.g. on init / preset load).
    pub fn snap_runtime_params(&mut self) {
        self.snap_smoothed_params();
    }

    fn snap_smoothed_params(&mut self) {
        self.param_smooth_counter = 0;
        self.smoothed_envelope_params = self.target_envelope_params;
        self.envelope
            .sync_coefficients(&self.smoothed_envelope_params, self.srate);
        self.filters.snap_coefficients();
        self.params.makeup_gain_linear = self.target_makeup_gain_linear;
        self.params.strength_multiplier = self.target_strength_multiplier;
        self.params.input_offset_db = self.target_input_offset_db;
        self.params.hold_ms = self.smoothed_envelope_params.hold_ms;
        self.smooth_lookahead_ms = self.target_lookahead_ms;
        self.lookahead.update_delay_ms(self.smooth_lookahead_ms);
        self.reported_latency_samples = self.lookahead.latency_samples();
    }

    fn tick_param_smoothing(&mut self) {
        if self.param_smooth_counter == 0 {
            return;
        }
        self.param_smooth_counter -= 1;

        let om = self.param_smooth_one_minus;
        smooth_envelope_params(
            &mut self.smoothed_envelope_params,
            &self.target_envelope_params,
            om,
        );
        self.envelope
            .update_runtime_params(&self.smoothed_envelope_params, self.srate, om);
        if !self.filters.coefficients_converged() {
            self.filters.smooth_coefficients(om);
        }

        self.params.makeup_gain_linear = smooth_step(
            self.params.makeup_gain_linear,
            self.target_makeup_gain_linear,
            om,
        );
        self.params.strength_multiplier = smooth_step(
            self.params.strength_multiplier,
            self.target_strength_multiplier,
            om,
        );
        self.params.input_offset_db =
            smooth_step(self.params.input_offset_db, self.target_input_offset_db, om);
        self.params.hold_ms = self.smoothed_envelope_params.hold_ms;

        self.smooth_lookahead_ms =
            smooth_step(self.smooth_lookahead_ms, self.target_lookahead_ms, om);
        self.lookahead.update_delay_ms(self.smooth_lookahead_ms);
    }

    /// Load a prepared graph snapshot supplied by the plugin adapter.
    pub fn sync_graph(&mut self, published: Arc<GraphSnapshot>) {
        if Arc::ptr_eq(&self.snapshot, &published) {
            return;
        }
        self.snapshot = published;
    }

    /// Unconditional graph sync (init / preset load).
    pub fn force_sync_graph(&mut self, published: Arc<GraphSnapshot>) {
        self.snapshot = published;
    }

    fn apply_block_params(&mut self, block: &BlockParamState) {
        self.detector_eq.update(&block.detector_eq);
        self.filters.set_target_coefficients(
            block.hp_freq as f64,
            block.lp_freq as f64,
            self.srate,
        );

        self.target_envelope_params = block.envelope;
        self.envelope
            .set_target_coefficients(&block.envelope, self.srate);
        self.target_makeup_gain_linear = block.target_makeup_gain_linear;
        self.target_strength_multiplier = block.target_strength_multiplier;
        self.target_input_offset_db = block.target_input_offset_db;
        self.target_lookahead_ms = block.target_lookahead_ms;

        self.params.detection_feedback = block.detection_feedback;
        self.params.rms_normalization = block.rms_normalization;
        self.params.rms_size_ms = block.rms_size_ms;
        self.params.brickwall_limiter = block.brickwall_limiter;
        self.params.mid_side_mode = block.mid_side_mode;
        self.params.sc_adjust_preview = block.sc_adjust_preview;
        self.params.use_sidechain = block.use_sidechain;
        self.params.harmonic_params = block.harmonic_params;
        self.params.harmonics_on = block.harmonics_on;
        self.rms.update_rms_targets(self.params.rms_size_ms);
        self.param_smooth_counter = (PARAM_SMOOTH_MS * self.srate / 1000.0).ceil() as usize;
    }

    pub fn update_settings(&mut self, block: BlockParamState, snapshot: Arc<GraphSnapshot>) {
        self.sync_graph(snapshot);
        if self.cached_block_params.as_ref() == Some(&block) {
            return;
        }
        self.apply_block_params(&block);
        self.cached_block_params = Some(block);
    }

    /// Force block param refresh (init / preset load).
    pub fn force_update_settings(&mut self, block: BlockParamState, snapshot: Arc<GraphSnapshot>) {
        self.force_sync_graph(snapshot);
        self.apply_block_params(&block);
        self.cached_block_params = Some(block);
    }

    pub fn begin_process_block(&mut self) {
        self.block_detector_max_db = MIN_DETECTOR_DB;
        self.block_gr_max_db = 0.0;
    }

    pub fn block_meter_detector_db(&self) -> f64 {
        self.block_detector_max_db
    }

    pub fn block_meter_gr_db(&self) -> f64 {
        self.block_gr_max_db
    }

    pub fn latency_changed(&mut self) -> Option<u32> {
        let latency = self.lookahead.latency_samples();
        if latency != self.reported_latency_samples {
            self.reported_latency_samples = latency;
            Some(latency)
        } else {
            None
        }
    }

    pub fn latency_samples(&self) -> u32 {
        self.lookahead.latency_samples()
    }

    pub fn meter_detector_db(&self) -> f64 {
        self.curve_input_db
    }

    pub fn meter_gr_db(&self) -> f64 {
        self.envelope.global_smoothed_gain_db
    }

    pub fn meter_target_gr_db(&self) -> f64 {
        self.last_target_gr_db
    }

    pub fn meter_lut_threshold_db(&self) -> f64 {
        self.snapshot.lut.threshold()
    }

    pub fn reset(&mut self) {
        self.filters.reset();
        self.detector_eq.reset();
        self.rms.reset();
        self.envelope.reset();
        self.lookahead.reset();
        self.limiter.reset();
        self.harmonics.reset();
        self.final_prev_l = 0.0;
        self.final_prev_r = 0.0;
        self.block_detector_max_db = MIN_DETECTOR_DB;
        self.block_gr_max_db = 0.0;
        self.denormal_quiet_i = 0;
        self.denormal_flush_done = false;
        self.last_target_gr_db = 0.0;
        self.cached_gr_db = 0.0;
        self.cached_gr_lin = 1.0;
        self.snap_smoothed_params();
    }

    pub fn set_filter_preview_drag(&mut self, active: bool) {
        self.listen_drag_active = active;
    }

    fn tick_listen_crossfade(&mut self) {
        let target = if self.params.sc_adjust_preview && self.listen_drag_active {
            1.0
        } else {
            0.0
        };
        self.listen_crossfade += (target - self.listen_crossfade) * self.listen_fade_coeff;
    }

    fn prepare_detection_inputs(
        &mut self,
        spl0: f64,
        spl1: f64,
        sidechain: Option<(f64, f64)>,
    ) -> (f64, f64, f64, f64) {
        let p = &self.params;
        let (mut proc_l, mut proc_r) = (spl0, spl1);
        if p.mid_side_mode {
            proc_l = (spl0 + spl1) * 0.5;
            proc_r = (spl0 - spl1) * 0.5;
        }

        let proc_peak = proc_l.abs().max(proc_r.abs());
        if proc_peak < 1e-10 {
            self.denormal_quiet_i += 1;
            if self.denormal_quiet_i >= self.denormal_flush_hold_samples
                && !self.denormal_flush_done
            {
                self.filters.flush_denormals();
                self.rms.flush_line_state();
                self.envelope.flush_denormals();
                self.limiter.flush_denormals();
                self.harmonics.flush_denormals();
                self.denormal_flush_done = true;
            }
        } else {
            self.denormal_quiet_i = 0;
            self.denormal_flush_done = false;
        }

        let (mut detect_l, mut detect_r) = if p.detection_feedback {
            (self.final_prev_l, self.final_prev_r)
        } else {
            (proc_l, proc_r)
        };

        if p.use_sidechain {
            if let Some((sc_l, sc_r)) = sidechain {
                detect_l = sc_l;
                detect_r = sc_r;
            }
        }

        (proc_l, proc_r, detect_l, detect_r)
    }

    fn detect_and_apply_gr(&mut self, mut detect_l: f64, mut detect_r: f64) -> f64 {
        let p = &self.params;

        (detect_l, detect_r) = self.filters.apply(detect_l, detect_r);
        [detect_l, detect_r] = self.detector_eq.process([detect_l, detect_r]);
        self.preview_detect_l = detect_l;
        self.preview_detect_r = detect_r;
        if !self.rms.coefficients_converged() {
            self.rms.smooth_coefficients();
        }

        let (_detector_level, detector_level_db) =
            self.rms
                .detect_level(detect_l, detect_r, p.rms_normalization);

        let gr_result = calculate_gain_reduction_from_db(
            &self.snapshot.lut,
            detector_level_db,
            p.input_offset_db,
            p.strength_multiplier,
        );
        let curve_input_db = gr_result.curve_input_db;
        self.curve_input_db = curve_input_db;
        self.block_detector_max_db = self.block_detector_max_db.max(curve_input_db);

        let target_gr_db = gr_result.target_gr_db;
        self.last_target_gr_db = target_gr_db;

        let can_skip_envelope =
            target_gr_db.abs() < 0.01 && self.envelope.global_smoothed_gain_db.abs() < 0.01;
        if can_skip_envelope {
            self.envelope.global_smoothed_gain_db = 0.0;
            self.envelope.global_smoothed_gain_db_before_strength = 0.0;
        } else {
            self.envelope.process_envelope_following(
                target_gr_db,
                curve_input_db,
                p.strength_multiplier,
                p.hold_ms,
            );
        }

        self.envelope.prev_detector_db = curve_input_db;

        let current_gr_db = clamp_gr_db(self.envelope.global_smoothed_gain_db);

        if current_gr_db.abs() > self.block_gr_max_db.abs() {
            self.block_gr_max_db = current_gr_db;
        }

        current_gr_db
    }

    fn apply_gain_and_output(
        &mut self,
        proc_l: f64,
        proc_r: f64,
        current_gr_db: f64,
        target_gr_db: f64,
    ) -> (f64, f64) {
        let p = &self.params;
        let (audio_l, audio_r) = self.lookahead.process(proc_l, proc_r);

        let (mut processed_l, mut processed_r) = if current_gr_db.abs() < 1e-6 {
            self.cached_gr_db = 0.0;
            self.cached_gr_lin = 1.0;
            (audio_l, audio_r)
        } else {
            if (current_gr_db - self.cached_gr_db).abs() > 1e-7 {
                self.cached_gr_lin = db_to_linear(current_gr_db);
                self.cached_gr_db = current_gr_db;
            }
            (audio_l * self.cached_gr_lin, audio_r * self.cached_gr_lin)
        };

        let hp = &p.harmonic_params;
        if p.harmonics_on && hp.mix > 0.0001 && target_gr_db.abs() > 0.0001 {
            let (combined_factor, intensity, _envelope_amount) =
                compute_harmonic_modulation(target_gr_db, self.envelope.global_smoothed_gain_db);
            (processed_l, processed_r) = apply_harmonic_stereo(
                &mut self.harmonics,
                processed_l,
                processed_r,
                hp,
                combined_factor,
                intensity,
            );
        }

        let mut final_l = processed_l * p.makeup_gain_linear;
        let mut final_r = processed_r * p.makeup_gain_linear;

        if p.brickwall_limiter {
            (final_l, final_r) = self.limiter.process_stereo(final_l, final_r);
        }

        let (mut out_l, mut out_r) = if p.mid_side_mode {
            (final_l + final_r, final_l - final_r)
        } else {
            (final_l, final_r)
        };

        let lf = self.listen_crossfade;
        if lf > 0.001 {
            let om = 1.0 - lf;
            out_l = out_l * om + self.preview_detect_l * lf;
            out_r = out_r * om + self.preview_detect_r * lf;
        }

        self.final_prev_l = out_l;
        self.final_prev_r = out_r;

        (out_l, out_r)
    }

    pub fn process_sample(
        &mut self,
        spl0: f64,
        spl1: f64,
        sidechain: Option<(f64, f64)>,
    ) -> (f64, f64) {
        self.tick_listen_crossfade();
        self.tick_param_smoothing();

        let (proc_l, proc_r, detect_l, detect_r) =
            self.prepare_detection_inputs(spl0, spl1, sidechain);
        let current_gr_db = self.detect_and_apply_gr(detect_l, detect_r);
        let target_gr_db = self.last_target_gr_db;
        self.apply_gain_and_output(proc_l, proc_r, current_gr_db, target_gr_db)
    }

    pub fn process_block(
        &mut self,
        channels: &mut [&mut [f32]],
        sc_l: Option<&[f32]>,
        sc_r: Option<&[f32]>,
    ) {
        if channels.is_empty() {
            return;
        }
        let n = channels[0].len();

        let sc_len = match (sc_l, sc_r) {
            (Some(l), Some(r)) => l.len().min(r.len()).min(n),
            _ => 0,
        };

        if channels.len() > 1 {
            let (left, right) = channels.split_at_mut(1);
            let l_slice = &mut left[0][..n];
            let r_slice = &mut right[0][..n];
            if sc_len > 0 {
                let sc_l = sc_l.unwrap();
                let sc_r = sc_r.unwrap();
                for i in 0..sc_len {
                    let (ol, or) = self.process_sample(
                        l_slice[i] as f64,
                        r_slice[i] as f64,
                        Some((sc_l[i] as f64, sc_r[i] as f64)),
                    );
                    l_slice[i] = ol as f32;
                    r_slice[i] = or as f32;
                }
                for i in sc_len..n {
                    let (ol, or) = self.process_sample(l_slice[i] as f64, r_slice[i] as f64, None);
                    l_slice[i] = ol as f32;
                    r_slice[i] = or as f32;
                }
            } else {
                for i in 0..n {
                    let (ol, or) = self.process_sample(l_slice[i] as f64, r_slice[i] as f64, None);
                    l_slice[i] = ol as f32;
                    r_slice[i] = or as f32;
                }
            }
        } else {
            let mono = &mut channels[0][..n];
            if sc_len > 0 {
                let sc_l = sc_l.unwrap();
                let sc_r = sc_r.unwrap();
                for i in 0..sc_len {
                    let sc = (sc_l[i] as f64 + sc_r[i] as f64) * 0.5;
                    let (out, _) =
                        self.process_sample(mono[i] as f64, mono[i] as f64, Some((sc, sc)));
                    mono[i] = out as f32;
                }
                for i in sc_len..n {
                    let s = mono[i] as f64;
                    let (out, _) = self.process_sample(s, s, None);
                    mono[i] = out as f32;
                }
            } else {
                for i in 0..n {
                    let s = mono[i] as f64;
                    let (out, _) = self.process_sample(s, s, None);
                    mono[i] = out as f32;
                }
            }
        }
    }

    /// Process a mono buffer in-place (for integration tests matching Python reference).
    pub fn process_block_mono(&mut self, samples: &mut [f64]) {
        for x in samples.iter_mut() {
            let (out, _) = self.process_sample(*x, *x, None);
            *x = out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::envelope::EnvelopeParams;
    use crate::dsp::graph::CompressionGraph;

    fn bent_chain(srate: f64) -> ProcessingChain {
        let mut chain = ProcessingChain::new(srate);
        let mut g = CompressionGraph::new();
        g.adjust_interior_output_y(1, 14.0);
        g.finalize_after_edit_no_sort();
        chain.set_test_snapshot(GraphSnapshot::from_graph(g));
        chain.set_envelope_params(EnvelopeParams {
            attack: 50000.0,
            release_ms: 50.0,
            strength: 400.0,
            ..EnvelopeParams::default()
        });
        chain.set_chain_params(ChainParams {
            strength_multiplier: 4.0,
            ..ChainParams::default()
        });
        chain.reset();
        chain
    }

    #[test]
    fn test_compressor_reduces_loud_input() {
        let mut chain = bent_chain(48000.0);
        let n = 48000;
        let mut x = vec![0.0; n];
        for val in x.iter_mut().take(3 * n / 4).skip(n / 4) {
            *val = 0.2;
        }
        let y = {
            let mut out = x.clone();
            chain.process_block_mono(&mut out);
            out
        };
        let mid = 2 * n / 3;
        let rms_in: f64 = x[mid - 2000..mid].iter().map(|v| v * v).sum::<f64>() / 2000.0;
        let rms_out: f64 = y[mid - 2000..mid].iter().map(|v| v * v).sum::<f64>() / 2000.0;
        let rms_in = rms_in.sqrt();
        let rms_out = rms_out.sqrt();
        assert!(rms_out < rms_in * 0.98, "rms_in={rms_in} rms_out={rms_out}");
    }

    #[test]
    fn harmonics_bypass_keeps_the_clean_output() {
        let mut chain = ProcessingChain::new(48_000.0);
        chain.params.brickwall_limiter = false;
        chain.params.harmonic_params.mix = 1.0;
        chain.params.harmonic_params.drive = 100.0;
        chain.envelope.global_smoothed_gain_db = -6.0;
        chain.params.harmonics_on = false;
        let dry = chain.apply_gain_and_output(0.25, 0.25, 0.0, -6.0);
        assert_eq!(dry, (0.25, 0.25));

        chain.params.harmonics_on = true;
        let wet = chain.apply_gain_and_output(0.25, 0.25, 0.0, -6.0);
        assert!((wet.0 - dry.0).abs() > 1e-6);
    }

    #[test]
    fn expansion_onset_aligns_with_graph_point() {
        use super::super::test_fixtures::{run_peak_sine_until_settled, setup_expansion_chain};

        let mut chain = setup_expansion_chain(48000.0);
        run_peak_sine_until_settled(&mut chain, -16.0, 8000);

        let curve_in = chain.meter_detector_db();
        let target_gr = chain.meter_target_gr_db();
        assert!(
            (curve_in - (-16.0)).abs() < 0.5,
            "meter should track -16 dBFS input, got {curve_in}"
        );
        assert!(
            target_gr > 2.0,
            "expect upward GR at expansion point, got {target_gr}"
        );
    }

    #[test]
    fn test_trace_shapes() {
        let mut chain = ProcessingChain::new(8000.0);
        let n = 4000;
        let x: Vec<f64> = (0..n)
            .map(|i| 0.1 * (40.0 * std::f64::consts::PI * i as f64 / n as f64).sin())
            .collect();
        let y = {
            let mut out = x.clone();
            chain.process_block_mono(&mut out);
            out
        };
        assert_eq!(y.len(), x.len());
    }

    #[test]
    fn sidechain_applies_for_partial_buffer_length() {
        let mut chain = ProcessingChain::new(48000.0);
        chain.set_chain_params(ChainParams {
            use_sidechain: true,
            ..ChainParams::default()
        });
        let mut l = vec![0.5f32; 4];
        let mut r = vec![0.5f32; 4];
        let sc_l = vec![0.05f32; 2];
        let sc_r = vec![0.05f32; 2];
        let mut channels: Vec<&mut [f32]> = vec![&mut l, &mut r];
        chain.process_block(&mut channels, Some(&sc_l), Some(&sc_r));
        assert!(l.iter().all(|s| s.is_finite()));
        assert!(r.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn silence_block_meter_stays_finite() {
        let mut chain = ProcessingChain::new(48000.0);
        chain.begin_process_block();
        assert_eq!(chain.block_meter_detector_db(), MIN_DETECTOR_DB);
        let (out_l, out_r) = chain.process_sample(0.0, 0.0, None);
        assert!(out_l.abs() < 1e-6);
        assert!(out_r.abs() < 1e-6);
        let det = chain.block_meter_detector_db();
        assert!(det.is_finite());
        assert!(det <= -100.0, "silent input must not paint a full-scale graph histogram: {det}");
    }
}
