use super::super::*;
use super::support::*;

fn degree_view(key: i32, scale: i32, spelling: i32) -> ChordboardView {
    let mut view = view(0, false);
    let params = Arc::get_mut(&mut view.params).unwrap();
    params.scale_layout = IntParam::new("Scale layout", 1, IntRange::Linear { min: 0, max: 2 });
    params.key = IntParam::new("Key", key, IntRange::Linear { min: 0, max: 11 });
    params.scale = IntParam::new("Scale", scale, IntRange::Linear { min: 0, max: 7 });
    params.key_spelling = IntParam::new("Spelling", spelling, IntRange::Linear { min: 0, max: 2 });
    view
}

#[test]
fn theory_teaches_borrowing_and_offers_a_playable_phrase() {
    let view = degree_view(0, 0, 0);
    let hint = view.degree_theory(27).unwrap(); // F: F minor below R: F major.
    assert!(
        hint.summary.contains("Fm · Borrowed iv · Parallel minor"),
        "{}",
        hint.summary
    );
    assert!(
        hint.summary.contains("A → Ab (lowered third)"),
        "{}",
        hint.summary
    );
    assert!(hint.detail.contains("Try R → F → Q"), "{}", hint.detail);
    assert_eq!(hint.changed_pitch, Some(8));
}

#[test]
fn theory_describes_possible_secondary_and_primary_dominants() {
    let view = degree_view(0, 0, 0);
    for (index, function, target, phrase) in [
        (1, "V/V", "G", "W → 2 → T"),
        (2, "V/vi", "Am", "E → 3 → Y"),
        (5, "V/ii", "Dm", "Y → 6 → W"),
    ] {
        let hint = view.degree_theory(index).unwrap();
        assert!(
            hint.summary.contains(&format!(
                "Secondary dominant {function} · Can lead to {target}"
            )),
            "{}",
            hint.summary
        );
        assert!(hint.detail.contains(phrase), "{}", hint.detail);
    }
    let minor = degree_view(0, 1, 0);
    let hint = minor.degree_theory(4).unwrap(); // G major above G minor.
    assert!(
        hint.summary.contains("Dominant V · Can lead to Cm"),
        "{}",
        hint.summary
    );
    assert!(!hint.summary.contains("Secondary"));
}

#[test]
fn theory_spells_altered_tones_by_their_original_letter() {
    for (key, scale, spelling, index, expected) in [
        (0, 0, 0, 0, "G → G# (raised fifth)"),
        (6, 0, 1, 24, "A# → A (lowered third)"),
        (6, 0, 2, 24, "Bb → Bbb (lowered third)"),
        (6, 6, 2, 1, "Cb → C (raised third)"),
    ] {
        let view = degree_view(key, scale, spelling);
        let hint = view.degree_theory(index).unwrap();
        assert!(hint.summary.contains(expected), "{}", hint.summary);
    }
    let view = degree_view(0, 0, 0);
    assert!(view
        .degree_theory(0)
        .unwrap()
        .summary
        .contains("Chromatic colour"));
}

#[test]
fn theory_handles_quality_limits_and_pentatonic_parent_triads() {
    let major = degree_view(0, 0, 0);
    let hint = major.degree_theory(30).unwrap(); // B diminished cannot darken.
    assert!(hint.summary.contains("quality limit reached"));
    assert_eq!(hint.changed_pitch, None);
    let harmonic_minor = degree_view(0, 2, 0);
    assert_eq!(harmonic_minor.degree_theory(2).unwrap().changed_pitch, None);
    let pentatonic = degree_view(6, 6, 2);
    let hint = pentatonic.degree_theory(13).unwrap();
    assert!(
        hint.detail.contains("Parent-scale triad · Ab–Cb–Eb"),
        "{}",
        hint.detail
    );
}

#[test]
fn theory_tracks_retained_chords_and_transposed_voicing_without_mislabeling_extensions() {
    let mut view = degree_view(0, 0, 0);
    assert!(view.selected_degree_theory().is_none());
    let mut chord = SavedChord {
        root: 53,
        quality: 1,
        second: None,
        additions: 0,
        transpose: 2,
        inversion: 1,
        spread: 2,
    };
    view.snapshot.captured = chord.encode();
    let hint = view.selected_degree_theory().unwrap();
    assert!(hint.summary.contains("Borrowed iv"));
    assert!(hint.detail.contains("Voicing transpose +2"));
    assert_eq!(view.theory_changed_pitch(), Some(10));
    chord.additions = 1 << 7;
    view.snapshot.captured = chord.encode();
    assert!(view.selected_degree_theory().is_none());
    chord.additions = 0;
    chord.second = Some(55);
    view.snapshot.captured = chord.encode();
    assert!(view.selected_degree_theory().is_none());
    chord.second = None;
    chord.quality = 2; // F7 is absent from the Degrees column.
    view.snapshot.captured = chord.encode();
    assert!(view.selected_degree_theory().is_none());
}

#[test]
fn theory_is_confined_to_degrees_and_hovering_never_plays() {
    let mut view = degree_view(0, 0, 0);
    let r = view.keyboard_key_rect(27);
    view.pointer = Some((r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
    assert!(view.hover_hint(&[]).unwrap().contains("Try R → F → Q"));
    assert!(view.bridge.commands.pop().is_none());
    for layout in [0, 2] {
        // Off (Chromatic/Fifths) and Passing.
        Arc::get_mut(&mut view.params).unwrap().scale_layout =
            IntParam::new("Scale layout", layout, IntRange::Linear { min: 0, max: 2 });
        assert!(view.degree_theory(27).is_none());
    }
}
