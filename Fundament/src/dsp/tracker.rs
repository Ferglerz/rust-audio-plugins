//! Stable voices from per-frame F0 candidates.

use super::salience::harmonics_for;
use super::types::{EngineSettings, F0Candidate, Peak, VoiceState, MAX_VOICES};
use std::f32::consts::LN_2;

const MATCH_CENTS: f32 = 80.0;
const PARTIAL_TOL_CENTS: f32 = 40.0;
const PRESENCE_FLOOR: f32 = 1e-3;
/// Unmatched active voices keep target presence while `miss` stays below this.
const HOLD_MISSES: u32 = 2;
const BIRTH_FRAMES: u16 = 2;

#[derive(Clone, Copy, Default)]
struct Pending {
    f0_hz: f32,
    frames: u16,
}

pub struct VoiceTracker {
    voices: [VoiceState; MAX_VOICES],
    miss: [u32; MAX_VOICES],
    matched: [bool; MAX_VOICES],
    pending: [Pending; MAX_VOICES],
    pending_len: usize,
    next_id: u32,
}

impl Default for VoiceTracker {
    fn default() -> Self {
        Self {
            voices: [VoiceState::default(); MAX_VOICES],
            miss: [0; MAX_VOICES],
            matched: [false; MAX_VOICES],
            pending: [Pending::default(); MAX_VOICES],
            pending_len: 0,
            next_id: 1,
        }
    }
}

