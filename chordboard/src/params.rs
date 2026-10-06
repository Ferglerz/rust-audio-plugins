use crate::engine::routing::{Route, ROUTE_COUNT, SOURCES, TARGETS};
use crate::engine::{Config, Lane, Mapping};
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
    #[id = "root_on_select"]
    pub root_on_select: BoolParam,
    #[nested(array, group = "Modulation")]
    pub routes: Box<[RouteParams; ROUTE_COUNT]>,
    #[nested(group = "Bass Lane", id_prefix = "lane_bass")]
    pub lane_bass: Box<LaneSoundParams>,
    #[nested(group = "Comp Lane", id_prefix = "lane_comp")]
    pub lane_comp: Box<LaneSoundParams>,
    #[nested(group = "Arp Lane", id_prefix = "lane_arp")]
    pub lane_arp: Box<LaneSoundParams>,
    #[nested(group = "Lead Lane", id_prefix = "lane_lead")]
    pub lane_lead: Box<LaneSoundParams>,
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
    #[persist = "arp-mute-mask"]
    pub arp_mute: AtomicU32,
    #[persist = "arp-skip-mask"]
    pub arp_skip: AtomicU32,
    #[persist = "arp-accents-v1"]
    pub arp_accents: AtomicU64,
    #[persist = "arp-vol-0"]
    pub arp_vol_0: AtomicU64,
    #[persist = "arp-vol-1"]
    pub arp_vol_1: AtomicU64,
    #[persist = "arp-vol-2"]
    pub arp_vol_2: AtomicU64,
    #[persist = "arp-vol-3"]
    pub arp_vol_3: AtomicU64,
    #[id = "mode"]
    pub mode: IntParam,
    #[id = "harmonization_mode"]
    pub harmonization_mode: IntParam,
    #[id = "chromatic_flavor"]
    pub chromatic_flavor: IntParam,
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
    #[id = "strum_hold"]
    pub strum_hold: BoolParam,
    #[id = "tempo_sync"]
    pub tempo_sync: BoolParam,
    #[id = "tempo"]
    pub tempo: FloatParam,
    #[id = "direction"]
    pub direction: IntParam,
    #[id = "contour"]
    pub contour: FloatParam,
    #[id = "contour_curve"]
    pub contour_curve: FloatParam,
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
    #[id = "loop_start"]
    pub loop_start: IntParam,
    #[id = "loop_end"]
    pub loop_end: IntParam,
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
    #[id = "comp_mode"]
    pub comp_mode: IntParam,
    #[id = "comp_channel"]
    pub comp_channel: IntParam,
    #[id = "comp_octave"]
    pub comp_octave: IntParam,
    #[id = "comp_velocity"]
    pub comp_velocity: FloatParam,
    #[id = "comp_steps"]
    pub comp_steps: IntParam,
    #[id = "comp_hits"]
    pub comp_hits: IntParam,
    #[id = "comp_lag_ms"]
    pub comp_lag_ms: FloatParam,
    #[id = "comp_interlock"]
    pub comp_interlock: IntParam,
    #[id = "bass_mode"]
    pub bass_mode: IntParam,
    #[id = "bass_octave"]
    pub bass_octave: IntParam,
    #[id = "bass_velocity"]
    pub bass_velocity: FloatParam,
    #[id = "routing_preset"]
    pub routing_preset: IntParam,
    #[id = "extensions"]
    pub extensions: FloatParam,
    #[id = "legato_retention"]
    pub legato_retention: BoolParam,
    #[id = "comp_guide_tone"]
    pub comp_guide_tone: BoolParam,
    #[id = "pad_swell_source"]
    pub pad_swell_source: IntParam,
    #[id = "lane_auto_mute_bass"]
    pub lane_auto_mute_bass: BoolParam,
    #[id = "lane_auto_mute_comp"]
    pub lane_auto_mute_comp: BoolParam,
    #[id = "lane_auto_mute_arp"]
    pub lane_auto_mute_arp: BoolParam,
    #[id = "lane_auto_mute_lead"]
    pub lane_auto_mute_lead: BoolParam,
    #[id = "console_view"]
    pub console_view: IntParam,
    #[id = "bottom_deck_mode"]
    pub bottom_deck_mode: IntParam,
    #[id = "degree_shift"]
    pub degree_shift: BoolParam,
    #[id = "bass_pad_trigger"]
    pub bass_pad_trigger: IntParam,
    #[id = "fx_drive"]
    pub fx_drive: FloatParam,
    #[id = "fx_space"]
    pub fx_space: FloatParam,
    #[id = "fx_tape"]
    pub fx_tape: BoolParam,
    #[id = "fx_delay"]
    pub fx_delay: BoolParam,
    #[id = "fx_reverb"]
    pub fx_reverb: BoolParam,
    #[id = "looper_slot"]
    pub looper_slot: IntParam,
    #[id = "looper_rec"]
    pub looper_rec: BoolParam,
    #[id = "pad_crossfade"]
    pub pad_crossfade: FloatParam,
    #[id = "pitch_bend"]
    pub pitch_bend: FloatParam,
    #[id = "mod_wheel"]
    pub mod_wheel: FloatParam,
    #[id = "open_register"]
    pub open_register: BoolParam,
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
            root_on_select: BoolParam::new("Root on select", false),
            routes: Box::new(std::array::from_fn(|i| {
                RouteParams::from_route(crate::engine::routing::default_routes()[i])
            })),
            lane_bass: Box::new(LaneSoundParams::new_bass()),
            lane_comp: Box::new(LaneSoundParams::new_comp()),
            lane_arp: Box::new(LaneSoundParams::new_arp()),
            lane_lead: Box::new(LaneSoundParams::new_lead()),
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
            arp_mute: AtomicU32::new(0),
            arp_skip: AtomicU32::new(0),
            arp_accents: AtomicU64::new(0),
            arp_vol_0: AtomicU64::new(0x6464646464646464),
            arp_vol_1: AtomicU64::new(0x6464646464646464),
            arp_vol_2: AtomicU64::new(0x6464646464646464),
            arp_vol_3: AtomicU64::new(0x6464646464646464),
            mode: IntParam::new("Play mode", 1, IntRange::Linear { min: 1, max: 3 })
                .with_value_to_string(Arc::new(|v| {
                    ["Arpeggiator Once", "Manual Strum", "Arpeggiator Loop"][(v - 1) as usize]
                        .to_string()
                })),
            harmonization_mode: IntParam::new(
                "Harmonization engine mode",
                0,
                IntRange::Linear { min: 0, max: 2 },
            )
            .with_value_to_string(Arc::new(|v| {
                match v {
                    1 => "Nopia static",
                    2 => "Nopia real",
                    _ => "Classic dual-touch",
                }
                .to_string()
            }))
            .with_string_to_value(Arc::new(|s| {
                let lower = s.to_ascii_lowercase();
                if lower.contains("static") || lower == "1" {
                    Some(1)
                } else if lower.contains("real") || lower == "2" {
                    Some(2)
                } else if lower.contains("classic") || lower.contains("dual") || lower == "0" {
                    Some(0)
                } else {
                    None
                }
            })),
            chromatic_flavor: IntParam::new(
                "Chromatic flavor",
                0,
                IntRange::Linear { min: 0, max: 1 },
            )
            .with_value_to_string(Arc::new(|v| {
                match v {
                    1 => "Modal interchange",
                    _ => "Secondary dominants",
                }
                .to_string()
            }))
            .with_string_to_value(Arc::new(|s| {
                let lower = s.to_ascii_lowercase();
                if lower.contains("modal") || lower.contains("interchange") || lower == "1" {
                    Some(1)
                } else if lower.contains("secondary") || lower.contains("dominant") || lower == "0" {
                    Some(0)
                } else {
                    None
                }
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
            arp_pattern: IntParam::new("Arp pattern", 0, IntRange::Linear { min: 0, max: 6 })
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
                        .to_string()
                })),
            rate: FloatParam::new(
                "Rate beats",
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
            loop_start: IntParam::new("Loop begin", 1, IntRange::Linear { min: 1, max: 32 })
                .with_value_to_string(Arc::new(|v| v.to_string()))
                .with_string_to_value(Arc::new(|s| {
                    s.trim()
                        .trim_end_matches("notes")
                        .trim_end_matches("steps")
                        .trim()
                        .parse::<i32>()
                        .ok()
                })),
            loop_end: IntParam::new("Loop end", 0, IntRange::Linear { min: 0, max: 32 })
                .with_value_to_string(Arc::new(|v| {
                    if v == 0 {
                        "Auto".to_string()
                    } else {
                        v.to_string()
                    }
                }))
                .with_string_to_value(Arc::new(|s| {
                    let s = s.trim();
                    if s.eq_ignore_ascii_case("auto") || s.eq_ignore_ascii_case("off") {
                        Some(0)
                    } else {
                        s.trim_end_matches("notes")
                            .trim_end_matches("steps")
                            .trim()
                            .parse::<i32>()
                            .ok()
                    }
                })),
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
            comp_mode: IntParam::new("Comp mode", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| ["Off", "Pad", "Rhythm"][v as usize].to_string())),
            comp_channel: IntParam::new("Comp channel", 1, IntRange::Linear { min: 1, max: 16 }),
            comp_octave: IntParam::new("Comp octave", 0, IntRange::Linear { min: -2, max: 2 }),
            comp_velocity: FloatParam::new("Comp velocity", 0.75, FloatRange::Linear { min: 0.0, max: 1.0 }),
            comp_steps: IntParam::new("Comp steps", 16, IntRange::Linear { min: 1, max: 32 }),
            comp_hits: IntParam::new("Comp hits", 4, IntRange::Linear { min: 0, max: 32 }),
            comp_lag_ms: FloatParam::new("Comp pocket lag", 0.0, FloatRange::Linear { min: -20.0, max: 50.0 }),
            comp_interlock: IntParam::new("Comp interlock", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| ["Off", "Duck Arp", "Lock Accent"][v as usize].to_string())),
            bass_mode: IntParam::new("Bass mode", 0, IntRange::Linear { min: 0, max: 4 })
                .with_value_to_string(Arc::new(|v| ["Off", "Hold", "Pulse", "Root-5th", "With Comp"][v as usize].to_string())),
            bass_octave: IntParam::new("Bass octave", -1, IntRange::Linear { min: -2, max: 0 }),
            bass_velocity: FloatParam::new("Bass velocity", 0.85, FloatRange::Linear { min: 0.0, max: 1.0 }),
            routing_preset: IntParam::new("Routing preset", 0, IntRange::Linear { min: 0, max: 1 })
                .with_value_to_string(Arc::new(|v| ["Single Track (Wurli)", "Split Tracks"][v as usize].to_string())),
            extensions: FloatParam::new(
                "Extensions macro",
                0.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| {
                if v < 0.20 {
                    "Root".to_string()
                } else if v < 0.40 {
                    "Power 1+5".to_string()
                } else if v < 0.50 {
                    "Triad".to_string()
                } else if v < 0.85 {
                    "7th".to_string()
                } else {
                    "Upper color".to_string()
                }
            })),
            legato_retention: BoolParam::new("Legato voice retention", true),
            comp_guide_tone: BoolParam::new("Comp guide tone only", false),
            pad_swell_source: IntParam::new("Pad swell source", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| {
                    ["Off", "CC 11 Expression", "CC 1 Mod Wheel"][v as usize].to_string()
                })),
            lane_auto_mute_bass: BoolParam::new("Auto-mute bass when external", false),
            lane_auto_mute_comp: BoolParam::new("Auto-mute comp when external", false),
            lane_auto_mute_arp: BoolParam::new("Auto-mute arp when external", false),
            lane_auto_mute_lead: BoolParam::new("Auto-mute lead when external", false),
            console_view: IntParam::new("Interface view", 0, IntRange::Linear { min: 0, max: 1 })
                .with_value_to_string(Arc::new(|v| ["Studio", "Live Console"][v as usize].to_string())),
            bottom_deck_mode: IntParam::new("Bottom deck mode", 0, IntRange::Linear { min: 0, max: 1 })
                .with_value_to_string(Arc::new(|v| ["4-Octave Keys", "Physical Strings Harp"][v as usize].to_string())),
            degree_shift: BoolParam::new("Degree shift", false),
            bass_pad_trigger: IntParam::new("Bass pad mode", 0, IntRange::Linear { min: 0, max: 1 })
                .with_value_to_string(Arc::new(|v| ["Root", "Alt Slash"][v as usize].to_string())),
            fx_drive: FloatParam::new("FX Drive", 0.35, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            fx_space: FloatParam::new("FX Space", 0.48, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            fx_tape: BoolParam::new("Tape FX", false),
            fx_delay: BoolParam::new("Delay FX", true),
            fx_reverb: BoolParam::new("Reverb FX", true),
            looper_slot: IntParam::new("Looper slot", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| ["Verse", "Chorus", "Bridge"][v as usize].to_string())),
            looper_rec: BoolParam::new("Looper record", false),
            pad_crossfade: FloatParam::new("Pad crossfade", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            pitch_bend: FloatParam::new("Pitch bend", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
            mod_wheel: FloatParam::new("Modulation wheel", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            open_register: BoolParam::new("Open register", false),
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
            affect_chords: self.affect_chords.value(),
            key_split: self.key_split.value(),
            split_note: self.split_note.value() as u8,
            bass_split: self.bass_split.value() as i16,
            bass_enabled: self.bass_enabled.value(),
            root_on_select: self.root_on_select.value(),
            routes: std::array::from_fn(|i| self.routes[i].route()),
            mode: self.mode.value() as u8,
            harmonization_mode: self.harmonization_mode.value() as u8,
            chromatic_flavor: self.chromatic_flavor.value() as u8,
            key: self.key.value() as u8,
            scale: self.scale.value() as u8,
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
            legacy_direct_x: self.schema_version.load(Ordering::Relaxed) < 2,
            strum_ms: self.strum_ms.value(),
            strum_sync: self.strum_sync.value(),
            strum_beats: self.strum_beats.value(),
            strum_hold: self.strum_hold.value(),
            direction: self.direction.value() as u8,
            contour: self.contour.value(),
            contour_curve: self.contour_curve.value(),
            arp_pattern: self.arp_pattern.value() as u8,
            rate: self.rate.value(),
            gate: self.gate.value(),
            swing: self.swing.value(),
            octaves: self.octaves.value() as u8,
            loop_start: self.loop_start.value() as u8,
            loop_end: self.loop_end.value() as u8,
            arp_volume: std::array::from_fn(|i| {
                let word = match i / 8 {
                    0 => self.arp_vol_0.load(Ordering::Relaxed),
                    1 => self.arp_vol_1.load(Ordering::Relaxed),
                    2 => self.arp_vol_2.load(Ordering::Relaxed),
                    _ => self.arp_vol_3.load(Ordering::Relaxed),
                };
                ((word >> ((i % 8) * 8)) & 0xFF) as u8
            }),
            arp_mute: self.arp_mute.load(Ordering::Relaxed),
            arp_skip: self.arp_skip.load(Ordering::Relaxed),
            arp_accents: std::array::from_fn(|i| {
                ((self.arp_accents.load(Ordering::Relaxed) >> (i * 2)) & 3) as u8
            }),
            comp_mode: self.comp_mode.value() as u8,
            comp_channel: (self.comp_channel.value() - 1) as u8,
            comp_octave: self.comp_octave.value() as i8,
            comp_velocity: self.comp_velocity.value(),
            comp_euclidean_steps: self.comp_steps.value() as u8,
            comp_euclidean_hits: self.comp_hits.value() as u8,
            comp_lag_ms: self.comp_lag_ms.value(),
            comp_interlock: self.comp_interlock.value() as u8,
            bass_mode: self.bass_mode.value() as u8,
            bass_octave: self.bass_octave.value() as i8,
            bass_velocity: self.bass_velocity.value(),
            routing_preset: self.routing_preset.value() as u8,
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
            bass_channel: (self.bass_channel.value() - 1) as u8,
            upper_channel: (self.upper_channel.value() - 1) as u8,
            control_base: self.control_base.load(Ordering::Relaxed).clamp(-1, 116) as i16,
            mappings: [
                Mapping::decode(self.map_x.load(Ordering::Relaxed)),
                Mapping::decode(self.map_y.load(Ordering::Relaxed)),
            ],
            extensions: self.extensions.value(),
            legato_retention: self.legato_retention.value(),
            comp_guide_tone: self.comp_guide_tone.value(),
            pad_swell_source: self.pad_swell_source.value() as u8,
            lane_auto_mute: [
                self.lane_auto_mute_bass.value(),
                self.lane_auto_mute_comp.value(),
                self.lane_auto_mute_arp.value(),
                self.lane_auto_mute_lead.value(),
            ],
        }
    }
    pub fn mapping(&self, axis: usize) -> &AtomicU32 {
        match axis {
            0 => &self.map_x,
            _ => &self.map_y,
        }
    }

    pub fn lane_params(&self, lane: Lane) -> &LaneSoundParams {
        match lane {
            Lane::Bass => &self.lane_bass,
            Lane::Comp => &self.lane_comp,
            Lane::Arp => &self.lane_arp,
            Lane::Lead => &self.lane_lead,
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
                for &lane in &Lane::ALL {
                    controls.extend(self.lane_params(lane).controls(lane));
                }
                controls
            }

            pub fn control(&self, id: &str) -> Option<Control> {
                if id.starts_with("lane_") {
                    for &lane in &Lane::ALL {
                        if let Some(c) = self.lane_params(lane).controls(lane).into_iter().find(|c| c.id == id) {
                            return Some(c);
                        }
                    }
                }
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
    toggle(bass_enabled, 0),
    control(split_note, 0),
    control(bass_split, 0),
    control(mode, 0),
    control(harmonization_mode, 0),
    control(chromatic_flavor, 0),
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
    toggle(strum_hold, 1),
    toggle(tempo_sync, 1),
    control(tempo, 1),
    control(direction, 1),
    control(contour, 1),
    control(contour_curve, 1),
    control(arp_pattern, 1),
    control(rate, 1),
    control(gate, 1),
    control(swing, 1),
    control(octaves, 1),
    control(loop_start, 1),
    control(loop_end, 1),
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
    control(comp_mode, 5),
    control(comp_channel, 5),
    control(comp_octave, 5),
    control(comp_velocity, 5),
    control(comp_steps, 5),
    control(comp_hits, 5),
    control(comp_lag_ms, 5),
    control(comp_interlock, 5),
    control(bass_mode, 5),
    control(bass_octave, 5),
    control(bass_velocity, 5),
    control(routing_preset, 5),
    control(console_view, 6),
    control(bottom_deck_mode, 6),
    toggle(degree_shift, 6),
    control(bass_pad_trigger, 6),
    control(extensions, 6),
    control(fx_drive, 6),
    control(fx_space, 6),
    toggle(fx_tape, 6),
    toggle(fx_delay, 6),
    toggle(fx_reverb, 6),
    control(looper_slot, 6),
    toggle(looper_rec, 6),
    control(pad_crossfade, 6),
    control(pitch_bend, 6),
    control(mod_wheel, 6),
    toggle(open_register, 6),
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

    pub fn string_volume(&self, index: usize) -> f32 {
        if index >= 32 {
            return 1.0;
        }
        let word = match index / 8 {
            0 => self.arp_vol_0.load(Ordering::Relaxed),
            1 => self.arp_vol_1.load(Ordering::Relaxed),
            2 => self.arp_vol_2.load(Ordering::Relaxed),
            _ => self.arp_vol_3.load(Ordering::Relaxed),
        };
        let byte = ((word >> ((index % 8) * 8)) & 0xFF) as u8;
        (byte as f32) / 100.0
    }

    pub fn set_string_volume(&self, index: usize, vol: f32) {
        if index >= 32 {
            return;
        }
        let byte = (vol.clamp(0.0, 1.0) * 100.0).round() as u64;
        let shift = (index % 8) * 8;
        let mask = !(0xFF_u64 << shift);
        let atomic = match index / 8 {
            0 => &self.arp_vol_0,
            1 => &self.arp_vol_1,
            2 => &self.arp_vol_2,
            _ => &self.arp_vol_3,
        };
        let mut old = atomic.load(Ordering::Relaxed);
        loop {
            let new = (old & mask) | (byte << shift);
            match atomic.compare_exchange_weak(old, new, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => old = actual,
            }
        }
    }

    pub fn set_string_volume_ramp(
        &self,
        start_idx: usize,
        start_vol: f32,
        end_idx: usize,
        end_vol: f32,
    ) {
        let (min_idx, max_idx) = (start_idx.min(end_idx), start_idx.max(end_idx));
        let delta = end_idx as f32 - start_idx as f32;
        for i in min_idx..=max_idx {
            let t = if delta.abs() < 0.001 {
                0.0
            } else {
                (i as f32 - start_idx as f32) / delta
            };
            let vol = (start_vol + t * (end_vol - start_vol)).clamp(0.0, 1.0);
            self.set_string_volume(i, vol);
        }
    }

    pub fn reset_all_string_volumes(&self) {
        self.arp_vol_0.store(0x6464646464646464, Ordering::Relaxed);
        self.arp_vol_1.store(0x6464646464646464, Ordering::Relaxed);
        self.arp_vol_2.store(0x6464646464646464, Ordering::Relaxed);
        self.arp_vol_3.store(0x6464646464646464, Ordering::Relaxed);
    }

    pub fn clear_all_string_mutes(&self) {
        self.arp_mute.store(0, Ordering::Relaxed);
    }

    pub fn clear_all_string_skips(&self) {
        self.arp_skip.store(0, Ordering::Relaxed);
    }

    pub fn reset_all_arp_loop(&self) {
        self.reset_all_string_volumes();
        self.clear_all_string_mutes();
        self.clear_all_string_skips();
    }

    pub fn is_string_muted(&self, index: usize) -> bool {
        if index >= 32 {
            return false;
        }
        (self.arp_mute.load(Ordering::Relaxed) & (1 << index)) != 0
    }

    pub fn toggle_string_muted(&self, index: usize) {
        if index >= 32 {
            return;
        }
        let bit = 1 << index;
        let old = self.arp_mute.fetch_xor(bit, Ordering::Relaxed);
        if (old & bit) == 0 {
            self.arp_skip.fetch_and(!bit, Ordering::Relaxed);
        }
    }

    pub fn is_string_skipped(&self, index: usize) -> bool {
        if index >= 32 {
            return false;
        }
        (self.arp_skip.load(Ordering::Relaxed) & (1 << index)) != 0
    }

    pub fn toggle_string_skipped(&self, index: usize) {
        if index >= 32 {
            return;
        }
        let bit = 1 << index;
        let old = self.arp_skip.fetch_xor(bit, Ordering::Relaxed);
        if (old & bit) == 0 {
            self.arp_mute.fetch_and(!bit, Ordering::Relaxed);
        }
    }

    pub fn step_accent(&self, step: usize) -> u8 {
        if step >= 32 {
            return 0;
        }
        ((self.arp_accents.load(Ordering::Relaxed) >> (step * 2)) & 3) as u8
    }

    pub fn set_step_accent(&self, step: usize, state: u8) {
        if step >= 32 {
            return;
        }
        let shift = step * 2;
        let mask = !(3u64 << shift);
        let val = (state as u64 & 3) << shift;
        let mut current = self.arp_accents.load(Ordering::Relaxed);
        loop {
            let new_val = (current & mask) | val;
            match self.arp_accents.compare_exchange_weak(
                current,
                new_val,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn cycle_step_accent(&self, step: usize) -> u8 {
        if step >= 32 {
            return 0;
        }
        let next = match self.step_accent(step) {
            0 => 1,
            1 => 2,
            _ => 0,
        };
        self.set_step_accent(step, next);
        next
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

pub const KARPLUS_PRESETS: [(&str, [f32; 6]); 6] = [
    ("Deep Bass", [0.80, 0.65, 0.35, 0.50, 0.70, 0.00]),
    ("Punchy Slap", [0.60, 0.35, 0.15, 0.85, 0.55, 0.00]),
    ("Acoustic Pluck", [0.60, 0.30, 0.20, 0.75, 0.40, 0.45]),
    ("Harp / Koto", [0.75, 0.20, 0.50, 0.60, 0.30, 0.60]),
    ("Singing Lead", [0.85, 0.22, 0.40, 0.65, 0.45, 0.25]),
    ("Muted Chime", [0.35, 0.50, 0.10, 0.90, 0.20, 0.30]),
];

pub const WURLI_PRESETS: [(&str, [f32; 6], bool, bool); 6] = [
    ("Classic 200A", [0.75, 0.35, 0.40, 0.25, 0.50, 0.50], false, true),
    ("Warm Velvet", [0.80, 0.00, 0.20, 0.50, 0.60, 0.30], false, false),
    ("R&B Tremolo", [0.85, 0.70, 0.65, 0.15, 0.45, 0.80], true, true),
    ("Bite & Drive", [0.90, 0.15, 0.30, 0.35, 0.70, 0.90], true, true),
    ("Soft Lounge", [0.70, 0.20, 0.30, 0.40, 0.40, 0.25], false, false),
    ("Heavy Stage", [0.85, 0.50, 0.50, 0.30, 0.55, 0.70], true, true),
];

#[derive(Params)]
pub struct LaneSoundParams {
    #[id = "engine"]
    pub engine: IntParam,
    #[id = "preset"]
    pub preset: IntParam,
    #[id = "level"]
    pub level: FloatParam,
    #[id = "pan"]
    pub pan: FloatParam,
    #[id = "mute"]
    pub mute: BoolParam,
    #[id = "m1"]
    pub m1: FloatParam,
    #[id = "m2"]
    pub m2: FloatParam,
    #[id = "m3"]
    pub m3: FloatParam,
    #[id = "m4"]
    pub m4: FloatParam,
    #[id = "m5"]
    pub m5: FloatParam,
    #[id = "m6"]
    pub m6: FloatParam,
    #[id = "opt_a"]
    pub opt_a: BoolParam,
    #[id = "opt_b"]
    pub opt_b: BoolParam,
}

impl Default for LaneSoundParams {
    fn default() -> Self {
        Self::new_bass()
    }
}

impl LaneSoundParams {
    pub fn new_bass() -> Self {
        Self::new(
            "Bass",
            1,
            0,
            0.85,
            0.0,
            false,
            0.80, 0.65, 0.35, 0.50, 0.70, 0.00,
            false, false,
        )
    }

    pub fn new_comp() -> Self {
        Self::new(
            "Comp",
            2,
            0,
            0.75,
            0.0,
            false,
            0.75, 0.35, 0.40, 0.25, 0.50, 0.50,
            false, true,
        )
    }

    pub fn new_arp() -> Self {
        Self::new(
            "Arp",
            1,
            2,
            0.80,
            0.15,
            false,
            0.60, 0.30, 0.20, 0.75, 0.40, 0.45,
            false, false,
        )
    }

    pub fn new_lead() -> Self {
        Self::new(
            "Lead",
            1,
            4,
            0.85,
            -0.10,
            false,
            0.85, 0.22, 0.40, 0.65, 0.45, 0.25,
            false, false,
        )
    }

    pub fn new(
        name: &'static str,
        engine: i32,
        preset: i32,
        level: f32,
        pan: f32,
        mute: bool,
        m1: f32,
        m2: f32,
        m3: f32,
        m4: f32,
        m5: f32,
        m6: f32,
        opt_a: bool,
        opt_b: bool,
    ) -> Self {
        Self {
            engine: IntParam::new(
                format!("{name} Engine"),
                engine,
                IntRange::Linear { min: 0, max: 2 },
            )
            .with_value_to_string(Arc::new(|v| match v {
                1 => "Karplus String".into(),
                2 => "OpenWurli".into(),
                _ => "Off".into(),
            })),
            preset: IntParam::new(
                format!("{name} Preset"),
                preset,
                IntRange::Linear { min: 0, max: 5 },
            ),
            level: FloatParam::new(
                format!("{name} Level"),
                level,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            pan: FloatParam::new(
                format!("{name} Pan"),
                pan,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| {
                if v.abs() < 0.01 {
                    "C".into()
                } else if v < 0.0 {
                    format!("{:.0}L", v.abs() * 100.0)
                } else {
                    format!("{:.0}R", v * 100.0)
                }
            })),
            mute: BoolParam::new(format!("{name} Mute"), mute),
            m1: FloatParam::new(
                format!("{name} Macro 1"),
                m1,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            m2: FloatParam::new(
                format!("{name} Macro 2"),
                m2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            m3: FloatParam::new(
                format!("{name} Macro 3"),
                m3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            m4: FloatParam::new(
                format!("{name} Macro 4"),
                m4,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            m5: FloatParam::new(
                format!("{name} Macro 5"),
                m5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            m6: FloatParam::new(
                format!("{name} Macro 6"),
                m6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),
            opt_a: BoolParam::new(format!("{name} Option A"), opt_a),
            opt_b: BoolParam::new(format!("{name} Option B"), opt_b),
        }
    }

    pub fn controls(&self, lane: Lane) -> Vec<Control> {
        let (e_id, pr_id, l_id, p_id, mu_id, m1_id, m2_id, m3_id, m4_id, m5_id, m6_id, oa_id, ob_id) = match lane {
            Lane::Bass => (
                "lane_bass_engine", "lane_bass_preset", "lane_bass_level", "lane_bass_pan", "lane_bass_mute",
                "lane_bass_m1", "lane_bass_m2", "lane_bass_m3", "lane_bass_m4", "lane_bass_m5", "lane_bass_m6",
                "lane_bass_opt_a", "lane_bass_opt_b",
            ),
            Lane::Comp => (
                "lane_comp_engine", "lane_comp_preset", "lane_comp_level", "lane_comp_pan", "lane_comp_mute",
                "lane_comp_m1", "lane_comp_m2", "lane_comp_m3", "lane_comp_m4", "lane_comp_m5", "lane_comp_m6",
                "lane_comp_opt_a", "lane_comp_opt_b",
            ),
            Lane::Arp => (
                "lane_arp_engine", "lane_arp_preset", "lane_arp_level", "lane_arp_pan", "lane_arp_mute",
                "lane_arp_m1", "lane_arp_m2", "lane_arp_m3", "lane_arp_m4", "lane_arp_m5", "lane_arp_m6",
                "lane_arp_opt_a", "lane_arp_opt_b",
            ),
            Lane::Lead => (
                "lane_lead_engine", "lane_lead_preset", "lane_lead_level", "lane_lead_pan", "lane_lead_mute",
                "lane_lead_m1", "lane_lead_m2", "lane_lead_m3", "lane_lead_m4", "lane_lead_m5", "lane_lead_m6",
                "lane_lead_opt_a", "lane_lead_opt_b",
            ),
        };
        vec![
            control(e_id, 7, &self.engine),
            control(pr_id, 7, &self.preset),
            control(l_id, 7, &self.level),
            control(p_id, 7, &self.pan),
            toggle(mu_id, 7, &self.mute),
            control(m1_id, 7, &self.m1),
            control(m2_id, 7, &self.m2),
            control(m3_id, 7, &self.m3),
            control(m4_id, 7, &self.m4),
            control(m5_id, 7, &self.m5),
            control(m6_id, 7, &self.m6),
            toggle(oa_id, 7, &self.opt_a),
            toggle(ob_id, 7, &self.opt_b),
        ]
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
    fn from_route(route: Route) -> Self {
        Self {
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
            ..Self::default()
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

    #[test]
    fn harmonization_mode_and_chromatic_flavor_params_configured() {
        let p = ChordboardParams::default();
        let config = p.config();
        assert_eq!(config.harmonization_mode, 0);
        assert_eq!(config.chromatic_flavor, 0);

        // Check value to string formatting
        assert_eq!(
            p.harmonization_mode.normalized_value_to_string(0.0, false),
            "Classic dual-touch"
        );
        assert_eq!(
            p.harmonization_mode.normalized_value_to_string(0.5, false),
            "Nopia static"
        );
        assert_eq!(
            p.harmonization_mode.normalized_value_to_string(1.0, false),
            "Nopia real"
        );

        assert_eq!(
            p.chromatic_flavor.normalized_value_to_string(0.0, false),
            "Secondary dominants"
        );
        assert_eq!(
            p.chromatic_flavor.normalized_value_to_string(1.0, false),
            "Modal interchange"
        );

        // Check string to value parsing
        assert_eq!(
            p.harmonization_mode.string_to_normalized_value("Static"),
            Some(0.5)
        );
        assert_eq!(
            p.harmonization_mode.string_to_normalized_value("Real"),
            Some(1.0)
        );
        assert_eq!(
            p.harmonization_mode.string_to_normalized_value("Classic"),
            Some(0.0)
        );

        assert_eq!(
            p.chromatic_flavor.string_to_normalized_value("Modal"),
            Some(1.0)
        );
        assert_eq!(
            p.chromatic_flavor.string_to_normalized_value("Secondary"),
            Some(0.0)
        );
    }
}
