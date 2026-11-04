//! Non-destructive global voicing and the seven-degree Custom palette.
use super::{intervals, recipe_intervals, scale_tones, voice_recipe, Notes, SavedChord};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlobalHarmony {
    pub inversion: i8,
    /// Original, Triad, 7th, 9th, 11th, 13th.
    pub complexity: u8,
    /// Added to the saved chord's transpose exactly once.
    pub transpose: i8,
    /// Saved, Close, Open, Wide, Drop 2, Drop 3.
    pub voicing: u8,
    pub key: u8,
    pub scale: u8,
}

fn alteration(chord: SavedChord) -> Option<u8> {
    chord
        .second
        .map(|n| (n as i16 - chord.root as i16).rem_euclid(12) as u8)
}

fn seventh(chord: SavedChord) -> u8 {
    match alteration(chord) {
        Some(n @ (9..=11)) => n,
        _ => match chord.quality {
            1 | 2 | 4 | 10 => 10,
            5 | 9 => 9,
            7 | 8 => 9,
            _ => 11,
        },
    }
}

/// Chord-relative pitch classes, independent of complexity and register.
/// The chord owns its structural tones; the key supplies the remaining colors.
pub fn degree_palette(chord: SavedChord, key: u8, scale: u8) -> [u8; 7] {
    let parent = match scale {
        6 => 0,
        7 => 1,
        _ => scale,
    };
    let scale = scale_tones(parent);
    let relative = (chord.root as i16 - key as i16).rem_euclid(12) as u8;
    let mut palette = [0, 2, 4, 5, 7, 9, 11];
    if let Some(root_degree) = scale.iter().position(|&n| n == relative) {
        for (degree, pc) in palette.iter_mut().enumerate() {
            *pc = (scale[(root_degree + degree) % 7] as i16 - relative as i16).rem_euclid(12) as u8;
        }
    } else {
        // Preserve seven named degrees even for chromatic roots. Prefer the
        // unaltered interval, then a lower neighbor, then an upper neighbor.
        for pc in &mut palette[1..] {
            if let Some(candidate) = [0i16, -1, 1].into_iter().find_map(|delta| {
                let candidate = (*pc as i16 + delta).rem_euclid(12) as u8;
                scale
                    .contains(&((relative + candidate) % 12))
                    .then_some(candidate)
            }) {
                *pc = candidate;
            }
        }
    }
    palette[0] = 0;
    let structural = intervals(chord.quality, alteration(chord));
    for &pc in structural.as_slice() {
        match pc {
            2 => palette[1] = 2,
            3 | 4 => palette[2] = pc,
            5 => palette[3] = 5,
            6..=8 => palette[4] = pc,
            9 if !matches!(chord.quality, 5 | 9) => palette[5] = 9,
            9..=11 => palette[6] = pc,
            13 => palette[1] = 1,
            _ => {}
        }
    }
    // A plain triad still defines its seventh family for both complexity and
    // Custom. Sixth chords keep the scale's seventh in the Custom palette.
    if !matches!(chord.quality, 7 | 8) && alteration(chord) != Some(9) {
        palette[6] = seventh(chord);
    }
    let has = |pc: u8| chord.additions & (1 << pc) != 0;
    // Natural additions are considered first; explicit chromatic alterations
    // take precedence when a recipe contains both.
    for (degree, candidates) in [
        (1, &[2, 3, 1][..]),
        (3, &[5, 6][..]),
        (5, &[9, 8][..]),
        (6, &[11, 10][..]),
    ] {
        for &pc in candidates {
            if has(pc) && !structural.as_slice().iter().any(|n| *n % 12 == pc) {
                palette[degree] = pc;
            }
        }
    }
    palette
}

pub(super) fn complexity_intervals(chord: SavedChord, globals: GlobalHarmony) -> Notes {
    if globals.complexity == 0 {
        return recipe_intervals(chord);
    }
    let level = globals.complexity.min(5);
    let structural = intervals(chord.quality, alteration(chord));
    let mut tones = Notes::default();
    for &tone in structural.as_slice() {
        // A suspension is structural, but a flat ninth selected with a second
        // note is still an extension and disappears in the triad setting.
        if tone <= 8 && tone != 1 {
            tones.push(tone as i16);
        }
    }
    // Explicit altered fifths survive simplification, provided the recipe
    // omitted the natural fifth. Otherwise they describe #11 / b13 colors.
    if !structural.as_slice().contains(&7) {
        for pc in [6, 8] {
            if chord.additions & (1 << pc) != 0 {
                tones.push(pc);
            }
        }
    }
    let palette = degree_palette(chord, globals.key, globals.scale);
    if level >= 2 {
        let seventh = if matches!(chord.quality, 7 | 8) || alteration(chord) == Some(9) {
            9
        } else {
            palette[6]
        };
        tones.push(seventh as i16);
    }
    for (minimum, degree) in [(3, 1), (4, 3), (5, 5)] {
        if level >= minimum {
            let pc = palette[degree];
            if !tones.as_slice().iter().any(|n| *n % 12 == pc) {
                tones.push(pc as i16 + 12);
            }
        }
    }
    tones.values[..tones.len].sort_unstable();
    tones
}

