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
                let max_val = (1u32 << (spec.bits_per_sample - 1)) as f32;
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

    // Replace the directory entry only after the entire pack is written. An
    // existing reader may still have the previous file memory-mapped.
    let parent = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut replacement = tempfile::NamedTempFile::new_in(parent)?;
    let mut final_file = BufWriter::new(replacement.as_file_mut());
    final_file.write_all(SCD_MAGIC)?;
    final_file.write_all(&index_len.to_le_bytes())?;
    final_file.write_all(&index_bytes)?;

    let mut audio_reader = std::io::BufReader::new(File::open(audio_path)?);
    std::io::copy(&mut audio_reader, &mut final_file)?;

    final_file.flush()?;
    drop(final_file);
    replacement.as_file().sync_all()?;
    replacement.persist(output_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_strike(frames: u32) -> StrikeEntry {
        StrikeEntry::new(
            36,
            1,
            127,
            1,
            0,
            KitPieceId::Kick,
            [
                Some(SampleSlice {
                    offset_bytes: 0,
                    length_frames: frames,
                    sample_rate: 44100,
                    channels: 1,
                }),
                None,
                None,
                None,
                None,
                None,
            ],
        )
    }

    fn audio_bytes(samples: &[f32]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }

    #[test]
    fn replacement_preserves_existing_mapped_pack() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let audio = dir.path().join("audio.tmp");
        let output = dir.path().join("kit.scdpack");
        let original = vec![0.25; 8192];
        std::fs::write(&audio, audio_bytes(&original))?;
        write_scdpack(&[sample_strike(original.len() as u32)], &audio, &output)?;
        let mapped = ScdPack::open(&output)?;
        let strike = mapped.find_strike(36, 64, 1).unwrap();
        let old_samples = mapped
            .get_sample_slice(strike.mic_slices[0].as_ref().unwrap())
            .unwrap();

        std::fs::write(&audio, audio_bytes(&[-0.5, 0.5]))?;
        write_scdpack(&[sample_strike(2)], &audio, &output)?;
        assert_eq!(old_samples, original);
        let replacement = ScdPack::open(&output)?;
        let strike = replacement.find_strike(36, 64, 1).unwrap();
        assert_eq!(
            replacement
                .get_sample_slice(strike.mic_slices[0].as_ref().unwrap())
                .unwrap(),
            [-0.5, 0.5]
        );
        Ok(())
    }

    #[test]
    fn failed_replacement_preserves_existing_output() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let output = dir.path().join("kit.scdpack");
        let original = b"previous pack";
        std::fs::write(&output, original)?;

        for invalid_audio in [dir.path().join("missing.audio"), dir.path().to_path_buf()] {
            assert!(write_scdpack(&[], &invalid_audio, &output).is_err());
            assert_eq!(std::fs::read(&output)?, original);
            assert_eq!(std::fs::read_dir(dir.path())?.count(), 1);
        }
        Ok(())
    }

    #[test]
    fn integer_pcm_keeps_polarity_at_every_supported_depth(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        for bits in [8, 16, 24, 32] {
            let wav = dir.path().join(format!("pcm{bits}.wav"));
            let audio = dir.path().join(format!("pcm{bits}.audio"));
            let mut writer = hound::WavWriter::create(
                &wav,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 44100,
                    bits_per_sample: bits,
                    sample_format: hound::SampleFormat::Int,
                },
            )?;
            let half_scale = 1i32 << (bits - 2);
            for sample in [half_scale, -half_scale, 0] {
                writer.write_sample(sample)?;
            }
            writer.finalize()?;

            let mut packer = AudioPacker::new(BufWriter::new(File::create(&audio)?));
            let slice = packer.pack_wav(&wav)?;
            packer.flush()?;
            assert_eq!(slice.length_frames, 3);
            assert_eq!(
                std::fs::read(&audio)?,
                audio_bytes(&[0.5, -0.5, 0.0]),
                "{bits}-bit PCM"
            );
        }
        Ok(())
    }

    #[test]
    fn float_pcm_samples_are_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let wav = dir.path().join("float.wav");
        let audio = dir.path().join("float.audio");
        let samples = [0.75f32, -0.75, 1.5, -1.5, 0.0];
        let mut writer = hound::WavWriter::create(
            &wav,
            hound::WavSpec {
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )?;
        for sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        let mut packer = AudioPacker::new(BufWriter::new(File::create(&audio)?));
        packer.pack_wav(&wav)?;
        packer.flush()?;
        assert_eq!(std::fs::read(&audio)?, audio_bytes(&samples));
        Ok(())
    }
}
