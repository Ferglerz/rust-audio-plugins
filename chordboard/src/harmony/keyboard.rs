use super::{keyboard_root, row_quality, scale_tones, KEY_COLUMNS};

pub const KEYBOARD_LAYOUTS: [&str; 4] = ["Chromatic", "Fifths", "Degrees", "Passing"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyboardChord {
    /// Semitones above the keyboard's base C, including octave repetitions.
    pub root: u8,
    pub quality: u8,
}

fn degree_root(scale: u8, column: usize) -> u8 {
    let tones = scale_tones(scale);
    tones[column % tones.len()] + (column / tones.len()) as u8 * 12
}

fn degree_quality(scale: u8, column: usize) -> u8 {
    // Pentatonic layouts use the triads of their parent major/natural-minor scale.
    let parent = match scale {
        6 => 0,
        7 => 1,
        _ => scale,
    };
    let tones = scale_tones(parent);
    let root = degree_root(scale, column) % 12;
    let degree = tones.iter().position(|&n| n == root).unwrap_or(0);
    let third = (tones[(degree + 2) % 7] + 12 - root) % 12;
    let fifth = (tones[(degree + 4) % 7] + 12 - root) % 12;
    match (third, fifth) {
        (3, 6) => 5,
        (3, 7) => 1,
        (4, 8) => 6,
        _ => 0,
    }
}

fn shade(quality: u8, brighter: bool) -> u8 {
    let qualities = [5, 1, 0, 6]; // Diminished, minor, major, augmented.
    let index = qualities.iter().position(|&q| q == quality).unwrap_or(2);
    qualities[if brighter {
        (index + 1).min(3)
    } else {
        index.saturating_sub(1)
    }]
}

pub fn keyboard_chord(layout: u8, key: u8, scale: u8, index: usize) -> Option<KeyboardChord> {
    if index >= KEY_COLUMNS * 3 {
        return None;
    }
    let row = index / KEY_COLUMNS;
    let column = index % KEY_COLUMNS;
    if layout < 2 {
        return Some(KeyboardChord {
            root: (keyboard_root(layout == 1, column)? + key) % 12,
            quality: row_quality(row),
        });
    }
    if layout == 3 && row == 0 {
        // The number key just before each Q-row degree supplies a chromatic
        // approach from below. Leave natural semitone gaps and key 1 empty.
        // This keeps 2 between Q and W, 3 between W and E, and so on.
        if column == 0 {
            return None;
        }
        let low = degree_root(scale, column - 1);
        let high = degree_root(scale, column);
        if high - low <= 1 {
            return None;
        }
        return Some(KeyboardChord {
            root: key + high - 1,
            quality: degree_quality(scale, column - 1),
        });
    }
    let quality = degree_quality(scale, column);
    Some(KeyboardChord {
        root: key + degree_root(scale, column),
        quality: match row {
            0 => shade(quality, true),
            2 => shade(quality, false),
            _ => quality,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn degrees_harmonize_the_scale_and_shade_triads() {
        for (scale, expected) in [
            (0, [0, 1, 1, 0, 0, 1, 5]),
            (1, [1, 5, 0, 1, 1, 0, 0]),
            (2, [1, 5, 6, 1, 0, 0, 5]),
        ] {
            for (column, quality) in expected.into_iter().enumerate() {
                let q = keyboard_chord(2, 5, scale, KEY_COLUMNS + column).unwrap();
                assert_eq!(q.quality, quality);
                assert_eq!(q.root, 5 + scale_tones(scale)[column]);
                assert_eq!(
                    keyboard_chord(2, 5, scale, column).unwrap().quality,
                    shade(quality, true)
                );
                assert_eq!(
                    keyboard_chord(2, 5, scale, KEY_COLUMNS * 2 + column)
                        .unwrap()
                        .quality,
                    shade(quality, false)
                );
            }
        }
    }
    #[test]
    fn passing_row_contains_only_missing_semitones_in_order() {
        for scale in 0..8 {
            let roots: Vec<_> = (0..KEY_COLUMNS)
                .filter_map(|i| keyboard_chord(3, 0, scale, i))
                .collect();
            assert!(roots.windows(2).all(|pair| pair[0].root < pair[1].root));
            assert!(roots
                .iter()
                .all(|c| !scale_tones(scale).contains(&(c.root % 12))));
        }
        let roots: Vec<_> = (0..KEY_COLUMNS)
            .filter_map(|i| keyboard_chord(3, 0, 2, i).map(|c| c.root))
            .collect();
        assert!(roots.contains(&10));
        assert!(keyboard_chord(3, 0, 0, 0).is_none());
        for (column, expected) in [
            (1, Some(1)),
            (2, Some(3)),
            (3, None),
            (4, Some(6)),
            (5, Some(8)),
            (6, Some(10)),
            (7, None),
        ] {
            assert_eq!(keyboard_chord(3, 0, 0, column).map(|c| c.root), expected);
        }
        assert_eq!(keyboard_chord(2, 10, 0, KEY_COLUMNS + 7).unwrap().root, 22);
    }
}
