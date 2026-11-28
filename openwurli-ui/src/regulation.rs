//! Per-instrument pickup regulation and pad-return controls, saved in physical units.
use crate::models::{ClavinetModel as Clav, Instrument};
use crate::params::{OpenWurliUiParams, PianoType};
use nih_plug::prelude::*;
use pianet_n::{claviset::Instrument as Cam, Model as Reed};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Adjustment {
    Gap,
    Bridge,
    ReedGap,
    PadReturn,
}

use Adjustment::*;

pub(crate) fn adjustments(model: Instrument) -> &'static [Adjustment] {
    match model {
        Instrument::Piano(PianoType::Grand) => &[],
        Instrument::Clavinet(_) | Instrument::Vibanet => &[Gap, Bridge],
        Instrument::Road => &[Gap, Bridge, ReedGap],
        Instrument::Duo => &[Gap, Bridge, ReedGap, PadReturn],
        Instrument::Reed(Reed::N | Reed::C | Reed::L | Reed::T) => &[Gap, PadReturn],
        _ => &[Gap],
    }
}

fn distance(name: &'static str, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, 0., FloatRange::Linear { min, max })
        .with_smoother(SmoothingStyle::Linear(5.))
        .with_unit(" mm")
        .with_value_to_string(std::sync::Arc::new(|x| format!("{x:+.3}")))
}
fn contribution(name: &'static str) -> FloatParam {
    FloatParam::new(name, 1., FloatRange::Linear { min: 0., max: 1. })
        .with_smoother(SmoothingStyle::Linear(5.))
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

// Keep each model's saved voicing independent, including the two pickups on each Clavinet.
macro_rules! settings {
    ($( $field:ident, $id:literal, $model:pat, $kind:ident, $default:expr; )+) => {
        #[derive(Params)]
        pub struct Settings { $( #[id = $id] pub $field: FloatParam, )+ }
        impl Default for Settings {
            fn default() -> Self { Self { $( $field: $default, )+ } }
        }
        pub(crate) fn restore_missing(state: &mut nih_plug::prelude::PluginState) {
            use nih_plug::wrapper::state::ParamValue;
            $( state.params.entry(concat!("pickup_", $id).to_owned())
                .or_insert(ParamValue::F32(if Adjustment::$kind == PadReturn {1.} else {0.})); )+
        }
        pub(crate) fn param(p: &OpenWurliUiParams, model: Instrument, kind: Adjustment) -> &FloatParam {
            match (model, kind) {
                (Instrument::Piano(PianoType::Stage), Gap) => &p.stage.gap,
                $( ($model, $kind) => &p.regulation.$field, )+
                _ => unreachable!("pickup adjustment does not apply to this model"),
            }
        }
    }
}
settings! {
    wurli_gap, "wurli_gap", Instrument::Piano(PianoType::Wurli), Gap, distance("OpenWurli Comb Distance", -0.08, 0.30);
    d6_centre, "d6_centre", Instrument::Clavinet(Clav::D6), Gap, distance("D6 Centre Pickup Distance", -1., 1.);
    vibanet_centre, "vibanet_centre", Instrument::Vibanet, Gap, distance("Vibanet Centre Pickup Distance", -1., 1.);
    d6_bridge, "d6_bridge", Instrument::Clavinet(Clav::D6), Bridge, distance("D6 Bridge Pickup Distance", -1., 1.);
    vibanet_bridge, "vibanet_bridge", Instrument::Vibanet, Bridge, distance("Vibanet Bridge Pickup Distance", -1., 1.);
    e7_centre, "e7_centre", Instrument::Clavinet(Clav::E7), Gap, distance("E7 Centre Pickup Distance", -1., 1.);
    e7_bridge, "e7_bridge", Instrument::Clavinet(Clav::E7), Bridge, distance("E7 Bridge Pickup Distance", -1., 1.);
    duo_centre, "duo_centre", Instrument::Duo, Gap, distance("Duo Centre Pickup Distance", -1., 1.);
    duo_bridge, "duo_bridge", Instrument::Duo, Bridge, distance("Duo Bridge Pickup Distance", -1., 1.);
    duo_reed, "duo_reed", Instrument::Duo, ReedGap, distance("Duo Reed Pickup Distance", -0.15, 1.);
    duo_return, "duo_return", Instrument::Duo, PadReturn, contribution("Duo Suction Cup Return");
    pianet_n, "pn_gap", Instrument::Reed(Reed::N), Gap, distance("Pianet N Electrode Distance", -0.15, 1.);
    pianet_c, "pc_gap", Instrument::Reed(Reed::C), Gap, distance("Pianet C Electrode Distance", -0.15, 1.);
    pianet_l, "pl_gap", Instrument::Reed(Reed::L), Gap, distance("Pianet L Electrode Distance", -0.15, 1.);
    pianet_t, "pt_gap", Instrument::Reed(Reed::T), Gap, distance("Pianet T Pickup Distance", -0.15, 1.);
    n_return, "pn_return", Instrument::Reed(Reed::N), PadReturn, contribution("Pianet N Adhesive Pad Return");
    c_return, "pc_return", Instrument::Reed(Reed::C), PadReturn, contribution("Pianet C Adhesive Pad Return");
    l_return, "pl_return", Instrument::Reed(Reed::L), PadReturn, contribution("Pianet L Adhesive Pad Return");
    t_return, "pt_return", Instrument::Reed(Reed::T), PadReturn, contribution("Pianet T Suction Cup Return");
    cembalet, "ceb_gap", Instrument::Reed(Reed::Cembalet), Gap, distance("Cembalet Electrode Distance", -0.15, 1.);
    cembalet_n, "cen_gap", Instrument::Reed(Reed::CembaletN), Gap, distance("Cembalet N Electrode Distance", -0.15, 1.);
    electra, "ele_gap", Instrument::Electra, Gap, distance("Electra Fork Pickup Distance", -0.25, 2.);
    vv64, "vv64_gap", Instrument::Vv64, Gap, distance("Vintage Vibe 64 Pickup Distance", -0.25, 2.);
    stereovibe, "stereovibe_gap", Instrument::StereoVibe, Gap, distance("Vintage Vibe StereoVibe Pickup Distance", -0.25, 2.);
    road_bass,"road_bass",Instrument::Road,Gap,distance("Roadmaster Bass Strip Distance",-1.,2.);
    road_mid,"road_mid",Instrument::Road,Bridge,distance("Roadmaster Middle Strip Distance",-1.,2.);
    road_high,"road_high",Instrument::Road,ReedGap,distance("Roadmaster Treble Strip Distance",-1.,2.);
    baldwin,"baldwin_pos",Instrument::Baldwin,Gap,distance("Baldwin Rail Pickup Position",-8.,8.);
    mk8, "mk8_gap", Instrument::Mk8, Gap, distance("Rhodes MK8 S Pickup Distance", -0.25, 2.);
    elepian, "ep_gap", Instrument::Elepian, Gap, distance("Elepian Pickup Distance", -0.15, 1.);
    valente, "val_gap", Instrument::Valente, Gap, distance("Valente Pickup Distance", -0.15, 1.);
    vivi, "vivi_gap", Instrument::Vivi, Gap, distance("Vivi-Tone Pickup Distance", -0.15, 1.);
    neo, "neo_gap", Instrument::Neo, Gap, distance("Neo-Bechstein Pickup Distance", -1., 1.);
    story, "story_gap", Instrument::Storytone, Gap, distance("Storytone Plate Pickup Distance", -0.25, 2.);
    cam, "cam_gap", Instrument::Claviset(Cam::Claviset200), Gap, distance("Claviset 200 Pickup Distance", -0.15, 1.);
    selmer, "sel_gap", Instrument::Claviset(Cam::Pianotron), Gap, distance("Pianotron Pickup Distance", -0.15, 1.);
}

impl Adjustment {
    pub(crate) fn label(self, model: Instrument) -> &'static str {
        match (model, self) {
            (Instrument::Road, Gap) => "Bass strip distance",
            (Instrument::Road, Bridge) => "Middle strip distance",
            (Instrument::Road, ReedGap) => "Treble strip distance",
            (Instrument::Baldwin, Gap) => "Rail pickup position",
            (Instrument::Clavinet(_) | Instrument::Duo | Instrument::Vibanet, Gap) => {
                "Centre pickup"
            }
            (_, Bridge) => "Bridge pickup",
            (_, ReedGap) => "Reed pickup",
            (Instrument::Reed(Reed::T) | Instrument::Duo, PadReturn) => "Suction cup return",
            (_, PadReturn) => "Adhesive pad return",
            (Instrument::Piano(PianoType::Wurli), Gap) => "Reed / comb distance",
            (Instrument::Piano(PianoType::Stage), Gap) => "Tine / pickup distance",
            (
                Instrument::Reed(Reed::N | Reed::C | Reed::L | Reed::Cembalet | Reed::CembaletN),
                Gap,
            ) => "Electrode distance",
            _ => "Pickup distance",
        }
    }
    pub(crate) fn help(self, model: Instrument) -> &'static str {
        match self {
            Gap if model==Instrument::Baldwin=>"Move the preloaded piezo elements laterally along their rail. Zero restores their stock positions.",
            PadReturn if matches!(model, Instrument::Reed(Reed::T) | Instrument::Duo) =>
                "Set the sound of the suction cup reseating at key-up. The reed still damps normally. Stock is 100%.",
            PadReturn => "Set the sound of the adhesive pad returning at key-up. The reed still damps normally. Stock is 100%.",
            Gap if model == Instrument::Piano(PianoType::Wurli) =>
                "Offset each reed’s resting clearance from its comb face. Positive opens both bass and treble gaps. Zero restores stock regulation.",
            _ => "Move this pickup closer to or farther from its vibrating element. Positive opens the gap. Zero restores each key’s stock regulation.",
        }
    }
}

/// Shared control frame for mechanical pickup adjustments. The Wurli and
/// Stage engines already interpolate these internally.
pub(super) struct PickupControls<'a> {
    gap: Option<&'a FloatParam>,
    bridge: Option<&'a FloatParam>,
    reed: Option<&'a FloatParam>,
    pad: Option<&'a FloatParam>,
    alignment: Option<&'a FloatParam>,
}

