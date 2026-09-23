use super::*;

#[test]
fn entries_accept_units_and_reject_invalid_numbers() {
    for (input, target, expected) in [
        ("1.25 kHz", ValueTarget::Band(0), 1250.0),
        ("20k", ValueTarget::Band(0), 20000.0),
        ("178 Hz", ValueTarget::Band(0), 178.0),
        ("-11.75 dB", ValueTarget::Band(1), -11.75),
        ("1.77", ValueTarget::Band(2), 1.77),
        ("4:1", ValueTarget::Band(4), 4.0),
        ("0.1 ms", ValueTarget::Band(5), 0.1),
        ("1.4 s", ValueTarget::Band(6), 1400.0),
        ("-20 dB", ValueTarget::Band(3), -20.0),
        ("12 dB", ValueTarget::Band(7), 12.0),
        ("8:1", ValueTarget::Global(6), 8.0),
        ("50 %", ValueTarget::Global(2), 50.0),
        ("25 %", ValueTarget::Global(3), 25.0),
        ("OFF", ValueTarget::Global(1), -80.0),
        ("100 ms", ValueTarget::Global(10), 1.0),
        ("c", ValueTarget::Global(10), 2.0),
        ("1.5 s", ValueTarget::Global(10), 4.0),
        ("2.5", ValueTarget::Global(10), 2.5),
        ("10 ms", ValueTarget::Global(5), 10.0),
        ("1.2 s", ValueTarget::Global(11), 1200.0),
        ("250 ms", ValueTarget::Global(11), 250.0),
    ] {
        assert_eq!(parse_value(input, target), Some(expected), "{input}");
    }
    for input in ["", "-", "NaN", "inf", "1e999", "12garbage", "2 ms", "--3"] {
        assert_eq!(parse_value(input, ValueTarget::Band(0)), None, "{input}");
    }
}

#[test]
fn editing_replaces_selection_and_deletes_without_touching_the_band() {
    let mut edit = ValueEdit::new(ValueTarget::Band(0), (0.0, 0.0, 100.0, 24.0), "1000".into());
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
        ("5 kHz", ValueTarget::Lift(0), 5000.0),
        ("12k", ValueTarget::Lift(0), 12000.0),
        ("400 Hz", ValueTarget::Lift(0), 400.0),
        ("0 dB", ValueTarget::Lift(1), 0.0),
        ("-18 dB", ValueTarget::Lift(1), -18.0),
        ("-50.5 dB", ValueTarget::Lift(1), -50.5),
        ("-100 dB", ValueTarget::Lift(1), -100.0),
        ("0.707", ValueTarget::Lift(2), 0.707),
        ("-18 dB", ValueTarget::Lift(3), -18.0),
        ("6:1", ValueTarget::Lift(4), 6.0),
        ("10 ms", ValueTarget::Lift(5), 10.0),
        ("150 ms", ValueTarget::Lift(6), 150.0),
        ("12 dB", ValueTarget::Lift(7), 12.0),
    ] {
        assert_eq!(parse_value(input, target), Some(expected), "{input}");
    }
}
