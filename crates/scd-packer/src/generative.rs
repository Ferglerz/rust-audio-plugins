use crate::pack::AudioPacker;
use scd_core::kit::{generate_kit_piece_strikes, KitDef, StrikePaths};
use scd_core::{MicChannel, StrikeEntry};
use std::path::Path;

pub fn pack_from_kit(
    kit: &KitDef,
    samples_dir: &Path,
    packer: &mut AudioPacker,
    strict: bool,
) -> Result<Vec<StrikeEntry>, Box<dyn std::error::Error>> {
    let mut strikes = Vec::new();
    let mut missing = Vec::new();

    for kit_piece in &kit.order {
        let articulations = kit
            .pieces
            .get(kit_piece)
            .ok_or_else(|| format!("kit definition missing piece: {}", kit_piece.name()))?;

        let templates = generate_kit_piece_strikes(*kit_piece, articulations);

        for template in templates {
            let entry = resolve_strike(template, samples_dir, packer, &mut missing)?;
            strikes.push(entry);
        }
    }

    if strict && !missing.is_empty() {
        missing.sort();
        missing.dedup();
        let preview = missing
            .iter()
            .take(10)
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ");
        let suffix = if missing.len() > 10 {
            format!("\n  ... and {} more", missing.len() - 10)
        } else {
            String::new()
        };
        return Err(format!(
            "missing {} WAV file(s) under {:?}:\n  {}{}",
            missing.len(),
            samples_dir,
            preview,
            suffix
        )
        .into());
    }

    if !missing.is_empty() {
        eprintln!(
            "Warning: {} missing WAV file(s) (strikes packed with partial mic coverage)",
            missing.len()
        );
    }

    Ok(strikes)
}

fn resolve_strike(
    template: StrikePaths,
    samples_dir: &Path,
    packer: &mut AudioPacker,
    missing: &mut Vec<std::path::PathBuf>,
) -> Result<StrikeEntry, Box<dyn std::error::Error>> {
    let mut mic_slices = [None; MicChannel::COUNT];

    for (mic, rel_path) in &template.mic_paths {
        let wav_path = samples_dir.join(rel_path);
        if wav_path.exists() {
            let slice = packer.pack_wav(&wav_path)?;
            mic_slices[*mic as usize] = Some(slice);
        } else {
            missing.push(wav_path);
        }
    }

    Ok(template.into_entry(mic_slices))
}
