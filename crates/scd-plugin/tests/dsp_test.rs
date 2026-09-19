use scd_plugin::dsp::HiHatTracker;

#[test]
fn test_hihat_cc_translation() {
    let mut tracker = HiHatTracker::new();

    // CC 0 = fully closed / firm
    tracker.set_cc4(0);
    assert_eq!(tracker.translate_note(46), 66); // Tip Firm
    assert_eq!(tracker.translate_note(26), 60); // Shoulder Firm

    // CC 20 = zone A
    tracker.set_cc4(20);
    assert_eq!(tracker.translate_note(46), 67); // Tip A
    assert_eq!(tracker.translate_note(26), 61); // Shoulder A

    // CC 120 = open
    tracker.set_cc4(120);
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
