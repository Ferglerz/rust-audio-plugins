use super::*;

#[test]
fn entries_accept_units_and_reject_invalid_numbers() {
    for (input, target, expected) in [
        ("1.25 kHz", ValueTarget::Band(BandField::Frequency), 1250.0),
        ("20k", ValueTarget::Band(BandField::Frequency), 20000.0),
        ("178 Hz", ValueTarget::Band(BandField::Frequency), 178.0),
        ("-11.75 dB", ValueTarget::Band(BandField::Gain), -11.75),
        ("1.77", ValueTarget::Band(BandField::Q), 1.77),
        ("4:1", ValueTarget::Band(BandField::Ratio), 4.0),
        ("0.1 ms", ValueTarget::Band(BandField::Attack), 0.1),
        ("1.4 s", ValueTarget::Band(BandField::Release), 1400.0),
        ("-20 dB", ValueTarget::Band(BandField::Threshold), -20.0),
        ("12 dB", ValueTarget::Band(BandField::Range), 12.0),
        ("8:1", ValueTarget::Global(GlobalControl::CompRatio), 8.0),
        ("50 %", ValueTarget::Global(GlobalControl::PseVoice), 50.0),
        ("25 %", ValueTarget::Global(GlobalControl::Dry), 25.0),
        ("OFF", ValueTarget::Global(GlobalControl::Gate), -80.0),
        ("100 ms", ValueTarget::Global(GlobalControl::PseTime), 1.0),
        ("c", ValueTarget::Global(GlobalControl::PseTime), 2.0),
        ("1.5 s", ValueTarget::Global(GlobalControl::PseTime), 4.0),
        ("2.5", ValueTarget::Global(GlobalControl::PseTime), 2.5),
        ("10 ms", ValueTarget::Global(GlobalControl::CompAttack), 10.0),
        ("1.2 s", ValueTarget::Global(GlobalControl::CompRelease), 1200.0),
        ("250 ms", ValueTarget::Global(GlobalControl::CompRelease), 250.0),
    ] {
        assert_eq!(parse_value(input, target), Some(expected), "{input}");
    }
    for input in ["", "-", "NaN", "inf", "1e999", "12garbage", "2 ms", "--3"] {
        assert_eq!(parse_value(input, ValueTarget::Band(BandField::Frequency)), None, "{input}");
    }
}

#[test]
fn editing_replaces_selection_and_deletes_without_touching_the_band() {
    let mut edit = ValueEdit::new(ValueTarget::Band(BandField::Frequency), (0.0, 0.0, 100.0, 24.0), "1000".into());
    edit.insert("1.25 kHz");
    assert_eq!(parse_value(&edit.text, edit.target), Some(1250.0));
    edit.cursor = 4;
    edit.anchor = 1;
    edit.insert(".5");
    assert_eq!(edit.text, "1.5 kHz");
    edit.erase(true);
    assert_eq!(edit.text, "1. kHz");
    edit.cursor = 0;
    edit.anchor = edit.text.len();
    edit.erase(false);
    assert!(edit.text.is_empty());
    edit.insert("NaN");
    assert_eq!(parse_value(&edit.text, edit.target), None);
    assert_eq!(edit.original, "1000");
}

#[test]
fn lift_entries_accept_units_and_parse_properly() {
    for (input, target, expected) in [
        ("5 kHz", ValueTarget::Lift(BandField::Frequency), 5000.0),
        ("12k", ValueTarget::Lift(BandField::Frequency), 12000.0),
        ("400 Hz", ValueTarget::Lift(BandField::Frequency), 400.0),
        ("0 dB", ValueTarget::Lift(BandField::Gain), 0.0),
        ("-18 dB", ValueTarget::Lift(BandField::Gain), -18.0),
        ("-50.5 dB", ValueTarget::Lift(BandField::Gain), -50.5),
        ("-100 dB", ValueTarget::Lift(BandField::Gain), -100.0),
        ("0.707", ValueTarget::Lift(BandField::Q), 0.707),
        ("-18 dB", ValueTarget::Lift(BandField::Threshold), -18.0),
        ("6:1", ValueTarget::Lift(BandField::Ratio), 6.0),
        ("10 ms", ValueTarget::Lift(BandField::Attack), 10.0),
        ("150 ms", ValueTarget::Lift(BandField::Release), 150.0),
        ("12 dB", ValueTarget::Lift(BandField::Range), 12.0),
    ] {
        assert_eq!(parse_value(input, target), Some(expected), "{input}");
    }
}
