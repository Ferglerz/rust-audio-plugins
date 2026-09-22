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

impl ArticulationDef {
    pub fn short_name(&self) -> String {
        let raw = self.name.replace('_', " ");
        let stripped = raw
            .strip_prefix("HH ")
            .or_else(|| raw.strip_prefix("NW Snare "))
            .or_else(|| raw.strip_prefix("Snare "))
            .or_else(|| raw.strip_prefix("Crash L "))
            .or_else(|| raw.strip_prefix("Crash R "))
            .or_else(|| raw.strip_prefix("Ride "))
            .or_else(|| raw.strip_prefix("Tom 1 "))
            .or_else(|| raw.strip_prefix("Tom 2 "))
            .or_else(|| raw.strip_prefix("Tom 3 "))
            .or_else(|| raw.strip_prefix("Kick "))
            .unwrap_or(&raw);
        stripped.to_string()
    }
}

/// Consecutive short names that share a leading word become one switch group.
/// Header holds the shared prefix. Segment labels keep the remainder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtSwitchGroup {
    pub header: Option<String>,
    pub start: usize,
    pub labels: Vec<String>,
}

impl ArtSwitchGroup {
    pub fn count(&self) -> usize {
        self.labels.len()
    }
}

pub fn art_switch_groups(names: &[String]) -> Vec<ArtSwitchGroup> {
    let words: Vec<Vec<&str>> = names
        .iter()
        .map(|name| name.split_whitespace().collect())
        .collect();
    let mut groups = Vec::new();
    let mut i = 0;
    while i < names.len() {
        let mut j = i + 1;
        if let Some(first) = words[i].first() {
            while j < names.len() && words[j].first() == Some(first) {
                j += 1;
            }
        }
        if j - i >= 2 {
            let mut prefix = words[i].len();
            for row in words.iter().take(j).skip(i) {
                prefix = prefix.min(common_word_prefix(&words[i], row));
            }
            while prefix > 0 && words[i..j].iter().any(|row| row.len() <= prefix) {
                prefix -= 1;
            }
            if prefix > 0 {
                groups.push(ArtSwitchGroup {
                    header: Some(words[i][..prefix].join(" ")),
                    start: i,
                    labels: words[i..j]
                        .iter()
                        .map(|row| row[prefix..].join(" "))
                        .collect(),
                });
                i = j;
                continue;
            }
        }
        groups.push(ArtSwitchGroup {
            header: None,
            start: i,
            labels: vec![names[i].clone()],
        });
        i += 1;
    }
    groups
}

fn common_word_prefix(a: &[&str], b: &[&str]) -> usize {
    a.iter().zip(b.iter()).take_while(|(l, r)| l == r).count()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KitPieceId;

    fn names(kit_piece: KitPieceId) -> Vec<String> {
        stonehouse()
            .arts(kit_piece)
            .iter()
            .map(ArticulationDef::short_name)
            .collect()
    }

    fn group(header: Option<&str>, start: usize, labels: &[&str]) -> ArtSwitchGroup {
        ArtSwitchGroup {
            header: header.map(str::to_string),
            start,
            labels: labels.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn hihat_groups_shoulder_and_tip() {
        assert_eq!(
            art_switch_groups(&names(KitPieceId::Hihat)),
            vec![
                group(None, 0, &["Splash"]),
                group(None, 1, &["Stomp"]),
                group(Some("Shoulder"), 2, &["Firm", "A", "B", "C", "Open"]),
                group(Some("Tip"), 7, &["Firm", "A", "B", "C", "Open"]),
            ]
        );
    }

    #[test]
    fn open_snare_lifts_center_and_edge_ruffs() {
        assert_eq!(
            art_switch_groups(&names(KitPieceId::OpenSnare)),
            vec![
                group(None, 0, &["Rim"]),
                group(Some("Center"), 1, &["Ruff Slower", "Ruff"]),
                group(Some("Edge"), 3, &["Ruff Slower", "Ruff"]),
                group(None, 5, &["Center"]),
                group(None, 6, &["Edge"]),
                group(None, 7, &["Rimshot"]),
                group(None, 8, &["XStick"]),
            ]
        );
    }

    #[test]
    fn crash_l_groups_bell_pair() {
        assert_eq!(
            art_switch_groups(&names(KitPieceId::LCrash)),
            vec![
                group(Some("Bell"), 0, &["Shoulder", "Tip"]),
                group(None, 2, &["Shoulder"]),
                group(None, 3, &["Tip"]),
                group(None, 4, &["Choke"]),
            ]
        );
    }

    #[test]
    fn toms_stay_ungrouped() {
        assert_eq!(
            art_switch_groups(&names(KitPieceId::Tom1)),
            vec![
                group(None, 0, &["Center"]),
                group(None, 1, &["Rim"]),
            ]
        );
    }
}