impl<'a> PickupControls<'a> {
    pub fn new(params: &'a OpenWurliUiParams, model: Instrument) -> Self {
        let applicable = if matches!(model, Instrument::Piano(_)) {
            &[][..]
        } else {
            adjustments(model)
        };
        let pick = |kind| {
            applicable
                .contains(&kind)
                .then(|| param(params, model, kind))
        };
        Self {
            gap: pick(Gap),
            bridge: pick(Bridge),
            reed: pick(ReedGap),
            pad: pick(PadReturn),
            alignment: (model == Instrument::Elepian).then_some(&params.elepian.alignment),
        }
    }

    fn fields(&self) -> [Option<&FloatParam>; 5] {
        [self.gap, self.bridge, self.reed, self.pad, self.alignment]
    }

    pub fn is_moving(&self) -> bool {
        self.fields()
            .into_iter()
            .flatten()
            .any(|p| p.smoothed.is_smoothing())
    }

    pub fn frame(&self, advance: bool) -> [f32; 5] {
        self.fields().map(|field| {
            field.map_or(0., |p| {
                if !p.smoothed.is_smoothing() {
                    p.value()
                } else if advance {
                    p.smoothed.next()
                } else {
                    p.smoothed.previous_value()
                }
            })
        })
    }

    pub fn snap(&self) {
        for p in self.fields().into_iter().flatten() {
            p.smoothed.reset(p.value());
        }
    }
}
