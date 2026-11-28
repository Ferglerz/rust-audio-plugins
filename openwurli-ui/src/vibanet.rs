//! Five-position bass, mid and treble controls on the Vibanet preamp.
use clavinet_d6::vibanet::{Controls, Selector};
use nih_plug::prelude::*;

pub const SELECTORS: [(Selector, &str); 4] = [
    (Selector::Lead, "Lead"),
    (Selector::Rhythm, "Rhythm"),
    (Selector::Fulltone, "Fulltone"),
    (Selector::Phase, "Phase"),
];

#[derive(Params)]
pub struct VibanetTone {
    #[id = "selector"]
    pub selector: IntParam,
    #[id = "low"]
    pub low: IntParam,
    #[id = "mid"]
    pub mid: IntParam,
    #[id = "high"]
    pub high: IntParam,
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "wah"]
    pub wah: BoolParam,
    #[id = "sens"]
    pub sensitivity: FloatParam,
    #[id = "mute"]
    pub mute: FloatParam,
}

impl Default for VibanetTone {
    fn default() -> Self {
        use std::sync::Arc;
        let eq = |name| {
            IntParam::new(name, 0, IntRange::Linear { min: -2, max: 2 })
                .with_value_to_string(Arc::new(|v| format!("{:+} dB", f64::from(v) * 7.5)))
                .with_string_to_value(Arc::new(|s| {
                    let gain: f32 = s.trim().trim_end_matches("dB").trim().parse().ok()?;
                    Some((gain / 7.5).round().clamp(-2., 2.) as i32)
                }))
        };
        let level = |name, stock| {
            FloatParam::new(name, stock, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            selector: IntParam::new("Vibanet Pickup", 2, IntRange::Linear { min: 0, max: 3 })
                .with_value_to_string(Arc::new(|i| SELECTORS[i.clamp(0, 3) as usize].1.to_owned()))
                .with_string_to_value(Arc::new(|s| {
                    SELECTORS
                        .iter()
                        .position(|(_, label)| label.eq_ignore_ascii_case(s.trim()))
                        .map(|i| i as i32)
                })),
            low: eq("Vibanet Bass"),
            mid: eq("Vibanet Mid"),
            high: eq("Vibanet Treble"),
            volume: level("Vibanet Volume", 1.),
            wah: BoolParam::new("Vibanet Auto-Wah", false),
            sensitivity: level("Vibanet Wah Sensitivity", 0.5),
            mute: level("Vibanet Mute Bar", 0.),
        }
    }
}
impl VibanetTone {
    pub fn controls(&self) -> Controls {
        Controls {
            pickup: SELECTORS[self.selector.value() as usize].0,
            eq: [&self.low, &self.mid, &self.high].map(|p| p.value() as i8),
            volume: self.volume.value().into(),
            wah: self.wah.value(),
            sensitivity: self.sensitivity.value().into(),
            mute: self.mute.value().into(),
        }
    }
}
