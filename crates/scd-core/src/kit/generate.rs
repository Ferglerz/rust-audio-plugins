use super::{ArticulationDef, KitDef};
use crate::{KitPieceId, MicChannel, SampleSlice, StrikeEntry};

/// Pack-time strike: archived `StrikeEntry` plus relative WAV paths (never serialized).
#[derive(Debug, Clone)]
pub struct StrikePaths {
    pub strike: StrikeEntry,
    pub mic_paths: Vec<(MicChannel, String)>,
}

impl StrikePaths {
    pub fn into_entry(self, mic_slices: [Option<SampleSlice>; MicChannel::COUNT]) -> StrikeEntry {
        self.strike.with_mic_slices(mic_slices)
    }
}

/// Mirrors `getRRGroupSize` in the HISE build scripts.
pub fn rr_group_size(sample_count: u32) -> u8 {
    if sample_count > 35 {
        6
    } else if sample_count > 24 {
        5
    } else if sample_count > 15 {
        4
    } else if sample_count > 11 {
        3
    } else if sample_count > 7 {
        2
    } else {
        1
    }
}

/// Mirrors `getVelocityRange` in the HISE build scripts.
pub fn velocity_range(layer_index: u32, total_layers: u32) -> (u8, u8) {
    let lo_vel = 1 + (126 * layer_index / total_layers) as u8;
    let hi_vel = (127 * (layer_index + 1) / total_layers) as u8;
    (lo_vel, hi_vel)
}

/// Mirrors `calculateMaxRRGroups` — shared RR depth for an entire kit piece.
pub fn calculate_max_rr_groups(articulations: &[ArticulationDef]) -> u8 {
    articulations
        .iter()
        .map(|a| rr_group_size(a.layers))
        .max()
        .unwrap_or(1)
}

/// Build the canonical WAV filename: `{kitpiece}-{articulation}-{mic}-{n}.wav`
pub fn sample_wav_name(
    kit_piece: KitPieceId,
    articulation: &str,
    mic: MicChannel,
    sample_num: u32,
) -> String {
    format!(
        "{}-{}-{}-{}.wav",
        kit_piece.name(),
        articulation,
        mic.name(),
        sample_num
    )
}

/// Strike count for one kit piece without allocating path strings.
pub fn count_kit_piece_strikes(articulations: &[ArticulationDef]) -> usize {
    let max_rr_groups = calculate_max_rr_groups(articulations);
    let mut capacity = 0usize;
    for art in articulations {
        let velocity_layers = art.layers.div_ceil(max_rr_groups as u32);
        let note_count = if art.note2.is_some() { 2 } else { 1 };
        capacity += (max_rr_groups as usize) * (velocity_layers as usize) * note_count;
    }
    capacity
}

/// Strike count for a full kit without allocating path strings.
pub fn count_kit_strikes(kit: &KitDef) -> usize {
    kit.order
        .iter()
        .map(|id| {
            kit.pieces
                .get(id)
                .map(|arts| count_kit_piece_strikes(arts))
                .unwrap_or(0)
        })
        .sum()
}

/// Generate all pack-time strikes for one kit piece (single-sampler layout).
pub fn generate_kit_piece_strikes(
    kit_piece: KitPieceId,
    articulations: &[ArticulationDef],
) -> Vec<StrikePaths> {
    let max_rr_groups = calculate_max_rr_groups(articulations);
    let mut strikes = Vec::with_capacity(count_kit_piece_strikes(articulations));
    let mut path_buf = String::with_capacity(96);

    for art in articulations {
        generate_articulation_strikes(kit_piece, art, max_rr_groups, &mut strikes, &mut path_buf);
    }

    strikes
}

fn generate_articulation_strikes(
    kit_piece: KitPieceId,
    art: &ArticulationDef,
    max_rr_groups: u8,
    out: &mut Vec<StrikePaths>,
    path_buf: &mut String,
) {
    let source_layers = art.layers;
    let velocity_layers = source_layers.div_ceil(max_rr_groups as u32);
    let final_sample_count = max_rr_groups as u32 * velocity_layers;
    let remainder = final_sample_count % source_layers;

    let mut sample_count = 0u32;

    for vel_layer in 0..velocity_layers {
        let (lo_vel, hi_vel) = velocity_range(vel_layer, velocity_layers);

        for rr in 1..=max_rr_groups {
            sample_count += 1;
            let sample_num = if sample_count + remainder > source_layers {
                sample_count - remainder
            } else {
                sample_count
            };

            let mic_paths = mic_paths_for_sample(kit_piece, &art.name, sample_num, path_buf);
            let mut push = |midi_note, paths| {
                out.push(StrikePaths {
                    strike: StrikeEntry::new(
                        midi_note,
                        lo_vel,
                        hi_vel,
                        rr,
                        art.rem_vol,
                        kit_piece,
                        [None; MicChannel::COUNT],
                    ),
                    mic_paths: paths,
                });
            };
            if let Some(note2) = art.note2 {
                push(art.note1, mic_paths.clone());
                push(note2, mic_paths);
            } else {
                push(art.note1, mic_paths);
            }
        }
    }
}

fn mic_paths_for_sample(
    kit_piece: KitPieceId,
    articulation: &str,
    sample_num: u32,
    path_buf: &mut String,
) -> Vec<(MicChannel, String)> {
    let mut paths = Vec::with_capacity(6);
    for mic in MicChannel::ALL {
        if kit_piece.omits_mic(mic) {
            continue;
        }
        *path_buf = sample_wav_name(kit_piece, articulation, mic, sample_num);
        paths.push((mic, path_buf.clone()));
    }
    paths
}
