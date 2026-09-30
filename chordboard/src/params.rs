use crate::engine::routing::{Route, ROUTE_COUNT, SOURCES, TARGETS};
use crate::engine::{Config, Mapping};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering},
    Arc,
};
#[derive(Params)]
pub struct ChordboardParams {
    #[id = "always_bass"]
    pub always_bass: BoolParam,
    #[id = "always_chord"]
    pub always_chord: BoolParam,
    #[id = "key_split"]
    pub key_split: BoolParam,
    #[id = "split_note"]
    pub split_note: IntParam,
    #[id = "root_on_select"]
    pub root_on_select: BoolParam,
    #[nested(array, group = "Modulation")]
    pub routes: [RouteParams; ROUTE_COUNT],
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[persist = "schema-version"]
    pub schema_version: AtomicU32,
    #[persist = "selected-quality-v1"]
    pub selected_quality: AtomicU32,
    #[persist = "control-zone-v1"]
    pub control_base: AtomicI32,
    #[persist = "map-x-v1"]
    pub map_x: AtomicU32,
    #[persist = "map-y-v1"]
    pub map_y: AtomicU32,
    #[persist = "chord-slot-1-v1"]
    pub slot_0: AtomicU64,
    #[persist = "chord-slot-2-v1"]
    pub slot_1: AtomicU64,
    #[persist = "chord-slot-3-v1"]
    pub slot_2: AtomicU64,
    #[persist = "chord-slot-4-v1"]
    pub slot_3: AtomicU64,
    #[persist = "chord-slot-5-v1"]
    pub slot_4: AtomicU64,
    #[persist = "chord-slot-6-v1"]
    pub slot_5: AtomicU64,
    #[persist = "chord-slot-7-v1"]
    pub slot_6: AtomicU64,
    #[persist = "chord-slot-8-v1"]
    pub slot_7: AtomicU64,
    #[id = "mode"]
    pub mode: IntParam,
    #[id = "quality"]
    pub quality: IntParam,
    #[id = "voice_leading"]
    pub voice_leading: IntParam,
    #[id = "inversion"]
    pub inversion: IntParam,
    #[id = "transpose"]
    pub transpose: IntParam,
    #[id = "spread"]
    pub spread: IntParam,
    #[id = "latch"]
    pub latch: BoolParam,
    #[id = "strum_latch"]
    pub strum_latch: BoolParam,
    #[id = "velocity"]
    pub velocity: FloatParam,
    #[id = "length_ms"]
    pub length_ms: FloatParam,
    #[id = "strings"]
    pub strings: IntParam,
    #[id = "strings_played"]
    pub strings_played: IntParam,
    #[id = "strum_ms"]
    pub strum_ms: FloatParam,
    #[id = "strum_sync"]
    pub strum_sync: BoolParam,
    #[id = "strum_beats"]
    pub strum_beats: FloatParam,
    #[id = "tempo_sync"]
    pub tempo_sync: BoolParam,
    #[id = "tempo"]
    pub tempo: FloatParam,
    #[id = "direction"]
    pub direction: IntParam,
    #[id = "contour"]
    pub contour: FloatParam,
    #[id = "arp_pattern"]
    pub arp_pattern: IntParam,
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "gate"]
    pub gate: FloatParam,
    #[id = "swing"]
    pub swing: FloatParam,
    #[id = "octaves"]
    pub octaves: IntParam,
    #[id = "humanize"]
    pub humanize: FloatParam,
    #[id = "mpe"]
    pub mpe: BoolParam,
    #[id = "output_mode"]
    pub output_mode: IntParam,
    pub detected_mpe: AtomicBool,
    #[id = "upper"]
    pub upper: BoolParam,
    #[id = "members"]
    pub members: IntParam,
    #[id = "bend_range"]
    pub bend_range: FloatParam,
    #[id = "master_range"]
    pub master_range: FloatParam,
    #[id = "output_channel"]
    pub output_channel: IntParam,
    #[id = "y_target"]
    pub y_target: IntParam,
    #[id = "y_cc"]
    pub y_cc: IntParam,
    #[id = "x_reverse"]
    pub x_reverse: BoolParam,
    #[id = "y_reverse"]
    pub y_reverse: BoolParam,
    #[id = "x_min"]
    pub x_min: FloatParam,
    #[id = "x_max"]
    pub x_max: FloatParam,
    #[id = "y_min"]
    pub y_min: FloatParam,
    #[id = "y_max"]
    pub y_max: FloatParam,
    #[id = "x"]
    pub x: FloatParam,
    #[id = "y"]
    pub y: FloatParam,
    #[id = "keyboard"]
    pub keyboard: BoolParam,
    #[id = "fifths"]
    pub fifths: BoolParam,
    #[id = "bank"]
    pub bank: IntParam,
    #[id = "keyboard_octave"]
    pub keyboard_octave: IntParam,
    #[id = "key"]
    pub key: IntParam,
    #[id = "scale"]
    pub scale: IntParam,
    // Retain the old parameter ID for saved projects; highlighting is always enabled.
    #[id = "highlight"]
    pub highlight: BoolParam,
    #[id = "filter"]
    pub filter: IntParam,
    #[id = "split_channels"]
    pub split_channels: BoolParam,
    #[id = "bass_channel"]
    pub bass_channel: IntParam,
    #[id = "upper_channel"]
    pub upper_channel: IntParam,
}
fn parse_split_note(text: &str) -> Option<i32> {
    let text = text.split('(').next()?.trim().to_ascii_uppercase();
    if let Ok(note) = text.parse::<i32>() {
        return (0..=127).contains(&note).then_some(note);
    }
    crate::harmony::NOTE_NAMES.iter().enumerate().find_map(|(pitch, name)| {
        let octave = text.strip_prefix(name)?.parse::<i32>().ok()?;
        let note = octave.checked_add(1)?.checked_mul(12)?.checked_add(pitch as i32)?;
        (0..=127).contains(&note).then_some(note)
    })
}

