use scd_core::kit::{
    calculate_max_rr_groups, count_kit_strikes, generate_kit_piece_strikes, load_stonehouse,
    rr_group_size, sample_wav_name, velocity_range,
};
use scd_core::{KitPieceId, MicChannel};

#[test]
fn rr_group_size_matches_hise_thresholds() {
    assert_eq!(rr_group_size(7), 1);
    assert_eq!(rr_group_size(8), 2);
    assert_eq!(rr_group_size(12), 3);
    assert_eq!(rr_group_size(16), 4);
    assert_eq!(rr_group_size(25), 5);
    assert_eq!(rr_group_size(36), 6);
}

#[test]
fn velocity_range_splits_full_midi_range() {
    let (lo, hi) = velocity_range(0, 1);
    assert_eq!((lo, hi), (1, 127));

    let layers = 6u32;
    let mut last_hi = 0u8;
    for layer in 0..layers {
        let (lo, hi) = velocity_range(layer, layers);
        assert!(lo <= hi);
        if layer > 0 {
            assert_eq!(lo, last_hi + 1);
        }
        last_hi = hi;
    }
    assert_eq!(last_hi, 127);
}

#[test]
fn sample_wav_name_follows_hise_convention() {
    let name = sample_wav_name(KitPieceId::Tom2, "Tom_2_Center", MicChannel::Close, 1);
    assert_eq!(name, "Tom 2-Tom_2_Center-Close-1.wav");
}

#[test]
fn stonehouse_tom2_strike_count_matches_hise_samplemap() {
    let kit = load_stonehouse();
    let arts = kit.pieces.get(&KitPieceId::Tom2).unwrap();
    let strikes = generate_kit_piece_strikes(KitPieceId::Tom2, arts);

    // Tom 2.xml has 85 <sample> entries in the HISE export.
    assert_eq!(strikes.len(), 85);
    assert_eq!(calculate_max_rr_groups(arts), 5);
}

#[test]
fn stonehouse_hihat_uses_six_rr_groups() {
    let kit = load_stonehouse();
    let arts = kit.pieces.get(&KitPieceId::Hihat).unwrap();
    assert_eq!(calculate_max_rr_groups(arts), 6);
}

#[test]
fn crashes_omit_close_mic_in_generated_paths() {
    let kit = load_stonehouse();
    let arts = kit.pieces.get(&KitPieceId::LCrash).unwrap();
    let strikes = generate_kit_piece_strikes(KitPieceId::LCrash, arts);
    assert!(!strikes.is_empty());

    for strike in &strikes {
        assert!(strike
            .mic_paths
            .iter()
            .all(|(mic, _)| *mic != MicChannel::Close));
    }
}

#[test]
fn stonehouse_total_strike_count() {
    let kit = load_stonehouse();
    assert_eq!(count_kit_strikes(&kit), 2142);

    let generated = kit
        .order
        .iter()
        .map(|id| generate_kit_piece_strikes(*id, kit.pieces.get(id).unwrap()).len())
        .sum::<usize>();
    assert_eq!(generated, 2142);
}
