use hound::WavReader;
use scd_core::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Packs WAV files with deduplication — the same source file is only encoded once.
pub struct AudioPacker {
    out_file: BufWriter<File>,
    audio_offset_bytes: u64,
    cache: HashMap<PathBuf, SampleSlice>,
}

impl AudioPacker {
    pub fn new(out_file: BufWriter<File>) -> Self {
        Self {
            out_file,
            audio_offset_bytes: 0,
            cache: HashMap::new(),
        }
    }

    pub fn audio_offset_bytes(&self) -> u64 {
        self.audio_offset_bytes
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.out_file.flush()
    }

    /// Pack a WAV if not already cached; returns the slice metadata.
    pub fn pack_wav(&mut self, path: &Path) -> Result<SampleSlice, Box<dyn std::error::Error>> {
        if let Some(slice) = self.cache.get(path) {
            return Ok(*slice);
        }

        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();

        let start_offset = self.audio_offset_bytes;
        let mut frame_count = 0u32;

        match spec.sample_format {
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                let mut bytes = Vec::new();
                for sample in reader.samples::<i32>() {
                    let s = sample? as f32 / max_val;
                    bytes.extend_from_slice(&s.to_le_bytes());
                    frame_count += 1;
                }
                self.out_file.write_all(&bytes)?;
            }
            hound::SampleFormat::Float => {
                let mut bytes = Vec::new();
                for sample in reader.samples::<f32>() {
                    let s = sample?;
                    bytes.extend_from_slice(&s.to_le_bytes());
                    frame_count += 1;
                }
                self.out_file.write_all(&bytes)?;
            }
        }

        let actual_frames = frame_count / spec.channels as u32;
        let bytes_written = frame_count as u64 * 4;
        self.audio_offset_bytes += bytes_written;

        let slice = SampleSlice {
            offset_bytes: start_offset,
            length_frames: actual_frames,
            sample_rate: spec.sample_rate,
            channels: spec.channels as u8,
        };

        self.cache.insert(path.to_path_buf(), slice);
        Ok(slice)
    }
}

pub fn write_scdpack(
    strikes: &[StrikeEntry],
    audio_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let index = PackIndex {
        strikes: strikes.to_vec(),
    };
    let index_bytes = rkyv::to_bytes::<_, 256>(&index)?;
    let index_len = index_bytes.len() as u64;

    let mut final_file = BufWriter::new(File::create(output_path)?);
    final_file.write_all(SCD_MAGIC)?;
    final_file.write_all(&index_len.to_le_bytes())?;
    final_file.write_all(&index_bytes)?;

    let mut audio_reader = std::io::BufReader::new(File::open(audio_path)?);
    std::io::copy(&mut audio_reader, &mut final_file)?;

    final_file.flush()?;
    Ok(())
}

pub fn finish_pack(
    mut packer: AudioPacker,
    temp_audio_path: &Path,
    output_path: &Path,
    strikes: &[StrikeEntry],
) -> Result<u64, Box<dyn std::error::Error>> {
    packer.flush()?;
    let audio_bytes = packer.audio_offset_bytes();
    drop(packer);
    write_scdpack(strikes, temp_audio_path, output_path)?;
    let _ = std::fs::remove_file(temp_audio_path);
    Ok(audio_bytes)
}
