//! Karplus-Strong polyphonic physical modeling plucked string engine in pure Rust.
//!
//! Inspired by the physical modeling architecture from the Pure Data reference patch
//! (ks.pd / pink.pd):
//! - 3-pole 3-zero pink noise excitation filter
//! - Velocity-sensitive dynamic attack shaping
//! - Fractional-delay allpass waveguide for exact tuning and artifact-free continuous pitch bending
//! - 1-pole loop damping filter (string brightness and material)
//! - Decay multiplier (sustain)
//! - 2-pole wooden body resonance
//! - Soft voice-stealing with click-free crossfade

use super::SoundEngine;

const MAX_VOICES: usize = 16;
const MAX_DELAY_SAMPLES: usize = 4096; // Down to ~11 Hz at 44.1 kHz

/// High quality pink noise generator (3-pole 3-zero IIR filter ported from pink.pd).
#[derive(Clone, Copy)]
pub struct PinkNoise {
    rng: u32,
    p0: f32,
    p1: f32,
    p2: f32,
    z0: f32,
    z1: f32,
    z2: f32,
}

impl Default for PinkNoise {
    fn default() -> Self {
        Self {
            rng: 0x9e3779b9,
            p0: 0.0,
            p1: 0.0,
            p2: 0.0,
            z0: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl PinkNoise {
    #[inline]
    fn white(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let white = self.white();

        // 3-pole, 3-zero filter cascade (from pink.pd)
        // Stage 0: zero 0.984436, pole 0.995728
        let mut x = white - 0.984436 * self.z0;
        self.z0 = white;
        let y0 = x + 0.995728 * self.p0;
        self.p0 = y0;

        // Stage 1: zero 0.833923, pole 0.947906
        x = y0 - 0.833923 * self.z1;
        self.z1 = y0;
        let y1 = x + 0.947906 * self.p1;
        self.p1 = y1;

        // Stage 2: zero 0.0756836, pole 0.535675
        x = y1 - 0.0756836 * self.z2;
        self.z2 = y1;
        let y2 = x + 0.535675 * self.p2;
        self.p2 = y2;

        y2 * 0.17
    }

    pub fn reset(&mut self) {
        self.p0 = 0.0;
        self.p1 = 0.0;
        self.p2 = 0.0;
        self.z0 = 0.0;
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Single plucked string voice using Karplus-Strong digital waveguide.
struct Voice {
    active: bool,
    note: u8,
    channel: u8,
    frequency: f32,
    bend_semitones: f32,

    // Excitation
    noise: PinkNoise,
    burst_samples_left: usize,
    burst_length: usize,
    burst_filter: f32,
    burst_filter_state: f32,

    // Delay line
    buffer: Vec<f32>,
    write_idx: usize,

    // Allpass fractional delay filter state
    allpass_state: f32,

    // Loop damping filter state (1-pole lowpass)
    filter_state: f32,

    // Release / fade out
    releasing: bool,
    fade_level: f32,
    fade_step: f32,

    // Age / priority
    age: u64,
}

impl Voice {
    fn new() -> Self {
        Self {
            active: false,
            note: 60,
            channel: 0,
            frequency: 261.63,
            bend_semitones: 0.0,
            noise: PinkNoise::default(),
            burst_samples_left: 0,
            burst_length: 100,
            burst_filter: 0.5,
            burst_filter_state: 0.0,
            buffer: vec![0.0; MAX_DELAY_SAMPLES],
            write_idx: 0,
            allpass_state: 0.0,
            filter_state: 0.0,
            releasing: false,
            fade_level: 1.0,
            fade_step: 0.0,
            age: 0,
        }
    }

    fn reset(&mut self) {
        self.active = false;
        self.buffer.fill(0.0);
        self.write_idx = 0;
        self.allpass_state = 0.0;
        self.filter_state = 0.0;
        self.burst_samples_left = 0;
        self.releasing = false;
        self.fade_level = 1.0;
        self.fade_step = 0.0;
        self.noise.reset();
    }

    fn pluck(
        &mut self,
        note: u8,
        channel: u8,
        velocity: f32,
        sample_rate: f32,
        bend_semitones: f32,
        _pick_position: f32,
        pluck_hardness: f32,
    ) {
        self.reset();
        self.active = true;
        self.note = note;
        self.channel = channel;
        self.bend_semitones = bend_semitones;
        self.frequency = midi_to_freq(note as f32 + bend_semitones);

        let period = (sample_rate / self.frequency).clamp(4.0, (MAX_DELAY_SAMPLES - 4) as f32);
        // Excitation burst lasts roughly one string period, scaled by pluck hardness and velocity
        let burst_len = ((period * (0.5 + 0.5 * pluck_hardness)).round() as usize).clamp(8, 256);
        self.burst_length = burst_len;
        self.burst_samples_left = burst_len;

        // Velocity darkens soft strikes and brightens hard strikes
        let cutoff = (pluck_hardness * 0.7 + velocity * 0.3).clamp(0.05, 0.95);
        self.burst_filter = cutoff;
        self.burst_filter_state = 0.0;

        self.releasing = false;
        self.fade_level = 1.0;
        self.fade_step = 0.0;
        self.age = 0;
    }

    fn release(&mut self) {
        if self.active && !self.releasing {
            self.releasing = true;
            // Short release damping
            self.fade_step = 1.0 / 256.0;
        }
    }

    fn steal(&mut self) {
        if self.active {
            // Rapid fade out over 64 samples to avoid pop
            self.releasing = true;
            self.fade_step = 1.0 / 64.0;
        }
    }

    fn update_bend(&mut self, bend_semitones: f32) {
        self.bend_semitones = bend_semitones;
        self.frequency = midi_to_freq(self.note as f32 + bend_semitones);
    }

    #[inline]
    fn process_sample(
        &mut self,
        sample_rate: f32,
        decay_factor: f32,
        damping_factor: f32,
    ) -> f32 {
        if !self.active {
            return 0.0;
        }
        self.age = self.age.saturating_add(1);

        // Effective fundamental period in samples
        let delay_len = (sample_rate / self.frequency.max(10.0)).clamp(4.0, (MAX_DELAY_SAMPLES - 4) as f32);

        // Allpass fractional delay:
        // Delay D = int_delay + frac
        let int_delay = delay_len.floor() as usize;
        let frac = delay_len - int_delay as f32;

        // Read from delay line with allpass interpolation:
        // Allpass coefficient C = (1 - frac) / (1 + frac)
        let tap_idx = (self.write_idx + MAX_DELAY_SAMPLES - int_delay) % MAX_DELAY_SAMPLES;
        let delayed_raw = self.buffer[tap_idx];

        let c = if frac > 0.001 {
            (1.0 - frac) / (1.0 + frac)
        } else {
            0.0
        };
        let delayed = c * delayed_raw + self.allpass_state;
        self.allpass_state = delayed_raw - c * delayed;

        // Loop damping filter: 1-pole lowpass
        // y[n] = (1 - alpha) * delayed + alpha * y[n-1]
        let alpha = damping_factor.clamp(0.01, 0.98);
        self.filter_state = (1.0 - alpha) * delayed + alpha * self.filter_state;

        // String decay loop feedback
        let feedback = self.filter_state * decay_factor;

        // Excitation injection
        let mut excitation = 0.0;
        if self.burst_samples_left > 0 {
            let progress = 1.0 - (self.burst_samples_left as f32 / self.burst_length as f32);
            // Hann-shaped window on the noise burst
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * progress).cos());
            let raw_noise = self.noise.next_sample() * window;

            // Lowpass excitation filter
            self.burst_filter_state += self.burst_filter * (raw_noise - self.burst_filter_state);
            excitation = self.burst_filter_state;

            self.burst_samples_left -= 1;
        }

        let input_sample = excitation + feedback;
        self.buffer[self.write_idx] = input_sample;
        self.write_idx = (self.write_idx + 1) % MAX_DELAY_SAMPLES;

        let mut output = input_sample;

        // Apply release / steal fade
        if self.releasing {
            output *= self.fade_level;
            self.fade_level -= self.fade_step;
            if self.fade_level <= 0.0 {
                self.active = false;
                self.fade_level = 0.0;
                return 0.0;
            }
        }

        // Energy threshold auto-silence (only after the string has had time to circulate)
        if self.burst_samples_left == 0
            && self.age > (delay_len * 2.0) as u64
            && output.abs() < 1e-5
            && self.filter_state.abs() < 1e-5
        {
            self.active = false;
            return 0.0;
        }

        output
    }
}

/// Wooden acoustic body resonance filter (2-pole resonant peak).
#[derive(Clone, Copy)]
struct BodyResonance {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BodyResonance {
    fn new(sample_rate: f32, freq: f32, q: f32, gain: f32) -> Self {
        let mut b = Self {
            b0: 1.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        };
        b.update(sample_rate, freq, q, gain);
        b
    }

    fn update(&mut self, sample_rate: f32, freq: f32, q: f32, gain: f32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let a0 = 1.0 + alpha / gain;
        self.b0 = (1.0 + alpha * gain) / a0;
        self.b2 = (1.0 - alpha * gain) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha / gain) / a0;
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input - self.b0 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// Polyphonic Karplus-Strong string synthesis engine.
pub struct KarplusEngine {
    sample_rate: f32,
    voices: [Voice; MAX_VOICES],
    body_l: BodyResonance,
    body_r: BodyResonance,

    // Sound parameters
    pub decay: f32,          // 0.0 .. 1.0 (maps to loop feedback 0.92 .. 0.9996)
    pub damping: f32,        // 0.0 .. 1.0 (maps to filter cutoff / brightness)
    pub pick_position: f32,  // 0.0 .. 1.0 (pluck harmonic placement)
    pub pluck_hardness: f32, // 0.0 .. 1.0 (attack brightness)
    pub body_amount: f32,    // 0.0 .. 1.0 (acoustic box warmth)
    pub stereo_width: f32,   // 0.0 .. 1.0 (voice pan spread)

    // Expression
    bends: [f32; 16],
    damper_held: bool,
}

impl KarplusEngine {
    pub fn new(sample_rate: f32) -> Self {
        let mut e = Self {
            sample_rate: sample_rate.max(1.0),
            voices: std::array::from_fn(|_| Voice::new()),
            body_l: BodyResonance::new(sample_rate, 115.0, 2.5, 1.8),
            body_r: BodyResonance::new(sample_rate, 145.0, 2.8, 1.8),
            decay: 0.65,
            damping: 0.35,
            pick_position: 0.25,
            pluck_hardness: 0.70,
            body_amount: 0.40,
            stereo_width: 0.30,
            bends: [0.0; 16],
            damper_held: false,
        };
        e.set_sample_rate(sample_rate);
        e
    }

    fn allocate_voice(&mut self) -> usize {
        // 1. Try to find an inactive voice
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return i;
        }

        // 2. Try to find a releasing voice with highest age
        if let Some((i, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.releasing)
            .max_by_key(|(_, v)| v.age)
        {
            self.voices[i].steal();
            return i;
        }

        // 3. Steal oldest active voice
        let oldest = self
            .voices
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| v.age)
            .map(|(i, _)| i)
            .unwrap_or(0);

        self.voices[oldest].steal();
        oldest
    }
}

impl SoundEngine for KarplusEngine {
    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.body_l.update(self.sample_rate, 115.0, 2.5, 1.8);
        self.body_r.update(self.sample_rate, 145.0, 2.8, 1.8);
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
        self.body_l.reset();
        self.body_r.reset();
        self.bends.fill(0.0);
        self.damper_held = false;
    }

    fn note_on(&mut self, channel: u8, note: u8, velocity: f32) {
        let ch = (channel as usize).min(15);
        let bend = self.bends[ch];
        let idx = self.allocate_voice();
        self.voices[idx].pluck(
            note,
            channel,
            velocity,
            self.sample_rate,
            bend,
            self.pick_position,
            self.pluck_hardness,
        );
    }

    fn note_off(&mut self, channel: u8, note: u8, _velocity: f32) {
        if self.damper_held {
            return;
        }
        for voice in &mut self.voices {
            if voice.active && voice.channel == channel && voice.note == note {
                voice.release();
            }
        }
    }

    fn bend(&mut self, channel: u8, value: f32) {
        let ch = (channel as usize).min(15);
        // Standard +/- 2 semitone range or mapped bend
        let semitones = (value - 0.5) * 4.0;
        self.bends[ch] = semitones;
        for voice in &mut self.voices {
            if voice.active && voice.channel == channel {
                voice.update_bend(semitones);
            }
        }
    }

    fn pressure(&mut self, _channel: u8, _value: f32) {}

    fn cc(&mut self, _channel: u8, cc: u8, value: f32) {
        if cc == 64 {
            self.damper_held = value >= 0.5;
            if !self.damper_held {
                // Damper released: release sustained voices
                for voice in &mut self.voices {
                    if voice.active {
                        voice.release();
                    }
                }
            }
        }
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.decay = value.clamp(0.0, 1.0),
            1 => self.damping = value.clamp(0.0, 1.0),
            2 => self.pick_position = value.clamp(0.0, 1.0),
            3 => self.pluck_hardness = value.clamp(0.0, 1.0),
            4 => self.body_amount = value.clamp(0.0, 1.0),
            5 => self.stereo_width = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let len = left.len().min(right.len());
        if len == 0 {
            return;
        }

        // Map sound params to DSP coefficients:
        // Decay 0..1 -> 0.91 .. 0.9995 loop feedback
        let decay_factor = 0.91 + 0.0895 * self.decay.powf(0.5);
        // Damping 0..1 -> 0.02 .. 0.85 1-pole filter alpha
        let damping_factor = 0.02 + 0.83 * (1.0 - self.damping);

        for s in 0..len {
            let mut sum_l = 0.0;
            let mut sum_r = 0.0;

            for (i, voice) in self.voices.iter_mut().enumerate() {
                if !voice.active {
                    continue;
                }
                let sample = voice.process_sample(self.sample_rate, decay_factor, damping_factor);
                // Stereo voice pan spread across polyphonic voices
                let pan = ((i as f32 / (MAX_VOICES - 1) as f32) * 2.0 - 1.0) * self.stereo_width;
                let pan_l = 0.5 * (1.0 - pan);
                let pan_r = 0.5 * (1.0 + pan);

                sum_l += sample * pan_l;
                sum_r += sample * pan_r;
            }

            // Acoustic body resonance injection
            let body_l = self.body_l.process(sum_l);
            let body_r = self.body_r.process(sum_r);

            let out_l = sum_l + body_l * self.body_amount * 0.4;
            let out_r = sum_r + body_r * self.body_amount * 0.4;

            left[s] += out_l;
            right[s] += out_r;
        }
    }
}

#[inline]
fn midi_to_freq(note: f32) -> f32 {
    440.0 * 2.0f32.powf((note - 69.0) / 12.0)
}
