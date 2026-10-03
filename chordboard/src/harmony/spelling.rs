use super::scale_tones;

// Explicit enharmonic tonic choices retain their identity even on the same MIDI pitch.
pub const KEY_CHOICES: [(&str, u8, i32); 17] = [
    ("C", 0, 0),
    ("C#", 1, 1),
    ("Db", 1, 2),
    ("D", 2, 0),
    ("D#", 3, 1),
    ("Eb", 3, 2),
    ("E", 4, 0),
    ("F", 5, 0),
    ("F#", 6, 1),
    ("Gb", 6, 2),
    ("G", 7, 0),
    ("G#", 8, 1),
    ("Ab", 8, 2),
    ("A", 9, 0),
    ("A#", 10, 1),
    ("Bb", 10, 2),
    ("B", 11, 0),
];
const NATURAL: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
const LETTERS: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
const SHARP_LETTER: [usize; 12] = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6];
const FLAT_LETTER: [usize; 12] = [0, 1, 1, 2, 2, 3, 4, 4, 5, 6, 6, 6];

fn delta(pitch: u8, letter: usize) -> i8 {
    ((pitch as i16 - NATURAL[letter] as i16 + 6).rem_euclid(12) - 6) as i8
}
fn degree_letter(scale: u8, degree: usize) -> usize {
    match scale {
        6 => [0, 1, 2, 4, 5][degree],
        7 => [0, 2, 3, 4, 6][degree],
        _ => degree,
    }
}
fn tonic_letter(key: u8, scale: u8, preference: i32) -> usize {
    let key = key % 12;
    let sharp = SHARP_LETTER[key as usize];
    let flat = FLAT_LETTER[key as usize];
    match preference {
        1 => sharp,
        2 => flat,
        _ => {
            let signature_scale = if scale == 2 { 1 } else { scale };
            let cost = |letter| {
                scale_tones(signature_scale)
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| {
                        delta(
                            (key + n) % 12,
                            (letter + degree_letter(signature_scale, i)) % 7,
                        )
                        .unsigned_abs() as u32
                    })
                    .sum::<u32>()
            };
            if cost(sharp) < cost(flat) || (cost(sharp) == cost(flat) && key == 6) {
                sharp
            } else {
                flat
            }
        }
    }
}
fn render(letter: usize, accidental: i8) -> String {
    format!(
        "{}{}",
        LETTERS[letter],
        if accidental < 0 {
            "b".repeat((-accidental) as usize)
        } else {
            "#".repeat(accidental as usize)
        }
    )
}
pub fn key_name(key: u8, scale: u8, preference: i32) -> String {
    let letter = tonic_letter(key, scale, preference);
    render(letter, delta(key % 12, letter))
}
fn spell(pitch: u8, key: u8, scale: u8, preference: i32) -> (usize, i8) {
    let pitch = pitch % 12;
    let tonic = tonic_letter(key, scale, preference);
    if let Some(degree) = scale_tones(scale)
        .iter()
        .position(|&n| (key + n) % 12 == pitch)
    {
        let letter = (tonic + degree_letter(scale, degree)) % 7;
        return (letter, delta(pitch, letter));
    }
    let signature_scale = if scale == 2 { 1 } else { scale };
    let signature: i32 = scale_tones(signature_scale)
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            delta(
                (key + n) % 12,
                (tonic + degree_letter(signature_scale, i)) % 7,
            ) as i32
        })
        .sum();
    let letter = if signature < 0 {
        FLAT_LETTER[pitch as usize]
    } else {
        SHARP_LETTER[pitch as usize]
    };
    (letter, delta(pitch, letter))
}
pub fn note_name_in_key(pitch: u8, key: u8, scale: u8, preference: i32) -> String {
    let (letter, accidental) = spell(pitch, key, scale, preference);
    render(letter, accidental)
}
pub fn midi_note_name_in_key(note: u8, key: u8, scale: u8, preference: i32) -> String {
    let (letter, accidental) = spell(note, key, scale, preference);
    let octave = (note as i16 - NATURAL[letter] as i16 - accidental as i16).div_euclid(12) - 1;
    format!("{}{octave}", render(letter, accidental))
}

pub fn roman_in_key(root: u8, quality: u8, key: u8, scale: u8, preference: i32) -> String {
    let (letter, _) = spell(root, key, scale, preference);
    let degree = (letter + 7 - tonic_letter(key, scale, preference)) % 7;
    let accidental =
        ((root as i16 - key as i16 - NATURAL[degree] as i16 + 6).rem_euclid(12) - 6) as i8;
    let mut result = if accidental < 0 {
        "b".repeat((-accidental) as usize)
    } else {
        "#".repeat(accidental as usize)
    };
    result.push_str(if [1, 4, 5, 8, 9, 10].contains(&quality) {
        ["i", "ii", "iii", "iv", "v", "vi", "vii"][degree]
    } else {
        ["I", "II", "III", "IV", "V", "VI", "VII"][degree]
    });
    let suffix = match quality {
        2 | 4 => "7",
        3 => "maj7",
        5 => "dim",
        6 => "+",
        7 | 8 => "6",
        9 => "dim7",
        10 => "m7b5",
        11 => "5",
        _ => "",
    };
    if !suffix.is_empty() {
        result.push(' ');
        result.push_str(suffix);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_use_the_correct_letters_and_accidentals() {
        for (key, scale, pref, expected) in [
            (5, 0, 0, vec!["F", "G", "A", "Bb", "C", "D", "E"]),
            (1, 0, 2, vec!["Db", "Eb", "F", "Gb", "Ab", "Bb", "C"]),
            (6, 0, 1, vec!["F#", "G#", "A#", "B", "C#", "D#", "E#"]),
            (6, 0, 2, vec!["Gb", "Ab", "Bb", "Cb", "Db", "Eb", "F"]),
            (2, 2, 0, vec!["D", "E", "F", "G", "A", "Bb", "C#"]),
        ] {
            let actual: Vec<_> = scale_tones(scale)
                .iter()
                .map(|n| note_name_in_key((key + n) % 12, key, scale, pref))
                .collect();
            assert_eq!(actual, expected);
        }
        assert_eq!(midi_note_name_in_key(59, 6, 0, 2), "Cb4");
        assert_eq!(midi_note_name_in_key(60, 1, 0, 1), "B#3");
        assert_eq!(roman_in_key(11, 0, 6, 0, 2), "IV");
        assert_eq!(roman_in_key(5, 5, 6, 0, 1), "vii dim");
    }
}
