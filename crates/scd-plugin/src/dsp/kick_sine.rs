/// Sub-bass sine synthesizer triggered by Kick MIDI note.
/// Matches BUILD3_KickSineModule.js

use std::f32::consts::PI;

pub struct KickSine {
    sample_rate: f32,
    phase: f32,
    active: bool,
    amp_env: f32,
    pitch_env: f32,
    decay_coeff: f32,
    pitch_decay_coeff: f32,
    // Delay offset ring buffer
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

impl KickSine {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay = (sample_rate * 0.1) as usize; // Max 100ms
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
        self.delay_buffer.fill(0.0);
        self.write_idx = 0;
        self.read_idx = 0;
    }

    pub fn trigger(&mut self, _velocity: f32, length_norm: f32, dive_semi: f32, speed_norm: f32, offset_ms: f32) {
        self.active = true;
        self.phase = 0.0;
        self.amp_env = 1.0;
        self.pitch_env = dive_semi;

        // Length: 0..1 maps to 100ms..2100ms decay
        let decay_ms = 100.0 + length_norm * 2000.0;
        self.decay_coeff = (-1.0 / (decay_ms * 0.001 * self.sample_rate)).exp();

        // Speed: higher = faster pitch drop (attack 10ms..1000ms)
        let pitch_drop_ms = 10.0 + (1.0 - speed_norm) * 990.0;
        self.pitch_decay_coeff = (-1.0 / (pitch_drop_ms * 0.001 * self.sample_rate)).exp();

        // Delay offset in samples
        self.delay_samples = (offset_ms * 0.001 * self.sample_rate) as usize;
        self.delay_samples = self.delay_samples.min(self.delay_buffer.len() - 1);
        self.read_idx = (self.write_idx + self.delay_buffer.len() - self.delay_samples) % self.delay_buffer.len();
    }

    pub fn process_sample(&mut self, vol_gain: f32) -> f32 {
        if !self.active && self.amp_env < 0.0001 {
            self.active = false;
            return 0.0;
        }

        // Base sub pitch ~40 Hz (MIDI 24 / C1), shifted by pitch envelope
        let base_freq = 40.0;
        let pitch_factor = 2.0_f32.powf(self.pitch_env / 12.0);
        let freq = base_freq * pitch_factor;

        let raw_sample = (self.phase * 2.0 * PI).sin() * self.amp_env * vol_gain;
        self.phase += freq / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        self.amp_env *= self.decay_coeff;
        self.pitch_env *= self.pitch_decay_coeff;

        // Write to delay ring buffer
        self.delay_buffer[self.write_idx] = raw_sample;
        let out_sample = self.delay_buffer[self.read_idx];

        self.write_idx = (self.write_idx + 1) % self.delay_buffer.len();
        self.read_idx = (self.read_idx + 1) % self.delay_buffer.len();

        out_sample
    }
}
