use crate::KitPieceId;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Upper bound for articulations on one kit piece (Stonehouse hi-hat is 11).
pub const MAX_ARTICULATIONS: usize = 16;

#[derive(Debug, Clone, Deserialize)]
pub struct ArticulationDef {
    pub name: String,
    pub layers: u32,
    pub rem_vol: i8,
    pub note1: u8,
    pub note2: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct KitDef {
    pub order: Vec<KitPieceId>,
    pub pieces: HashMap<KitPieceId, Vec<ArticulationDef>>,
}

#[derive(Deserialize)]
struct KitDataFile {
    order: Vec<String>,
    kit_pieces: HashMap<String, KitPieceFile>,
}

#[derive(Deserialize)]
struct KitPieceFile {
    articulations: Vec<ArticulationDef>,
}

/// Factory Stonehouse kit — ported from `SCD/Scripts/BUILD_DATA.js`.
pub fn load_stonehouse() -> KitDef {
    load_kit(include_str!("../../kit_data/stonehouse.json"))
}

pub fn stonehouse() -> &'static KitDef {
    static KIT: OnceLock<KitDef> = OnceLock::new();
    KIT.get_or_init(load_stonehouse)
}

impl KitDef {
    pub fn arts(&self, kit_piece: KitPieceId) -> &[ArticulationDef] {
        self.pieces
            .get(&kit_piece)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

fn load_kit(json: &str) -> KitDef {
    let file: KitDataFile = serde_json::from_str(json).expect("invalid kit JSON");
    let order = file
        .order
        .iter()
        .map(|name| KitPieceId::from_name(name).expect("unknown kit piece in order"))
        .collect();

    let pieces = file
        .kit_pieces
        .into_iter()
        .map(|(name, piece)| {
            let id = KitPieceId::from_name(&name).expect("unknown kit piece in kit_pieces");
            (id, piece.articulations)
        })
        .collect();

    KitDef { order, pieces }
}
