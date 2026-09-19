use hound::WavReader;
use scd_core::*;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: scd-packer <samplemaps_dir> <raw_samples_dir> <output.scdpack>");
        std::process::exit(1);
    }

    let samplemaps_dir = Path::new(&args[1]);
    let raw_samples_dir = Path::new(&args[2]);
    let output_path = Path::new(&args[3]);

    println!("Packer: scanning samplemaps in {:?}", samplemaps_dir);
    println!("Packer: reading audio from {:?}", raw_samples_dir);
    println!("Packer: output will be {:?}", output_path);

    let temp_audio_path = output_path.with_extension("audio.tmp");
    let mut temp_audio = BufWriter::new(File::create(&temp_audio_path)?);

    let mut strikes: Vec<StrikeEntry> = Vec::new();
    let mut audio_offset_bytes: u64 = 0;

    let samplemap_files = std::fs::read_dir(samplemaps_dir)?;
    for entry in samplemap_files {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("xml") {
            let kitpiece_name = path.file_stem().unwrap().to_str().unwrap();
            let kit_piece = match kitpiece_name {
                "Kick" => KitPieceId::Kick,
                "Snare" => KitPieceId::Snare,
                "Open Snare" => KitPieceId::OpenSnare,
                "Hihat" => KitPieceId::Hihat,
                "Tom 1" => KitPieceId::Tom1,
                "Tom 2" => KitPieceId::Tom2,
                "Floor Tom" => KitPieceId::FloorTom,
                "Ride" => KitPieceId::Ride,
                "China" => KitPieceId::China,
                "Stack" => KitPieceId::Stack,
                "Splash" => KitPieceId::Splash,
                "L Crash" => KitPieceId::LCrash,
                "R Crash" => KitPieceId::RCrash,
                _ => continue,
            };

            parse_and_pack_samplemap(
                &path,
                raw_samples_dir,
                kit_piece,
                &mut strikes,
                &mut temp_audio,
                &mut audio_offset_bytes,
            )?;
        }
    }

    temp_audio.flush()?;
    drop(temp_audio);

    println!("Packer: strikes indexed: {}", strikes.len());
    println!("Packer: total audio bytes: {}", audio_offset_bytes);

    let index = PackIndex { strikes };
    let index_bytes = rkyv::to_bytes::<_, 256>(&index)?;
    let index_len = index_bytes.len() as u64;

    println!("Packer: writing final .scdpack with header & index ({} bytes)...", index_len);
    let mut final_file = BufWriter::new(File::create(output_path)?);
    final_file.write_all(SCD_MAGIC)?;
    final_file.write_all(&index_len.to_le_bytes())?;
    final_file.write_all(&index_bytes)?;

    // Stream temp audio into final file
    let mut temp_audio_reader = BufReader::new(File::open(&temp_audio_path)?);
    let mut copy_buf = [0u8; 64 * 1024];
    loop {
        let n = temp_audio_reader.read(&mut copy_buf)?;
        if n == 0 {
            break;
        }
        final_file.write_all(&copy_buf[..n])?;
    }

    final_file.flush()?;
    drop(final_file);
    let _ = std::fs::remove_file(temp_audio_path);

    println!("Packer complete! Output written to {:?}", output_path);
    Ok(())
}

fn parse_and_pack_samplemap(
    xml_path: &Path,
    samples_dir: &Path,
    kit_piece: KitPieceId,
    strikes: &mut Vec<StrikeEntry>,
    out_file: &mut BufWriter<File>,
    audio_offset_bytes: &mut u64,
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
                        b"LoVel" => lo_vel = std::str::from_utf8(&attr.value)?.parse::<f32>().unwrap_or(1.0) as u8,
                        b"HiVel" => hi_vel = std::str::from_utf8(&attr.value)?.parse::<f32>().unwrap_or(127.0) as u8,
                        b"RRGroup" => rr = std::str::from_utf8(&attr.value)?.parse().unwrap_or(1),
                        b"Volume" => vol = std::str::from_utf8(&attr.value)?.parse().unwrap_or(0),
                        _ => {}
                    }
                }

                current_strike = Some(StrikeEntry {
                    midi_note: note,
                    lo_vel,
                    hi_vel,
                    rr_group: rr,
                    volume_db: vol,
                    kit_piece,
                    mic_slices: [None, None, None, None, None, None],
                });
            }
            quick_xml::events::Event::Empty(ref e) if e.name().as_ref() == b"file" => {
                if let Some(ref mut strike) = current_strike {
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"FileName" {
                            let raw_name = std::str::from_utf8(&attr.value)?;
                            let filename = raw_name.replace("{PROJECT_FOLDER}", "");
                            let wav_path = samples_dir.join(&filename);

                            if let Some(mic_idx) = detect_mic_channel(&filename) {
                                if wav_path.exists() {
                                    if let Ok(slice) = pack_wav(&wav_path, out_file, audio_offset_bytes) {
                                        strike.mic_slices[mic_idx as usize] = Some(slice);
                                    }
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

fn detect_mic_channel(filename: &str) -> Option<MicChannel> {
    if filename.contains("Close") {
        Some(MicChannel::Close)
    } else if filename.contains("XY") {
        Some(MicChannel::XY)
    } else if filename.contains("Mono") {
        Some(MicChannel::Mono)
    } else if filename.contains("Wide") {
        Some(MicChannel::Wide)
    } else if filename.contains("Front MS") {
        Some(MicChannel::FrontMS)
    } else if filename.contains("Room") {
        Some(MicChannel::Room)
    } else {
        None
    }
}

fn pack_wav(
    path: &Path,
    out_file: &mut BufWriter<File>,
    audio_offset_bytes: &mut u64,
) -> Result<SampleSlice, Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    let start_offset = *audio_offset_bytes;
    let mut frame_count = 0u32;

    match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            for sample in reader.samples::<i32>() {
                let s = sample? as f32 / max_val;
                out_file.write_all(&s.to_le_bytes())?;
                frame_count += 1;
            }
        }
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let s = sample?;
                out_file.write_all(&s.to_le_bytes())?;
                frame_count += 1;
            }
        }
    }

    let actual_frames = frame_count / (spec.channels as u32);
    let bytes_written = (frame_count as u64) * 4;
    *audio_offset_bytes += bytes_written;

    Ok(SampleSlice {
        offset_bytes: start_offset,
        length_frames: actual_frames,
        sample_rate: spec.sample_rate,
        channels: spec.channels as u8,
    })
}