fn rotate_raw(values: &mut [i16], amount: i8) {
    if values.is_empty() {
        return;
    }
    values.sort_unstable();
    for _ in 0..amount.unsigned_abs() {
        if amount > 0 {
            values[0] += 12;
        } else {
            let last = values.len() - 1;
            values[last] -= 12;
        }
        values.sort_unstable();
    }
}

/// Shift successive lowest/highest voices by one octave, retaining signed
/// intermediate pitches until all inversions have been applied.
pub fn rotate_voicing(notes: Notes, amount: i8) -> Notes {
    let mut values = [0i16; 12];
    for (out, &note) in values.iter_mut().zip(notes.as_slice()) {
        *out = note as i16;
    }
    rotate_raw(&mut values[..notes.len], amount);
    let mut result = Notes::default();
    for &note in &values[..notes.len] {
        result.push(note);
    }
    result
}

/// Resolve a temporary voicing; the caller's saved recipe is never modified.
pub fn resolve_chord(chord: SavedChord, globals: GlobalHarmony) -> Notes {
    resolve_loop_chord(chord, globals, 0)
}

/// Apply global and loop inversion before filtering pitches to MIDI bounds.
pub fn resolve_loop_chord(chord: SavedChord, globals: GlobalHarmony, loop_inversion: i8) -> Notes {
    if globals.complexity == 0
        && globals.inversion == 0
        && globals.transpose == 0
        && loop_inversion == 0
    {
        let spread = if globals.voicing == 0 {
            chord.spread
        } else {
            globals.voicing.min(5) - 1
        };
        return voice_recipe(chord, spread);
    }
    let (values, len) = resolved_raw(chord, globals, loop_inversion);
    let mut notes = Notes::default();
    for &note in &values[..len] {
        notes.push(note);
    }
    notes
}

fn resolved_raw(
    chord: SavedChord,
    globals: GlobalHarmony,
    loop_inversion: i8,
) -> ([i16; 12], usize) {
    let spread = if globals.voicing == 0 {
        chord.spread
    } else {
        globals.voicing.min(5) - 1
    };
    let tones = complexity_intervals(chord, globals);
    if tones.len == 0 {
        return ([0; 12], 0);
    }
    let mut values = [0i16; 12];
    let mut previous = -1;
    let inversion = chord.inversion as usize % tones.len;
    for (i, value) in values.iter_mut().enumerate().take(tones.len) {
        let mut interval = tones.values[(i + inversion) % tones.len] as i16;
        while interval <= previous {
            interval += 12;
        }
        previous = interval;
        let open = match spread.min(4) {
            1 if i % 2 == 1 => 12,
            2 => (i / 2) as i16 * 12,
            3 if tones.len >= 3 && i == tones.len - 2 => -12,
            4 if tones.len >= 3 && i == tones.len - 3 => -12,
            _ => 0,
        };
        *value =
            chord.root as i16 + chord.transpose as i16 + globals.transpose as i16 + interval + open;
    }
    rotate_raw(&mut values[..tones.len], globals.inversion);
    rotate_raw(&mut values[..tones.len], loop_inversion);
    (values, tones.len)
}

/// A one-based degree retains its pitch class under inversion. Only the
/// register anchor follows the resolved chord's bass.
pub fn custom_degree_note(
    chord: SavedChord,
    globals: GlobalHarmony,
    loop_inversion: i8,
    degree: u8,
) -> Option<u8> {
    let note = custom_degree_pitch(chord, globals, loop_inversion, degree)?;
    (0..=127).contains(&note).then_some(note as u8)
}

