use pleasant_dsp::units::linear_to_db_with_floor;

pub const PSE_RMS_TIMES: [f64; 6] = [0.050, 0.100, 0.200, 0.750, 1.500, 3.000];
pub const PSE_PEAK_RELEASES: [f64; 6] = [0.020, 0.200, 1.000, 2.000, 5.000, 30.000];
pub const VOICE_THRESH_SHIFT_DB: f64 = 3.0;

pub fn pse_time_to_times(position: f64, peak: bool) -> (f64, f64) {
    let position = position.clamp(0.0, 5.0);
    let index = (position.floor() as usize).min(4);
    let fraction = position - index as f64;
    if peak {
        let start = PSE_PEAK_RELEASES[index];
        let end = PSE_PEAK_RELEASES[index + 1];
        let release = (start.ln() * (1.0 - fraction) + end.ln() * fraction).exp();
        (0.020, release)
    } else {
        let start = PSE_RMS_TIMES[index];
        let end = PSE_RMS_TIMES[index + 1];
        let time = (start.ln() * (1.0 - fraction) + end.ln() * fraction).exp();
        (time, time)
    }
}

pub fn seconds_to_pse_time_pos(seconds: f64) -> f64 {
    if seconds <= PSE_RMS_TIMES[0] {
        return 0.0;
    }
    if seconds >= PSE_RMS_TIMES[5] {
        return 5.0;
    }
    for index in 0..5 {
        let start = PSE_RMS_TIMES[index];
        let end = PSE_RMS_TIMES[index + 1];
        if seconds <= end {
            let fraction = (seconds.ln() - start.ln()) / (end.ln() - start.ln());
            return index as f64 + fraction;
        }
    }
    5.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum PseTimeConstant {
    A,
    B,
    #[default]
    C,
    D,
    E,
    F,
}

impl PseTimeConstant {
    pub fn label(self) -> &'static str {
        ["A", "B", "C", "D", "E", "F"][self as usize]
    }

    pub fn times(self, peak: bool) -> (f64, f64) {
        pse_time_to_times(self as usize as f64, peak)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PseSettings {
    pub depth: f64,
    pub hysteresis: f64,
    pub knee: f64,
    pub peak: bool,
    pub time: f64,
    pub listen: bool,
    pub speech_env: f64,
    pub vad_assist: f64,
}

impl Default for PseSettings {
    fn default() -> Self {
        Self {
            depth: 10.0,
            hysteresis: 3.0,
            knee: 6.0,
            peak: false,
            time: 2.0,
            listen: false,
            speech_env: 0.0,
            vad_assist: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pse {
    rms_squared: f64,
    effective_threshold: f64,
    pub reduction_db: f64,
    sample_rate: f64,
    rms_coefficient: f64,
    hysteresis_coefficient: f64,
    gain_coefficient: f64,
    release_coefficient: f64,
    peak_coefficient: f64,
    peak_envelope: f64,
    peak_mode: bool,
    time: f64,
}

impl Default for Pse {
    fn default() -> Self {
        Self {
            rms_squared: 0.0,
            effective_threshold: -80.0,
            reduction_db: 0.0,
            sample_rate: 0.0,
            rms_coefficient: 0.0,
            hysteresis_coefficient: 0.0,
            gain_coefficient: 0.0,
            release_coefficient: 0.0,
            peak_coefficient: 0.0,
            peak_envelope: 0.0,
            peak_mode: false,
            time: 2.0,
        }
    }
}

impl Pse {
    pub fn tick(
        &mut self,
        filtered: f64,
        threshold: f64,
        sample_rate: f64,
        settings: PseSettings,
    ) -> f64 {
        self.process(filtered, threshold, sample_rate, settings)
    }

    pub(crate) fn process(
        &mut self,
        filtered: f64,
        threshold: f64,
        sample_rate: f64,
        settings: PseSettings,
    ) -> f64 {
        if self.sample_rate != sample_rate
            || self.peak_mode != settings.peak
            || (self.time - settings.time).abs() > 1.0e-4
        {
            self.peak_mode = settings.peak;
            self.time = settings.time;
            self.sample_rate = sample_rate;
            self.rms_coefficient = (-1.0 / (sample_rate * 0.010)).exp();
            self.hysteresis_coefficient = (-1.0 / (sample_rate * 0.005)).exp();
            let (attack, release) = pse_time_to_times(settings.time, settings.peak);
            self.gain_coefficient = (-1.0 / (sample_rate * attack)).exp();
            self.release_coefficient = (-1.0 / (sample_rate * release)).exp();
            self.peak_coefficient = (-1.0 / (sample_rate * 0.0015)).exp();
        }
        self.rms_squared = self.rms_squared * self.rms_coefficient
            + filtered * filtered * (1.0 - self.rms_coefficient);
        if self.rms_squared.abs() < 1.0e-25 {
            self.rms_squared = 0.0;
        }
        self.peak_envelope = self.peak_envelope * self.peak_coefficient
            + filtered.abs() * (1.0 - self.peak_coefficient);
        let level = if settings.peak {
            if self.peak_envelope > 1.0e-7 {
                linear_to_db_with_floor(self.peak_envelope, 1.0e-12)
            } else {
                -140.0
            }
        } else if self.rms_squared > 1.0e-14 {
            10.0 * self.rms_squared.log10()
        } else {
            -140.0
        };
        let voice_shift = settings.vad_assist * VOICE_THRESH_SHIFT_DB * settings.speech_env;
        let voice_threshold = if threshold <= -79.9 {
            threshold
        } else {
            threshold - voice_shift
        };
        let voice_depth = settings.depth * (1.0 - settings.vad_assist * 0.15 * settings.speech_env);
        let target_threshold = if level > self.effective_threshold {
            voice_threshold - settings.hysteresis
        } else {
            voice_threshold
        };
        self.effective_threshold = self.effective_threshold * self.hysteresis_coefficient
            + target_threshold * (1.0 - self.hysteresis_coefficient);
        let delta = level - self.effective_threshold;
        let knee = settings.knee.max(0.01);
        let weight = ((delta + knee * 0.5) / knee).clamp(0.0, 1.0);
        let weight = weight * weight * (3.0 - 2.0 * weight);
        let target = if threshold <= -79.9 {
            0.0
        } else {
            -voice_depth * (1.0 - weight)
        };
        let coefficient = if target > self.reduction_db {
            self.gain_coefficient
        } else {
            self.release_coefficient
        };
        self.reduction_db = self.reduction_db * coefficient + target * (1.0 - coefficient);
        if self.reduction_db.abs() < 1.0e-20 {
            self.reduction_db = 0.0;
        }
        10.0_f64.powf(self.reduction_db / 20.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_positions_round_trip_at_anchors() {
        for (index, seconds) in PSE_RMS_TIMES.into_iter().enumerate() {
            assert!((seconds_to_pse_time_pos(seconds) - index as f64).abs() < 1.0e-10);
        }
    }
}
