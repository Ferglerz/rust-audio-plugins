use scd_core::*;
use std::io::Write;

#[test]
fn test_pack_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let pack_path = dir.path().join("test.scdpack");

    let strikes = vec![StrikeEntry {
        midi_note: 38,
        lo_vel: 1,
        hi_vel: 127,
        rr_group: 1,
        volume_db: 0,
        kit_piece: KitPieceId::Snare,
        mic_slices: [
            Some(SampleSlice {
                offset_bytes: 0,
                length_frames: 4,
                sample_rate: 44100,
                channels: 1,
            }),
            None,
            None,
            None,
            None,
            None,
        ],
    }];

    let index = PackIndex { strikes };
    let index_bytes = rkyv::to_bytes::<_, 256>(&index)?;
    let index_len = index_bytes.len() as u64;

    let mut file = std::fs::File::create(&pack_path)?;
    file.write_all(SCD_MAGIC)?;
    file.write_all(&index_len.to_le_bytes())?;
    file.write_all(&index_bytes)?;

    // Dummy 4 float samples: [0.1, 0.2, 0.3, 0.4]
    let audio: [f32; 4] = [0.1, 0.2, 0.3, 0.4];
    for s in audio {
        file.write_all(&s.to_le_bytes())?;
    }
    file.flush()?;
    drop(file);

    // Test ScdPack open & lookup
    let pack = ScdPack::open(&pack_path)?;
    let strike = pack.find_strike(38, 64, 1).expect("strike should match");
    assert_eq!(strike.kit_piece, KitPieceId::Snare);

    let slice = strike.mic_slices[0].as_ref().unwrap();
    let samples = pack.get_sample_slice(slice).expect("samples should exist");
    assert_eq!(samples.len(), 4);
    assert_eq!(samples[0], 0.1);
    assert_eq!(samples[3], 0.4);

    Ok(())
}
