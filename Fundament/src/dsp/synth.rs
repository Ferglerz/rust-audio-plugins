use super::types::{EngineSettings, VoiceState, Waveform, MAX_HARMONICS, MAX_VOICES};

const MAX_SLOTS: usize = MAX_VOICES * MAX_HARMONICS;
const AMP_SMOOTH_SEC: f32 = 0.005;
const LIVE_EPS: f32 = 1e-7;

#[derive(Clone, Copy, Default)]
struct Slot {
    phase: f64,
    current_freq: f32,
    target_freq: f32,
    current_amp: f32,
    target_amp: f32,
    id: u32,
}

pub struct AdditiveSynth {
    sample_rate: f32,
    amp_coeff: f32,
    glide_coeff: f32,
    slots: [Slot; MAX_SLOTS],
}

impl Default for AdditiveSynth {
    fn default() -> Self {
        Self {
            sample_rate: 0.0,
            amp_coeff: 0.0,
            glide_coeff: 0.0,
            slots: [Slot::default(); MAX_SLOTS],
        }
    }
}

impl AdditiveSynth {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.slots = [Slot::default(); MAX_SLOTS];
        self.glide_coeff = 0.0;
        self.amp_coeff = amp_coeff(self.sample_rate);
    }

    pub fn set_targets(&mut self, voices: &[VoiceState; MAX_VOICES], settings: &EngineSettings) {
        let sr = self.sample_rate;
        self.glide_coeff = if settings.glide_ms <= 0.0 || sr <= 0.0 {
            0.0
        } else {
            (-1.0 / (settings.glide_ms / 1000.0 * sr)).exp()
        };

        let level = 10.0f32.powf(settings.synth_level_db / 20.0);
        let tone = settings.tone.clamp(0.0, 1.0);
        let n_harm = settings.harmonics.min(MAX_HARMONICS);
        let nyquist_lim = 0.45 * sr;

        for v in 0..MAX_VOICES {
            let voice = &voices[v];
            let base = v * MAX_HARMONICS;
            let live = voice.active && voice.presence > 0.0;

            if !live {
                for h in 0..MAX_HARMONICS {
                    self.slots[base + h].target_amp = 0.0;
                }
            } else {
                let mut ref_mag = 0.0f32;
                for h in 0..n_harm {
                    if voice.partials[h].mag > ref_mag {
                        ref_mag = voice.partials[h].mag;
                    }
                }
                if ref_mag == 0.0 {
                    ref_mag = voice.partials[0].mag;
                }

                for h in 0..MAX_HARMONICS {
                    let slot = &mut self.slots[base + h];
                    if h >= n_harm {
                        slot.target_amp = 0.0;
                        continue;
                    }
                    let n = (h + 1) as i32;
                    let measured = voice.partials[h].mag;
                    let shaped = ref_mag * waveform_weight(settings.waveform, n);
                    let blend = measured * (1.0 - tone) + shaped * tone;
                    let mut target_amp = blend * voice.presence * level;
                    let target_freq = if voice.partials[h].freq_hz > 0.0 {
                        voice.partials[h].freq_hz
                    } else {
                        voice.f0_hz * n as f32
                    };
                    if target_freq >= nyquist_lim || target_freq <= 0.0 {
                        target_amp = 0.0;
                    }
                    slot.target_amp = target_amp;
                    slot.target_freq = target_freq;
                }
            }

            for h in 0..MAX_HARMONICS {
                let slot = &mut self.slots[base + h];
                if voice.id != slot.id {
                    slot.id = voice.id;
                    slot.phase = 0.0;
                    slot.current_amp = 0.0;
                    slot.current_freq = slot.target_freq;
                }
            }
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let sr = self.sample_rate;
        if !(sr > 0.0) {
            return 0.0;
        }
        let sr_f64 = f64::from(sr);
        let glide = self.glide_coeff;
        let amp_c = self.amp_coeff;
        let mut sum = 0.0f64;
        for slot in &mut self.slots {
            if slot.current_amp.abs() <= LIVE_EPS && slot.target_amp.abs() <= LIVE_EPS {
                continue;
            }
            slot.current_freq = slot.target_freq + (slot.current_freq - slot.target_freq) * glide;
            slot.current_amp = slot.target_amp + (slot.current_amp - slot.target_amp) * amp_c;
            slot.phase += f64::from(slot.current_freq) / sr_f64;
            slot.phase = slot.phase.rem_euclid(1.0);
            sum += f64::from(slot.current_amp) * (std::f64::consts::TAU * slot.phase).sin();
        }
        sum as f32
    }
}

