use crate::models::{ClavinetModel, Instrument};
use crate::params::{GrandModel, PianoType};

/// Headphone and speaker terminal voltages are aligned before their temporal crossfade.
pub(super) fn early_local_gain(model: pianet_n::Model) -> f64 {
    let db: f64 = match model {
        pianet_n::Model::C => -18.15,
        pianet_n::Model::L => -32.11,
        _ => return 1.,
    };
    10_f64.powf(db / 20.)
}

// Fixed dB trims relative to the original jack outputs, rounded down to 0.01 dB.
// Reproduce the −1 dBFS performance peak with src/tests/calibration.rs.
pub(super) fn trim(piano: impl Into<Instrument>, grand: GrandModel, original_cp70: bool) -> f32 {
    nih_plug::util::db_to_gain(match piano.into() {
        Instrument::Electra => 25.40,
        Instrument::Storytone => -0.36,
        Instrument::Neo => -18.71,
        Instrument::Elepian => 6.39,
        Instrument::Vv64 => -5.77,
        Instrument::StereoVibe => -9.54,
        Instrument::Road => 32.05,
        Instrument::Baldwin => -16.75,
        Instrument::Mk8 => -0.50,
        Instrument::Vibanet => 9.40,
        Instrument::Valente => -3.73,
        Instrument::Vivi => 44.50,
        Instrument::Claviset(pianet_n::claviset::Instrument::Claviset200) => -6.63,
        Instrument::Claviset(pianet_n::claviset::Instrument::Pianotron) => -13.39,
        Instrument::Duo => -10.97,
        Instrument::Clavinet(ClavinetModel::D6) => -7.19,
        Instrument::Clavinet(ClavinetModel::E7) => 1.15,
        Instrument::Reed(pianet_n::Model::N) => -5.26,
        Instrument::Reed(pianet_n::Model::C) => 17.75,
        Instrument::Reed(pianet_n::Model::L) => 21.09,
        Instrument::Reed(pianet_n::Model::Cembalet) => 0.05,
        Instrument::Reed(pianet_n::Model::CembaletN) => -0.22,
        Instrument::Reed(pianet_n::Model::T) => 3.13,
        Instrument::Piano(PianoType::Wurli) => -3.21,
        Instrument::Piano(PianoType::Stage) => -4.66,
        Instrument::Piano(PianoType::Grand) => match grand {
            GrandModel::Cp80 => 1.24,
            GrandModel::Cp70B if original_cp70 => 1.89,
            GrandModel::Cp70B => 0.88,
        },
    })
}

/// De-click rapid model changes without cold-starting or rendering two engines.
#[derive(Default)]
pub(super) struct Transition {
    last: [f32; 2],
    from: [f32; 2],
    remaining: u16,
}
impl Transition {
    const LENGTH: u16 = 128;
    pub fn start(&mut self) {
        self.from = self.last;
        self.remaining = Self::LENGTH;
    }
    pub fn apply(&mut self, mut frame: [f32; 2]) -> [f32; 2] {
        if self.remaining > 0 {
            let t = f32::from(self.remaining) / f32::from(Self::LENGTH);
            for (sample, old) in frame.iter_mut().zip(self.from) {
                *sample += t * (old - *sample);
            }
            self.remaining -= 1;
        }
        self.last = frame;
        frame
    }
}
