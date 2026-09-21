/// Sub-bass sine synthesizer triggered by Kick MIDI note.
/// Matches BUILD3_KickSineModule.js
use super::exp_coeff;
use super::semitone_ratio;
use std::f32::consts::PI;

const ENV_FLOOR: f32 = 0.0001;
const BASE_FREQ_HZ: f32 = 40.0;

pub struct KickSine {
    sample_rate: f32,
    phase: f32,
    active: bool,
    amp_env: f32,
    pitch_env: f32,
    decay_coeff: f32,
    pitch_decay_coeff: f32,
    delay_buffer: Vec<f32>,
    write_idx: usize,
    read_idx: usize,
    delay_samples: usize,
}

impl Default for KickSine {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

/// `length` 0..1 maps to 100 ms..2100 ms one-pole decay.
#[inline]
pub fn length_to_decay_ms(length_norm: f32) -> f32 {
    100.0 + length_norm.clamp(0.0, 1.0) * 2000.0
}

impl KickSine {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay = delay_len(sample_rate);
        Self {
            sample_rate,
            phase: 0.0,
            active: false,
            amp_env: 0.0,
            pitch_env: 0.0,
            decay_coeff: 0.9995,
            pitch_decay_coeff: 0.995,
            delay_buffer: vec![0.0; max_delay],
            write_idx: 0,
            read_idx: 0,
            delay_samples: 0,
        }
    }

    pub fn reset(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.phase = 0.0;
        self.active = false;
        self.amp_env = 0.0;
        self.pitch_env = 0.0;
        self.delay_samples = 0;
        let max_delay = delay_len(sample_rate);
        if self.delay_buffer.len() != max_delay {
            self.delay_buffer.resize(max_delay, 0.0);
        }
        self.delay_buffer.fill(0.0);
        self.write_idx = 0;
        self.read_idx = 0;
    }

    pub fn trigger(&mut self, length_norm: f32, dive_semi: f32, speed_norm: f32, offset_ms: f32) {
        self.active = true;
        self.phase = 0.0;
        self.amp_env = 1.0;
        self.pitch_env = dive_semi;

        self.decay_coeff = exp_coeff(length_to_decay_ms(length_norm), self.sample_rate);

        let pitch_drop_ms = 10.0 + (1.0 - speed_norm) * 990.0;
        self.pitch_decay_coeff = exp_coeff(pitch_drop_ms, self.sample_rate);

        let len = self.delay_buffer.len().max(1);
        self.delay_samples = (offset_ms * 0.001 * self.sample_rate) as usize;
        self.delay_samples = self.delay_samples.min(len - 1);
        self.read_idx = (self.write_idx + len - self.delay_samples) % len;
    }

    pub fn process_sample(&mut self, vol_gain: f32) -> f32 {
        if !self.active || self.amp_env < ENV_FLOOR {
            self.active = false;
            self.amp_env = 0.0;
            return 0.0;
        }

        let freq = BASE_FREQ_HZ * semitone_ratio(self.pitch_env);
        let raw_sample = (self.phase * 2.0 * PI).sin() * self.amp_env * vol_gain;
        self.phase += freq / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        self.amp_env *= self.decay_coeff;
        self.pitch_env *= self.pitch_decay_coeff;

        if self.delay_samples == 0 {
            return raw_sample;
        }

        let len = self.delay_buffer.len();
        self.delay_buffer[self.write_idx] = raw_sample;
        let out_sample = self.delay_buffer[self.read_idx];
        self.write_idx += 1;
        if self.write_idx >= len {
            self.write_idx = 0;
        }
        self.read_idx += 1;
        if self.read_idx >= len {
            self.read_idx = 0;
        }
        out_sample
    }
}

fn delay_len(sample_rate: f32) -> usize {
    ((sample_rate * 0.1) as usize).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_poles() {
        assert!((length_to_decay_ms(0.0) - 100.0).abs() < 1e-6);
        assert!((length_to_decay_ms(1.0) - 2100.0).abs() < 1e-6);
    }

    #[test]
    fn goes_idle_after_decay() {
        let mut kick = KickSine::new(44100.0);
        kick.trigger(0.0, 0.0, 1.0, 0.0);
        let mut heard = 0.0f32;
        for _ in 0..44100 {
            heard = heard.max(kick.process_sample(1.0).abs());
        }
        assert!(heard > 0.0);
        assert!(!kick.active);
        assert_eq!(kick.process_sample(1.0), 0.0);
    }
}
