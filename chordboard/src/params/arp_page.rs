use super::catalog::{control, toggle};
use super::*;

#[derive(Params)]
pub struct ArpPageParams {
    #[id = "arp_mode"]
    pub arp_mode: IntParam,
    #[id = "arp_pattern"]
    pub arp_pattern: IntParam,
    #[id = "arp_octaves"]
    pub arp_octaves: IntParam,
    #[id = "arp_sync"]
    pub arp_sync: BoolParam,
    #[id = "arp_free_ms"]
    pub arp_free_ms: FloatParam,
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "swing"]
    pub swing: FloatParam,
    #[id = "humanize"]
    pub humanize: FloatParam,
}

impl Default for ArpPageParams {
    fn default() -> Self {
        Self::from_settings(crate::arpeggiator::Settings::default())
    }
}

impl ArpPageParams {
    /// UI-only adapters keep the existing ranges and formatting for tracker row controls.
    pub fn from_settings(settings: crate::arpeggiator::Settings) -> Self {
        Self {
            arp_mode: IntParam::new(
                "Arp repeat",
                settings.mode as i32,
                IntRange::Linear { min: 0, max: 3 },
            )
            .with_value_to_string(Arc::new(|v| {
                crate::arpeggiator::MODE_NAMES[v as usize].into()
            })),
            arp_pattern: IntParam::new(
                "Ordering",
                settings.pattern as i32,
                IntRange::Linear { min: 0, max: 7 },
            )
            .with_value_to_string(Arc::new(|v| {
                crate::arpeggiator::PATTERN_NAMES[v as usize].into()
            })),
            arp_octaves: IntParam::new(
                "Loops",
                settings.octaves as i32,
                IntRange::Linear { min: 1, max: 4 },
            ),
            arp_sync: BoolParam::new("Arp tempo sync", settings.sync),
            arp_free_ms: FloatParam::new(
                "Arp rate ms",
                settings.free_ms,
                FloatRange::Linear {
                    min: 1.0,
                    max: 1500.0,
                },
            ),
            rate: FloatParam::new(
                "Arp rate beats",
                settings.rate,
                FloatRange::Linear {
                    min: 0.0625,
                    max: 2.0,
                },
            ),
            swing: FloatParam::new(
                "Swing",
                settings.swing,
                FloatRange::Linear {
                    min: -0.75,
                    max: 0.75,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{:+.0}%", v / 0.75 * 100.0)))
            .with_string_to_value(Arc::new(|s| {
                s.trim()
                    .trim_end_matches('%')
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .map(|v| v * 0.75 / 100.0)
            })),
            humanize: FloatParam::new(
                "Humanize",
                settings.humanize,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
        }
    }

    pub fn settings(&self) -> crate::arpeggiator::Settings {
        crate::arpeggiator::Settings {
            mode: crate::arpeggiator::ArpMode::from_index(self.arp_mode.value() as u8),
            pattern: self.arp_pattern.value() as u8,
            octaves: self.arp_octaves.value() as u8,
            sync: self.arp_sync.value(),
            rate: self.rate.value(),
            free_ms: self.arp_free_ms.value(),
            swing: self.swing.value(),
            humanize: self.humanize.value(),
            ..Default::default()
        }
    }
}

nested_control_catalog! {
    ArpPageParams, 8,
    control("arp_mode", arp_mode, 1),
    control("arp_pattern", arp_pattern, 1),
    control("arp_octaves", arp_octaves, 1),
    toggle("arp_sync", arp_sync, 1),
    control("arp_free_ms", arp_free_ms, 1),
    control("rate", rate, 1),
    control("swing", swing, 1),
    control("humanize", humanize, 1),
}
