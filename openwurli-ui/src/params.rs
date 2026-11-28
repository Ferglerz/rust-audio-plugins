use crate::models::{ClavinetModel, D6Tone, Instrument, ModelBank, PianetRevision, ReedModel};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

pub(crate) const PICKUP_DRIVE_MAX: f32 = 1.04;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum PianoType {
    #[id = "wurli"]
    #[name = "OpenWurli"]
    Wurli,
    #[id = "stage"]
    #[name = "Stage 73"]
    Stage,
    #[id = "grand"]
    #[name = "Electric Grand"]
    Grand,
}
impl PianoType {
    pub const ALL: [Self; 3] = [Self::Wurli, Self::Stage, Self::Grand];

    pub fn native_range(self, grand: GrandModel) -> std::ops::RangeInclusive<u8> {
        match self {
            Self::Wurli => openwurli_dsp::tables::MIDI_LO..=openwurli_dsp::tables::MIDI_HI,
            Self::Stage => stage73::engine::LOW..=stage73::engine::HIGH,
            Self::Grand => match grand {
                GrandModel::Cp80 => cp80::engine::LOW..=cp80::engine::HIGH,
                GrandModel::Cp70B => cp80::seventy::LOW..=cp80::seventy::HIGH,
            },
        }
    }
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum GrandModel {
    #[id = "cp80"]
    #[name = "CP-80"]
    Cp80,
    #[id = "cp70b"]
    #[name = "CP-70B"]
    Cp70B,
}

#[derive(Params)]
pub struct OpenWurliUiParams {
    pub(crate) keyboard: Arc<pleasant_ui::playable_keyboard::KeyboardLink>,
    pub(crate) electra_pedals: std::sync::atomic::AtomicU8,
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[id = "volume"]
    pub volume: FloatParam,

    #[id = "trem_depth"]
    pub tremolo_depth: FloatParam,

    #[id = "speaker"]
    pub speaker_character: FloatParam,

    #[id = "model_bank"]
    pub model_bank: EnumParam<ModelBank>,
    #[id = "clavinet_model"]
    pub clavinet_model: EnumParam<ClavinetModel>,
    #[id = "early_reed_model"]
    pub reed_model: EnumParam<ReedModel>,
    #[id = "pianet_revision"]
    pub pianet_revision: EnumParam<PianetRevision>,
    #[id = "pianet_early_knee"]
    pub early_knee: FloatParam,
    #[id = "pianet_c_vibrato"]
    pub early_vibrato: BoolParam,
    #[id = "pianet_local_out"]
    pub early_local_output: BoolParam,
    #[id = "clavinet_e7_power"]
    pub clavinet_e7_power: BoolParam,
    #[id = "claviset_model"]
    pub claviset_model: EnumParam<crate::models::ClavisetModel>,
    #[nested(id_prefix = "claviset", group = "Claviset family")]
    pub claviset: crate::claviset::ClavisetTone,
    #[nested(id_prefix = "smod", group = "Pianet sustain modifications")]
    pub sustain_mod: crate::sustain_mod::Settings,
    #[nested(id_prefix = "pickup", group = "Pickup regulation")]
    pub regulation: crate::regulation::Settings,
    #[id = "vv64_enable"]
    pub vv64_enabled: BoolParam,
    #[nested(id_prefix = "vv64", group = "Vintage Vibe 64")]
    pub vv64: crate::vv64::Vv64Tone,
    #[id = "stereovibe_enable"]
    pub stereovibe_enabled: BoolParam,
    #[nested(id_prefix = "stereovibe", group = "Vintage Vibe StereoVibe")]
    pub stereovibe: crate::stereovibe::StereoVibeTone,
    #[id = "road_enable"]
    pub road_enabled: BoolParam,
    #[nested(id_prefix = "road", group = "Roadmaster 64")]
    pub road: crate::road::RoadTone,
    #[id = "baldwin_enable"]
    pub baldwin_enabled: BoolParam,
    #[nested(id_prefix = "baldwin", group = "Baldwin Electro-Piano")]
    pub baldwin: crate::baldwin::BaldwinTone,
    #[id = "mk8_enable"]
    pub mk8_enabled: BoolParam,
    #[nested(id_prefix = "mk8", group = "Rhodes MK8 S")]
    pub mk8: crate::mk8::Mk8Tone,
    #[id = "vibanet_enable"]
    pub vibanet_enabled: BoolParam,
    #[nested(id_prefix = "vibanet", group = "Vibanet")]
    pub vibanet: crate::vibanet::VibanetTone,
    #[id = "elepian_enable"]
    pub elepian_enabled: BoolParam,
    #[nested(id_prefix = "elepian", group = "Columbia Elepian")]
    pub elepian: crate::elepian::ElepianTone,
    #[id = "valente_enable"]
    pub valente_enabled: BoolParam,
    #[nested(id_prefix = "valente", group = "Valente")]
    pub valente: crate::valente::ValenteTone,
    #[id = "vivi_enable"]
    pub vivi_enabled: BoolParam,
    #[nested(id_prefix = "vivi", group = "Vivi")]
    pub vivi: crate::vivi::ViviTone,
    #[id = "neo_enable"]
    pub neo_enabled: BoolParam,
    #[nested(id_prefix = "neo", group = "Neo-Bechstein")]
    pub neo: crate::neo::NeoTone,
    #[id = "storytone_enable"]
    pub storytone_enabled: BoolParam,
    #[nested(id_prefix = "story", group = "Storytone")]
    pub storytone: crate::storytone::StorytoneTone,
    #[id = "duo_enable"]
    pub duo_enabled: BoolParam,
    #[nested(id_prefix = "duo", group = "Hohner Duo")]
    pub duo: crate::duo::DuoTone,
    #[id = "clavinet_e7_volume"]
    pub clavinet_e7_volume: FloatParam,
    #[id = "cembalet_swell"]
    pub cembalet_swell: FloatParam,
    #[id = "cembalet_vibrato"]
    pub cembalet_vibrato: BoolParam,
    #[id = "electra_enable"]
    pub electra_enabled: BoolParam,
    #[id = "electra_volume"]
    pub electra_volume: FloatParam,
    #[id = "electra_soft"]
    pub electra_soft: BoolParam,
    #[id = "electra_soft_depth"]
    pub electra_soft_depth: FloatParam,
    #[id = "electra_sustain"]
    pub electra_sustain: BoolParam,
    #[nested(id_prefix = "d6", group = "Clavinet D6")]
    pub d6: D6Tone,

