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
use crate::dsp::lookahead::LookaheadBuffer;
use crate::dsp::param_smooth::{smooth_envelope_params, smooth_step, PARAM_SMOOTH_MS};
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
    harmonics: HarmonicProcessor,
    params: ChainParams,
    final_prev_l: f64,
    final_prev_r: f64,
    block_detector_max_db: f64,
    block_input_max_db: f64,
    block_input_rate_activity: f64,
    block_input_dependence_activity: f64,
    block_gr_dependence_activity: f64,
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
    target_makeup_gain_linear: f64,
    target_strength_multiplier: f64,
    target_input_offset_db: f64,
    target_harmonics: HarmonicParams,
    harmonic_ramp_left: usize,
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
            harmonics: HarmonicProcessor::new(srate),
            params: ChainParams::default(),
            final_prev_l: 0.0,
            final_prev_r: 0.0,
            block_detector_max_db: MIN_DETECTOR_DB,
            block_input_max_db: MIN_DETECTOR_DB,
            block_input_rate_activity: 0.0,
            block_input_dependence_activity: 0.0,
            block_gr_dependence_activity: 0.0,
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
            target_makeup_gain_linear: 1.0,
            target_strength_multiplier: 1.0,
            target_input_offset_db: 0.0,
            target_harmonics: ChainParams::default().harmonic_params,
            harmonic_ramp_left: 0,
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
        self.target_harmonics = p.harmonic_params;
        self.lookahead.update_delay_ms(p.lookahead_ms);
        if self.params.mid_side_mode != p.mid_side_mode {
            self.harmonics.reset();
        }
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
        self.params.harmonic_params = self.target_harmonics;
        self.harmonic_ramp_left = 0;
        self.params.hold_ms = self.smoothed_envelope_params.hold_ms;
    }

    fn tick_param_smoothing(&mut self) {
        if self.harmonic_ramp_left > 0 {
            let step = 1.0 / self.harmonic_ramp_left as f64;
            let current = &mut self.params.harmonic_params;
            let target = self.target_harmonics;
            current.drive = smooth_step(current.drive, target.drive, step);
            current.mix = smooth_step(current.mix, target.mix, step);
            current.even_boost = smooth_step(current.even_boost, target.even_boost, step);
            current.odd_boost = smooth_step(current.odd_boost, target.odd_boost, step);
            current.harmonic_amount =
                smooth_step(current.harmonic_amount, target.harmonic_amount, step);
            self.harmonic_ramp_left -= 1;
        }
        if self.param_smooth_counter == 0 {
            return;
        }
        let step = 1.0 / self.param_smooth_counter as f64;
        self.param_smooth_counter -= 1;

        smooth_envelope_params(
            &mut self.smoothed_envelope_params,
            &self.target_envelope_params,
            step,
        );
        self.envelope
            .update_runtime_params(&self.smoothed_envelope_params, self.srate, step);
        if self.param_smooth_counter == 0 {
            self.filters.snap_coefficients();
        } else if !self.filters.coefficients_converged() {
            self.filters.smooth_coefficients(step);
        }

        self.params.makeup_gain_linear = smooth_step(
            self.params.makeup_gain_linear,
            self.target_makeup_gain_linear,
            step,
        );
        self.params.strength_multiplier = smooth_step(
            self.params.strength_multiplier,
            self.target_strength_multiplier,
            step,
        );
        self.params.input_offset_db = smooth_step(
            self.params.input_offset_db,
            self.target_input_offset_db,
            step,
        );
        self.params.hold_ms = self.smoothed_envelope_params.hold_ms;
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
        // Audition switches, detector EQ edits and lookahead have independent
        // transitions. They must not restart an in-flight parameter ramp.
        let ramp_changed = self.cached_block_params.as_ref().is_none_or(|previous| {
            previous.hp_freq != block.hp_freq
                || previous.lp_freq != block.lp_freq
                || previous.envelope != block.envelope
                || previous.target_makeup_gain_linear != block.target_makeup_gain_linear
                || previous.target_strength_multiplier != block.target_strength_multiplier
                || previous.target_input_offset_db != block.target_input_offset_db
        });
        if ramp_changed {
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
            self.param_smooth_counter = (PARAM_SMOOTH_MS * self.srate / 1000.0).ceil() as usize;
        }
        self.lookahead.update_delay_ms(block.target_lookahead_ms);

        self.params.detection_feedback = block.detection_feedback;
        self.params.rms_normalization = block.rms_normalization;
        self.params.rms_size_ms = block.rms_size_ms;
        if self.params.mid_side_mode != block.mid_side_mode {
            self.harmonics.reset();
        }
        self.params.mid_side_mode = block.mid_side_mode;
        self.params.sc_adjust_preview = block.sc_adjust_preview;
        self.params.use_sidechain = block.use_sidechain;
        self.params.harmonic_params.harmonic_type = block.harmonic_params.harmonic_type;
        self.target_harmonics.harmonic_type = block.harmonic_params.harmonic_type;
        if self.target_harmonics != block.harmonic_params {
            self.target_harmonics = block.harmonic_params;
            self.harmonic_ramp_left = (PARAM_SMOOTH_MS * self.srate / 1000.0).ceil() as usize;
        }
        self.params.harmonics_on = block.harmonics_on;
        self.rms.update_rms_targets(self.params.rms_size_ms);
    }

    pub fn update_settings(&mut self, block: BlockParamState, snapshot: Arc<GraphSnapshot>) {
        self.sync_graph(snapshot);
        self.update_block_params(block);
    }

    pub fn update_block_params(&mut self, block: BlockParamState) {
        if self.cached_block_params.as_ref() == Some(&block) {
            return;
        }
        self.apply_block_params(&block);
        self.cached_block_params = Some(block);
    }

    /// Force block param refresh (init / preset load).
    pub fn force_update_settings(&mut self, block: BlockParamState, snapshot: Arc<GraphSnapshot>) {
        self.force_sync_graph(snapshot);
        self.cached_block_params = None;
        self.apply_block_params(&block);
        self.cached_block_params = Some(block);
    }

    pub(crate) fn set_activity_metering(&mut self, enabled: bool) {
        self.envelope.set_activity_metering(enabled);
    }

    pub fn begin_process_block(&mut self) {
        self.block_detector_max_db = MIN_DETECTOR_DB;
        self.block_input_max_db = MIN_DETECTOR_DB;
        self.block_input_rate_activity = 0.0;
        self.block_input_dependence_activity = 0.0;
        self.block_gr_dependence_activity = 0.0;
        self.block_gr_max_db = 0.0;
    }

    pub fn block_meter_detector_db(&self) -> f64 {
        self.block_detector_max_db
    }

    /// Detector level after sidechain EQ and RMS detection, before input offset.
    pub fn block_meter_input_db(&self) -> f64 {
        self.block_input_max_db
    }

    pub fn block_meter_input_rate_activity(&self) -> f64 {
        self.block_input_rate_activity
    }

    pub fn block_meter_input_dependence_activity(&self) -> f64 {
        self.block_input_dependence_activity
    }
    pub fn block_meter_gr_dependence_activity(&self) -> f64 {
        self.block_gr_dependence_activity
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
        self.harmonics.reset();
        self.final_prev_l = 0.0;
        self.final_prev_r = 0.0;
        self.block_detector_max_db = MIN_DETECTOR_DB;
        self.block_input_max_db = MIN_DETECTOR_DB;
        self.block_input_rate_activity = 0.0;
        self.block_input_dependence_activity = 0.0;
        self.block_gr_dependence_activity = 0.0;
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

    fn processing_channels(mid_side: bool, left: f64, right: f64) -> (f64, f64) {
        if mid_side {
            ((left + right) * 0.5, (left - right) * 0.5)
        } else {
            (left, right)
        }
    }

    fn prepare_detection_inputs(
        &mut self,
        left: f64,
        right: f64,
        sidechain: Option<(f64, f64)>,
    ) -> (f64, f64) {
        let internal = if self.params.detection_feedback {
            (self.final_prev_l, self.final_prev_r)
        } else {
            Self::processing_channels(self.params.mid_side_mode, left, right)
        };
        let detector = if self.params.use_sidechain {
            sidechain.unwrap_or(internal)
        } else {
            internal
        };
        let peak = left
            .abs()
            .max(right.abs())
            .max(detector.0.abs())
            .max(detector.1.abs());
        if peak < 1e-10 {
            self.denormal_quiet_i = self.denormal_quiet_i.saturating_add(1);
            if self.denormal_quiet_i >= self.denormal_flush_hold_samples
                && !self.denormal_flush_done
            {
                self.filters.flush_denormals();
                self.rms.flush_line_state();
                self.envelope.flush_denormals();
                self.harmonics.flush_denormals();
                self.denormal_flush_done = true;
            }
        } else {
            self.denormal_quiet_i = 0;
            self.denormal_flush_done = false;
        }

        detector
    }

    fn detect_and_apply_gr(&mut self, mut detect_l: f64, mut detect_r: f64) -> f64 {
        let p = &self.params;

        (detect_l, detect_r) = self.filters.apply(detect_l, detect_r);
        [detect_l, detect_r] = self.detector_eq.process([detect_l, detect_r]);
        self.preview_detect_l = detect_l;
        self.preview_detect_r = detect_r;
        self.rms.smooth_coefficients();

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
        self.block_input_max_db = self.block_input_max_db.max(detector_level_db);

        let target_gr_db = gr_result.target_gr_db;
        self.last_target_gr_db = target_gr_db;

        let can_skip_envelope = target_gr_db.abs() < 0.01
            && self.envelope.global_smoothed_gain_db.abs() < 0.01
            && !self.envelope.has_pending_hold();
        if can_skip_envelope {
            self.envelope.input_rate_activity = 0.0;
            self.envelope.input_dependence_activity = 0.0;
            self.envelope.gr_dependence_activity = 0.0;
            self.envelope.global_smoothed_gain_db = 0.0;
            self.envelope.global_smoothed_gain_db_before_strength = 0.0;
        } else {
            self.envelope.process_envelope_following(
                target_gr_db,
                detector_level_db,
                p.strength_multiplier,
                p.hold_ms,
            );
        }

        if self.envelope.activity_metering() {
            if self.envelope.input_rate_activity.abs() > self.block_input_rate_activity.abs() {
                self.block_input_rate_activity = self.envelope.input_rate_activity;
            }
            if self.envelope.input_dependence_activity.abs()
                > self.block_input_dependence_activity.abs()
            {
                self.block_input_dependence_activity = self.envelope.input_dependence_activity;
            }
            if self.envelope.gr_dependence_activity.abs() > self.block_gr_dependence_activity.abs()
            {
                self.block_gr_dependence_activity = self.envelope.gr_dependence_activity;
            }
        }
        self.envelope.prev_detector_db = detector_level_db;

        let current_gr_db = clamp_gr_db(self.envelope.global_smoothed_gain_db);

        if current_gr_db.abs() > self.block_gr_max_db.abs() {
            self.block_gr_max_db = current_gr_db;
        }

        current_gr_db
    }

    fn apply_gain_and_output(
        &mut self,
        left: f64,
        right: f64,
        current_gr_db: f64,
        target_gr_db: f64,
    ) -> (f64, f64) {
        let p = &self.params;
        let (left, right) = self.lookahead.process(left, right);
        let (audio_l, audio_r) = Self::processing_channels(p.mid_side_mode, left, right);

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
        } else {
            self.harmonics.track_dry(processed_l, processed_r);
        }

        let final_l = processed_l * p.makeup_gain_linear;
        let final_r = processed_r * p.makeup_gain_linear;

        let (mut out_l, mut out_r) = if p.mid_side_mode {
            (final_l + final_r, final_l - final_r)
        } else {
            (final_l, final_r)
        };

        self.final_prev_l = out_l;
        self.final_prev_r = out_r;

        let lf = self.listen_crossfade;
        if lf > 0.001 {
            let om = 1.0 - lf;
            out_l = out_l * om + self.preview_detect_l * lf;
            out_r = out_r * om + self.preview_detect_r * lf;
        }

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

        let (detect_l, detect_r) = self.prepare_detection_inputs(spl0, spl1, sidechain);
        let current_gr_db = self.detect_and_apply_gr(detect_l, detect_r);
        let target_gr_db = self.last_target_gr_db;
        self.apply_gain_and_output(spl0, spl1, current_gr_db, target_gr_db)
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

    fn host_settings() -> BlockParamState {
        crate::parameter_adapter::capture_block_state(&crate::params::ComposureParams::default())
    }

    #[test]
    fn harmonic_mix_retargets_from_its_current_sample() {
        let mut chain = ProcessingChain::new(1000.0);
        let mut block = host_settings();
        block.harmonic_params.mix = 0.0;
        chain.update_block_params(block.clone());
        chain.snap_runtime_params();
        block.harmonic_params.mix = 1.0;
        chain.update_block_params(block.clone());
        for _ in 0..5 {
            chain.process_sample(0.0, 0.0, None);
        }
        assert!((chain.params.harmonic_params.mix - 0.5).abs() < 1e-12);
        block.harmonic_params.mix = 0.0;
        chain.update_block_params(block);
        for frame in 1..=10 {
            chain.process_sample(0.0, 0.0, None);
            let expected = 0.5 - 0.05 * frame as f64;
            assert!((chain.params.harmonic_params.mix - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn live_automation_reaches_requested_gain_timing_filters_and_latency() {
        for rate in [44_100.0, 48_000.0, 96_000.0] {
            let mut chain = ProcessingChain::new(rate);
            let mut block = host_settings();
            chain.update_block_params(block.clone());
            chain.snap_runtime_params();
            for _ in 0..(rate * 0.15) as usize {
                chain.process_sample(0.125, -0.25, None);
            }

            block.target_makeup_gain_linear = db_to_linear(12.0);
            block.target_strength_multiplier = 0.25;
            block.target_input_offset_db = 13.5;
            block.target_lookahead_ms = 100.0;
            block.envelope.attack = 9000.0;
            block.envelope.release_ms = 220.0;
            block.envelope.hold_ms = 100.0;
            block.lp_freq = 8000.0;
            chain.update_block_params(block.clone());
            assert_eq!(chain.latency_changed(), Some((rate / 10.0) as u32));
            assert_eq!(chain.latency_changed(), None);

            let ramp = (PARAM_SMOOTH_MS * rate / 1000.0).ceil() as usize;
            for i in 0..ramp * 3 {
                if i == ramp / 2 {
                    block.sc_adjust_preview = !block.sc_adjust_preview;
                    chain.update_block_params(block.clone());
                }
                let (left, right) = chain.process_sample(0.125, -0.25, None);
                assert!(left >= 0.125 - 1e-14 && right <= -0.25 + 1e-14);
                if i + 1 >= ramp {
                    assert!((left - 0.125 * block.target_makeup_gain_linear).abs() < 1e-14);
                    assert_eq!(chain.params.strength_multiplier, 0.25);
                    assert_eq!(chain.params.input_offset_db, 13.5);
                    assert_eq!(chain.smoothed_envelope_params, block.envelope);
                    assert!(chain.filters.coefficients_converged());
                }
            }
        }
    }

    #[test]
    fn feedback_detector_ignores_the_audition_mix() {
        let mut block = host_settings();
        block.detection_feedback = true;
        block.sc_adjust_preview = true;
        block.detector_eq[0] = pleasant_eq::BandSettings {
            enabled: true,
            shape: pleasant_eq::EqShape::LowShelf,
            frequency_hz: 1000.0,
            gain_db: 6.0,
            ..Default::default()
        };
        let mut clean = ProcessingChain::new(48_000.0);
        let mut audition = ProcessingChain::new(48_000.0);
        for chain in [&mut clean, &mut audition] {
            chain.update_block_params(block.clone());
            chain.snap_runtime_params();
        }
        audition.set_filter_preview_drag(true);
        let mut heard_eq = false;
        for i in 0..4800 {
            let x = if i < 800 {
                0.01 * (std::f64::consts::TAU * 440.0 * i as f64 / 48_000.0).sin()
            } else {
                0.0
            };
            let dry = clean.process_sample(x, x, None);
            let monitored = audition.process_sample(x, x, None);
            assert_eq!(clean.meter_detector_db(), audition.meter_detector_db());
            assert!(monitored.0.abs() < 0.03);
            heard_eq |= (dry.0 - monitored.0).abs() > 1e-4;
            if i == 4799 {
                assert!(monitored.0.abs() < 1e-12);
            }
        }
        assert!(heard_eq);
    }

    #[test]
    fn silent_program_keeps_an_active_external_rms_key() {
        let mut chain = ProcessingChain::new(48_000.0);
        chain.set_chain_params(ChainParams {
            rms_size_ms: 100.0,
            use_sidechain: true,
            ..ChainParams::default()
        });
        for _ in 0..96_000 {
            chain.process_sample(0.25, 0.25, Some((1.0, 1.0)));
        }
        for _ in 0..4800 {
            chain.process_sample(0.0, 0.0, Some((1.0, 1.0)));
            assert!(chain.meter_detector_db().abs() < 1e-6);
        }
    }

    #[test]
    fn queued_stereo_audio_survives_mid_side_switches_in_both_directions() {
        let mut chain = ProcessingChain::new(48_000.0);
        let mut block = host_settings();
        block.target_lookahead_ms = 10.0;
        chain.update_block_params(block.clone());
        chain.snap_runtime_params();
        let frame = (0.125, 0.5);
        for _ in 0..480 {
            assert_eq!(chain.process_sample(frame.0, frame.1, None), (0.0, 0.0));
        }
        for mode in [true, false] {
            block.mid_side_mode = mode;
            chain.update_block_params(block.clone());
            for _ in 0..481 {
                assert_eq!(chain.process_sample(frame.0, frame.1, None), frame);
            }
        }
    }

    #[test]
    fn independent_influences_reach_zero_after_automation_ramp() {
        for srate in [44100.0, 48000.0, 96000.0, 192000.0] {
            let mut chain = ProcessingChain::new(srate);
            chain.set_envelope_params(EnvelopeParams {
                input_dependence: 100.0,
                gr_dependence: 100.0,
                input_rate_amount: 5.0,
                ..EnvelopeParams::default()
            });
            chain.target_envelope_params.input_dependence = 0.0;
            chain.target_envelope_params.gr_dependence = 0.0;
            chain.target_envelope_params.input_rate_amount = 0.0;
            let samples = (PARAM_SMOOTH_MS * srate / 1000.0).ceil() as usize;
            chain.param_smooth_counter = samples;
            for _ in 0..samples {
                chain.process_sample(0.1, 0.1, None);
            }
            assert_eq!(chain.smoothed_envelope_params.input_dependence, 0.0);
            assert_eq!(chain.smoothed_envelope_params.gr_dependence, 0.0);
            assert_eq!(chain.smoothed_envelope_params.input_rate_amount, 0.0);
            assert_eq!(chain.envelope.input_rate_activity, 0.0);
            assert_eq!(chain.envelope.input_dependence_activity, 0.0);
            assert_eq!(chain.envelope.gr_dependence_activity, 0.0);
        }
    }

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
    fn clean_output_and_makeup_are_not_limited_or_soft_clipped() {
        let mut chain = ProcessingChain::new(48_000.0);
        for (left, right) in [(0.25, -0.5), (1.25, -1.5), (2.0, -3.0)] {
            assert_eq!(
                chain.apply_gain_and_output(left, right, 0.0, 0.0),
                (left, right)
            );
        }
        chain.params.makeup_gain_linear = 2.0;
        assert_eq!(
            chain.apply_gain_and_output(0.75, -0.8, 0.0, 0.0),
            (1.5, -1.6)
        );
    }

    #[test]
    fn harmonics_bypass_keeps_the_clean_output() {
        let mut chain = ProcessingChain::new(48_000.0);
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
        assert!(
            det <= -100.0,
            "silent input must not paint a full-scale graph histogram: {det}"
        );
    }

    #[test]
    fn input_meter_and_program_detector_remain_before_offset() {
        let mut plain = ProcessingChain::new(48000.0);
        let mut offset = ProcessingChain::new(48000.0);
        offset.params.input_offset_db = 12.0;
        plain.begin_process_block();
        offset.begin_process_block();
        for _ in 0..100 {
            plain.detect_and_apply_gr(0.1, 0.1);
            offset.detect_and_apply_gr(0.1, 0.1);
        }
        assert_eq!(plain.block_meter_input_db(), offset.block_meter_input_db());
        assert_eq!(
            plain.envelope.prev_detector_db,
            offset.envelope.prev_detector_db
        );
        assert!(
            (offset.block_meter_detector_db() - plain.block_meter_detector_db() - 12.0).abs()
                < 1e-9
        );
    }
}
