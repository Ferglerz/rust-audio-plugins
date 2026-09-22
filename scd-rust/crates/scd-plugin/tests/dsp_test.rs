use scd_core::{KitPieceId, MicChannel};
use scd_plugin::dsp::HiHatTracker;
use scd_plugin::ScdParams;

#[test]
fn test_hihat_cc_translation() {
    let mut tracker = HiHatTracker::new();

    // CC 0 = fully closed / firm
    tracker.set_cc(0);
    assert_eq!(tracker.translate_note(46), 66); // Tip Firm
    assert_eq!(tracker.translate_note(26), 60); // Shoulder Firm

    // CC 20 = zone A
    tracker.set_cc(20);
    assert_eq!(tracker.translate_note(46), 67); // Tip A
    assert_eq!(tracker.translate_note(26), 61); // Shoulder A

    // CC 120 = open
    tracker.set_cc(120);
    assert_eq!(tracker.translate_note(46), 70); // Tip Open
    assert_eq!(tracker.translate_note(26), 64); // Shoulder Open
}

#[test]
fn test_hihat_choke_detection() {
    assert!(HiHatTracker::is_choke_trigger(44)); // Stomp
    assert!(HiHatTracker::is_choke_trigger(60)); // Firm shoulder
    assert!(HiHatTracker::is_choke_trigger(66)); // Firm tip
    assert!(!HiHatTracker::is_choke_trigger(70)); // Open hat
}

#[test]
fn omitted_close_crash_send_is_silent() {
    let params = ScdParams::default();
    assert_eq!(
        params
            .l_crash
            .mix_gain(KitPieceId::LCrash, MicChannel::Close),
        0.0
    );
    assert_eq!(
        params
            .r_crash
            .mix_gain(KitPieceId::RCrash, MicChannel::Close),
        0.0
    );
    assert!(params.kick.mix_gain(KitPieceId::Kick, MicChannel::Close) > 0.5);
}

#[test]
fn hihat_cc_invert_matches_original() {
    assert_eq!(HiHatTracker::process_cc(0, true), 127);
    assert_eq!(HiHatTracker::process_cc(127, true), 0);
    assert_eq!(HiHatTracker::process_cc(64, false), 64);
}

#[test]
fn peak_to_meter_uses_hise_db_scale() {
    use nih_plug::prelude::util;
    use scd_plugin::params::peak_to_meter;

    assert!((peak_to_meter(1.0) - 1.0).abs() < 1e-5);
    let expected = 1.0 + util::gain_to_db(0.1) / 100.0;
    assert!((peak_to_meter(0.1) - expected).abs() < 1e-4);
    assert_eq!(peak_to_meter(0.0), 0.0);
}

#[test]
fn default_hihat_cc_is_four_and_inverted() {
    use nih_plug::prelude::Params;

    let params = ScdParams::default();
    assert_eq!(params.cc_number.value(), 4);
    assert!(params.invert_cc.value());
    assert!(
        params.param_map().iter().any(|(id, _, _)| id == "cc_inv"),
        "invert_cc must be in the host param map"
    );
}

#[test]
fn kit_piece_param_ids_are_unique() {
    use nih_plug::prelude::Params;
    use std::collections::HashSet;

    let params = ScdParams::default();
    let map = params.param_map();
    let mut seen = HashSet::new();
    let mut dupes = Vec::new();
    for (id, _, _) in &map {
        if !seen.insert(id.clone()) {
            dupes.push(id.clone());
        }
    }
    assert!(dupes.is_empty(), "duplicate param ids: {dupes:?}");

    let kick_pitch = map
        .iter()
        .find(|(id, _, _)| id == "kick_pitch")
        .expect("kick_pitch");
    let crash_pitch = map
        .iter()
        .find(|(id, _, _)| id == "rcrash_pitch")
        .expect("rcrash_pitch");
    assert_ne!(kick_pitch.1, crash_pitch.1);
}

#[test]
fn identity_vel_map_passthrough() {
    let params = ScdParams::default();
    for kit_piece in KitPieceId::ALL {
        for vel in [1u8, 40, 64, 127] {
            assert_eq!(params.vel_maps.lookup(kit_piece, 0, vel), vel);
        }
    }
}

#[test]
fn skewed_vel_map_changes_sample_not_gain() {
    use scd_plugin::vel_map::VelCurve;

    let params = ScdParams::default();
    let midi_vel = 40u8;
    let velocity = midi_vel as f32 / 127.0;
    let gain = velocity * velocity;

    let mut curve = VelCurve::identity();
    curve.move_node(0, 0.0, 1.0);
    curve.move_node(1, 1.0, 1.0);
    params.vel_maps.set_curve(KitPieceId::Kick, 0, curve);

    let lookup = params.vel_maps.lookup(KitPieceId::Kick, 0, midi_vel);
    assert!(lookup > midi_vel, "skew should pick a harder sample");
    let lookup_gain = (lookup as f32 / 127.0) * (lookup as f32 / 127.0);
    assert!(
        (gain - velocity * velocity).abs() < 1e-6,
        "playback gain still follows input velocity"
    );
    assert!(gain < lookup_gain);
}

#[test]
fn init_resets_vel_maps() {
    use scd_plugin::vel_map::VelCurve;

    let params = ScdParams::default();
    let mut curve = VelCurve::identity();
    curve.move_node(0, 0.0, 1.0);
    params.vel_maps.set_curve(KitPieceId::Snare, 0, curve);
    assert_ne!(params.vel_maps.lookup(KitPieceId::Snare, 0, 40), 40);
    params.vel_maps.reset_all();
    assert_eq!(params.vel_maps.lookup(KitPieceId::Snare, 0, 40), 40);
}
