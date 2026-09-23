use atomic_float::AtomicF32;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use scd_core::{KitPieceId, MicChannel};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use crate::note_map::NoteMapState;
use crate::presets::{default_send_db, UserPresetState};
use crate::vel_map::VelMapState;

pub const EDITOR_WIDTH: u32 = 1181;
pub const EDITOR_HEIGHT: u32 = 611;
pub const EDITOR_ZOOM_MIN: f64 = 0.5;
pub const EDITOR_ZOOM_MAX: f64 = 2.0;
pub const FADER_MIN_DB: f32 = -72.0;
pub const FADER_MAX_DB: f32 = 12.0;
pub const VU_PEAK_COUNT: usize = MicChannel::COUNT * 2 + 2;

/// Compatibility base size for sessions saved with the old percentage menu.
/// New resizing is continuous and persisted by ViziaState on top of this base.
/// Keep the old percentage stable so restoring and saving never compounds it.
pub fn editor_logical_size(zoom: f64) -> (u32, u32) {
    let z = zoom.clamp(EDITOR_ZOOM_MIN, EDITOR_ZOOM_MAX);
    (
        (EDITOR_WIDTH as f64 * z).round() as u32,
        (EDITOR_HEIGHT as f64 * z).round() as u32,
    )
}

fn editor_state_for_zoom(zoom_pct: Arc<AtomicU32>) -> Arc<ViziaState> {
    ViziaState::new_screen_sized("SoundChef Drums", move || {
        editor_logical_size(zoom_pct.load(Ordering::Relaxed) as f64 / 100.0)
    })
}

fn db_param(name: impl Into<String>, def_db: f32, min_db: f32, max_db: f32) -> FloatParam {
    FloatParam::new(
        name,
        util::db_to_gain(def_db),
        FloatRange::Skewed {
            min: util::db_to_gain(min_db),
            max: util::db_to_gain(max_db),
            factor: FloatRange::gain_skew_factor(min_db, max_db),
        },
    )
    .with_unit(" dB")
    .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
    .with_string_to_value(formatters::s2v_f32_gain_to_db())
}

/// HISE `UI_Meters.js`: `0.01 * (100 + dB)` so 0 dBFS fills the meter and −100 dB is empty.
pub fn peak_to_meter(level: f32) -> f32 {
    if level <= 1.0e-5 {
        0.0
    } else {
        (1.0 + util::gain_to_db(level) / 100.0).clamp(0.0, 1.0)
    }
}

/// Peak levels shared between audio thread and UI.
/// Layout: [mic0_L, mic0_R, mic1_L, mic1_R, ... mic5_L, mic5_R, master_L, master_R] = 14
pub struct VuMeters {
    pub peaks: [AtomicF32; VU_PEAK_COUNT],
    pub kit: [AtomicF32; KitPieceId::COUNT],
}

impl Default for VuMeters {
    fn default() -> Self {
        Self {
            peaks: std::array::from_fn(|_| AtomicF32::new(0.0)),
            kit: std::array::from_fn(|_| AtomicF32::new(0.0)),
        }
    }
}

impl VuMeters {
    /// Read all 14 peak values. Resets each to 0 after reading (peak-hold done in UI).
    pub fn read_and_clear(&self) -> [f32; VU_PEAK_COUNT] {
        std::array::from_fn(|i| self.peaks[i].swap(0.0, Ordering::Relaxed))
    }

    pub fn read_and_clear_kit(&self) -> [f32; KitPieceId::COUNT] {
        std::array::from_fn(|i| self.kit[i].swap(0.0, Ordering::Relaxed))
    }

    /// Peek without clearing. Used by the editor timer to skip idle redraws.
    pub fn any_above(&self, eps: f32) -> bool {
        self.peaks.iter().any(|p| p.load(Ordering::Relaxed) > eps)
            || self.kit.iter().any(|p| p.load(Ordering::Relaxed) > eps)
    }

    /// Update mic peak (hold max).
    #[inline]
    pub fn update_mic(&self, mic: usize, l: f32, r: f32) {
        Self::hold(&self.peaks, mic * 2, l.abs());
        Self::hold(&self.peaks, mic * 2 + 1, r.abs());
    }

    /// Update master peak (hold max).
    #[inline]
    pub fn update_master(&self, l: f32, r: f32) {
        Self::hold(&self.peaks, MicChannel::COUNT * 2, l.abs());
        Self::hold(&self.peaks, MicChannel::COUNT * 2 + 1, r.abs());
    }

