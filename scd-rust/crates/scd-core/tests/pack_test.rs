use scd_core::*;
use std::io::Write;

fn write_pack(
    path: &std::path::Path,
    strikes: Vec<StrikeEntry>,
    audio: &[f32],
) -> Result<(), Box<dyn std::error::Error>> {
    let index = PackIndex { strikes };
    let index_bytes = rkyv::to_bytes::<_, 256>(&index)?;
    let index_len = index_bytes.len() as u64;

    let mut file = std::fs::File::create(path)?;
    file.write_all(SCD_MAGIC)?;
    file.write_all(&index_len.to_le_bytes())?;
    file.write_all(&index_bytes)?;
    for s in audio {
        file.write_all(&((s * 32768.0) as i16).to_le_bytes())?;
    }
    file.flush()?;
    Ok(())
}

fn slice(offset_bytes: u64) -> SampleSlice {
    SampleSlice {
        offset_bytes,
        length_frames: 4,
        sample_rate: 44100,
        channels: 1,
    }
}

#[test]
fn test_pack_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let pack_path = dir.path().join("test.scdpack");

    let strikes = vec![StrikeEntry::new(
        38,
        1,
        127,
        1,
        0,
        KitPieceId::Snare,
        [Some(slice(0)), None, None, None, None, None],
    )];

    write_pack(&pack_path, strikes, &[0.125, 0.25, 0.375, 0.5])?;

    let pack = ScdPack::open(&pack_path)?;
    let strike = pack.find_strike(38, 64, 1).expect("strike should match");
    assert_eq!(strike.kit_piece, KitPieceId::Snare);
    assert_eq!(pack.max_rr(38), 1);
    assert!(!pack.is_kick_note(38));
    assert_eq!(pack.kit_piece_for_note(38), Some(KitPieceId::Snare));
    assert_eq!(pack.kit_piece_for_note(37), None);
    assert!(pack.find_strike(37, 64, 1).is_none());

    let slice = strike.mic_slices[0].as_ref().unwrap();
    let samples = pack.get_sample_slice(slice).expect("samples should exist");
    assert_eq!(samples.len(), 4);
    assert_eq!(samples.sample(0), 0.125);
    assert_eq!(samples.sample(3), 0.5);

    Ok(())
}

#[test]
fn note_index_isolates_notes_and_rr() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let pack_path = dir.path().join("notes.scdpack");

    let empty = [None; MicChannel::COUNT];
    let mut kick_rr2 = empty;
    kick_rr2[0] = Some(slice(0));
    let mut snare = empty;
    snare[0] = Some(slice(8));

    let strikes = vec![
        StrikeEntry::new(36, 1, 64, 1, 0, KitPieceId::Kick, empty),
        StrikeEntry::new(36, 1, 64, 4, 0, KitPieceId::Kick, kick_rr2),
        StrikeEntry::new(36, 65, 127, 1, 0, KitPieceId::Kick, empty),
        StrikeEntry::new(38, 1, 127, 1, 0, KitPieceId::Snare, snare),
    ];
    write_pack(&pack_path, strikes, &[0.0; 8])?;

    let pack = ScdPack::open(&pack_path)?;
    assert!(pack.is_kick_note(36));
    assert!(!pack.is_kick_note(38));
    assert_eq!(pack.kit_piece_for_note(36), Some(KitPieceId::Kick));
    assert_eq!(pack.kit_piece_for_note(38), Some(KitPieceId::Snare));
    assert_eq!(pack.max_rr(36), 4);
    assert_eq!(pack.max_rr(38), 1);

    let low = pack.find_strike(36, 40, 4).expect("kick rr4");
    assert_eq!(low.rr_group, 4);
    assert!(pack.find_strike(36, 40, 2).is_none());
    assert_eq!(pack.find_strike(36, 90, 1).unwrap().lo_vel, 65);
    assert_eq!(
        pack.find_strike(38, 90, 1).unwrap().kit_piece,
        KitPieceId::Snare
    );

    let identity = pack.find_strike(36, 40, 1).unwrap();
    let remapped = pack.find_strike(36, 90, 1).unwrap();
    assert_eq!(identity.lo_vel, 1);
    assert_eq!(remapped.lo_vel, 65);

    Ok(())
}

