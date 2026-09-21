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
        file.write_all(&s.to_le_bytes())?;
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

    write_pack(&pack_path, strikes, &[0.1, 0.2, 0.3, 0.4])?;

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
    assert_eq!(samples[0], 0.1);
    assert_eq!(samples[3], 0.4);

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
    snare[0] = Some(slice(16));

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