/// Signed counterpart for schedulers which still need to apply octave offsets.
pub fn custom_degree_pitch(
    chord: SavedChord,
    globals: GlobalHarmony,
    loop_inversion: i8,
    degree: u8,
) -> Option<i16> {
    let (voiced, len) = resolved_raw(chord, globals, loop_inversion);
    let bass = *voiced[..len].first()?;
    let degree = degree.clamp(1, 7) as usize - 1;
    let palette = degree_palette(chord, globals.key, globals.scale);
    let pc = (chord.root as i16
        + chord.transpose as i16
        + globals.transpose as i16
        + palette[degree] as i16)
        .rem_euclid(12);
    Some(bass + (pc - bass).rem_euclid(12))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(quality: u8) -> SavedChord {
        SavedChord {
            root: 60,
            second: None,
            additions: 0,
            quality,
            inversion: 0,
            spread: 0,
            transpose: 0,
        }
    }

    #[test]
    fn original_preserves_every_saved_voicing() {
        for quality in 0..12 {
            for spread in 0..5 {
                for inversion in 0..12 {
                    let c = SavedChord {
                        spread,
                        inversion,
                        additions: 1 << 1,
                        ..chord(quality)
                    };
                    assert_eq!(
                        resolve_chord(c, GlobalHarmony::default()),
                        voice_recipe(c, spread)
                    );
                }
            }
        }
    }

    #[test]
    fn levels_add_chord_seventh_and_scale_colors() {
        let c = chord(0);
        let expected: &[&[u8]] = &[
            &[60, 64, 67],
            &[60, 64, 67],
            &[60, 64, 67, 71],
            &[60, 64, 67, 71, 74],
            &[60, 64, 67, 71, 74, 78],
            &[60, 64, 67, 71, 74, 78, 81],
        ];
        for (complexity, expected) in expected.iter().enumerate() {
            assert_eq!(
                resolve_chord(
                    c,
                    GlobalHarmony {
                        complexity: complexity as u8,
                        key: 7,
                        ..GlobalHarmony::default()
                    }
                )
                .as_slice(),
                *expected
            );
        }
    }

    #[test]
    fn qualities_keep_their_sevenths_and_sixths() {
        for (quality, seventh) in [
            (0, 11),
            (1, 10),
            (2, 10),
            (3, 11),
            (4, 10),
            (5, 9),
            (6, 11),
            (7, 9),
            (8, 9),
            (9, 9),
            (10, 10),
        ] {
            let notes = resolve_chord(
                chord(quality),
                GlobalHarmony {
                    complexity: 2,
                    ..GlobalHarmony::default()
                },
            );
            assert_eq!(
                notes.as_slice().last(),
                Some(&(60 + seventh)),
                "quality {quality}"
            );
        }
    }

    #[test]
    fn simplify_preserves_suspensions_and_altered_fifths() {
        let globals = GlobalHarmony {
            complexity: 1,
            ..GlobalHarmony::default()
        };
        for (second, expected) in [
            (62, &[60, 62, 67][..]),
            (65, &[60, 65, 67][..]),
            (66, &[60, 64, 66][..]),
            (68, &[60, 64, 68][..]),
        ] {
            let c = SavedChord {
                second: Some(second),
                additions: (1 << 1) | (1 << 9),
                ..chord(3)
            };
            assert_eq!(resolve_chord(c, globals).as_slice(), expected);
        }
    }

    #[test]
    fn explicit_flat_ninth_overrides_scale_and_is_removed_below_ninth() {
        let c = SavedChord {
            additions: 1 << 1,
            ..chord(2)
        };
        assert_eq!(degree_palette(c, 0, 0)[1], 1);
        assert_eq!(
            resolve_chord(
                c,
                GlobalHarmony {
                    complexity: 3,
                    ..GlobalHarmony::default()
                }
            )
            .as_slice(),
            &[60, 64, 67, 70, 73]
        );
        assert_eq!(
            resolve_chord(
                c,
                GlobalHarmony {
                    complexity: 2,
                    ..GlobalHarmony::default()
                }
            )
            .as_slice(),
            &[60, 64, 67, 70]
        );
    }

    #[test]
    fn palette_is_chord_relative_and_pentatonics_use_parents() {
        let dm = SavedChord {
            root: 62,
            ..chord(1)
        };
        assert_eq!(degree_palette(dm, 0, 0), [0, 2, 3, 5, 7, 9, 10]);
        assert_eq!(degree_palette(dm, 0, 6), degree_palette(dm, 0, 0));
        assert_eq!(degree_palette(dm, 0, 7), degree_palette(dm, 0, 1));
        for scale in 0..23 {
            assert!(degree_palette(dm, 0, scale).iter().all(|pc| *pc < 12));
        }
    }

    #[test]
    fn chromatic_root_uses_nearest_scale_tones_but_keeps_chord_identity() {
        let db = SavedChord {
            root: 61,
            ..chord(0)
        };
        // Db's ninth is Eb; D and E are both in C major, so choose D.
        let palette = degree_palette(db, 0, 0);
        assert_eq!(palette[1], 1);
        assert_eq!(palette[2], 4);
        assert_eq!(palette[4], 7);
        assert_eq!(palette[6], 11);
    }

    #[test]
    fn inversions_move_both_directions_and_transpose_only_once() {
        for (inversion, expected) in [
            (-1, &[55, 60, 64][..]),
            (1, &[64, 67, 72][..]),
            (-3, &[48, 52, 55][..]),
            (3, &[72, 76, 79][..]),
        ] {
            assert_eq!(
                resolve_chord(
                    chord(0),
                    GlobalHarmony {
                        inversion,
                        ..GlobalHarmony::default()
                    }
                )
                .as_slice(),
                expected
            );
        }
        let c = SavedChord {
            transpose: 12,
            ..chord(0)
        };
        assert_eq!(
            resolve_chord(
                c,
                GlobalHarmony {
                    transpose: -5,
                    ..GlobalHarmony::default()
                }
            )
            .as_slice(),
            &[67, 71, 74]
        );
        assert_eq!(c.transpose, 12);
    }

    #[test]
    fn rotations_filter_midi_bounds_only_after_signed_work() {
        let low = SavedChord {
            root: 0,
            ..chord(0)
        };
        assert_eq!(
            resolve_chord(
                low,
                GlobalHarmony {
                    inversion: -1,
                    ..GlobalHarmony::default()
                }
            )
            .as_slice(),
            &[0, 4]
        );
        let high = SavedChord {
            root: 126,
            ..chord(0)
        };
        assert_eq!(
            resolve_chord(
                high,
                GlobalHarmony {
                    inversion: -3,
                    ..GlobalHarmony::default()
                }
            )
            .as_slice(),
            &[114, 118, 121]
        );
    }

    #[test]
    fn custom_degrees_keep_pitch_classes_and_follow_inversion_register() {
        let c = chord(0);
        assert_eq!(
            custom_degree_note(c, GlobalHarmony::default(), 0, 1),
            Some(60)
        );
        assert_eq!(
            custom_degree_note(c, GlobalHarmony::default(), 1, 1),
            Some(72)
        );
        assert_eq!(
            custom_degree_note(c, GlobalHarmony::default(), -1, 1),
            Some(60)
        );
        assert_eq!(
            custom_degree_note(c, GlobalHarmony::default(), 0, 2),
            Some(62)
        );
        for degree in 1..=7 {
            let base = custom_degree_note(c, GlobalHarmony::default(), 0, degree).unwrap();
            let inverted = custom_degree_note(
                c,
                GlobalHarmony {
                    inversion: 2,
                    ..GlobalHarmony::default()
                },
                -1,
                degree,
            )
            .unwrap();
            assert_eq!(base % 12, inverted % 12);
        }
    }

    #[test]
    fn voice_leading_retains_expanded_harmony_and_global_voicing() {
        let previous = chord(0);
        let next = SavedChord {
            root: 67,
            ..chord(0)
        };
        let globals = GlobalHarmony {
            complexity: 3,
            voicing: 2,
            ..GlobalHarmony::default()
        };
        let previous_notes = resolve_chord(previous, globals);
        let base = resolve_chord(next, globals);
        let led = super::super::lead_with_globals(
            next,
            next.spread,
            previous,
            previous_notes,
            base,
            false,
            &[],
            globals,
        );
        assert_eq!(led.as_slice(), &[59, 66, 74, 79, 81]);
        assert!(
            super::super::movement(previous_notes, led)
                < super::super::movement(previous_notes, base)
        );

        let globals = GlobalHarmony {
            voicing: 2,
            ..GlobalHarmony::default()
        };
        let previous_notes = resolve_chord(previous, globals);
        let base = resolve_chord(next, globals);
        let led = super::super::lead_with_globals(
            next,
            next.spread,
            previous,
            previous_notes,
            base,
            false,
            &[],
            globals,
        );
        assert_eq!(led.as_slice(), &[59, 67, 74]);
    }

    #[test]
    fn signed_intermediates_survive_opposite_loop_inversion_and_octave_offsets() {
        let low = SavedChord {
            root: 0,
            ..chord(0)
        };
        assert_eq!(
            resolve_loop_chord(
                low,
                GlobalHarmony {
                    inversion: -3,
                    ..GlobalHarmony::default()
                },
                3
            )
            .as_slice(),
            &[0, 4, 7]
        );
        let high = SavedChord {
            root: 126,
            ..chord(0)
        };
        assert_eq!(
            resolve_loop_chord(high, GlobalHarmony::default(), -3).as_slice(),
            &[114, 118, 121]
        );
        assert_eq!(
            custom_degree_note(
                high,
                GlobalHarmony {
                    key: 6,
                    ..GlobalHarmony::default()
                },
                0,
                2
            ),
            None
        );
        let pitch = custom_degree_pitch(
            high,
            GlobalHarmony {
                key: 6,
                ..GlobalHarmony::default()
            },
            0,
            2,
        )
        .unwrap();
        assert_eq!(pitch, 128);
        assert_eq!(pitch - 12, 116);
    }
}