impl VoiceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears voices and pending births. `next_id` keeps counting so reused slots stay unique.
    pub fn reset(&mut self) {
        let next_id = self.next_id;
        *self = Self::default();
        self.next_id = next_id;
    }

    pub fn update(
        &mut self,
        candidates: &[F0Candidate],
        peaks: &[Peak],
        settings: &EngineSettings,
        frame_dt: f32,
    ) {
        let limit = settings.max_voices.min(MAX_VOICES);
        self.matched = [false; MAX_VOICES];

        let mut targets = [0.0_f32; MAX_VOICES];
        let mut claimed = [0usize; MAX_VOICES];
        let claimed_len = self.match_actives(candidates, limit, &mut targets, &mut claimed);
        self.birth_unclaimed(candidates, &claimed[..claimed_len], limit, &mut targets);

        for slot in limit..MAX_VOICES {
            targets[slot] = 0.0;
            self.matched[slot] = false;
        }

        self.smooth_presence(&targets, settings, frame_dt);
        self.refresh_matched_partials(peaks, settings, limit);
    }

    pub fn voices(&self) -> &[VoiceState; MAX_VOICES] {
        &self.voices
    }

    fn take_id(&mut self) -> u32 {
        let id = if self.next_id == 0 { 1 } else { self.next_id };
        self.next_id = id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }
        id
    }

    /// Greedy closest pairs within [`MATCH_CENTS`]. Returns how many candidates were claimed.
    fn match_actives(
        &mut self,
        candidates: &[F0Candidate],
        limit: usize,
        targets: &mut [f32; MAX_VOICES],
        claimed: &mut [usize; MAX_VOICES],
    ) -> usize {
        let mut partner = [None; MAX_VOICES];
        let mut claimed_len = 0usize;

        for _ in 0..limit {
            let mut best: Option<(usize, usize, f32)> = None;
            for slot in 0..limit {
                if !self.voices[slot].active || partner[slot].is_some() {
                    continue;
                }
                for (ci, cand) in candidates.iter().enumerate() {
                    if claimed[..claimed_len].contains(&ci) {
                        continue;
                    }
                    let Some(hz) = positive_hz(cand.f0_hz) else {
                        continue;
                    };
                    let dist = cents_between(self.voices[slot].f0_hz, hz);
                    if dist <= MATCH_CENTS && best.is_none_or(|(_, _, best_dist)| dist < best_dist)
                    {
                        best = Some((slot, ci, dist));
                    }
                }
            }
            let Some((slot, ci, _)) = best else {
                break;
            };
            partner[slot] = Some(ci);
            claimed[claimed_len] = ci;
            claimed_len += 1;
        }

        for slot in 0..limit {
            if !self.voices[slot].active {
                targets[slot] = 0.0;
                continue;
            }
            if let Some(ci) = partner[slot] {
                let cand_hz = candidates[ci].f0_hz;
                let f0 = self.voices[slot].f0_hz;
                if let (Some(f0), Some(cand_hz)) = (positive_hz(f0), positive_hz(cand_hz)) {
                    let smoothed = (0.5 * f0.ln() + 0.5 * cand_hz.ln()).exp();
                    if let Some(smoothed) = positive_hz(smoothed) {
                        self.voices[slot].f0_hz = smoothed;
                    }
                }
                self.miss[slot] = 0;
                self.matched[slot] = true;
                targets[slot] = 1.0;
            } else {
                self.miss[slot] = self.miss[slot].saturating_add(1);
                targets[slot] = if self.miss[slot] < HOLD_MISSES {
                    1.0
                } else {
                    0.0
                };
            }
        }
        claimed_len
    }

    fn birth_unclaimed(
        &mut self,
        candidates: &[F0Candidate],
        voice_claimed: &[usize],
        limit: usize,
        targets: &mut [f32; MAX_VOICES],
    ) {
        let mut pend_taken = [false; MAX_VOICES];
        let mut pend_keep = [false; MAX_VOICES];
        let mut cand_taken = [0usize; MAX_VOICES];
        let mut cand_taken_len = 0usize;

        for _ in 0..self.pending_len {
            let mut best: Option<(usize, usize, f32)> = None;
            for (ci, cand) in candidates.iter().enumerate() {
                if voice_claimed.contains(&ci) || cand_taken[..cand_taken_len].contains(&ci) {
                    continue;
                }
                let Some(hz) = positive_hz(cand.f0_hz) else {
                    continue;
                };
                for pi in 0..self.pending_len {
                    if pend_taken[pi] {
                        continue;
                    }
                    let dist = cents_between(hz, self.pending[pi].f0_hz);
                    if dist <= MATCH_CENTS && best.is_none_or(|(_, _, best_dist)| dist < best_dist)
                    {
                        best = Some((ci, pi, dist));
                    }
                }
            }
            let Some((ci, pi, _)) = best else {
                break;
            };
            pend_taken[pi] = true;
            cand_taken[cand_taken_len] = ci;
            cand_taken_len += 1;

            let hz = candidates[ci].f0_hz;
            self.pending[pi].f0_hz = hz;
            self.pending[pi].frames = self.pending[pi].frames.saturating_add(1);
            if self.pending[pi].frames >= BIRTH_FRAMES {
                if let Some(slot) = self.spawn(limit, hz) {
                    targets[slot] = 1.0;
                    continue;
                }
            }
            pend_keep[pi] = true;
        }

        let mut next = [Pending::default(); MAX_VOICES];
        let mut next_len = 0usize;
        for pi in 0..self.pending_len {
            if pend_keep[pi] && next_len < MAX_VOICES {
                next[next_len] = self.pending[pi];
                next_len += 1;
            }
        }
        for (ci, cand) in candidates.iter().enumerate() {
            if next_len >= MAX_VOICES {
                break;
            }
            if voice_claimed.contains(&ci) || cand_taken[..cand_taken_len].contains(&ci) {
                continue;
            }
            let Some(hz) = positive_hz(cand.f0_hz) else {
                continue;
            };
            next[next_len] = Pending {
                f0_hz: hz,
                frames: 1,
            };
            next_len += 1;
        }
        self.pending = next;
        self.pending_len = next_len;
    }

    fn spawn(&mut self, limit: usize, f0_hz: f32) -> Option<usize> {
        let slot = (0..limit).find(|&slot| !self.voices[slot].active)?;
        self.voices[slot] = VoiceState {
            active: true,
            id: self.take_id(),
            f0_hz,
            presence: 0.0,
            partials: Default::default(),
        };
        self.miss[slot] = 0;
        self.matched[slot] = false;
        Some(slot)
    }

    fn smooth_presence(
        &mut self,
        targets: &[f32; MAX_VOICES],
        settings: &EngineSettings,
        frame_dt: f32,
    ) {
        for slot in 0..MAX_VOICES {
            let target = targets[slot].clamp(0.0, 1.0);
            let current = self.voices[slot].presence;
            let ms = if target > current {
                settings.attack_ms
            } else {
                settings.release_ms
            };
            let coef = presence_coef(frame_dt, ms);
            let mut next = current + coef * (target - current);
            if !next.is_finite() {
                next = target;
            }
            next = next.clamp(0.0, 1.0);
            if target == 0.0 && next < PRESENCE_FLOOR {
                self.voices[slot].active = false;
                self.voices[slot].presence = 0.0;
                for partial in &mut self.voices[slot].partials {
                    partial.mag = 0.0;
                }
                self.miss[slot] = 0;
                self.matched[slot] = false;
            } else {
                self.voices[slot].presence = next;
            }
        }
    }

    fn refresh_matched_partials(
        &mut self,
        peaks: &[Peak],
        settings: &EngineSettings,
        limit: usize,
    ) {
        for slot in 0..limit {
            if !self.matched[slot] || !self.voices[slot].active {
                continue;
            }
            let f0 = self.voices[slot].f0_hz;
            harmonics_for(
                f0,
                peaks,
                settings.harmonics,
                PARTIAL_TOL_CENTS,
                &mut self.voices[slot].partials,
            );
        }
    }
}

