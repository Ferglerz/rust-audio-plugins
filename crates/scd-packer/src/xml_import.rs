use crate::pack::AudioPacker;
use scd_core::*;
use std::path::Path;

pub fn pack_from_xml(
    samplemaps_dir: &Path,
    samples_dir: &Path,
    packer: &mut AudioPacker,
) -> Result<Vec<StrikeEntry>, Box<dyn std::error::Error>> {
    let mut strikes = Vec::new();

    for entry in std::fs::read_dir(samplemaps_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("xml") {
            continue;
        }

        let kitpiece_name = path.file_stem().unwrap().to_str().unwrap();
        let kit_piece = KitPieceId::from_name(kitpiece_name)
            .ok_or_else(|| format!("unknown samplemap kit piece: {kitpiece_name}"))?;

        parse_samplemap(&path, samples_dir, kit_piece, &mut strikes, packer)?;
    }

    Ok(strikes)
}

fn parse_samplemap(
    xml_path: &Path,
    samples_dir: &Path,
    kit_piece: KitPieceId,
    strikes: &mut Vec<StrikeEntry>,
    packer: &mut AudioPacker,
) -> Result<(), Box<dyn std::error::Error>> {
    let xml_content = std::fs::read_to_string(xml_path)?;
    let mut reader = quick_xml::Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut current_strike: Option<StrikeEntry> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            quick_xml::events::Event::Start(ref e) if e.name().as_ref() == b"sample" => {
                let mut note = 0u8;
                let mut lo_vel = 1u8;
                let mut hi_vel = 127u8;
                let mut rr = 1u8;
                let mut vol = 0i8;

                for attr in e.attributes() {
                    let attr = attr?;
                    match attr.key.as_ref() {
                        b"Root" => note = std::str::from_utf8(&attr.value)?.parse().unwrap_or(0),
                        b"LoVel" => {
                            lo_vel = std::str::from_utf8(&attr.value)?
                                .parse::<f32>()
                                .unwrap_or(1.0) as u8;
                        }
                        b"HiVel" => {
                            hi_vel = std::str::from_utf8(&attr.value)?
                                .parse::<f32>()
                                .unwrap_or(127.0) as u8;
                        }
                        b"RRGroup" => rr = std::str::from_utf8(&attr.value)?.parse().unwrap_or(1),
                        b"Volume" => vol = std::str::from_utf8(&attr.value)?.parse().unwrap_or(0),
                        _ => {}
                    }
                }

                current_strike = Some(StrikeEntry::new(
                    note,
                    lo_vel,
                    hi_vel,
                    rr,
                    vol,
                    kit_piece,
                    [None; MicChannel::COUNT],
                ));
            }
            quick_xml::events::Event::Empty(ref e) if e.name().as_ref() == b"file" => {
                if let Some(ref mut strike) = current_strike {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"FileName" {
                            let raw_name = std::str::from_utf8(&attr.value)?;
                            let filename = raw_name.replace("{PROJECT_FOLDER}", "");
                            let wav_path = samples_dir.join(&filename);

                            if let Some(mic) = MicChannel::from_filename(&filename) {
                                if wav_path.exists() {
                                    let slice = packer.pack_wav(&wav_path)?;
                                    strike.mic_slices[mic as usize] = Some(slice);
                                }
                            }
                        }
                    }
                }
            }
            quick_xml::events::Event::End(ref e) if e.name().as_ref() == b"sample" => {
                if let Some(strike) = current_strike.take() {
                    strikes.push(strike);
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}
