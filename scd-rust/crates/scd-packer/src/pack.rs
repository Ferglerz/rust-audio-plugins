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
        if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
            return Err(format!("{}: expected 16-bit integer PCM WAV, found {:?} {}-bit; refusing implicit quantization",
                path.display(), spec.sample_format, spec.bits_per_sample).into());
        }
        let mut bytes = Vec::new();
        for sample in reader.samples::<i16>() {
            bytes.extend_from_slice(&sample?.to_le_bytes());
        }
        let mic = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(MicChannel::from_filename);
        if matches!(
            mic,
            Some(MicChannel::Close | MicChannel::Mono | MicChannel::Room)
        ) && spec.channels == 2
            && !bytes
                .as_chunks::<4>()
                .0
                .iter()
                .all(|frame| frame[..2] == frame[2..])
        {
            return Err(format!(
                "{}: Close, Mono, and Room must contain mono audio; stereo channels differ",
                path.display()
            )
            .into());
        }
        let slice = self.pack_pcm16(&bytes, spec.sample_rate, spec.channels)?;

        self.cache.insert(path.to_path_buf(), slice);
        Ok(slice)
    }
    fn pack_pcm16(
        &mut self,
        bytes: &[u8],
        sample_rate: u32,
        channels: u16,
    ) -> Result<SampleSlice, Box<dyn std::error::Error>> {
        if !(1..=2).contains(&channels)
            || sample_rate == 0
            || !bytes.len().is_multiple_of(channels as usize * 2)
        {
            return Err("invalid PCM16 channel count, sample rate, or frame alignment".into());
        }
        // A stereo container with identical channels carries mono audio.
        // Check every frame before discarding the redundant channel.
        let mono_bytes;
        let (bytes, channels) = if channels == 2
            && bytes
                .as_chunks::<4>()
                .0
                .iter()
                .all(|frame| frame[..2] == frame[2..])
        {
            mono_bytes = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|frame| [frame[0], frame[1]])
                .collect::<Vec<_>>();
            (mono_bytes.as_slice(), 1)
        } else {
            (bytes, channels)
        };
        let slice = SampleSlice {
            offset_bytes: self.audio_offset_bytes,
            length_frames: u32::try_from(bytes.len() / (channels as usize * 2))?,
            sample_rate,
            channels: channels as u8,
        };
        self.out_file.write_all(bytes)?;
        self.audio_offset_bytes += bytes.len() as u64;
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
            .flat_map(|sample| ((sample * 32768.0) as i16).to_le_bytes())
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
        assert_eq!(
            (0..old_samples.len())
                .map(|i| old_samples.sample(i))
                .collect::<Vec<_>>(),
            original
        );
        let replacement = ScdPack::open(&output)?;
        let strike = replacement.find_strike(36, 64, 1).unwrap();
        assert_eq!(
            {
                let data = replacement
                    .get_sample_slice(strike.mic_slices[0].as_ref().unwrap())
                    .unwrap();
                (0..data.len()).map(|i| data.sample(i)).collect::<Vec<_>>()
            },
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
    fn pcm16_is_bit_exact_and_deduplicated() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let wav = dir.path().join("pcm16.wav");
        let audio = dir.path().join("pcm16.audio");
        let samples = [i16::MIN, -16384, -1, 0, 1, 16384, i16::MAX, 0];
        let mut writer = hound::WavWriter::create(
            &wav,
            hound::WavSpec {
                channels: 2,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )?;
        for sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        let mut packer = AudioPacker::new(BufWriter::new(File::create(&audio)?));
        let slice = packer.pack_wav(&wav)?;
        assert_eq!(slice, packer.pack_wav(&wav)?);
        packer.flush()?;
        assert_eq!(slice.length_frames, 4);
        assert_eq!(slice.channels, 2);
        assert_eq!(packer.audio_offset_bytes(), 16);
        assert_eq!(
            std::fs::read(&audio)?,
            samples
                .iter()
                .flat_map(|s| s.to_le_bytes())
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn rejects_non_pcm16_sources() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        for (bits, format) in [
            (8, hound::SampleFormat::Int),
            (24, hound::SampleFormat::Int),
            (32, hound::SampleFormat::Int),
            (32, hound::SampleFormat::Float),
        ] {
            let wav = dir.path().join("source.wav");
            let writer = hound::WavWriter::create(
                &wav,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 44100,
                    bits_per_sample: bits,
                    sample_format: format,
                },
            )?;
            writer.finalize()?;
            let mut packer =
                AudioPacker::new(BufWriter::new(File::create(dir.path().join("audio"))?));
            assert!(packer
                .pack_wav(&wav)
                .unwrap_err()
                .to_string()
                .contains("refusing implicit quantization"));
            assert_eq!(packer.audio_offset_bytes(), 0);
        }
        Ok(())
    }
    #[test]
    fn collapses_only_exact_dual_mono() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let audio = dir.path().join("audio");
        let mut packer = AudioPacker::new(BufWriter::new(File::create(&audio)?));
        let mono = [i16::MIN, -1, 0, i16::MAX];
        let stereo = mono
            .iter()
            .flat_map(|s| [*s, *s])
            .flat_map(|s| s.to_le_bytes())
            .collect::<Vec<_>>();
        let collapsed = packer.pack_pcm16(&stereo, 44100, 2)?;
        assert_eq!(collapsed.channels, 1);
        assert_eq!(collapsed.length_frames, 4);
        assert_eq!(packer.audio_offset_bytes(), 8);
        let mut different = stereo.clone();
        *different.last_mut().unwrap() ^= 1;
        let preserved = packer.pack_pcm16(&different, 44100, 2)?;
        assert_eq!(preserved.channels, 2);
        assert_eq!(preserved.length_frames, 4);
        packer.flush()?;
        let expected = mono
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .chain(different)
            .collect::<Vec<_>>();
        assert_eq!(std::fs::read(audio)?, expected);
        Ok(())
    }
    #[test]
    fn required_mono_mics_reject_differing_stereo_channels(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        for mic in ["Close", "Mono", "Room"] {
            let wav = dir.path().join(format!("Kick-Hit-{mic}-1.wav"));
            let mut writer = hound::WavWriter::create(
                &wav,
                hound::WavSpec {
                    channels: 2,
                    sample_rate: 44100,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )?;
            for value in [100i16, 100, 200, 201] {
                writer.write_sample(value)?;
            }
            writer.finalize()?;
            let mut packer =
                AudioPacker::new(BufWriter::new(File::create(dir.path().join("audio"))?));
            assert!(packer
                .pack_wav(&wav)
                .unwrap_err()
                .to_string()
                .contains("stereo channels differ"));
            assert_eq!(packer.audio_offset_bytes(), 0);
        }
        Ok(())
    }
}