impl Default for ChordboardParams {
    fn default() -> Self {
        Self {
            always_bass: BoolParam::new("Always play bass", false),
            always_chord: BoolParam::new("Always play full chord", false),
            key_split: BoolParam::new("Key split", false),
            split_note: IntParam::new("First right-hand note", 60, IntRange::Linear { min: 0, max: 127 })
                .with_value_to_string(Arc::new(|v| format!("{}{} ({v})", crate::harmony::NOTE_NAMES[v as usize % 12], v / 12 - 1)))
                .with_string_to_value(Arc::new(parse_split_note)),
            root_on_select: BoolParam::new("Root on select", false),
            routes: std::array::from_fn(|_| RouteParams::default()),
            editor_state: ViziaState::new_screen_sized("Chordboard", || (1120, 856)),
            schema_version: AtomicU32::new(1),
            selected_quality: AtomicU32::new(0),
            control_base: AtomicI32::new(-1),
            map_x: AtomicU32::new(Mapping::cc(1, 16).encode()),
            map_y: AtomicU32::new(Mapping::default().encode()),
            slot_0: AtomicU64::new(0),
            slot_1: AtomicU64::new(0),
            slot_2: AtomicU64::new(0),
            slot_3: AtomicU64::new(0),
            slot_4: AtomicU64::new(0),
            slot_5: AtomicU64::new(0),
            slot_6: AtomicU64::new(0),
            slot_7: AtomicU64::new(0),
            mode: IntParam::new("Play mode", 1, IntRange::Linear { min: 1, max: 3 })
                .with_value_to_string(Arc::new(|v| {
                    ["Auto Strum", "Manual Strum", "Arpeggiator"][(v - 1) as usize].to_string()
                })),
            quality: IntParam::new("Base quality", 0, IntRange::Linear { min: 0, max: 11 })
                .with_value_to_string(Arc::new(|v| {
                    [
                        "Major", "Minor", "7", "Maj7", "Min7", "Dim", "Aug", "6", "Min6", "Dim7",
                        "Half dim", "Power",
                    ][v as usize]
                        .to_string()
                })),
            voice_leading: IntParam::new("Voice leading", 2, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| {
                    ["Nearest resolution", "Furthest dominant resolution", "Off"][v as usize]
                        .to_string()
                })),
            inversion: IntParam::new("Inversion", 0, IntRange::Linear { min: 0, max: 5 }),
            transpose: IntParam::new("Transpose", 0, IntRange::Linear { min: -24, max: 24 }),
            spread: IntParam::new("Spread", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| {
                    ["Close", "Open", "Wide"][v as usize].to_string()
                })),
            latch: BoolParam::new("Latch", false),
            strum_latch: BoolParam::new("Trackpad latch", false),
            velocity: FloatParam::new(
                "Velocity",
                0.8,
                FloatRange::Linear {
                    min: 0.01,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            length_ms: FloatParam::new(
                "Note length ms",
                350.0,
                FloatRange::Linear {
                    min: 20.0,
                    max: 3000.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            strings: IntParam::new("Strings", 8, IntRange::Linear { min: 3, max: 12 }),
            strings_played: IntParam::new(
                "Strings played",
                12,
                IntRange::Linear { min: 1, max: 12 },
            ),
            strum_ms: FloatParam::new(
                "Strum time ms",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1500.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            strum_sync: BoolParam::new("Sweep sync", false),
            strum_beats: FloatParam::new(
                "Sweep beats",
                0.25,
                FloatRange::Linear {
                    min: 0.0625,
                    max: 2.0,
                },
            ),
            tempo_sync: BoolParam::new("Host sync", true),
            tempo: FloatParam::new(
                "Tempo BPM",
                120.0,
                FloatRange::Linear {
                    min: 20.0,
                    max: 400.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.1}"))),
            direction: IntParam::new("Strum direction", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| {
                    ["Up", "Down", "Alternate"][v as usize].to_string()
                })),
            contour: FloatParam::new(
                "Velocity contour",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            arp_pattern: IntParam::new("Arp pattern", 0, IntRange::Linear { min: 0, max: 4 })
                .with_value_to_string(Arc::new(|v| {
                    ["Up", "Down", "Up/Down", "Played order", "Random"][v as usize].to_string()
                })),
            rate: FloatParam::new(
                "Arp beat length",
                0.25,
                FloatRange::Linear {
                    min: 0.0625,
                    max: 2.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            gate: FloatParam::new(
                "Arp gate",
                0.65,
                FloatRange::Linear {
                    min: 0.05,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            swing: FloatParam::new(
                "Swing",
                0.0,
                FloatRange::Linear {
                    min: -0.75,
                    max: 0.75,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            octaves: IntParam::new("Arp octaves", 1, IntRange::Linear { min: 1, max: 4 }),
            humanize: FloatParam::new("Humanize", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            mpe: BoolParam::new("MPE output (legacy)", false).hide(),
            output_mode: IntParam::new("Output protocol", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| ["AUTO", "MPE", "REG"][v as usize].into())),
            detected_mpe: AtomicBool::new(false),
            upper: BoolParam::new("Upper MPE zone", false),
            members: IntParam::new("MPE members", 15, IntRange::Linear { min: 1, max: 15 }),
            bend_range: FloatParam::new(
                "Member bend range",
                48.0,
                FloatRange::Linear {
                    min: 1.0,
                    max: 96.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            master_range: FloatParam::new(
                "Master bend range",
                2.0,
                FloatRange::Linear {
                    min: 1.0,
                    max: 24.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            output_channel: IntParam::new(
                "Standard MIDI channel",
                1,
                IntRange::Linear { min: 1, max: 16 },
            ),
            y_target: IntParam::new("Y destination", 0, IntRange::Linear { min: 0, max: 5 })
                .with_value_to_string(Arc::new(|v| {
                    [
                        "Velocity",
                        "Gate",
                        "Pressure",
                        "Timbre",
                        "Bend",
                        "Custom CC",
                    ][v as usize]
                        .to_string()
                })),
            y_cc: IntParam::new("Y custom CC", 11, IntRange::Linear { min: 0, max: 119 }),
            x_reverse: BoolParam::new("Reverse X", false),
            y_reverse: BoolParam::new("Reverse Y", false),
            x_min: FloatParam::new(
                "X minimum",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.99,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            x_max: FloatParam::new(
                "X maximum",
                1.0,
                FloatRange::Linear {
                    min: 0.01,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            y_min: FloatParam::new(
                "Y minimum",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.99,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            y_max: FloatParam::new(
                "Y maximum",
                1.0,
                FloatRange::Linear {
                    min: 0.01,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            x: FloatParam::new(
                "Strum position",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            y: FloatParam::new(
                "Expression position",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            keyboard: BoolParam::new("Keyboard play", true),
            fifths: BoolParam::new("Fifths layout", false),
            bank: IntParam::new("Root bank", 0, IntRange::Linear { min: 0, max: 1 })
                .with_value_to_string(Arc::new(|v| ["1", "2"][v as usize].to_string())),
            keyboard_octave: IntParam::new(
                "Keyboard base C",
                48,
                IntRange::Linear { min: 12, max: 96 },
            ),
            key: IntParam::new("Key", 0, IntRange::Linear { min: 0, max: 11 })
                .with_value_to_string(Arc::new(|v| {
                    [
                        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                    ][v as usize]
                        .to_string()
                })),
            scale: IntParam::new("Scale", 0, IntRange::Linear { min: 0, max: 7 })
                .with_value_to_string(Arc::new(|v| {
                    [
                        "Major",
                        "Natural minor",
                        "Harmonic minor",
                        "Dorian",
                        "Mixolydian",
                        "Lydian",
                        "Major pentatonic",
                        "Minor pentatonic",
                    ][v as usize]
                        .to_string()
                })),
            highlight: BoolParam::new("Key highlighting", true)
                .hide()
                .hide_in_generic_ui(),
            filter: IntParam::new("Note filter", 0, IntRange::Linear { min: 0, max: 5 })
                .with_value_to_string(Arc::new(|v| {
                    [
                        "All",
                        "Bass",
                        "Top",
                        "Bass + Top",
                        "Odd tones",
                        "Even tones",
                    ][v as usize]
                        .to_string()
                })),
            split_channels: BoolParam::new("Split bass / upper (non-MPE)", false),
            bass_channel: IntParam::new("Bass channel", 1, IntRange::Linear { min: 1, max: 16 }),
            upper_channel: IntParam::new("Upper channel", 2, IntRange::Linear { min: 1, max: 16 }),
        }
    }
}
impl ChordboardParams {
    pub fn mpe_enabled(&self) -> bool {
        match self.output_mode.value() {
            1 => true,
            2 => false,
            _ => self.detected_mpe.load(Ordering::Relaxed),
        }
    }
    pub fn config(&self) -> Config {
        Config {
            always_bass: self.always_bass.value(),
            always_chord: self.always_chord.value(),
            key_split: self.key_split.value(),
            split_note: self.split_note.value() as u8,
            root_on_select: self.root_on_select.value(),
            routes: std::array::from_fn(|i| self.routes[i].route()),
            mode: self.mode.value() as u8,
            quality: self.quality.value() as u8,
            voice_leading: self.voice_leading.value() as u8,
            inversion: self.inversion.value() as u8,
            transpose: self.transpose.value() as i8,
            spread: self.spread.value() as u8,
            latch: self.latch.value(),
            velocity: self.velocity.value(),
            length_ms: self.length_ms.value(),
            strings: self.strings.value() as u8,
            strings_played: self.strings_played.value() as u8,
            routed_x: None,
            strum_ms: self.strum_ms.value(),
            strum_sync: self.strum_sync.value(),
            strum_beats: self.strum_beats.value(),
            direction: self.direction.value() as u8,
            contour: self.contour.value(),
            arp_pattern: self.arp_pattern.value() as u8,
            rate: self.rate.value(),
            gate: self.gate.value(),
            swing: self.swing.value(),
            octaves: self.octaves.value() as u8,
            humanize: self.humanize.value(),
            mpe: self.mpe_enabled(),
            upper: self.upper.value(),
            members: self.members.value() as u8,
            bend_range: self.bend_range.value(),
            master_range: self.master_range.value(),
            output_channel: (self.output_channel.value() - 1) as u8,
            y_target: self.y_target.value() as u8,
            y_cc: self.y_cc.value() as u8,
            x_reverse: self.x_reverse.value(),
            y_reverse: self.y_reverse.value(),
            x_min: self.x_min.value(),
            x_max: self.x_max.value(),
            y_min: self.y_min.value(),
            y_max: self.y_max.value(),
            filter: self.filter.value() as u8,
            split_channels: self.split_channels.value(),
            bass_channel: (self.bass_channel.value() - 1) as u8,
            upper_channel: (self.upper_channel.value() - 1) as u8,
            control_base: self.control_base.load(Ordering::Relaxed).clamp(-1, 116) as i16,
            mappings: [
                Mapping::decode(self.map_x.load(Ordering::Relaxed)),
                Mapping::decode(self.map_y.load(Ordering::Relaxed)),
            ],
        }
    }
    pub fn mapping(&self, axis: usize) -> &AtomicU32 {
        match axis {
            0 => &self.map_x,
            _ => &self.map_y,
        }
    }
}

// One catalog supports both full UI snapshots and allocation-light single lookups.
macro_rules! control_catalog {
    ($($builder:ident($field:ident, $group:literal)),* $(,)?) => {
        impl ChordboardParams {
            pub fn controls(&self) -> Vec<Control> {
                let mut controls = vec![$($builder(stringify!($field), $group, &self.$field)),*];
                for route in &self.routes { controls.extend(route.controls()); }
                controls
            }

            pub fn control(&self, id: &str) -> Option<Control> {
                match id {
                    $(stringify!($field) => Some($builder(stringify!($field), $group, &self.$field)),)*
                    _ => None,
                }
            }
        }
    };
}

control_catalog! {
    toggle(always_bass, 0),
    toggle(always_chord, 0),
    toggle(key_split, 0),
    control(split_note, 0),
    control(mode, 0),
    control(quality, 0),
    control(inversion, 0),
    control(voice_leading, 0),
    control(transpose, 0),
    control(spread, 0),
    toggle(latch, 0),
    toggle(root_on_select, 0),
    toggle(strum_latch, 1),
    control(velocity, 0),
    control(length_ms, 0),
    control(strings, 1),
    control(strings_played, 1),
    control(strum_ms, 1),
    toggle(strum_sync, 1),
    control(strum_beats, 1),
    toggle(tempo_sync, 1),
    control(tempo, 1),
    control(direction, 1),
    control(contour, 1),
    control(arp_pattern, 1),
    control(rate, 1),
    control(gate, 1),
    control(swing, 1),
    control(octaves, 1),
    control(humanize, 1),
    control(output_mode, 2),
    toggle(upper, 2),
    control(members, 2),
    control(bend_range, 2),
    control(master_range, 2),
    control(output_channel, 2),
    control(y_target, 3),
    control(y_cc, 3),
    toggle(x_reverse, 3),
    toggle(y_reverse, 3),
    control(x_min, 3),
    control(x_max, 3),
    control(y_min, 3),
    control(y_max, 3),
    control(x, 4),
    control(y, 4),
    toggle(keyboard, 4),
    toggle(fifths, 4),
    control(bank, 4),
    control(keyboard_octave, 4),
    control(key, 5),
    control(scale, 5),
    control(filter, 5),
    toggle(split_channels, 5),
    control(bass_channel, 5),
    control(upper_channel, 5),
}

#[derive(Clone)]
pub struct Control {
    pub id: &'static str,
    pub group: usize,
    pub name: String,
    pub ptr: ParamPtr,
    pub norm: f32,
    pub prev: f32,
    pub next: f32,
    pub value: String,
    pub toggle: bool,
}
fn control(id: &'static str, group: usize, p: &impl Param) -> Control {
    let norm = p.unmodulated_normalized_value();
    Control {
        id,
        group,
        name: p.name().to_string(),
        ptr: p.as_ptr(),
        norm,
        prev: p.previous_normalized_step(norm, false),
        next: p.next_normalized_step(norm, false),
        value: p.normalized_value_to_string(norm, true),
        toggle: false,
    }
}
fn toggle(id: &'static str, group: usize, p: &BoolParam) -> Control {
    let mut c = control(id, group, p);
    c.toggle = true;
    c
}
impl ChordboardParams {
    pub fn slot(&self, index: usize) -> &AtomicU64 {
        match index {
            0 => &self.slot_0,
            1 => &self.slot_1,
            2 => &self.slot_2,
            3 => &self.slot_3,
            4 => &self.slot_4,
            5 => &self.slot_5,
            6 => &self.slot_6,
            _ => &self.slot_7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_note_readout_roundtrips_and_rejects_out_of_range_notes() {
        for note in 0..=127 {
            let label = format!("{}{} ({note})", crate::harmony::NOTE_NAMES[note as usize % 12], note / 12 - 1);
            assert_eq!(parse_split_note(&label), Some(note));
        }
        assert_eq!(parse_split_note("C#4"), Some(61));
        assert_eq!(parse_split_note("60"), Some(60));
        assert_eq!(parse_split_note("128"), None);
        assert_eq!(parse_split_note("C-2"), None);
    }

    #[test]
    fn main_buttons_resolve_their_parameters_without_settings_duplicates() {
        let params = ChordboardParams::default();
        let controls = params.controls();
        assert!(params.control("highlight").is_none());
        for (id, ptr) in [
            ("keyboard", params.keyboard.as_ptr()),
            ("latch", params.latch.as_ptr()),
            ("strum_latch", params.strum_latch.as_ptr()),
            ("output_mode", params.output_mode.as_ptr()),
            ("fifths", params.fifths.as_ptr()),
        ] {
            assert_eq!(
                controls.iter().find(|c| c.id == id).map(|c| c.ptr),
                Some(ptr),
                "{id}"
            );
        }
    }

    #[test]
    fn saved_fields_roundtrip_without_held_notes() {
        let params = ChordboardParams::default();
        let chord = crate::harmony::SavedChord {
            root: 60,
            second: Some(66),
            quality: 0,
            inversion: 1,
            spread: 1,
            transpose: 12,
        };
        params.control_base.store(36, Ordering::Relaxed);
        params.slot(7).store(chord.encode(), Ordering::Relaxed);
        params.map_x.store(
            Mapping {
                kind: 2,
                number: 1,
                channel: 0,
            }
            .encode(),
            Ordering::Relaxed,
        );
        let fields = params.serialize_fields();
        let loaded = ChordboardParams::default();
        loaded.deserialize_fields(&fields);
        assert_eq!(loaded.slot(7).load(Ordering::Relaxed), chord.encode());
        assert_eq!(loaded.config().control_base, 36);
        assert_eq!(loaded.config().mappings[0].kind, 2);
        assert!(!fields
            .keys()
            .any(|k| k.contains("held") || k.contains("voice")));
        let ids: Vec<_> = params.param_map().into_iter().map(|p| p.0).collect();
        let unique: std::collections::BTreeSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
    }
}

#[derive(Params)]
pub struct RouteParams {
    #[id = "route_enabled"]
    pub enabled: BoolParam,
    #[id = "route_source"]
    pub source: IntParam,
    #[id = "route_target"]
    pub target: IntParam,
    #[id = "route_min"]
    pub min: FloatParam,
    #[id = "route_max"]
    pub max: FloatParam,
    #[id = "route_curve"]
    pub curve: FloatParam,
}
impl Default for RouteParams {
    fn default() -> Self {
        Self {
            enabled: BoolParam::new("Enabled", true),
            source: IntParam::new(
                "Source",
                0,
                IntRange::Linear {
                    min: 0,
                    max: (SOURCES.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| SOURCES[v as usize].into())),
            target: IntParam::new(
                "Destination",
                0,
                IntRange::Linear {
                    min: 0,
                    max: (TARGETS.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| TARGETS[v as usize].name.into())),
            min: FloatParam::new("Minimum", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0)))
                .with_string_to_value(Arc::new(|s| {
                    s.trim()
                        .trim_end_matches('%')
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .map(|v| v / 100.0)
                })),
            max: FloatParam::new("Maximum", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0)))
                .with_string_to_value(Arc::new(|s| {
                    s.trim()
                        .trim_end_matches('%')
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .map(|v| v / 100.0)
                })),
            curve: FloatParam::new(
                "Curve",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| {
                if v.abs() < 0.005 {
                    "Linear".into()
                } else {
                    format!("{v:+.2}")
                }
            }))
            .with_string_to_value(Arc::new(|s| {
                if s.trim().eq_ignore_ascii_case("linear") {
                    Some(0.0)
                } else {
                    s.trim().parse().ok()
                }
            })),
        }
    }
}
impl RouteParams {
    pub fn route(&self) -> Route {
        Route {
            enabled: self.enabled.value(),
            source: self.source.value() as u8,
            target: self.target.value() as u8,
            min: self.min.value(),
            max: self.max.value(),
            curve: self.curve.value(),
        }
    }
    pub fn controls(&self) -> Vec<Control> {
        vec![
            toggle("route_enabled", 6, &self.enabled),
            control("route_source", 6, &self.source),
            control("route_target", 6, &self.target),
            control("route_min", 6, &self.min),
            control("route_max", 6, &self.max),
            control("route_curve", 6, &self.curve),
        ]
    }
    pub fn control(&self, id: &str) -> Option<Control> {
        self.controls().into_iter().find(|c| c.id == id)
    }
}

#[cfg(test)]
mod routing_state_tests {
    use super::*;
    #[test]
    fn all_route_fields_have_stable_unique_host_ids_and_legacy_defaults_are_inert() {
        let p = ChordboardParams::default();
        let ids: Vec<_> = p.param_map().into_iter().map(|v| v.0).collect();
        for field in [
            "route_source",
            "route_target",
            "route_min",
            "route_max",
            "route_curve",
            "route_enabled",
        ] {
            assert_eq!(
                ids.iter().filter(|id| id.starts_with(field)).count(),
                ROUTE_COUNT,
                "{field}"
            );
        }
        assert_eq!(
            ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
            ids.len()
        );
        assert!(!p.config().root_on_select);
        assert!(p.config().routes.iter().all(|r| !r.active()));
        let range = &p.routes[0];
        assert_eq!(range.min.normalized_value_to_string(0.5, true), "50%");
    }
}