    #[inline]
    pub fn update_kit(&self, kit_piece: usize, mag: f32) {
        Self::hold(&self.kit, kit_piece, mag.abs());
    }

    #[inline]
    fn hold(slots: &[AtomicF32], idx: usize, mag: f32) {
        let prev = slots[idx].load(Ordering::Relaxed);
        if mag > prev {
            slots[idx].store(mag, Ordering::Relaxed);
        }
    }
}

#[derive(Params)]
pub struct SubKickParams {
    #[id = "sub_vol"]
    pub vol: FloatParam,
    #[id = "sub_length"]
    pub length: FloatParam,
    #[id = "sub_dive"]
    pub dive: FloatParam,
    #[id = "sub_speed"]
    pub speed: FloatParam,
    #[id = "sub_offset"]
    pub offset: FloatParam,
}

impl Default for SubKickParams {
    fn default() -> Self {
        Self {
            vol: db_param("Sub Vol", -6.0, FADER_MIN_DB, 0.0),

            length: FloatParam::new("Sub Length", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(1)),

            dive: FloatParam::new(
                "Sub Dive",
                -6.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 0.0,
                },
            )
            .with_unit(" st")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            speed: FloatParam::new("Sub Speed", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(1)),

            offset: FloatParam::new(
                "Sub Offset",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
        }
    }
}

#[derive(Params)]
pub struct ChannelStripParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "pan"]
    pub pan: FloatParam,
    #[id = "pitch"]
    pub pitch: FloatParam,
    #[id = "punch"]
    pub punch: FloatParam,
    #[id = "send_close"]
    pub send_close: FloatParam,
    #[id = "send_xy"]
    pub send_xy: FloatParam,
    #[id = "send_mono"]
    pub send_mono: FloatParam,
    #[id = "send_wide"]
    pub send_wide: FloatParam,
    #[id = "send_front_ms"]
    pub send_front_ms: FloatParam,
    #[id = "send_room"]
    pub send_room: FloatParam,
}

