use super::{finite, source};
use pleasant_eq::{BandCoefficients, BandSettings};
use serde::{Deserialize, Serialize};

const MAX_BANDS: usize = 16;
pub(crate) const MIDI_EQ_GAIN_DB: f32 = 24.0;
// Fixed reference rate keeps a saved MIDI curve independent of the host rate.
pub(crate) const MIDI_EQ_RATE: f64 = 192_000.0;

pub(crate) fn note_frequency(note: f64) -> f64 {
    440.0 * ((note - 69.0) / 12.0).exp2()
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiEqBand {
    pub note: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl Default for MidiEqBand {
    fn default() -> Self {
        Self {
            note: 60.0,
            gain_db: 0.0,
            q: 1.0,
        }
    }
}

impl MidiEqBand {
    pub(crate) fn coefficients(self) -> BandCoefficients {
        BandCoefficients::prepare(
            &BandSettings {
                frequency_hz: note_frequency(f64::from(self.note)),
                gain_db: f64::from(self.gain_db),
                q: f64::from(self.q),
                ..BandSettings::default()
            },
            MIDI_EQ_RATE,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiEqSettings {
    pub enabled: bool,
    pub sources: u8,
    pub bands: [Option<MidiEqBand>; MAX_BANDS],
}

impl Default for MidiEqSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sources: source::ALL,
            bands: [None; MAX_BANDS],
        }
    }
}

impl MidiEqSettings {
    pub(super) fn sanitize(&mut self) {
        self.sources &= source::ALL;
        for band in self.bands.iter_mut().flatten() {
            band.note = finite(band.note, 60.0).clamp(0.0, 127.0);
            band.gain_db = finite(band.gain_db, 0.0).clamp(-MIDI_EQ_GAIN_DB, MIDI_EQ_GAIN_DB);
            band.q = finite(band.q, 1.0).clamp(0.15, 18.0);
        }
    }

    pub(crate) fn prepare_gains(&self) -> [f32; 128] {
        let coefficients: Vec<_> = self
            .bands
            .iter()
            .flatten()
            .map(|band| band.coefficients())
            .collect();
        if coefficients.is_empty() {
            return [1.0; 128];
        }
        std::array::from_fn(|note| {
            let hz = note_frequency(note as f64);
            let db: f64 = coefficients
                .iter()
                .map(|band| band.response_db(hz, MIDI_EQ_RATE))
                .sum();
            10.0_f64.powf(db / 20.0) as f32
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_fx_eq_migrates_legacy_octave_gains_to_bells() {
        let mut eq: MidiEqSettings =
            serde_json::from_str(r#"{"enabled":true,"gains":[1,1,1,1,1,0.5,1,1,1,1,1]}"#).unwrap();
        eq.sanitize();
        assert!(eq.enabled);
        assert_eq!(eq.bands.iter().flatten().count(), 1);
        let band = eq.bands[5].unwrap();
        assert_eq!(band.note, 60.0);
        let gains = eq.prepare_gains();
        assert!((gains[60] - 0.5).abs() < 1.0e-5);
        assert!(gains[48] > gains[60] && gains[72] > gains[60]);
        let persisted = serde_json::to_string(&eq).unwrap();
        assert!(!persisted.contains("\"gains\""));
        assert_eq!(
            serde_json::from_str::<MidiEqSettings>(&persisted).unwrap(),
            eq
        );
    }
}
