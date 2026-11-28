//! Duo console settings. Its individual host parameter IDs are independent of the D6 and T.
use crate::models::Pickup;
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Registration {
    #[id = "preset"]
    #[name = "Preset"]
    Preset,
    #[id = "p_treble"]
    #[name = "Clavinet bass / Pianet treble"]
    PianetTreble,
    #[id = "c_treble"]
    #[name = "Pianet bass / Clavinet treble"]
    ClavinetTreble,
    #[id = "both_treble"]
    #[name = "Pianet bass / Both treble"]
    BothTreble,
}
impl Registration {
    pub const ALL: [Self; 4] = [
        Self::Preset,
        Self::PianetTreble,
        Self::ClavinetTreble,
        Self::BothTreble,
    ];
    fn split(self) -> hohner_duo::Split {
        match self {
            Self::Preset => hohner_duo::Split::Preset,
            Self::PianetTreble => hohner_duo::Split::PianetTreble,
            Self::ClavinetTreble => hohner_duo::Split::ClavinetTreble,
            Self::BothTreble => hohner_duo::Split::BothTreble,
        }
    }
}

#[derive(Params)]
pub struct DuoTone {
    #[id = "split"]
    pub split: EnumParam<Registration>,
    #[id = "two"]
    pub two: BoolParam,
    #[id = "three"]
    pub three: BoolParam,
    #[id = "foot"]
    pub foot: BoolParam,
    #[id = "pickup"]
    pub pickup: EnumParam<Pickup>,
    #[id = "soft"]
    pub soft: BoolParam,
    #[id = "medium"]
    pub medium: BoolParam,
    #[id = "treble"]
    pub treble: BoolParam,
    #[id = "brill"]
    pub brilliant: BoolParam,
    #[id = "c_vol"]
    pub clavinet_volume: FloatParam,
    #[id = "p_vol"]
    pub pianet_volume: FloatParam,
    #[id = "mute"]
    pub mute: BoolParam,
    #[id = "power"]
    pub power: BoolParam,
    #[id = "record"]
    pub recording: BoolParam,
    midi_foot: AtomicBool,
}
impl Default for DuoTone {
    fn default() -> Self {
        let slider = |name| {
            FloatParam::new(name, 1., FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(std::sync::Arc::new(|x| format!("{:.1}", x * 5.)))
                .with_string_to_value(std::sync::Arc::new(|s| {
                    s.trim().parse::<f32>().ok().map(|x| x / 5.)
                }))
        };
        Self {
            split: EnumParam::new("Duo Keyboard Selector", Registration::Preset),
            two: BoolParam::new("Duo Tab 2", true),
            three: BoolParam::new("Duo Tab 3", true),
            foot: BoolParam::new("Duo Footswitch B", false),
            pickup: EnumParam::new("Duo String Pickup", Pickup::Centre),
            soft: BoolParam::new("Duo Soft", true),
            medium: BoolParam::new("Duo Medium", true),
            treble: BoolParam::new("Duo Treble", true),
            brilliant: BoolParam::new("Duo Brilliant", true),
            clavinet_volume: slider("Duo Clavinet Volume"),
            pianet_volume: slider("Duo Pianet Volume"),
            mute: BoolParam::new("Duo String Mute", false),
            power: BoolParam::new("Duo Power", true),
            recording: BoolParam::new("Duo Separate Recording Outputs", false),
            midi_foot: AtomicBool::new(false),
        }
    }
}
impl DuoTone {
    pub fn tabs(&self) -> [&BoolParam; 4] {
        [&self.soft, &self.medium, &self.treble, &self.brilliant]
    }
    pub fn foot_is_b(&self) -> bool {
        self.foot.value() || self.midi_foot.load(Ordering::Relaxed)
    }
    pub fn controls(&self, midi_foot: bool) -> hohner_duo::Controls {
        self.midi_foot.store(midi_foot, Ordering::Relaxed);
        self.board_controls(self.foot.value() || midi_foot)
    }
    pub fn display_state(&self) -> (hohner_duo::Controls, bool) {
        (
            self.board_controls(self.foot_is_b()),
            self.separate_outputs(),
        )
    }
    fn board_controls(&self, foot: bool) -> hohner_duo::Controls {
        hohner_duo::Controls {
            split: self.split.value().split(),
            tab_two: self.two.value(),
            tab_three: self.three.value(),
            foot,
            pickup: self.pickup.value().selector(),
            tones: self
                .tabs()
                .iter()
                .enumerate()
                .fold(0, |a, (i, v)| a | (u8::from(v.value()) << i)),
            clavinet_volume: self.clavinet_volume.value().into(),
            pianet_volume: self.pianet_volume.value().into(),
            mute: self.mute.value(),
            power: self.power.value(),
            ..Default::default()
        }
    }
    pub fn separate_outputs(&self) -> bool {
        self.recording.value()
    }
}