impl ChannelStripParams {
    pub fn new(name: &str, default_pan: f32, default_punch: f32) -> Self {
        Self {
            gain: db_param(format!("{name} Gain"), 0.0, FADER_MIN_DB, FADER_MAX_DB),
            pan: FloatParam::new(
                format!("{name} Pan"),
                default_pan,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            pitch: FloatParam::new(
                format!("{name} Pitch"),
                0.0,
                FloatRange::Linear {
                    min: -5.0,
                    max: 5.0,
                },
            )
            .with_unit(" st")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            punch: FloatParam::new(
                format!("{name} Punch"),
                default_punch,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            // BUILD_DATA StartingVolume: Close + Front MS at fader 0.715 ≈ 0 dB, others silent.
            send_close: db_param(
                format!("{name} Close Send"),
                default_send_db(MicChannel::Close),
                FADER_MIN_DB,
                FADER_MAX_DB,
            ),
            send_xy: db_param(
                format!("{name} XY Send"),
                default_send_db(MicChannel::XY),
                FADER_MIN_DB,
                FADER_MAX_DB,
            ),
            send_mono: db_param(
                format!("{name} Mono Send"),
                default_send_db(MicChannel::Mono),
                FADER_MIN_DB,
                FADER_MAX_DB,
            ),
            send_wide: db_param(
                format!("{name} Wide Send"),
                default_send_db(MicChannel::Wide),
                FADER_MIN_DB,
                FADER_MAX_DB,
            ),
            send_front_ms: db_param(
                format!("{name} Front MS Send"),
                default_send_db(MicChannel::FrontMS),
                FADER_MIN_DB,
                FADER_MAX_DB,
            ),
            send_room: db_param(
                format!("{name} Room Send"),
                default_send_db(MicChannel::Room),
                FADER_MIN_DB,
                FADER_MAX_DB,
            ),
        }
    }

    pub fn send_for(&self, mic: MicChannel) -> &FloatParam {
        match mic {
            MicChannel::Close => &self.send_close,
            MicChannel::XY => &self.send_xy,
            MicChannel::Mono => &self.send_mono,
            MicChannel::Wide => &self.send_wide,
            MicChannel::FrontMS => &self.send_front_ms,
            MicChannel::Room => &self.send_room,
        }
    }

    pub fn fader_param(&self, sof: Option<MicChannel>) -> &FloatParam {
        match sof {
            None => &self.gain,
            Some(mic) => self.send_for(mic),
        }
    }

    /// Strip gain × mic send, with omitted mics forced silent.
    pub fn mix_gain(&self, kit_piece: KitPieceId, mic: MicChannel) -> f32 {
        if kit_piece.omits_mic(mic) {
            0.0
        } else {
            self.gain.value() * self.send_for(mic).value()
        }
    }
}

#[derive(Params)]
pub struct ScdParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    /// Legacy size basis only; the editor no longer writes or snaps this value.
    #[persist = "editor-zoom"]
    pub editor_zoom_pct: Arc<AtomicU32>,

    #[persist = "vel-maps"]
    pub vel_maps: Arc<VelMapState>,

    #[persist = "note-maps"]
    pub note_maps: Arc<NoteMapState>,

    #[persist = "user-presets"]
    pub user_presets: Arc<UserPresetState>,

    #[id = "master_gain"]
    pub master_gain: FloatParam,

    #[id = "fader_lock"]
    pub fader_lock: BoolParam,

    #[id = "cc_num"]
    pub cc_number: IntParam,

    #[id = "cc_inv"]
    pub invert_cc: BoolParam,

    #[nested(group = "sub_kick")]
    pub sub_kick: SubKickParams,

    // `group` is only the host tree label. Each strip must also get a unique
    // `id_prefix` or every `pitch`/`pan`/`gain` hashes to the last strip (R Crash).
    #[nested(id_prefix = "kick", group = "kick")]
    pub kick: ChannelStripParams,
    #[nested(id_prefix = "snare", group = "snare")]
    pub snare: ChannelStripParams,
    #[nested(id_prefix = "osnare", group = "open_snare")]
    pub open_snare: ChannelStripParams,
    #[nested(id_prefix = "hihat", group = "hihat")]
    pub hihat: ChannelStripParams,
    #[nested(id_prefix = "tom1", group = "tom1")]
    pub tom1: ChannelStripParams,
    #[nested(id_prefix = "tom2", group = "tom2")]
    pub tom2: ChannelStripParams,
    #[nested(id_prefix = "floor", group = "floor_tom")]
    pub floor_tom: ChannelStripParams,
    #[nested(id_prefix = "ride", group = "ride")]
    pub ride: ChannelStripParams,
    #[nested(id_prefix = "china", group = "china")]
    pub china: ChannelStripParams,
    #[nested(id_prefix = "stack", group = "stack")]
    pub stack: ChannelStripParams,
    #[nested(id_prefix = "splash", group = "splash")]
    pub splash: ChannelStripParams,
    #[nested(id_prefix = "lcrash", group = "l_crash")]
    pub l_crash: ChannelStripParams,
    #[nested(id_prefix = "rcrash", group = "r_crash")]
    pub r_crash: ChannelStripParams,

    /// Not a DSP param — shared with UI for VU meter display.
    pub vu: Arc<VuMeters>,

    /// Last processed hi-hat CC (0–127). `255` means none received yet.
    pub cc_display: AtomicU8,

    /// Pending incoming velocity per resolved articulation; zero means no new hit.
    pub midi_velocities: [[AtomicU8; crate::vel_map::MAX_ARTS]; KitPieceId::COUNT],
}

impl Default for ScdParams {
    fn default() -> Self {
        let editor_zoom_pct = Arc::new(AtomicU32::new(100));
        Self {
            editor_state: editor_state_for_zoom(editor_zoom_pct.clone()),
            editor_zoom_pct,
            vel_maps: Arc::new(VelMapState::identity()),
            note_maps: Arc::new(NoteMapState::identity()),
            user_presets: Arc::new(UserPresetState::empty()),
            master_gain: db_param("Master Gain", 0.0, FADER_MIN_DB, FADER_MAX_DB),

            fader_lock: BoolParam::new("Fader Lock", false),
            cc_number: IntParam::new("Hihat CC", 4, IntRange::Linear { min: 0, max: 127 }),
            invert_cc: BoolParam::new("Invert CC", true),
            sub_kick: SubKickParams::default(),

            kick: ChannelStripParams::new("Kick", 0.0, 0.3),
            snare: ChannelStripParams::new("Snare", 0.0, 0.25),
            open_snare: ChannelStripParams::new("Open Snare", 0.0, 0.0),
            hihat: ChannelStripParams::new("Hihat", 10.0, 0.15),
            tom1: ChannelStripParams::new("Tom 1", 10.0, 0.0),
            tom2: ChannelStripParams::new("Tom 2", -10.0, 0.0),
            floor_tom: ChannelStripParams::new("Floor Tom", -20.0, 0.0),
            ride: ChannelStripParams::new("Ride", -25.0, 0.0),
            china: ChannelStripParams::new("China", 0.0, 0.0),
            stack: ChannelStripParams::new("Stack", 10.0, 0.0),
            splash: ChannelStripParams::new("Splash", 0.0, 0.0),
            l_crash: ChannelStripParams::new("L Crash", 0.0, 0.0),
            r_crash: ChannelStripParams::new("R Crash", 0.0, 0.0),
            vu: Arc::new(VuMeters::default()),
            cc_display: AtomicU8::new(255),
            midi_velocities: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(0))),
        }
    }
}

