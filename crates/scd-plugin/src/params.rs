use nih_plug::prelude::*;
use scd_core::KitPieceId;

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
            vol: FloatParam::new(
                "Sub Vol",
                util::db_to_gain(-6.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-72.0),
                    max: util::db_to_gain(0.0),
                    factor: FloatRange::gain_skew_factor(-72.0, 0.0),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            length: FloatParam::new("Sub Length", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(1)),

            dive: FloatParam::new("Sub Dive", -6.0, FloatRange::Linear { min: -12.0, max: 0.0 })
                .with_unit(" st")
                .with_value_to_string(formatters::v2s_f32_rounded(1)),

            speed: FloatParam::new("Sub Speed", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(1)),

            offset: FloatParam::new("Sub Offset", 0.0, FloatRange::Linear { min: 0.0, max: 50.0 })
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
    // Send gains for Close, XY, Mono, Wide, Front MS, Room
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
        let make_gain = |desc: &str, def_db: f32| {
            FloatParam::new(
                desc,
                util::db_to_gain(def_db),
                FloatRange::Skewed {
                    min: util::db_to_gain(-72.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-72.0, 12.0),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db())
        };

        Self {
            gain: make_gain(&format!("{name} Gain"), 0.0),
            pan: FloatParam::new(
                &format!("{name} Pan"),
                default_pan,
                FloatRange::Linear { min: -100.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            pitch: FloatParam::new(
                &format!("{name} Pitch"),
                0.0,
                FloatRange::Linear { min: -5.0, max: 5.0 },
            )
            .with_unit(" st")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            punch: FloatParam::new(
                &format!("{name} Punch"),
                default_punch,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            send_close: make_gain(&format!("{name} Close Send"), 0.0),
            send_xy: make_gain(&format!("{name} XY Send"), 0.0),
            send_mono: make_gain(&format!("{name} Mono Send"), 0.0),
            send_wide: make_gain(&format!("{name} Wide Send"), 0.0),
            send_front_ms: make_gain(&format!("{name} Front MS Send"), 0.0),
            send_room: make_gain(&format!("{name} Room Send"), 0.0),
        }
    }
}

#[derive(Params)]
pub struct ScdParams {
    #[id = "master_gain"]
    pub master_gain: FloatParam,

    #[id = "fader_lock"]
    pub fader_lock: BoolParam,

    #[nested(group = "sub_kick")]
    pub sub_kick: SubKickParams,

    #[nested(group = "kick")]
    pub kick: ChannelStripParams,
    #[nested(group = "snare")]
    pub snare: ChannelStripParams,
    #[nested(group = "open_snare")]
    pub open_snare: ChannelStripParams,
    #[nested(group = "hihat")]
    pub hihat: ChannelStripParams,
    #[nested(group = "tom1")]
    pub tom1: ChannelStripParams,
    #[nested(group = "tom2")]
    pub tom2: ChannelStripParams,
    #[nested(group = "floor_tom")]
    pub floor_tom: ChannelStripParams,
    #[nested(group = "ride")]
    pub ride: ChannelStripParams,
    #[nested(group = "china")]
    pub china: ChannelStripParams,
    #[nested(group = "stack")]
    pub stack: ChannelStripParams,
    #[nested(group = "splash")]
    pub splash: ChannelStripParams,
    #[nested(group = "l_crash")]
    pub l_crash: ChannelStripParams,
    #[nested(group = "r_crash")]
    pub r_crash: ChannelStripParams,
}

impl Default for ScdParams {
    fn default() -> Self {
        Self {
            master_gain: FloatParam::new(
                "Master Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-72.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-72.0, 12.0),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            fader_lock: BoolParam::new("Fader Lock", false),
            sub_kick: SubKickParams::default(),

            // Default pans and punches from BUILD_DATA.js
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
}