fn amp_coeff(sample_rate: f32) -> f32 {
    if sample_rate <= 0.0 {
        0.0
    } else {
        (-1.0 / (AMP_SMOOTH_SEC * sample_rate)).exp()
    }
}

fn waveform_weight(waveform: Waveform, n: i32) -> f32 {
    match waveform {
        Waveform::Sine => {
            if n == 1 {
                1.0
            } else {
                0.0
            }
        }
        Waveform::Saw => 1.0 / n as f32,
        Waveform::Square => {
            if n % 2 == 0 {
                0.0
            } else {
                1.0 / n as f32
            }
        }
        Waveform::Triangle => {
            if n % 2 == 0 {
                0.0
            } else {
                let sign = if ((n - 1) / 2) % 2 == 0 { 1.0 } else { -1.0 };
                sign / (n as f32 * n as f32)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::Partial;
    use super::*;

    const SR: f32 = 48_000.0;

    fn quiet_voices() -> [VoiceState; MAX_VOICES] {
        [VoiceState::default(); MAX_VOICES]
    }

    fn settings(harmonics: usize, tone: f32, waveform: Waveform) -> EngineSettings {
        EngineSettings {
            harmonics,
            tone,
            waveform,
            synth_level_db: 0.0,
            glide_ms: 15.0,
            ..EngineSettings::default()
        }
    }

    fn voice_fundamental(id: u32, f0: f32, mag: f32, presence: f32) -> VoiceState {
        let mut v = VoiceState {
            active: true,
            id,
            f0_hz: f0,
            presence,
            ..VoiceState::default()
        };
        v.partials[0] = Partial { freq_hz: f0, mag };
        v
    }

    fn primed(voices: &[VoiceState; MAX_VOICES], settings: &EngineSettings) -> AdditiveSynth {
        let mut synth = AdditiveSynth::new();
        synth.prepare(SR);
        synth.set_targets(voices, settings);
        synth
    }

    fn skip(synth: &mut AdditiveSynth, n: usize) {
        for _ in 0..n {
            let _ = synth.next_sample();
        }
    }

    fn collect(synth: &mut AdditiveSynth, n: usize) -> Vec<f32> {
        (0..n).map(|_| synth.next_sample()).collect()
    }

    fn rms(xs: &[f32]) -> f32 {
        (xs.iter().map(|x| x * x).sum::<f32>() / xs.len() as f32).sqrt()
    }

    fn positive_zero_cross_hz(xs: &[f32], sr: f32) -> f32 {
        let mut count = 0usize;
        for w in xs.windows(2) {
            if w[0] <= 0.0 && w[1] > 0.0 {
                count += 1;
            }
        }
        count as f32 * sr / xs.len() as f32
    }

    fn goertzel(xs: &[f32], freq: f32, sr: f32) -> f32 {
        let w = std::f32::consts::TAU * freq / sr;
        let coeff = 2.0 * w.cos();
        let mut s1 = 0.0f32;
        let mut s2 = 0.0f32;
        for &x in xs {
            let s0 = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let real = s1 - s2 * w.cos();
        let imag = s2 * w.sin();
        (real * real + imag * imag).sqrt()
    }

    #[test]
    fn sine_rms_and_frequency() {
        let mut voices = quiet_voices();
        voices[0] = voice_fundamental(1, 220.0, 0.5, 1.0);
        let cfg = settings(1, 0.0, Waveform::Sine);
        let mut synth = primed(&voices, &cfg);

        skip(&mut synth, 4_800);
        let rms_buf = collect(&mut synth, 4_800);
        let expected = 0.5 / 2.0f32.sqrt();
        let rel = (rms(&rms_buf) - expected).abs() / expected;
        assert!(rel <= 0.05, "rms relative error {rel}");

        let freq_buf = collect(&mut synth, 48_000);
        let hz = positive_zero_cross_hz(&freq_buf, SR);
        assert!((hz - 220.0).abs() <= 2.0, "freq {hz}");
    }

    #[test]
    fn sine_tone_kills_upper_harmonics() {
        let mut voices = quiet_voices();
        voices[0] = VoiceState {
            active: true,
            id: 1,
            f0_hz: 220.0,
            presence: 1.0,
            partials: {
                let mut p = [Partial::default(); MAX_HARMONICS];
                p[0] = Partial {
                    freq_hz: 220.0,
                    mag: 0.2,
                };
                p[1] = Partial {
                    freq_hz: 440.0,
                    mag: 0.9,
                };
                p[2] = Partial {
                    freq_hz: 660.0,
                    mag: 0.8,
                };
                p[3] = Partial {
                    freq_hz: 880.0,
                    mag: 0.7,
                };
                p
            },
        };
        let cfg = settings(4, 1.0, Waveform::Sine);
        let mut synth = primed(&voices, &cfg);
        skip(&mut synth, 4_800);
        let buf = collect(&mut synth, 8_192);
        let mag220 = goertzel(&buf, 220.0, SR);
        let mag440 = goertzel(&buf, 440.0, SR);
        assert!(mag220 > 0.0, "missing fundamental");
        assert!(
            mag440 < 0.01 * mag220,
            "440 mag {mag440} vs 220 mag {mag220}"
        );
    }

    #[test]
    fn presence_zero_fades_out() {
        let mut voices = quiet_voices();
        voices[0] = voice_fundamental(1, 220.0, 0.5, 1.0);
        let cfg = settings(1, 0.0, Waveform::Sine);
        let mut synth = primed(&voices, &cfg);
        skip(&mut synth, 4_800);

        voices[0].presence = 0.0;
        synth.set_targets(&voices, &cfg);
        skip(&mut synth, 4_800);
        let tail = collect(&mut synth, 64);
        let peak = tail.iter().fold(0.0f32, |a, x| a.max(x.abs()));
        assert!(peak < 1e-4, "peak {peak}");
    }

    #[test]
    fn glide_is_click_free() {
        let mut voices = quiet_voices();
        voices[0] = voice_fundamental(1, 220.0, 0.5, 1.0);
        let cfg = settings(1, 0.0, Waveform::Sine);
        let mut synth = primed(&voices, &cfg);

        let mut prev = synth.next_sample();
        let mut max_delta = 0.0f32;
        for _ in 1..2_400 {
            let s = synth.next_sample();
            max_delta = max_delta.max((s - prev).abs());
            prev = s;
        }

        voices[0].f0_hz = 233.0;
        voices[0].partials[0].freq_hz = 233.0;
        synth.set_targets(&voices, &cfg);
        for _ in 0..4_800 {
            let s = synth.next_sample();
            max_delta = max_delta.max((s - prev).abs());
            prev = s;
        }
        assert!(max_delta < 0.03, "max delta {max_delta}");
    }

    #[test]
    fn above_nyquist_guard_is_silent() {
        let mut voices = quiet_voices();
        voices[0] = VoiceState {
            active: true,
            id: 1,
            f0_hz: 15_000.0,
            presence: 1.0,
            partials: {
                let mut p = [Partial::default(); MAX_HARMONICS];
                p[1] = Partial {
                    freq_hz: 30_000.0,
                    mag: 1.0,
                };
                p
            },
        };
        let cfg = settings(2, 0.0, Waveform::Sine);
        let mut synth = primed(&voices, &cfg);
        skip(&mut synth, 256);
        let buf = collect(&mut synth, 1_024);
        let peak = buf.iter().fold(0.0f32, |a, x| a.max(x.abs()));
        assert!(peak == 0.0, "peak {peak}");
    }
}
