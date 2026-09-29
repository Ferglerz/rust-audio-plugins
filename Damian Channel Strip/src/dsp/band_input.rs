use crate::band::Band;
use pleasant_dsp::{
    filters::{BiquadCoefficients, BiquadKind, TransposedDirectForm2},
    units::linear_to_db_with_floor,
};

// Existing engine coverage includes 200 bands; reserve room for that case
// before the audio callback can create meters or publish their levels.
pub(crate) const DYNAMIC_BAND_METER_RESERVE: usize = 256;

/// Steep-ish bandpass level meters on pre-EQ audio for dynamic bands.
pub struct BandInputMeters {
    meters: Vec<BandInputMeter>,
    sample_rate: f64,
}

#[derive(Clone)]
struct BandInputMeter {
    id: u64,
    frequency_hz: f64,
    q: f64,
    filters: [TransposedDirectForm2; 2],
    coefficients: BiquadCoefficients,
    envelope: f64,
    attack: f64,
    release: f64,
}

impl BandInputMeters {
    #[cfg(test)]
    pub fn new(sample_rate: f64) -> Self {
        Self::with_capacity(sample_rate, DYNAMIC_BAND_METER_RESERVE)
    }

    pub(crate) fn with_capacity(sample_rate: f64, capacity: usize) -> Self {
        Self {
            meters: Vec::with_capacity(capacity.max(DYNAMIC_BAND_METER_RESERVE)),
            sample_rate: sample_rate.max(1.0),
        }
    }

    pub(crate) fn inherit(&mut self, previous: &Self) {
        self.meters.extend(previous.meters.iter().cloned());
    }

    pub fn reset(&mut self) {
        for meter in &mut self.meters {
            meter.filters = [TransposedDirectForm2::default(); 2];
            meter.envelope = 0.0;
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(1.0);
        for meter in &mut self.meters {
            meter.rebuild(self.sample_rate);
        }
    }

    /// Keep meters for dynamic gain bands and drop the rest.
    pub fn sync<'a, I>(&mut self, bands: I)
    where
        I: IntoIterator<Item = &'a Band>,
        I::IntoIter: Clone,
    {
        let bands = bands.into_iter();
        self.meters.retain(|m| {
            bands
                .clone()
                .any(|b| b.dynamic && b.shape.has_gain() && b.id == m.id)
        });
        for band in bands.filter(|b| b.dynamic && b.shape.has_gain()) {
            if let Some(meter) = self.meters.iter_mut().find(|m| m.id == band.id) {
                meter.update(band.freq, band.q, self.sample_rate);
            } else if self.meters.len() < self.meters.capacity() {
                self.meters.push(BandInputMeter::new(
                    band.id,
                    band.freq,
                    band.q,
                    self.sample_rate,
                ));
            }
        }
    }

    pub fn tick(&mut self, input: [f64; 2]) {
        for meter in &mut self.meters {
            meter.tick(input);
        }
    }

    pub fn write_levels_db(&self, levels: &mut Vec<(u64, f32)>) {
        levels.clear();
        // Telemetry is bounded independently of the number of audible EQ bands.
        levels.extend(
            self.meters
                .iter()
                .take(levels.capacity())
                .map(|m| (m.id, m.level_db() as f32)),
        );
    }
}

impl BandInputMeter {
    fn new(id: u64, frequency_hz: f64, q: f64, sample_rate: f64) -> Self {
        let mut meter = Self {
            id,
            frequency_hz,
            q,
            filters: [TransposedDirectForm2::default(); 2],
            coefficients: BiquadCoefficients::IDENTITY,
            envelope: 0.0,
            attack: 0.0,
            release: 0.0,
        };
        meter.rebuild(sample_rate);
        meter
    }

    fn steep_q(q: f64) -> f64 {
        (q * 2.5).clamp(2.0, 18.0)
    }

    fn rebuild(&mut self, sample_rate: f64) {
        let sample_rate = sample_rate.max(1.0);
        self.coefficients = BiquadCoefficients::design(
            BiquadKind::BandPass,
            self.frequency_hz,
            0.0,
            Self::steep_q(self.q),
            sample_rate,
        );
        self.attack = (-1.0 / (sample_rate * 0.003)).exp();
        self.release = (-1.0 / (sample_rate * 0.080)).exp();
    }

    fn update(&mut self, frequency_hz: f64, q: f64, sample_rate: f64) {
        if (frequency_hz - self.frequency_hz).abs() > 0.01 || (q - self.q).abs() > 0.001 {
            self.frequency_hz = frequency_hz;
            self.q = q;
            self.rebuild(sample_rate);
        }
    }

    fn tick(&mut self, input: [f64; 2]) {
        let mut peak = 0.0_f64;
        for (channel, sample) in input.iter().enumerate() {
            peak = peak.max(
                self.filters[channel]
                    .process(*sample, self.coefficients)
                    .abs(),
            );
        }
        let coefficient = if peak > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope = coefficient * self.envelope + (1.0 - coefficient) * peak;
    }

    fn level_db(&self) -> f64 {
        linear_to_db_with_floor(self.envelope, 1.0e-12).clamp(-90.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::band::Shape;
    use std::f64::consts::PI;

    #[test]
    fn steep_bandpass_reads_in_band_tone_above_out_of_band() {
        let sr = 48000.0;
        let mut meters = BandInputMeters::new(sr);
        meters.sync([&Band {
            id: 1,
            shape: Shape::Bell,
            freq: 1000.0,
            q: 1.0,
            dynamic: true,
            ..Band::default()
        }]);
        for n in 0..4800 {
            let t = n as f64 / sr;
            let sample = (2.0 * PI * 1000.0 * t).sin() * 0.5;
            meters.tick([sample, sample]);
        }
        let mut levels = Vec::with_capacity(DYNAMIC_BAND_METER_RESERVE);
        meters.write_levels_db(&mut levels);
        let in_band = levels[0].1;
        meters.sync([&Band {
            id: 1,
            shape: Shape::Bell,
            freq: 1000.0,
            q: 1.0,
            dynamic: true,
            ..Band::default()
        }]);
        meters.reset();
        for n in 0..4800 {
            let t = n as f64 / sr;
            let sample = (2.0 * PI * 200.0 * t).sin() * 0.5;
            meters.tick([sample, sample]);
        }
        meters.write_levels_db(&mut levels);
        let out_of_band = levels[0].1;
        assert!(
            in_band > out_of_band + 8.0,
            "in-band {in_band} should beat out-of-band {out_of_band}"
        );
    }
}