fn positive_hz(hz: f32) -> Option<f32> {
    if hz.is_finite() && hz > 0.0 {
        Some(hz)
    } else {
        None
    }
}

fn cents_between(a_hz: f32, b_hz: f32) -> f32 {
    match (positive_hz(a_hz), positive_hz(b_hz)) {
        (Some(a), Some(b)) => (1200.0 * (a / b).ln() / LN_2).abs(),
        _ => f32::MAX,
    }
}

fn presence_coef(frame_dt: f32, ms: f32) -> f32 {
    if !(ms > 0.0) {
        return 1.0;
    }
    if !(frame_dt > 0.0) {
        return 0.0;
    }
    let tau = ms * 0.001;
    1.0 - (-frame_dt / tau).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_DT: f32 = 256.0 / 48_000.0;

    fn tone(f0_hz: f32, salience: f32) -> F0Candidate {
        F0Candidate { f0_hz, salience }
    }

    fn hold(tracker: &mut VoiceTracker, f0_hz: f32, frames: usize, settings: &EngineSettings) {
        let cand = [tone(f0_hz, 1.0)];
        for _ in 0..frames {
            tracker.update(&cand, &[], settings, FRAME_DT);
        }
    }

    fn active<'a>(tracker: &'a VoiceTracker) -> Vec<&'a VoiceState> {
        tracker
            .voices()
            .iter()
            .filter(|voice| voice.active)
            .collect()
    }

    fn assert_hz(actual: f32, expected: f32) {
        let err = (actual - expected).abs();
        assert!(err <= 0.75, "f0 {actual} expected near {expected}");
    }

    #[test]
    fn constant_candidate_settles_one_voice() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        let mut born_id = 0u32;
        for frame in 0..30 {
            tracker.update(&[tone(110.0, 1.0)], &[], &settings, FRAME_DT);
            let voices = active(&tracker);
            if frame == 0 {
                assert!(voices.is_empty());
                continue;
            }
            assert_eq!(voices.len(), 1);
            if born_id == 0 {
                born_id = voices[0].id;
            } else {
                assert_eq!(voices[0].id, born_id);
            }
        }
        let voices = active(&tracker);
        assert_eq!(voices.len(), 1);
        assert_ne!(voices[0].id, 0);
        assert_eq!(voices[0].id, born_id);
        assert_hz(voices[0].f0_hz, 110.0);
        assert!(voices[0].presence > 0.9);
        assert!((voices[0].partials[0].freq_hz - voices[0].f0_hz).abs() <= 0.05);
    }

    #[test]
    fn one_frame_dropout_keeps_id() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        hold(&mut tracker, 110.0, 10, &settings);
        let id = active(&tracker)[0].id;
        tracker.update(&[], &[], &settings, FRAME_DT);
        hold(&mut tracker, 110.0, 10, &settings);
        let voices = active(&tracker);
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].id, id);
        assert_hz(voices[0].f0_hz, 110.0);
        assert!(voices[0].active);
    }

    #[test]
    fn disappeared_candidate_releases() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        hold(&mut tracker, 110.0, 30, &settings);
        let id = active(&tracker)[0].id;
        let before = active(&tracker)[0].presence;
        assert!(before > 0.9);

        let mut presence = before;
        let mut inactive = false;
        let limit = (1.5 / FRAME_DT).ceil() as usize;
        for frame in 0..limit {
            tracker.update(&[], &[], &settings, FRAME_DT);
            let voice = tracker
                .voices()
                .iter()
                .find(|voice| voice.id == id)
                .expect("id kept");
            presence = voice.presence;
            if !voice.active {
                inactive = true;
                break;
            }
            if frame == 39 {
                assert!(presence < before);
            }
        }
        assert!(inactive, "presence {presence} after {limit} frames");
        assert!(active(&tracker).is_empty());
    }

    #[test]
    fn two_candidates_two_voices() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        let cands = [tone(110.0, 1.0), tone(165.0, 0.8)];
        for _ in 0..30 {
            tracker.update(&cands, &[], &settings, FRAME_DT);
        }
        let voices = active(&tracker);
        assert_eq!(voices.len(), 2);
        assert_ne!(voices[0].id, voices[1].id);
        assert_ne!(voices[0].id, 0);
        assert_ne!(voices[1].id, 0);
        let mut freqs = [voices[0].f0_hz, voices[1].f0_hz];
        freqs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_hz(freqs[0], 110.0);
        assert_hz(freqs[1], 165.0);
    }

    #[test]
    fn max_voices_limits_births() {
        let mut settings = EngineSettings::default();
        settings.max_voices = 1;
        let mut tracker = VoiceTracker::new();
        let cands = [tone(110.0, 1.0), tone(165.0, 0.8)];
        for _ in 0..30 {
            tracker.update(&cands, &[], &settings, FRAME_DT);
        }
        assert_eq!(active(&tracker).len(), 1);
    }

    #[test]
    fn slow_glide_keeps_id() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        hold(&mut tracker, 110.0, 10, &settings);
        let id = active(&tracker)[0].id;
        let mut f0 = 110.0_f32;
        while f0 + 0.5 <= 116.0 + 1e-3 {
            f0 += 0.5;
            tracker.update(&[tone(f0, 1.0)], &[], &settings, FRAME_DT);
            let voices = active(&tracker);
            assert_eq!(voices.len(), 1);
            assert_eq!(voices[0].id, id);
        }
        assert!((f0 - 116.0).abs() <= 1e-3);
        assert!(active(&tracker)[0].f0_hz > 114.0);
    }

    #[test]
    fn octave_jump_births_new_id_and_releases_old() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        hold(&mut tracker, 110.0, 10, &settings);
        let old_id = active(&tracker)[0].id;
        let before = active(&tracker)[0].presence;

        tracker.update(&[tone(220.0, 1.0)], &[], &settings, FRAME_DT);
        assert!(active(&tracker).iter().all(|voice| voice.id == old_id));

        tracker.update(&[tone(220.0, 1.0)], &[], &settings, FRAME_DT);
        let voices = active(&tracker);
        let born = voices
            .iter()
            .find(|voice| voice.id != old_id)
            .expect("new id");
        assert_hz(born.f0_hz, 220.0);
        let old = tracker
            .voices()
            .iter()
            .find(|voice| voice.id == old_id)
            .expect("old id");
        assert!(old.presence < before);

        let new_id = born.id;
        for _ in 0..250 {
            tracker.update(&[tone(220.0, 1.0)], &[], &settings, FRAME_DT);
        }
        let voices = active(&tracker);
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].id, new_id);
        assert_hz(voices[0].f0_hz, 220.0);
        assert!(tracker
            .voices()
            .iter()
            .all(|voice| voice.id != old_id || !voice.active));
    }

    #[test]
    fn single_frame_spurious_never_births() {
        let settings = EngineSettings::default();
        let mut tracker = VoiceTracker::new();
        tracker.update(&[tone(440.0, 1.0)], &[], &settings, FRAME_DT);
        assert!(active(&tracker).is_empty());
        tracker.update(&[], &[], &settings, FRAME_DT);
        assert!(active(&tracker).is_empty());
    }
}
