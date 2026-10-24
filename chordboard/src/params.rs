use crate::engine::routing::{Route, ROUTE_COUNT, SOURCE_COUNT, SOURCES, TARGETS};
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
    #[id = "affect_chords"]
    pub affect_chords: BoolParam,
    #[id = "always_chord"]
    pub always_chord: BoolParam,
    #[id = "key_split"]
    pub key_split: BoolParam,
    #[id = "bass_enabled"]
    pub bass_enabled: BoolParam,
    #[id = "bass_split"]
    pub bass_split: IntParam,
    #[id = "split_note"]
    pub split_note: IntParam,
    #[id = "strum_enabled"]
    pub strum_enabled: BoolParam,
    #[id = "root_on_select"]
    pub root_on_select: BoolParam,
    #[nested(array, group = "Modulation")]
    pub routes: [RouteParams; ROUTE_COUNT],
    #[nested(array, group = "Modulator Bypass")]
    pub modulators: [ModulatorParams; SOURCE_COUNT],
    #[nested(array, group = "Quick MIDI Learn")]
    pub quick_midi: [crate::midi_learn::Binding; crate::midi_learn::COUNT],
    #[persist = "arpeggiator-v1"]
    pub arpeggiator: Arc<crate::arpeggiator::Store>,
    #[nested(array, group = "Arpeggiator Pages")]
    pub arp_pages: [ArpPageParams; 8],
    #[id = "arp_page"]
    pub arp_page: IntParam,
    #[id = "arp_enabled"]
    pub arp_enabled: BoolParam,
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
    #[persist = "chord-slot-9-v1"]
    pub slot_8: AtomicU64,
    #[persist = "chord-slot-10-v1"]
    pub slot_9: AtomicU64,
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
    // Kept for preset compatibility; the string count now follows chord tones × octaves.
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
    #[id = "strum_hold"]
    pub strum_hold: BoolParam,
    #[id = "tempo_sync"]
    pub tempo_sync: BoolParam,
    #[id = "tempo"]
    pub tempo: FloatParam,
    #[id = "meter_num"]
    pub meter_num: IntParam,
    #[id = "meter_den"]
    pub meter_den: IntParam,
    #[id = "direction"]
    pub direction: IntParam,
    #[id = "contour"]
    pub contour: FloatParam,
    #[id = "contour_curve"]
    pub contour_curve: FloatParam,
    #[id = "octaves"]
    pub octaves: IntParam,
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
    // Retained for preset compatibility; computer-key playing is always enabled.
    #[id = "keyboard"]
    pub keyboard: BoolParam,
    #[id = "fifths"]
    pub fifths: BoolParam,
    #[id = "scale_layout"]
    pub scale_layout: IntParam,
    #[id = "key_spelling"]
    pub key_spelling: IntParam,
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
    crate::harmony::NOTE_NAMES
        .iter()
        .enumerate()
        .find_map(|(pitch, name)| {
            let octave = text.strip_prefix(name)?.parse::<i32>().ok()?;
            let note = octave
                .checked_add(1)?
                .checked_mul(12)?
                .checked_add(pitch as i32)?;
            (0..=127).contains(&note).then_some(note)
        })
}