    #[id = "pianet_n_swell"]
    pub pianet_swell: FloatParam,
    #[id = "pianet_n_vibrato"]
    pub pianet_vibrato: BoolParam,

    #[id = "piano_type"]
    pub piano: EnumParam<PianoType>,
    #[nested(id_prefix = "stage", group = "Stage 73")]
    pub stage: stage73::params::StageTone,
    #[nested(id_prefix = "grand", group = "Electric Grand")]
    pub grand: cp80::plugin::GrandTone,
    #[id = "stage_panel_vol"]
    pub stage_panel_volume: FloatParam,
    #[id = "stage_cable_pf"]
    pub stage_cable: FloatParam,
    #[id = "grand_model"]
    pub grand_model: EnumParam<GrandModel>,
    // Keep both normalized endpoints of the original grand_model automation lane.
    #[id = "grand_70_original"]
    pub grand_70_original: BoolParam,
    #[id = "grand_jack_vol"]
    pub grand_jack_volume: FloatParam,

    #[id = "reed_decay"]
    pub reed_decay: FloatParam,

    #[id = "hammer_hardness"]
    pub hammer_hardness: FloatParam,

    #[id = "pickup_drive"]
    pub pickup_drive: FloatParam,

    #[id = "trem_response"]
    pub tremolo_response: FloatParam,

    #[id = "extended_notes"]
    pub extended_notes: BoolParam,

    #[id = "rail_sag"]
    pub rail_sag: BoolParam,

    #[id = "pickup_height"]
    pub pickup_height: FloatParam,
}

impl OpenWurliUiParams {
    pub(crate) fn stage_controls(&self) -> stage73::engine::Controls {
        stage73::engine::Controls {
            volume: self.stage_panel_volume.value().into(),
            cable_pf: self.stage_cable.value().into(),
            ..self.stage.controls()
        }
    }

    pub(crate) fn original_cp70(&self) -> bool {
        self.grand_model.value() == GrandModel::Cp70B && self.grand_70_original.value()
    }

    pub(crate) fn electra_pedal_state(&self) -> u8 {
        self.electra_pedals
            .load(std::sync::atomic::Ordering::Relaxed)
            | u8::from(self.electra_sustain.value())
            | (u8::from(self.electra_soft.value()) << 1)
    }

    pub(crate) fn electra_controls(&self, midi_soft: bool) -> electra_piano::circuit::Controls {
        electra_piano::circuit::Controls {
            volume: self.electra_volume.value().into(),
            soft_depth: self.electra_soft_depth.value().into(),
            soft_pedal: midi_soft || self.electra_soft.value(),
            ..Default::default()
        }
    }

    pub(crate) fn clavinet_volume(&self) -> &FloatParam {
        if self.clavinet_model.value() == ClavinetModel::E7 {
            &self.clavinet_e7_volume
        } else {
            &self.d6.volume
        }
    }