#[test]
fn rejects_truncated_and_corrupt_indices() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("invalid-index.scdpack");

    std::fs::write(
        &path,
        [SCD_MAGIC.as_slice(), &u64::MAX.to_le_bytes()].concat(),
    )?;
    assert!(ScdPack::open(&path).is_err(), "oversized index length");

    std::fs::write(
        &path,
        [SCD_MAGIC.as_slice(), &32u64.to_le_bytes(), &[0xff; 32]].concat(),
    )?;
    assert!(ScdPack::open(&path).is_err(), "invalid archive contents");

    Ok(())
}

#[test]
fn rejects_invalid_audio_payload_and_sample_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("invalid-audio.scdpack");

    write_pack(&path, vec![], &[0.25])?;
    let mut bytes = std::fs::read(&path)?;
    bytes.push(0);
    std::fs::write(&path, bytes)?;
    assert!(ScdPack::open(&path).is_err(), "partial i16 payload");

    for bad_slice in [
        slice(1),
        slice(8),
        SampleSlice {
            channels: 0,
            ..slice(0)
        },
        SampleSlice {
            offset_bytes: u64::MAX - 3,
            ..slice(0)
        },
    ] {
        let strike = StrikeEntry::new(
            38,
            1,
            127,
            1,
            0,
            KitPieceId::Snare,
            [Some(bad_slice), None, None, None, None, None],
        );
        write_pack(&path, vec![strike], &[0.0; 4])?;
        assert!(ScdPack::open(&path).is_err(), "bad slice {bad_slice:?}");
    }

    write_pack(&path, vec![], &[0.0; 4])?;
    let pack = ScdPack::open(&path)?;
    assert!(pack.get_sample_slice(&slice(1)).is_none());
    assert!(pack.get_sample_slice(&slice(8)).is_none());

    Ok(())
}

#[test]
fn rejects_index_that_exceeds_note_index_capacity() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("too-many-strikes.scdpack");
    let strike = StrikeEntry::new(
        36,
        1,
        127,
        1,
        0,
        KitPieceId::Kick,
        [None; MicChannel::COUNT],
    );
    write_pack(&path, vec![strike; usize::from(u16::MAX) + 2], &[])?;

    let error = ScdPack::open(&path)
        .err()
        .expect("oversized note index must fail");
    assert!(error.to_string().contains("too many strikes"));
    Ok(())
}

#[test]
fn pcm16_reads_extremes_stereo_and_two_byte_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("pcm16.scdpack");
    let mut stream = slice(2);
    stream.length_frames = 2;
    stream.channels = 2;
    let strike = StrikeEntry::new(
        38,
        1,
        127,
        1,
        0,
        KitPieceId::Snare,
        [Some(stream), None, None, None, None, None],
    );
    let index = rkyv::to_bytes::<_, 256>(&PackIndex {
        strikes: vec![strike],
    })?;
    let mut bytes = Vec::from(SCD_MAGIC.as_slice());
    bytes.extend_from_slice(&(index.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&index);
    for pcm in [123i16, i16::MIN, i16::MAX, -1, 0] {
        bytes.extend_from_slice(&pcm.to_le_bytes());
    }
    std::fs::write(&path, &bytes)?;
    let pack = ScdPack::open(&path)?;
    assert_eq!(pack.audio_samples().len(), 5);
    assert_eq!(pack.sample_range(&stream), Some(1..5));
    let audio = pack.get_sample_slice(&stream).unwrap();
    assert_eq!(audio.sample(0), -1.0);
    assert_eq!(audio.sample(1), 32767.0 / 32768.0);
    assert_eq!(audio.sample(2), -1.0 / 32768.0);
    assert_eq!(audio.sample(3), 0.0);
    assert!(pack
        .get_sample_slice(&SampleSlice {
            offset_bytes: 1,
            ..stream
        })
        .is_none());
    assert!(pack
        .get_sample_slice(&SampleSlice {
            offset_bytes: 4,
            ..stream
        })
        .is_none());
    // Whole frames but a partial integer sample are not a valid payload.
    bytes.push(0);
    std::fs::write(&path, &bytes)?;
    assert!(ScdPack::open(&path).is_err());
    Ok(())
}

#[test]
fn rejects_old_float_format() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("old.scdpack");
    write_pack(&path, vec![], &[])?;
    let mut bytes = std::fs::read(&path)?;
    bytes[..8].copy_from_slice(b"SCDPACK1");
    std::fs::write(&path, bytes)?;
    assert!(ScdPack::open(&path).is_err());
    Ok(())
}
