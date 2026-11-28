//! Keep the original three-position host parameter and its automation thresholds.
use crate::params::{OpenWurliUiParams, PianoType};
use nih_plug::prelude::*;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ModelBank {
    #[id = "original"]
    #[name = "Piano selector"]
    Piano,
    #[id = "d6"]
    #[name = "Clavinet D6"]
    D6,
    #[id = "pianet_n"]
    #[name = "Pianet N"]
    PianetN,
    #[id = "pianet_t"]
    #[name = "Pianet T"]
    PianetT,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClavinetModel {
    #[id = "d6"]
    #[name = "D6"]
    D6,
    #[id = "e7"]
    #[name = "E7"]
    E7,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReedModel {
    #[id = "pianet_n"]
    #[name = "Pianet N"]
    PianetN,
    #[id = "cembalet"]
    #[name = "Cembalet"]
    Cembalet,
    #[id = "cembalet_n"]
    #[name = "Cembalet N"]
    CembaletN,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PianetRevision {
    #[id = "n"]
    #[name = "Pianet N"]
    N,
    #[id = "c"]
    #[name = "Pianet C"]
    C,
    #[id = "l"]
    #[name = "Pianet L"]
    L,
}

use pianet_n::{claviset::Instrument as Claviset, Model as Reed};

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClavisetModel {
    #[id = "off"]
    #[name = "Piano selector"]
    Off,
    #[id = "claviset_200"]
    #[name = "Claviset 200"]
    Claviset200,
    #[id = "pianotron"]
    #[name = "Selmer Pianotron"]
    Pianotron,
}
impl From<Instrument> for ClavisetModel {
    fn from(value: Instrument) -> Self {
        match value {
            Instrument::Claviset(Claviset::Claviset200) => Self::Claviset200,
            Instrument::Claviset(Claviset::Pianotron) => Self::Pianotron,
            _ => Self::Off,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum Instrument {
    Piano(PianoType),
    Clavinet(ClavinetModel),
    Reed(Reed),
    Claviset(Claviset),
    Electra,
    Duo,
    Storytone,
    Neo,
    Elepian,
    Vv64,
    StereoVibe,
    Road,
    Baldwin,
    Mk8,
    Vibanet,
    Valente,
    Vivi,
}

impl Instrument {
    pub const HOHNER: [Self; 10] = [
        Self::Clavinet(ClavinetModel::D6),
        Self::Clavinet(ClavinetModel::E7),
        Self::Duo,
        Self::Reed(Reed::N),
        Self::Reed(Reed::C),
        Self::Reed(Reed::L),
        Self::Reed(Reed::T),
        Self::Reed(Reed::Cembalet),
        Self::Reed(Reed::CembaletN),
        Self::Electra,
    ];
    pub const HELPINSTILL: [Self; 1] = [Self::Road];
    pub const BALDWIN: [Self; 1] = [Self::Baldwin];
    pub const RHODES: [Self; 1] = [Self::Mk8];
    pub const VINTAGE_VIBE: [Self; 3] = [Self::Vv64, Self::StereoVibe, Self::Vibanet];
    pub const VALENTE: [Self; 1] = [Self::Valente];
    pub const COLUMBIA: [Self; 1] = [Self::Elepian];
    pub const HISTORIC_GRANDS: [Self; 1] = [Self::Neo];
    pub const UPRIGHTS: [Self; 2] = [Self::Storytone, Self::Vivi];
    pub const CLAVISET: [Self; 2] = [
        Self::Claviset(Claviset::Claviset200),
        Self::Claviset(Claviset::Pianotron),
    ];
    pub const GROUPS: &'static [(&'static str, &'static [Self])] = &[
        ("Hohner", &Self::HOHNER),
        ("Claviset family", &Self::CLAVISET),
        ("Electric Uprights", &Self::UPRIGHTS),
        ("Historical Grands", &Self::HISTORIC_GRANDS),
        ("Columbia", &Self::COLUMBIA),
        ("Helpinstill", &Self::HELPINSTILL),
        ("Baldwin", &Self::BALDWIN),
        ("Rhodes", &Self::RHODES),
        ("Vintage Vibe", &Self::VINTAGE_VIBE),
        ("Valente", &Self::VALENTE),
    ];
    pub const VIEW_COUNT: usize = 3
        + Self::HOHNER.len()
        + Self::UPRIGHTS.len()
        + Self::CLAVISET.len()
        + Self::HISTORIC_GRANDS.len()
        + Self::COLUMBIA.len()
        + Self::HELPINSTILL.len()
        + Self::BALDWIN.len()
        + Self::RHODES.len()
        + Self::VINTAGE_VIBE.len()
        + Self::VALENTE.len();

    pub fn extra_models() -> impl Iterator<Item = Self> {
        Self::GROUPS
            .iter()
            .flat_map(|(_, models)| models.iter().copied())
    }

    pub fn index(self) -> usize {
        match self {
            Self::Piano(p) => p.to_index(),
            Self::Clavinet(m) => 3 + m.to_index(),
            Self::Reed(m) => 5 + m as usize,
            Self::Electra => 5 + Reed::ALL.len(),
            Self::Duo => 6 + Reed::ALL.len(),
            Self::Storytone => 7 + Reed::ALL.len(),
            Self::Claviset(m) => 8 + Reed::ALL.len() + m as usize,
            Self::Neo => 8 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Elepian => 9 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Vv64 => 10 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::StereoVibe => 17 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Road => 15 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Baldwin => 16 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Mk8 => 14 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Vibanet => 12 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Valente => 11 + Reed::ALL.len() + Self::CLAVISET.len(),
            Self::Vivi => 13 + Reed::ALL.len() + Self::CLAVISET.len(),
        }
    }
    pub fn bank(self) -> ModelBank {
        match self {
            Self::Piano(_)
            | Self::Electra
            | Self::Storytone
            | Self::Neo
            | Self::Vv64
            | Self::StereoVibe
            | Self::Road
            | Self::Baldwin
            | Self::Mk8
            | Self::Vibanet
            | Self::Valente
            | Self::Vivi
            | Self::Elepian
            | Self::Claviset(_) => ModelBank::Piano,
            Self::Clavinet(_) | Self::Duo => ModelBank::D6,
            Self::Reed(Reed::T) => ModelBank::PianetT,
            Self::Reed(_) => ModelBank::PianetN,
        }
    }
    pub fn reed_selection(self) -> Option<ReedModel> {
        match self {
            Self::Reed(Reed::N | Reed::C | Reed::L) => Some(ReedModel::PianetN),
            Self::Reed(Reed::Cembalet) => Some(ReedModel::Cembalet),
            Self::Reed(Reed::CembaletN) => Some(ReedModel::CembaletN),
            _ => None,
        }
    }
    pub fn pianet_revision(self) -> Option<PianetRevision> {
        match self {
            Self::Reed(Reed::N) => Some(PianetRevision::N),
            Self::Reed(Reed::C) => Some(PianetRevision::C),
            Self::Reed(Reed::L) => Some(PianetRevision::L),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Piano(p) => PianoType::variants()[p.to_index()],
            Self::Clavinet(ClavinetModel::D6) => "Clavinet D6",
            Self::Clavinet(ClavinetModel::E7) => "Clavinet E7",
            Self::Reed(Reed::N) => "Pianet N",
            Self::Reed(Reed::T) => "Pianet T",
            Self::Reed(Reed::C) => "Pianet C",
            Self::Reed(Reed::L) => "Pianet L",
            Self::Reed(Reed::Cembalet) => "Cembalet",
            Self::Reed(Reed::CembaletN) => "Cembalet N",
            Self::Electra => "Electra-Piano",
            Self::Duo => "Pianet/Clavinet Duo",
            Self::Storytone => "Storytone",
            Self::Neo => "Neo-Bechstein",
            Self::Elepian => "Elepian EP-61C",
            Self::Vv64 => "Vintage Vibe 64",
            Self::StereoVibe => "Vintage Vibe StereoVibe",
            Self::Road => "Roadmaster 64",
            Self::Baldwin => "Baldwin Electro-Piano",
            Self::Mk8 => "Rhodes MK8",
            Self::Vibanet => "Vibanet",
            Self::Valente => "Valente Electric Piano",
            Self::Vivi => "Vivi-Tone Clavier",
            Self::Claviset(Claviset::Claviset200) => "Claviset 200",
            Self::Claviset(Claviset::Pianotron) => "Selmer Pianotron",
        }
    }
}

impl From<PianoType> for Instrument {
    fn from(p: PianoType) -> Self {
        Self::Piano(p)
    }
}

impl OpenWurliUiParams {
    pub(crate) fn instrument(&self) -> Instrument {
        match self.model_bank.value() {
            ModelBank::Piano if self.claviset_model.value() == ClavisetModel::Claviset200 => {
                Instrument::Claviset(Claviset::Claviset200)
            }
            ModelBank::Piano if self.claviset_model.value() == ClavisetModel::Pianotron => {
                Instrument::Claviset(Claviset::Pianotron)
            }
            ModelBank::Piano if self.vv64_enabled.value() => Instrument::Vv64,
            ModelBank::Piano if self.stereovibe_enabled.value() => Instrument::StereoVibe,
            ModelBank::Piano if self.road_enabled.value() => Instrument::Road,
            ModelBank::Piano if self.baldwin_enabled.value() => Instrument::Baldwin,
            ModelBank::Piano if self.mk8_enabled.value() => Instrument::Mk8,
            ModelBank::Piano if self.vibanet_enabled.value() => Instrument::Vibanet,
            ModelBank::Piano if self.valente_enabled.value() => Instrument::Valente,
            ModelBank::Piano if self.vivi_enabled.value() => Instrument::Vivi,
            ModelBank::Piano if self.elepian_enabled.value() => Instrument::Elepian,
            ModelBank::Piano if self.neo_enabled.value() => Instrument::Neo,
            ModelBank::Piano if self.storytone_enabled.value() => Instrument::Storytone,
            ModelBank::Piano if self.electra_enabled.value() => Instrument::Electra,
            ModelBank::Piano => Instrument::Piano(self.piano.value()),
            ModelBank::D6 if self.duo_enabled.value() => Instrument::Duo,
            ModelBank::D6 => Instrument::Clavinet(self.clavinet_model.value()),
            ModelBank::PianetT => Instrument::Reed(Reed::T),
            ModelBank::PianetN => Instrument::Reed(match self.reed_model.value() {
                ReedModel::PianetN => match self.pianet_revision.value() {
                    PianetRevision::C => Reed::C,
                    PianetRevision::L => Reed::L,
                    PianetRevision::N => Reed::N,
                },
                ReedModel::Cembalet => Reed::Cembalet,
                ReedModel::CembaletN => Reed::CembaletN,
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn select_model(&mut self, model: Instrument) {
        self.model_bank = EnumParam::new("Instrument Family", model.bank());
        self.electra_enabled = BoolParam::new("Electra-Piano", model == Instrument::Electra);
        self.claviset_model = EnumParam::new("Claviset Family", model.into());
        self.vv64_enabled = BoolParam::new("Vintage Vibe 64", model == Instrument::Vv64);
        self.stereovibe_enabled =
            BoolParam::new("Vintage Vibe StereoVibe", model == Instrument::StereoVibe);
        self.road_enabled = BoolParam::new("Roadmaster 64", model == Instrument::Road);
        self.baldwin_enabled =
            BoolParam::new("Baldwin Electro-Piano", model == Instrument::Baldwin);
        self.mk8_enabled = BoolParam::new("Rhodes MK8 S", model == Instrument::Mk8);
        self.vibanet_enabled = BoolParam::new("Vibanet", model == Instrument::Vibanet);
        self.valente_enabled =
            BoolParam::new("Valente Electric Piano", model == Instrument::Valente);
        self.vivi_enabled = BoolParam::new("Vivi-Tone Clavier", model == Instrument::Vivi);
        self.elepian_enabled = BoolParam::new("Elepian EP-61C", model == Instrument::Elepian);
        self.neo_enabled = BoolParam::new("Neo-Bechstein", model == Instrument::Neo);
        self.storytone_enabled = BoolParam::new("Storytone", model == Instrument::Storytone);
        self.duo_enabled = BoolParam::new("Pianet/Clavinet Duo", model == Instrument::Duo);
        if let Instrument::Clavinet(m) = model {
            self.clavinet_model = EnumParam::new("Clavinet Model", m);
        }
        if let Some(revision) = model.pianet_revision() {
            self.pianet_revision = EnumParam::new("Pianet Revision", revision);
        }
        if let Some(m) = model.reed_selection() {
            self.reed_model = EnumParam::new("Electrostatic Reed Model", m);
        }
    }
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pickup {
    #[id = "ca"]
    #[name = "CA · Centre"]
    Centre,
    #[id = "cb"]
    #[name = "CB · Bridge"]
    Bridge,
    #[id = "da"]
    #[name = "DA · Both"]
    Da,
    #[id = "db"]
    #[name = "DB · Both"]
    Db,
}
impl Pickup {
    pub const ALL: [Self; 4] = [Self::Centre, Self::Bridge, Self::Da, Self::Db];
    pub fn selector(self) -> clavinet_d6::Switch {
        use clavinet_d6::Switch;
        match self {
            Self::Centre => Switch::CA,
            Self::Bridge => Switch::CB,
            Self::Da => Switch::DA,
            Self::Db => Switch::DB,
        }
    }
}

#[derive(Params)]
pub struct D6Tone {
    #[id = "pickup"]
    pub pickup: EnumParam<Pickup>,
    #[id = "soft"]
    pub soft: BoolParam,
    #[id = "medium"]
    pub medium: BoolParam,
    #[id = "treble"]
    pub treble: BoolParam,
    #[id = "brilliant"]
    pub brilliant: BoolParam,
    #[id = "invert_da"]
    pub invert_da: BoolParam,
    #[id = "mute"]
    pub mute: BoolParam,
    #[id = "volume"]
    pub volume: FloatParam,
}
impl Default for D6Tone {
    fn default() -> Self {
        Self {
            pickup: EnumParam::new("D6 Pickup", Pickup::Centre),
            soft: BoolParam::new("Soft", true),
            medium: BoolParam::new("Medium", true),
            treble: BoolParam::new("Treble", true),
            brilliant: BoolParam::new("Brilliant", true),
            invert_da: BoolParam::new("Reverse Paired Pickups", false),
            mute: BoolParam::new("String Mute", false),
            volume: FloatParam::new(
                "D6 Panel Volume",
                1.,
                FloatRange::Linear { min: 0., max: 1. },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}
impl D6Tone {
    pub fn tabs(&self) -> [&BoolParam; 4] {
        [&self.soft, &self.medium, &self.treble, &self.brilliant]
    }
    pub fn mask(&self) -> u8 {
        self.tabs()
            .iter()
            .enumerate()
            .fold(0, |mask, (i, b)| mask | (u8::from(b.value()) << i))
    }
}