impl Default for ChordboardParams {
    fn default() -> Self {
        Self {
            always_bass: BoolParam::new("Always play bass", false),
            affect_chords: BoolParam::new("Melody affects chords", false),
            always_chord: BoolParam::new("Always play full chord", false),
            key_split: BoolParam::new("Key split", false),
            bass_enabled: BoolParam::new("Bass enabled", true),
            bass_split: IntParam::new("Bass below", -1, IntRange::Linear { min: -1, max: 127 })
                .with_value_to_string(Arc::new(|v| {
                    if v < 0 {
                        "Control octave".into()
                    } else {
                        format!(
                            "{}{} ({v})",
                            crate::harmony::NOTE_NAMES[v as usize % 12],
                            v / 12 - 1
                        )
                    }
                }))
                .with_string_to_value(Arc::new(|text| {
                    if text.trim().eq_ignore_ascii_case("control octave")
                        || text.trim().eq_ignore_ascii_case("auto")
                    {
                        Some(-1)
                    } else {
                        parse_split_note(text)
                    }
                })),
            split_note: IntParam::new(
                "First right-hand note",
                60,
                IntRange::Linear { min: 0, max: 127 },
            )
            .with_value_to_string(Arc::new(|v| {
                format!(
                    "{}{} ({v})",
                    crate::harmony::NOTE_NAMES[v as usize % 12],
                    v / 12 - 1
                )
            }))
            .with_string_to_value(Arc::new(parse_split_note)),
            strum_enabled: BoolParam::new("Strumfield enabled", true),
            root_on_select: BoolParam::new("Root on select", false),
            routes: std::array::from_fn(|i| {
                RouteParams::from_route(crate::engine::routing::default_routes()[i])
            }),
            modulators: std::array::from_fn(|_| ModulatorParams::default()),
            quick_midi: std::array::from_fn(|_| crate::midi_learn::Binding::default()),
            arpeggiator: crate::arpeggiator::Store::new(),
            arp_pages: std::array::from_fn(|_| ArpPageParams::default()),
            arp_page: IntParam::new("Arp page", 0, IntRange::Linear { min: 0, max: 7 }),
            arp_enabled: BoolParam::new("Arpeggiator", false),
            editor_state: ViziaState::new_screen_sized(
                "Chordboard",
                crate::ui::initial_editor_size,
            ),
            schema_version: AtomicU32::new(2),
            selected_quality: AtomicU32::new(0),
            control_base: AtomicI32::new(12),
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
            slot_8: AtomicU64::new(0),
            slot_9: AtomicU64::new(0),
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
            inversion: IntParam::new("Inversion", 0, IntRange::Linear { min: 0, max: 11 }),
            transpose: IntParam::new("Transpose", 0, IntRange::Linear { min: -24, max: 24 }),
            spread: IntParam::new(
                "Voicing",
                0,
                IntRange::Linear {
                    min: 0,
                    max: (crate::harmony::VOICING_NAMES.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| {
                crate::harmony::VOICING_NAMES[v as usize].to_string()
            })),
            latch: BoolParam::new("Latch", false),
            strum_latch: BoolParam::new("Trackpad latch", false),
            strum_hold: BoolParam::new("Hold", false),
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
                "Rate",
                125.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1500.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),
            strum_sync: BoolParam::new("Rate tempo sync", true),
            strum_beats: FloatParam::new(
                "Sweep beats (legacy)",
                0.25,
                FloatRange::Linear {
                    min: 0.0625,
                    max: 2.0,
                },
            )
            .hide(),
            meter_num: IntParam::new("Meter numerator", 4, IntRange::Linear { min: 1, max: 32 }),
            meter_den: IntParam::new(
                "Meter denominator power",
                2,
                IntRange::Linear { min: 0, max: 5 },
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
            direction: IntParam::new(
                "Strum direction (legacy)",
                0,
                IntRange::Linear { min: 0, max: 4 },
            )
            .with_value_to_string(Arc::new(|v| {
                ["Up", "Down", "Alternate", "Up in 3rds", "Down in 3rds"][v as usize].to_string()
            }))
            .hide(),
            contour: FloatParam::new(
                "Velocity contour",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{v:.2}")))
            .hide(),
            contour_curve: FloatParam::new(
                "Contour curve",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .hide(),
            octaves: IntParam::new(
                "Manual strum octaves",
                1,
                IntRange::Linear { min: 1, max: 4 },
            ),
            mpe: BoolParam::new("MPE output (legacy)", false).hide(),
            output_mode: IntParam::new("Output protocol", 2, IntRange::Linear { min: 0, max: 2 })
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
            keyboard: BoolParam::new("Keyboard play", true)
                .hide()
                .hide_in_generic_ui(),
            fifths: BoolParam::new("Fifths layout", false),
            scale_layout: IntParam::new("Scale layout", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| {
                    ["Off", "Degrees", "Passing"][v as usize].into()
                })),
            key_spelling: IntParam::new("Key spelling", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| {
                    ["Automatic", "Sharps", "Flats"][v as usize].into()
                })),
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
    /// Suspend outgoing modulation without changing the saved route settings.
    pub fn required_arp_loops(&self) -> [u8; 8] {
        let routes = std::array::from_fn::<_, ROUTE_COUNT, _>(|i| self.effective_route(self.routes[i].route()));
        std::array::from_fn(|p| crate::engine::routing::maximum_for_routes("arp_octaves", self.arp_pages[p].arp_octaves.value() as f32, &routes).round().clamp(1.0, 4.0) as u8)
    }
    pub fn effective_route(&self, mut route: Route) -> Route {
        if let Some(source) = route.source.checked_sub(1) {
            if self.modulators.get(source as usize).is_some_and(|p| p.bypass.value()) {
                route.enabled = false;
            }
        }
        route
    }
    pub fn config(&self) -> Config {
        let quick = |i| crate::midi_learn::value(self, i);
        let arp_page = quick(7) as usize;
        Config {
            always_bass: self.always_bass.value(),
            always_chord: self.always_chord.value(),
            affect_chords: self.affect_chords.value(),
            key_split: self.key_split.value(),
            split_note: self.split_note.value() as u8,
            bass_split: self.bass_split.value() as i16,
            bass_enabled: self.bass_enabled.value(),
            strum_enabled: self.strum_enabled.value(),
            root_on_select: self.root_on_select.value(),
            routes: std::array::from_fn(|i| self.effective_route(self.routes[i].route())),
            mode: crate::engine::MANUAL,
            arp_enabled: quick(6) != 0,
            arp_page: arp_page as u8,
            arp_settings: std::array::from_fn(|i| self.arp_pages[i].settings()),
            arp_loop_octaves: [0; 4],
            quality: quick(4) as u8,
            key: self.key.value() as u8,
            scale: self.scale.value() as u8,
            scale_layout: self.scale_layout.value() as u8,
            voice_leading: quick(1) as u8,
            inversion: quick(2) as u8,
            transpose: quick(3) as i8,
            spread: quick(0) as u8,
            latch: quick(5) != 0,
            velocity: self.velocity.value(),
            length_ms: self.length_ms.value(),
            strings: self.strings.value() as u8,
            strings_played: self.strings_played.value() as u8,
            routed_x: None,
            legacy_direct_x: self.schema_version.load(Ordering::Relaxed) < 2,
            strum_ms: self.strum_ms.value(),
            strum_sync: self.strum_sync.value(),
            strum_beats: self.strum_beats.value(),
            strum_hold: self.strum_hold.value(),
            direction: self.direction.value() as u8,
            contour: self.contour.value(),
            contour_curve: self.contour_curve.value(),
            arp_pattern: self.arp_pages[arp_page]
                .arp_pattern
                .value() as u8,
            rate: self.arp_pages[arp_page].rate.value(),
            gate: 0.65,
            swing: self.arp_pages[arp_page].swing.value(),
            arp_mode: self.arp_pages[arp_page]
                .settings()
                .mode,
            arp_octaves: self.arp_pages[arp_page]
                .arp_octaves
                .value() as u8,
            arp_sync: self.arp_pages[arp_page]
                .arp_sync
                .value(),
            arp_free_ms: self.arp_pages[arp_page]
                .arp_free_ms
                .value(),
            host_sync: self.tempo_sync.value(),
            octaves: self.octaves.value() as u8,
            humanize: self.arp_pages[arp_page]
                .humanize
                .value(),
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

// Nested catalogs keep UI IDs, ordering, lookup, and polling values together.
macro_rules! nested_control_catalog {
    ($params:ty, $count:literal, $($builder:ident($id:literal, $field:ident, $group:literal)),* $(,)?) => {
        impl $params {
            fn values(&self) -> [f32; $count] {
                [$(self.$field.unmodulated_normalized_value()),*]
            }

            pub fn controls(&self) -> Vec<Control> {
                vec![$($builder($id, $group, &self.$field)),*]
            }

            pub fn control(&self, id: &str) -> Option<Control> {
                match id {
                    $($id => Some($builder($id, $group, &self.$field)),)*
                    _ => None,
                }
            }
        }
    };
}

// One catalog supports both full UI snapshots and allocation-light single lookups.
macro_rules! control_catalog {
    ($($builder:ident($field:ident, $group:literal)),* $(,)?) => {
        impl ChordboardParams {
            /// Update the polling signature without allocating formatted controls.
            pub fn update_control_signature(&self, signature: &mut Vec<f32>) -> bool {
                let mut index = 0;
                let mut changed = false;
                let mut visit = |value| {
                    if let Some(previous) = signature.get_mut(index) {
                        if *previous != value {
                            *previous = value;
                            changed = true;
                        }
                    } else {
                        signature.push(value);
                        changed = true;
                    }
                    index += 1;
                };
                $(visit(self.$field.unmodulated_normalized_value());)*
                for modulator in &self.modulators { visit(modulator.bypass.unmodulated_normalized_value()); }
                for route in &self.routes {
                    for value in route.values() { visit(value); }
                }
                for value in self.arp_pages[self.arp_page.value() as usize].values() { visit(value); }
                changed |= signature.len() != index;
                signature.truncate(index);
                changed
            }

            pub fn controls(&self) -> Vec<Control> {
                let mut controls = vec![$($builder(stringify!($field), $group, &self.$field)),*];
                for modulator in &self.modulators { controls.push(toggle("modulator_bypass", 6, &modulator.bypass)); }
                for route in &self.routes { controls.extend(route.controls()); }
                controls.extend(self.arp_pages[self.arp_page.value() as usize].controls());
                controls
            }

            pub fn control(&self, id: &str) -> Option<Control> {
                match id {
                    $(stringify!($field) => Some($builder(stringify!($field), $group, &self.$field)),)*
                    _ => self.arp_pages[self.arp_page.value() as usize].control(id),
                }
            }
        }
    };
}

control_catalog! {
    toggle(always_bass, 0),
    toggle(always_chord, 0),
    toggle(key_split, 0),
    toggle(bass_enabled, 0),
    control(split_note, 0),
    control(bass_split, 0),
    toggle(arp_enabled, 0),
    control(arp_page, 0),
    control(quality, 0),
    control(inversion, 0),
    control(voice_leading, 0),
    control(transpose, 0),
    control(spread, 0),
    toggle(latch, 0),
    toggle(strum_enabled, 0),
    toggle(root_on_select, 0),
    toggle(strum_latch, 1),
    control(velocity, 0),
    control(length_ms, 0),
    control(strings, 1),
    control(strings_played, 1),
    control(strum_ms, 1),
    toggle(strum_sync, 1),
    control(strum_beats, 1),
    toggle(strum_hold, 1),
    toggle(tempo_sync, 1),
    control(tempo, 1),
    control(direction, 1),
    control(contour, 1),
    control(contour_curve, 1),
    control(octaves, 1),
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
    toggle(fifths, 4),
    control(scale_layout, 4),
    control(key_spelling, 5),
    control(bank, 4),
    control(keyboard_octave, 4),
    control(key, 5),
    control(scale, 5),
    control(filter, 5),
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
            7 => &self.slot_7,
            8 => &self.slot_8,
            _ => &self.slot_9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_note_readout_roundtrips_and_rejects_out_of_range_notes() {
        for note in 0..=127 {
            let label = format!(
                "{}{} ({note})",
                crate::harmony::NOTE_NAMES[note as usize % 12],
                note / 12 - 1
            );
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
        assert!(params.control("keyboard").is_none());
        for (id, ptr) in [
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
            additions: 0,
            root: 60,
            second: Some(66),
            quality: 0,
            inversion: 1,
            spread: 1,
            transpose: 12,
        };
        params.control_base.store(36, Ordering::Relaxed);
        for i in 7..10 {
            params.slot(i).store(chord.encode(), Ordering::Relaxed);
        }
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
        for i in 7..10 {
            assert_eq!(loaded.slot(i).load(Ordering::Relaxed), chord.encode());
        }
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
pub struct ModulatorParams {
    #[id = "modulator_bypass"]
    pub bypass: BoolParam,
}
impl Default for ModulatorParams {
    fn default() -> Self {
        Self { bypass: BoolParam::new("Bypass modulator", false) }
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
    #[id = "route_smoothing_ms"]
    pub smoothing_ms: FloatParam,
}
impl Default for RouteParams {
    fn default() -> Self {
        Self::from_route(Route::default())
    }
}
impl RouteParams {
    fn from_route(route: Route) -> Self {
        Self {
            smoothing_ms: FloatParam::new(
                "Smoothing",
                route.smoothing_ms,
                FloatRange::Linear {
                    min: 0.0,
                    max: 2000.0,
                },
            )
            .with_unit(" ms")
            .with_value_to_string(Arc::new(|v| format!("{v:.0}")))
            .with_string_to_value(Arc::new(|s| {
                s.trim().trim_end_matches("ms").trim().parse().ok()
            })),
            enabled: BoolParam::new("Enabled", route.enabled),
            source: IntParam::new(
                "Source",
                route.source as i32,
                IntRange::Linear {
                    min: 0,
                    max: (SOURCES.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| SOURCES[v as usize].into())),
            target: IntParam::new(
                "Destination",
                route.target as i32,
                IntRange::Linear {
                    min: 0,
                    max: (TARGETS.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| TARGETS[v as usize].name.into())),
            min: FloatParam::new("Minimum", route.min, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0)))
                .with_string_to_value(Arc::new(|s| {
                    s.trim()
                        .trim_end_matches('%')
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .map(|v| v / 100.0)
                })),
            max: FloatParam::new("Maximum", route.max, FloatRange::Linear { min: 0.0, max: 1.0 })
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
                route.curve,
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

    pub fn route(&self) -> Route {
        Route {
            enabled: self.enabled.value(),
            source: self.source.value() as u8,
            target: self.target.value() as u8,
            min: self.min.value(),
            max: self.max.value(),
            curve: self.curve.value(),
            smoothing_ms: self.smoothing_ms.value(),
        }
    }
}

nested_control_catalog! {
    RouteParams, 7,
    toggle("route_enabled", enabled, 6),
    control("route_source", source, 6),
    control("route_target", target, 6),
    control("route_min", min, 6),
    control("route_max", max, 6),
    control("route_curve", curve, 6),
    control("route_smoothing_ms", smoothing_ms, 6),
}

#[cfg(test)]
mod routing_state_tests {
    use super::*;
    #[test]
    fn modulator_bypass_preserves_routes_and_resumes_every_source() {
        let mut p = ChordboardParams::default();
        for source in 0..SOURCE_COUNT {
            let route = Route { source: source as u8 + 1, target: 4, min: 0.2, max: 0.7,
                curve: -0.3, smoothing_ms: 50.0, ..Route::default() };
            p.routes[0] = RouteParams::from_route(route);
            p.routes[1] = RouteParams::from_route(Route { target: 5, ..route });
            p.routes[2] = RouteParams::from_route(Route { enabled: false, ..route });
            p.routes[3] = RouteParams::from_route(Route { source: (source as u8 + 1) % SOURCE_COUNT as u8 + 1, ..route });
            assert_eq!(p.config().routes[0], route);
            p.modulators[source].bypass = BoolParam::new("Bypass", true);
            let config = p.config();
            assert!(!config.routes[0].active());
            assert!(!config.routes[1].active());
            assert!(config.routes[3].active());
            assert_eq!(p.routes[0].route(), route);
            assert_eq!(config.routes[0], Route { enabled: false, ..route });
            p.modulators[source].bypass = BoolParam::new("Bypass", false);
            assert_eq!(p.config().routes[0], route);
            assert!(p.config().routes[1].active());
            assert!(!p.config().routes[2].active());
        }
        let ids: Vec<_> = p.param_map().into_iter().map(|v| v.0).collect();
        for index in 1..=SOURCE_COUNT {
            assert_eq!(ids.iter().filter(|id| **id == format!("modulator_bypass_{index}")).count(), 1);
        }
    }

    #[test]
    fn modulator_bypass_restores_bases_and_breaks_outgoing_chains() {
        let mut p = ChordboardParams::default();
        let target = |id| TARGETS.iter().position(|t| t.id == id).unwrap() as u8;
        p.routes[0] = RouteParams::from_route(Route { source: 2, target: target("source_9"), ..Route::default() });
        p.routes[1] = RouteParams::from_route(Route { source: 2, target: target("velocity"), smoothing_ms: 100.0, ..Route::default() });
        p.routes[2] = RouteParams::from_route(Route { source: 10, target: target("transpose"), ..Route::default() });
        let mut engine = crate::engine::Engine::default();
        engine.sources[1] = 1.0;
        engine.configure(p.config(), &mut |_| {});
        assert_eq!(engine.config.transpose, 24);
        assert_eq!(engine.config.velocity, 1.0);
        p.modulators[1].bypass = BoolParam::new("Bypass", true);
        engine.configure(p.config(), &mut |_| {});
        assert_eq!(engine.config.transpose, p.config().transpose);
        assert_eq!(engine.config.velocity, p.config().velocity);
        assert_eq!(engine.effective_sources()[1], 1.0);
        engine.sources[1] = 0.5;
        p.modulators[1].bypass = BoolParam::new("Bypass", false);
        engine.configure(p.config(), &mut |_| {});
        assert_eq!(engine.config.velocity, TARGETS[target("velocity") as usize].plain(0.5));
    }

    #[test]
    fn route_constructor_preserves_every_nondefault_field() {
        let route = Route {
            enabled: false,
            source: 5,
            target: 3,
            min: 0.23,
            max: 0.81,
            curve: -0.4,
            smoothing_ms: 327.0,
        };
        assert_eq!(RouteParams::from_route(route).route(), route);
        assert_eq!(RouteParams::default().route(), Route::default());
    }

    #[test]
    fn arp_pages_are_independent_of_manual_play() {
        let p = ChordboardParams::default();
        assert_eq!(p.config().mode, crate::engine::MANUAL);
        assert!(!p.config().arp_enabled);
        assert!(p
            .config()
            .arp_settings
            .iter()
            .all(|p| p.mode == crate::arpeggiator::ArpMode::SyncLoop));
    }
    #[test]
    fn all_route_fields_have_stable_unique_host_ids_and_default_strumfield_link() {
        let p = ChordboardParams::default();
        let ids: Vec<_> = p.param_map().into_iter().map(|v| v.0).collect();
        for field in [
            "route_source",
            "route_target",
            "route_min",
            "route_max",
            "route_curve",
            "route_enabled",
            "route_smoothing_ms",
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
        assert_eq!(p.config().routes, crate::engine::routing::default_routes());
        assert!(p.routes[0].route().active());
        assert!(p.config().routes[1..].iter().all(|r| !r.active()));
        let range = &p.routes[0];
        assert_eq!(range.min.normalized_value_to_string(0.5, true), "50%");
    }
}

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
        Self {
            arp_mode: IntParam::new("Arp repeat", 2, IntRange::Linear { min: 0, max: 3 })
                .with_value_to_string(Arc::new(|v| {
                    crate::arpeggiator::MODE_NAMES[v as usize].into()
                })),
            arp_pattern: IntParam::new("Ordering", 0, IntRange::Linear { min: 0, max: 6 })
                .with_value_to_string(Arc::new(|v| {
                    [
                        "Up",
                        "Down",
                        "Up / Down",
                        "Up in 3rds",
                        "Down in 3rds",
                        "Played order",
                        "Random",
                    ][v as usize]
                        .into()
                })),
            arp_octaves: IntParam::new("Loop Length", 1, IntRange::Linear { min: 1, max: 4 }),
            arp_sync: BoolParam::new("Arp tempo sync", true),
            arp_free_ms: FloatParam::new(
                "Arp rate ms",
                125.0,
                FloatRange::Linear {
                    min: 1.0,
                    max: 1500.0,
                },
            ),
            rate: FloatParam::new(
                "Arp rate beats",
                0.25,
                FloatRange::Linear {
                    min: 0.0625,
                    max: 2.0,
                },
            ),
            swing: FloatParam::new(
                "Arp swing",
                0.0,
                FloatRange::Linear {
                    min: -0.75,
                    max: 0.75,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{:+.0}%", v / 0.75 * 100.0)))
            .with_string_to_value(Arc::new(|s| s.trim().trim_end_matches('%').trim().parse::<f32>().ok().map(|v| v * 0.75 / 100.0))),
            humanize: FloatParam::new(
                "Arp humanize",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
        }
    }
}
impl ArpPageParams {
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

#[cfg(test)]
mod signature_tests {
    use super::*;

    fn assert_control_matches(actual: Control, expected: &Control) {
        assert_eq!(
            (
                actual.id,
                actual.group,
                &actual.name,
                actual.ptr,
                actual.norm,
                actual.prev,
                actual.next,
                &actual.value,
                actual.toggle,
            ),
            (
                expected.id,
                expected.group,
                &expected.name,
                expected.ptr,
                expected.norm,
                expected.prev,
                expected.next,
                &expected.value,
                expected.toggle,
            ),
            "{}",
            expected.id,
        );
    }

    #[test]
    fn nested_control_lookups_match_catalog_values_and_parameter_pointers() {
        let mut params = ChordboardParams::default();
        params.arp_page = IntParam::new("Arp page", 5, IntRange::Linear { min: 0, max: 7 });
        let page = &params.arp_pages[5];
        let route = &params.routes[0];
        let route_controls = route.controls();
        assert_eq!(
            route_controls.iter().map(|c| c.id).collect::<Vec<_>>(),
            [
                "route_enabled",
                "route_source",
                "route_target",
                "route_min",
                "route_max",
                "route_curve",
                "route_smoothing_ms",
            ],
        );
        for expected in &route_controls {
            assert_control_matches(route.control(expected.id).unwrap(), expected);
        }
        assert_eq!(
            route.values().as_slice(),
            route_controls.iter().map(|c| c.norm).collect::<Vec<_>>(),
        );
        assert!(route.control("missing").is_none());

        let page_controls = page.controls();
        assert_eq!(
            page_controls.iter().map(|c| c.id).collect::<Vec<_>>(),
            [
                "arp_mode",
                "arp_pattern",
                "arp_octaves",
                "arp_sync",
                "arp_free_ms",
                "rate",
                "swing",
                "humanize",
            ],
        );
        for expected in &page_controls {
            assert_control_matches(page.control(expected.id).unwrap(), expected);
            assert_control_matches(params.control(expected.id).unwrap(), expected);
        }
        assert_eq!(
            page.values().as_slice(),
            page_controls.iter().map(|c| c.norm).collect::<Vec<_>>(),
        );
        assert!(page.control("missing").is_none());
        assert!(params.control("missing").is_none());
    }

    #[test]
    fn polling_matches_catalog_and_reuses_storage_when_values_change() {
        let mut params = ChordboardParams::default();
        let mut signature = Vec::new();
        assert!(params.update_control_signature(&mut signature));
        let storage = signature.as_ptr();
        assert!(!params.update_control_signature(&mut signature));
        params.latch = BoolParam::new("Latch", !params.latch.value());
        params.routes[0].min =
            FloatParam::new("Minimum", 0.37, FloatRange::Linear { min: 0.0, max: 1.0 });
        assert!(params.update_control_signature(&mut signature));
        assert_eq!(
            signature,
            params
                .controls()
                .iter()
                .map(|control| control.norm)
                .collect::<Vec<_>>()
        );
        assert_eq!(storage, signature.as_ptr());
        assert!(!params.update_control_signature(&mut signature));
    }
}