impl ScdParams {
    pub fn get_strip(&self, id: KitPieceId) -> &ChannelStripParams {
        match id {
            KitPieceId::Kick => &self.kick,
            KitPieceId::Snare => &self.snare,
            KitPieceId::OpenSnare => &self.open_snare,
            KitPieceId::Hihat => &self.hihat,
            KitPieceId::Tom1 => &self.tom1,
            KitPieceId::Tom2 => &self.tom2,
            KitPieceId::FloorTom => &self.floor_tom,
            KitPieceId::Ride => &self.ride,
            KitPieceId::China => &self.china,
            KitPieceId::Stack => &self.stack,
            KitPieceId::Splash => &self.splash,
            KitPieceId::LCrash => &self.l_crash,
            KitPieceId::RCrash => &self.r_crash,
        }
    }

    /// Per-kit-piece, per-mic mix coefficients for the current parameter values.
    pub fn mix_gain_table(&self) -> [[f32; MicChannel::COUNT]; KitPieceId::COUNT] {
        let mut table = [[0.0f32; MicChannel::COUNT]; KitPieceId::COUNT];
        for kit_piece in KitPieceId::ALL {
            let strip = self.get_strip(kit_piece);
            for mic in MicChannel::ALL {
                table[kit_piece as usize][mic as usize] = strip.mix_gain(kit_piece, mic);
            }
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_logical_size_is_identity_at_100_percent() {
        assert_eq!(editor_logical_size(1.0), (EDITOR_WIDTH, EDITOR_HEIGHT));
    }

    #[test]
    fn editor_state_size_tracks_persisted_zoom() {
        let params = ScdParams::default();
        assert_eq!(
            params.editor_state.inner_logical_size(),
            (EDITOR_WIDTH, EDITOR_HEIGHT)
        );
        params.editor_zoom_pct.store(150, Ordering::Relaxed);
        assert_eq!(
            params.editor_state.inner_logical_size(),
            editor_logical_size(1.5)
        );
        // Restore a non-preset scale through the actual persistence path.
        let restored: ViziaState = serde_json::from_str(r#"{"scale_factor":1.137}"#).unwrap();
        nih_plug::params::persist::PersistentField::set(&params.editor_state, restored);
        assert_eq!(
            serde_json::to_value(&*params.editor_state).unwrap()["scale_factor"],
            1.137
        );
        let (w, h) = editor_logical_size(1.5);
        let scale = params.editor_state.user_scale_factor();
        // Opening on a smaller monitor may reduce the restored scale.
        assert!(scale > 0.0 && scale <= 1.137);
        assert_eq!(
            params.editor_state.scaled_logical_size(),
            (
                (w as f64 * scale).round() as u32,
                (h as f64 * scale).round() as u32,
            )
        );
        assert_eq!(params.editor_zoom_pct.load(Ordering::Relaxed), 150);
    }

    #[test]
    fn editor_state_round_trip_keeps_legacy_basis_and_continuous_scale() {
        let params = ScdParams::default();
        let mut fields = params.serialize_fields();
        fields.insert("editor-zoom".into(), "125".into());
        fields.insert("editor-state".into(), r#"{"scale_factor":1.0}"#.into());
        params.deserialize_fields(&fields);
        assert_eq!(
            params.editor_state.inner_logical_size(),
            editor_logical_size(1.25)
        );
        let scale: ViziaState = serde_json::from_str(r#"{"scale_factor":1.137}"#).unwrap();
        nih_plug::params::persist::PersistentField::set(&params.editor_state, scale);
        for _ in 0..3 {
            let saved = params.serialize_fields();
            let reopened = ScdParams::default();
            reopened.deserialize_fields(&saved);
            assert_eq!(
                reopened.editor_state.inner_logical_size(),
                editor_logical_size(1.25)
            );
            assert_eq!(reopened.serialize_fields()["editor-zoom"], "125");
            let state: serde_json::Value =
                serde_json::from_str(&reopened.serialize_fields()["editor-state"]).unwrap();
            assert_eq!(state["scale_factor"], 1.137);
            params.deserialize_fields(&reopened.serialize_fields());
        }
    }
}