    pub(crate) fn reed_controls(&self) -> (&FloatParam, &BoolParam) {
        if matches!(self.instrument(), Instrument::Reed(m) if m.is_cembalet()) {
            (&self.cembalet_swell, &self.cembalet_vibrato)
        } else if matches!(
            self.instrument(),
            Instrument::Reed(pianet_n::Model::C | pianet_n::Model::L)
        ) {
            (&self.early_knee, &self.early_vibrato)
        } else {
            (&self.pianet_swell, &self.pianet_vibrato)
        }
    }

    pub(crate) fn grand_controls(&self) -> cp80::panel::Controls {
        cp80::panel::Controls {
            volume: self.grand_jack_volume.value().into(),
            ..self.grand.controls()
        }
    }

    pub(crate) fn key_range(&self) -> std::ops::RangeInclusive<u8> {
        if self.extended_notes.value() {
            return 0..=127;
        }
        match self.instrument() {
            Instrument::Clavinet(_) => return clavinet_d6::live::LOW..=clavinet_d6::live::HIGH,
            Instrument::Reed(model) => return model.key_range(),
            Instrument::Claviset(_) => return pianet_n::claviset::LOW..=pianet_n::claviset::HIGH,
            Instrument::Vv64 => return vintage_vibe_64::live::LOW..=vintage_vibe_64::live::HIGH,
            Instrument::StereoVibe => {
                return vintage_vibe_64::live::LOW..=vintage_vibe_64::live::HIGH
            }
            Instrument::Road => return roadmaster::LOW..=roadmaster::HIGH,
            Instrument::Baldwin => {
                return baldwin_electropiano::action::LOW..=baldwin_electropiano::action::HIGH
            }
            Instrument::Mk8 => return rhodes_mk8::live::LOW..=rhodes_mk8::live::HIGH,
            Instrument::Vibanet => return clavinet_d6::live::LOW..=clavinet_d6::live::HIGH,
            Instrument::Elepian => return pianet_n::elepian::LOW..=pianet_n::elepian::HIGH,
            Instrument::Valente => return pianet_n::valente::LOW..=pianet_n::valente::HIGH,
            Instrument::Vivi => return pianet_n::vivi::LOW..=pianet_n::vivi::HIGH,
            Instrument::Neo => return neo_bechstein::action::LOW..=neo_bechstein::action::HIGH,
            Instrument::Storytone => return ::storytone::engine::LOW..=::storytone::engine::HIGH,
            Instrument::Duo => return hohner_duo::LOW..=hohner_duo::HIGH,
            Instrument::Electra => return electra_piano::live::LOW..=electra_piano::live::HIGH,
            Instrument::Piano(_) => {}
        }
        self.piano.value().native_range(self.grand_model.value())
    }
}

impl Default for OpenWurliUiParams {
    fn default() -> Self {
        Self {
            keyboard: Arc::default(),
            electra_pedals: Default::default(),
            editor_state: ViziaState::new_screen_sized("Electric Pianos", || {
                (crate::ui::WIDTH as u32, crate::ui::HEIGHT as u32)
            }),
            volume: percent_param("Volume", 0.5).with_smoother(SmoothingStyle::Linear(5.0)),
            tremolo_depth: percent_param("Tremolo Depth", 0.5)
                .with_smoother(SmoothingStyle::Linear(50.0)),
            speaker_character: percent_param("Speaker Character", 0.0)
                .with_smoother(SmoothingStyle::Linear(50.0)),
            model_bank: EnumParam::new("Instrument Family", ModelBank::Piano),
            clavinet_model: EnumParam::new("Clavinet Model", ClavinetModel::D6),
            reed_model: EnumParam::new("Electrostatic Reed Model", ReedModel::PianetN),
            pianet_revision: EnumParam::new("Pianet Revision", PianetRevision::N),
            early_knee: percent_param("Early Pianet Knee", 1.),
            early_vibrato: BoolParam::new("Pianet C Vibrato", false),
            early_local_output: BoolParam::new("Early Pianet Local Output", false),
            clavinet_e7_power: BoolParam::new("E7 Power", true),
            claviset_model: EnumParam::new("Claviset Family", crate::models::ClavisetModel::Off),
            claviset: Default::default(),
            regulation: Default::default(),
            sustain_mod: Default::default(),
            vv64_enabled: BoolParam::new("Vintage Vibe 64", false),
            stereovibe_enabled: BoolParam::new("Vintage Vibe StereoVibe", false),
            road_enabled: BoolParam::new("Roadmaster 64", false),
            baldwin_enabled: BoolParam::new("Baldwin Electro-Piano", false),
            mk8_enabled: BoolParam::new("Rhodes MK8 S", false),
            vibanet_enabled: BoolParam::new("Vibanet", false),
            vv64: Default::default(),
            stereovibe: Default::default(),
            road: Default::default(),
            baldwin: Default::default(),
            mk8: Default::default(),
            vibanet: Default::default(),
            elepian_enabled: BoolParam::new("Elepian EP-61C", false),
            valente_enabled: BoolParam::new("Valente Electric Piano", false),
            vivi_enabled: BoolParam::new("Vivi-Tone Clavier", false),
            elepian: Default::default(),
            valente: Default::default(),
            vivi: Default::default(),
            neo_enabled: BoolParam::new("Neo-Bechstein", false),
            neo: Default::default(),
            storytone_enabled: BoolParam::new("Storytone", false),
            storytone: Default::default(),
            duo_enabled: BoolParam::new("Pianet/Clavinet Duo", false),
            duo: Default::default(),
            clavinet_e7_volume: percent_param("E7 Slider Volume", 1.),
            cembalet_swell: percent_param("Cembalet Swell", 1.),
            cembalet_vibrato: BoolParam::new("Cembalet Vibrato", false),
            electra_enabled: BoolParam::new("Electra-Piano", false),
            electra_volume: percent_param("Electra Volume", 0.75),
            electra_soft: BoolParam::new("Electra Soft Pedal", false),
            electra_soft_depth: percent_param("Electra Soft Depth", 1.),
            electra_sustain: BoolParam::new("Electra Sustain Pedal", false),
            d6: Default::default(),
            pianet_swell: percent_param("Pianet N Swell", 1.),
            pianet_vibrato: BoolParam::new("Pianet N Vibrato", false),
            piano: EnumParam::new("Piano", PianoType::Wurli),
            stage: Default::default(),
            grand: Default::default(),
            grand_model: EnumParam::new("Grand Model", GrandModel::Cp80),
            grand_70_original: BoolParam::new("Original CP-70", false),
            stage_panel_volume: percent_param("Stage Panel Volume", 1.),
            stage_cable: FloatParam::new(
                "Stage Cable Load",
                stage73::circuit::DEFAULT_CABLE_PF as f32,
                FloatRange::Linear {
                    min: stage73::circuit::MIN_CABLE_PF as f32,
                    max: stage73::circuit::MAX_CABLE_PF as f32,
                },
            )
            .with_unit(" pF")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
            grand_jack_volume: percent_param("Grand Jack Volume", 1.),
            reed_decay: FloatParam::new(
                "Reed Decay",
                1.0,
                FloatRange::Skewed {
                    min: 0.5,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit("×")
            .with_value_to_string(Arc::new(|value| format!("{value:.2}")))
            .with_string_to_value(Arc::new(|text| {
                text.trim().trim_end_matches('×').parse().ok()
            })),
            hammer_hardness: multiplier_param("Hammer Hardness", 2.0),
            pickup_drive: multiplier_param("Pickup Drive", PICKUP_DRIVE_MAX),
            tremolo_response: FloatParam::new(
                "Tremolo Response",
                1.0,
                FloatRange::Skewed {
                    min: openwurli_dsp::tremolo::RESPONSE_MIN as f32,
                    max: openwurli_dsp::tremolo::RESPONSE_MAX as f32,
                    // Keeps the stock cell at the original one-third knob position.
                    factor: 0.5,
                },
            )
            .with_unit("×")
            .with_value_to_string(Arc::new(|value| format!("{value:.3}")))
            .with_string_to_value(Arc::new(|text| {
                text.trim().trim_end_matches('×').parse().ok()
            })),
            extended_notes: BoolParam::new("Extended Notes", false),
            rail_sag: BoolParam::new("Extra Sag", false),
            pickup_height: FloatParam::new(
                "Pickup Height",
                0.0,
                FloatRange::Linear {
                    min: -(openwurli_dsp::pickup::MAX_HEIGHT_TRIM_MM as f32),
                    max: openwurli_dsp::pickup::MAX_HEIGHT_TRIM_MM as f32,
                },
            )
            .with_unit(" mm")
            .with_value_to_string(Arc::new(|mm| format!("{mm:+.3}"))),
        }
    }
}

fn multiplier_param(name: &'static str, max: f32) -> FloatParam {
    FloatParam::new(name, 1.0, FloatRange::Linear { min: 0.5, max })
        .with_unit("×")
        .with_value_to_string(Arc::new(|value| format!("{value:.2}")))
        .with_string_to_value(Arc::new(|text| {
            text.trim().trim_end_matches('×').parse().ok()
        }))
}

fn percent_param(name: &'static str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
