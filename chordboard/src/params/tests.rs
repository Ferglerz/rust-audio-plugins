use super::*;

#[test]
fn glissando_range_and_readout_support_milliseconds_and_seconds() {
    let params = ChordboardParams::default();
    let time = &params.gliss_ms;
    assert_eq!(time.preview_plain(0.0), 0.0);
    assert_eq!(time.preview_plain(0.5), 1000.0);
    assert_eq!(time.preview_plain(1.0), 4000.0);
    for (ms, label) in [(127.60255, "128 ms"), (1200.0, "1.2s"), (4000.0, "4.0s")] {
        assert_eq!(
            time.normalized_value_to_string(time.preview_normalized(ms), true),
            label
        );
    }
    for (text, ms) in [
        ("128 ms", 128.0),
        ("1.2s", 1200.0),
        ("1.2 s", 1200.0),
        ("128", 128.0),
    ] {
        assert_eq!(
            time.string_to_normalized_value(text),
            Some(time.preview_normalized(ms))
        );
    }
    assert_eq!(time.string_to_normalized_value("NaN"), None);
}

#[test]
fn split_note_readout_roundtrips_and_rejects_out_of_range_notes() {
    for note in 0..=127 {
        let label = format!(
            "{}{} ({note})",
            crate::harmony::NOTE_NAMES[note as usize % 12],
            note / 12 - 1
        );
        assert_eq!(parse_split_note(&label), Some(note));
    }
    assert_eq!(parse_split_note("C#4"), Some(61));
    assert_eq!(parse_split_note("60"), Some(60));
    assert_eq!(parse_split_note("128"), None);
    assert_eq!(parse_split_note("C-2"), None);
}

#[test]
fn main_buttons_resolve_their_parameters_without_settings_duplicates() {
    let params = ChordboardParams::default();
    let controls = params.controls();
    assert!(params.control("highlight").is_none());
    assert!(params.control("keyboard").is_none());
    for (id, ptr) in [
        ("latch", params.latch.as_ptr()),
        ("strum_latch", params.strum_latch.as_ptr()),
        ("output_mode", params.output_mode.as_ptr()),
        ("fifths", params.fifths.as_ptr()),
    ] {
        assert_eq!(
            controls.iter().find(|c| c.id == id).map(|c| c.ptr),
            Some(ptr),
            "{id}"
        );
    }
}

#[test]
fn saved_fields_roundtrip_without_held_notes() {
    let params = ChordboardParams::default();
    let chord = crate::harmony::SavedChord {
        additions: 0,
        root: 60,
        second: Some(66),
        quality: 0,
        inversion: 1,
        spread: 1,
        transpose: 12,
    };
    params.control_base.store(36, Ordering::Relaxed);
    for i in 7..10 {
        params.slot(i).store(chord.encode(), Ordering::Relaxed);
    }
    params.map_x.store(
        Mapping {
            kind: 2,
            number: 1,
            channel: 0,
        }
        .encode(),
        Ordering::Relaxed,
    );
    let fields = params.serialize_fields();
    let loaded = ChordboardParams::default();
    loaded.deserialize_fields(&fields);
    for i in 7..10 {
        assert_eq!(loaded.slot(i).load(Ordering::Relaxed), chord.encode());
    }
    assert_eq!(loaded.config().control_base, 36);
    assert_eq!(loaded.config().mappings[0].kind, 2);
    assert!(!fields
        .keys()
        .any(|k| k.contains("held") || k.contains("voice")));
    let ids: Vec<_> = params.param_map().into_iter().map(|p| p.0).collect();
    let unique: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len());
}

#[cfg(test)]
mod global_control_tests {
    use super::*;

    #[test]
    fn global_control_defaults_and_custom_ordering_preserve_legacy_ranges() {
        let params = ChordboardParams::default();
        assert_eq!(params.base_inversion.value(), 0);
        assert_eq!(params.complexity.value(), 0);
        assert_eq!(params.voicing_override.value(), 0);
        assert_eq!(params.gate_scale.value(), 1.0);
        assert_eq!(params.velocity_scale.value(), 1.0);
        assert_eq!(params.velocity_scale.preview_plain(0.0), 0.0);
        assert_eq!(params.velocity_scale.preview_plain(1.0), 2.0);
        assert_eq!(params.velocity.preview_plain(0.0), 0.01);
        assert_eq!(params.velocity.preview_plain(1.0), 1.0);
        assert_eq!(params.inversion.preview_plain(0.0), 0);
        assert_eq!(params.inversion.preview_plain(1.0), 11);
        assert_eq!(params.spread.preview_plain(1.0), 4);
        assert_eq!(params.base_inversion.preview_plain(0.0), -12);
        assert_eq!(params.base_inversion.preview_plain(1.0), 12);
        for id in [
            "base_inversion",
            "complexity",
            "voicing_override",
            "gate_scale",
            "velocity_scale",
        ] {
            assert!(params.control(id).is_some(), "missing control {id}");
        }
        let adapter = ArpPageParams::from_settings(crate::arpeggiator::Settings {
            pattern: 7,
            ..Default::default()
        });
        assert_eq!(adapter.settings().pattern, 7);
        assert_eq!(
            adapter.arp_pattern.normalized_value_to_string(1.0, false),
            "Custom"
        );
    }
}

#[cfg(test)]
mod global_value_tests {
    use super::*;
    #[test]
    fn complexity_readout_round_trips_every_label() {
        let params = ChordboardParams::default();
        for value in 0..=5 {
            let normalized = params.complexity.preview_normalized(value);
            let label = params
                .complexity
                .normalized_value_to_string(normalized, true);
            assert_eq!(
                params.complexity.string_to_normalized_value(&label),
                Some(normalized)
            );
        }
    }
}
