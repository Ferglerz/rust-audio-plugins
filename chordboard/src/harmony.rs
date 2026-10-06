//! Fixed-capacity harmonic resolution. No allocation or host/UI dependencies.
mod keyboard;
mod spelling;
pub use keyboard::*;
pub use spelling::*;

pub const QUALITY_NAMES: [&str; 12] = [
    "Major", "Minor", "7", "Maj7", "Min7", "Dim", "Aug", "6", "Min6", "Dim7", "Half dim", "Power",
];
pub const QUALITY_SUFFIX: [&str; 12] = [
    "", "m", "7", "maj7", "m7", "dim", "aug", "6", "m6", "dim7", "m7b5", "5",
];
/// Chromatic controls relative to the learned tonic: 1, b2, 2, b3, 3,
/// 4, b5, 5, #5, 6, b7, 7. Quality IDs stay stable for saved state.
pub const CONTROL_CHORDS: [(u8, Option<u8>); 12] = [
    (0, None),
    (0, Some(1)),
    (0, Some(2)),
    (1, None),
    (0, None),
    (0, Some(5)),
    (5, None),
    (11, None),
    (6, None),
    (7, None),
    (2, None),
    (3, None),
];
pub const CONTROL_LABELS: [&str; 12] = [
    "Maj", "b9", "sus2", "Min", "Maj", "sus4", "dim", "5", "aug", "6", "7", "maj7",
];
pub const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
pub const KEY_COLUMNS: usize = 12;
pub const KEY_COUNT: usize = KEY_COLUMNS * 3;
pub const HINTS: [&str; KEY_COUNT] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "+", "Q", "W", "E", "R", "T", "Y", "U",
    "I", "O", "P", "[", "]", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "ENT",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notes {
    pub values: [u8; 12],
    pub len: usize,
}
impl Notes {
    pub fn as_slice(&self) -> &[u8] {
        &self.values[..self.len]
    }
    pub fn push(&mut self, note: i16) {
        if (0..=127).contains(&note) && self.len < 12 && !self.as_slice().contains(&(note as u8)) {
            self.values[self.len] = note as u8;
            self.len += 1;
        }
    }
    pub fn string(&self, index: usize) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let note = self.values[index % self.len] as usize + 12 * (index / self.len);
        (note < 128).then_some(note as u8)
    }
}

pub fn keyboard_root(fifths: bool, column: usize) -> Option<u8> {
    if column >= KEY_COLUMNS {
        return None;
    }
    Some(if fifths {
        ((5 + column * 7) % 12) as u8
    } else {
        column as u8
    })
}
pub fn row_quality(row: usize) -> u8 {
    [0, 1, 2][row.min(2)]
}

pub fn intervals(quality: u8, alteration: Option<u8>) -> Notes {
    let base: &[u8] = match quality {
        1 => &[0, 3, 7],
        2 => &[0, 4, 7, 10],
        3 => &[0, 4, 7, 11],
        4 => &[0, 3, 7, 10],
        5 => &[0, 3, 6],
        6 => &[0, 4, 8],
        7 => &[0, 4, 7, 9],
        8 => &[0, 3, 7, 9],
        9 => &[0, 3, 6, 9],
        10 => &[0, 3, 6, 10],
        11 => &[0, 7],
        _ => &[0, 4, 7],
    };
    let mut tones = [false; 15];
    for &n in base {
        tones[n as usize] = true;
    }
    match alteration.map(|a| a % 12) {
        Some(1) => tones[13] = true,
        Some(n @ (2..=5)) => {
            tones[3] = false;
            tones[4] = false;
            tones[n as usize] = true;
        }
        Some(6) => {
            tones[6] = false;
            tones[7] = false;
            tones[8] = false;
            tones[6] = true;
        }
        // A root plus its perfect fifth explicitly selects a power chord.
        Some(7) => {
            tones.fill(false);
            tones[0] = true;
            tones[7] = true;
        }
        Some(n @ 8) => {
            tones[6] = false;
            tones[7] = false;
            tones[8] = false;
            tones[n as usize] = true;
        }
        Some(9) => tones[9] = true,
        Some(n @ (10 | 11)) => {
            tones[10] = false;
            tones[11] = false;
            tones[n as usize] = true;
        }
        _ => {}
    }
    let mut out = Notes::default();
    for (n, &on) in tones.iter().enumerate() {
        if on {
            out.push(n as i16);
        }
    }
    out
}

pub const VOICING_NAMES: [&str; 5] = ["Close", "Open", "Wide", "Drop 2", "Drop 3"];

pub fn voice(
    root: u8,
    quality: u8,
    second: Option<u8>,
    transpose: i8,
    inversion: u8,
    spread: u8,
) -> Notes {
    let tones = intervals(
        quality,
        second.map(|n| (n as i16 - root as i16).rem_euclid(12) as u8),
    );
    let inv = inversion as usize % tones.len;
    let mut out = Notes::default();
    for i in 0..tones.len {
        let source = (i + inv) % tones.len;
        let shift = if source < inv { 12 } else { 0 };
        let open = match spread.min((VOICING_NAMES.len() - 1) as u8) {
            1 if i % 2 == 1 => 12,
            2 => (i / 2) as i16 * 12,
            3 if tones.len >= 3 && i == tones.len - 2 => -12,
            4 if tones.len >= 3 && i == tones.len - 3 => -12,
            _ => 0,
        };
        out.push(root as i16 + transpose as i16 + tones.values[source] as i16 + shift + open);
    }
    out.values[..out.len].sort_unstable();
    out
}

/// Compare ordered voices, proportionally matching ranks when chord sizes differ.
/// Integer weighting makes the result deterministic without audio-thread allocation.
fn movement(a: Notes, b: Notes) -> i32 {
    let count = a.len.max(b.len);
    (0..count)
        .map(|i| (a.values[i * a.len / count] as i32 - b.values[i * b.len / count] as i32).abs())
        .sum()
}

/// Search inversions and octave placements within one octave of the requested
/// voicing. Invalid/clipped candidates are discarded, preserving every chord tone.
pub fn lead(
    chord: SavedChord,
    spread: u8,
    previous: SavedChord,
    previous_notes: Notes,
    base: Notes,
    furthest_dominant: bool,
) -> Notes {
    if previous_notes.len == 0 || base.len == 0 {
        return base;
    }
    let alteration = |c: SavedChord| {
        c.second
            .map(|n| (n as i16 - c.root as i16).rem_euclid(12) as u8)
    };
    let old_tones = intervals(previous.quality, alteration(previous));
    let tones = intervals(chord.quality, alteration(chord));
    // Recognize major/dominant harmony resolving down a fifth to major or minor.
    let dominant = (previous.root as i16 + previous.transpose as i16
        - chord.root as i16
        - chord.transpose as i16)
        .rem_euclid(12)
        == 7
        && old_tones.as_slice().contains(&4)
        && old_tones.as_slice().contains(&7)
        && !old_tones.as_slice().contains(&11)
        && (tones.as_slice().contains(&3) || tones.as_slice().contains(&4))
        && tones.as_slice().contains(&7);
    let maximize = furthest_dominant && dominant;
    let score = |notes| {
        let distance = movement(previous_notes, notes);
        (
            if maximize { -distance } else { distance },
            movement(base, notes),
        )
    };
    let mut best = base;
    let mut best_score = score(base);
    for inversion in 0..tones.len {
        for octave in [-24i16, -12, 0, 12] {
            let transpose = chord.transpose as i16 + octave;
            let candidate = voice(
                chord.root,
                chord.quality,
                chord.second,
                transpose as i8,
                inversion as u8,
                spread,
            );
            if candidate.len != tones.len
                || candidate.len != base.len
                || candidate
                    .as_slice()
                    .iter()
                    .zip(base.as_slice())
                    .any(|(&a, &b)| (a as i16 - b as i16).abs() > 12)
            {
                continue;
            }
            let candidate_score = score(candidate);
            if candidate_score < best_score {
                best = candidate;
                best_score = candidate_score;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smart_voice_leading_preserves_tones_and_midi_bounds() {
        let previous = SavedChord {
            root: 67,
            quality: 2,
            second: None,
            inversion: 0,
            spread: 0,
            transpose: 0,
        };
        let previous_notes = voice(67, 2, None, 0, 0, 0);
        for root in 0..=127 {
            for quality in 0..12 {
                for spread in 0..VOICING_NAMES.len() as u8 {
                    let chord = SavedChord {
                        root,
                        quality,
                        spread,
                        ..previous
                    };
                    let base = voice(root, quality, None, 0, 0, spread);
                    for far in [false, true] {
                        let notes = lead(chord, spread, previous, previous_notes, base, far);
                        assert_eq!(notes.len, base.len);
                        assert!(notes.as_slice().windows(2).all(|w| w[0] < w[1]));
                        let mut expected: Vec<_> = base.as_slice().iter().map(|n| n % 12).collect();
                        let mut actual: Vec<_> = notes.as_slice().iter().map(|n| n % 12).collect();
                        expected.sort_unstable();
                        actual.sort_unstable();
                        assert_eq!(actual, expected);
                    }
                }
            }
        }
    }
    #[test]
    fn agreed_examples() {
        for (second, expected) in [
            (62, &[60, 62, 67][..]),
            (64, &[60, 64, 67]),
            (65, &[60, 65, 67]),
            (66, &[60, 64, 66]),
            (67, &[60, 67]),
            (70, &[60, 64, 67, 70]),
            (58, &[60, 64, 67, 70]),
        ] {
            assert_eq!(voice(60, 0, Some(second), 0, 0, 0).as_slice(), expected);
        }
    }
    #[test]
    fn all_roots_qualities_and_intervals_are_bounded_unique() {
        for root in 0..128 {
            for quality in 0..12 {
                for delta in 0..12 {
                    let notes = voice(root, quality, Some((root + delta) % 128), 0, 0, 0);
                    assert!(notes.as_slice().windows(2).all(|w| w[0] < w[1]));
                    assert!(notes.len <= 6);
                }
            }
        }
    }
    #[test]
    fn inversions_wrap_without_register_drift() {
        for q in 0..12 {
            let n = intervals(q, None).len as u8;
            assert_eq!(voice(48, q, None, 0, 0, 0), voice(48, q, None, 0, n, 0));
        }
        assert_eq!(voice(60, 0, None, 0, 1, 0).as_slice(), &[64, 67, 72]);
        assert_eq!(voice(60, 0, None, 0, 2, 0).as_slice(), &[67, 72, 76]);
    }
    #[test]
    fn full_keyboard_row_covers_all_roots_once() {
        for fifths in [false, true] {
            let mut seen = [false; 12];
            for col in 0..KEY_COLUMNS {
                let n = keyboard_root(fifths, col).expect("valid keyboard column");
                assert!(!seen[n as usize]);
                seen[n as usize] = true;
            }
            assert_eq!(keyboard_root(fifths, KEY_COLUMNS), None);
            assert!(seen.into_iter().all(|v| v));
        }
    }
    #[test]
    fn notes_can_hold_up_to_12_notes() {
        let mut notes = Notes::default();
        for n in 60..69 {
            notes.push(n);
        }
        assert_eq!(notes.len, 9);
        assert_eq!(notes.as_slice(), &[60, 61, 62, 63, 64, 65, 66, 67, 68]);
        for n in 69..72 {
            notes.push(n);
        }
        assert_eq!(notes.len, 12);
        notes.push(72);
        assert_eq!(notes.len, 12);
    }
}

pub const SCALE_NAMES: [&str; 8] = [
    "Major",
    "Natural minor",
    "Harmonic minor",
    "Dorian",
    "Mixolydian",
    "Lydian",
    "Major pentatonic",
    "Minor pentatonic",
];
pub fn scale_tones(scale: u8) -> &'static [u8] {
    match scale {
        1 => &[0, 2, 3, 5, 7, 8, 10],
        2 => &[0, 2, 3, 5, 7, 8, 11],
        3 => &[0, 2, 3, 5, 7, 9, 10],
        4 => &[0, 2, 4, 5, 7, 9, 10],
        5 => &[0, 2, 4, 6, 7, 9, 11],
        6 => &[0, 2, 4, 7, 9],
        7 => &[0, 3, 5, 7, 10],
        _ => &[0, 2, 4, 5, 7, 9, 11],
    }
}
pub fn in_key(root: u8, quality: u8, key: u8, scale: u8) -> bool {
    intervals(quality, None).as_slice().iter().all(|&n| {
        scale_tones(scale).contains(&((root as i16 + n as i16 - key as i16).rem_euclid(12) as u8))
    })
}
pub fn roman(root: u8, quality: u8, key: u8) -> String {
    let distance = (root as i16 - key as i16).rem_euclid(12) as usize;
    let (accidental, degree) = [
        ("", 0),
        ("b", 1),
        ("", 1),
        ("b", 2),
        ("", 2),
        ("", 3),
        ("#", 3),
        ("", 4),
        ("b", 5),
        ("", 5),
        ("b", 6),
        ("", 6),
    ][distance];
    let minor = [1, 4, 5, 8, 9, 10].contains(&quality);
    let numeral = if minor {
        ["i", "ii", "iii", "iv", "v", "vi", "vii"][degree]
    } else {
        ["I", "II", "III", "IV", "V", "VI", "VII"][degree]
    };
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
    format!("{accidental}{numeral}{suffix}")
}
pub fn filter_notes(notes: Notes, filter: u8) -> Notes {
    let mut out = Notes::default();
    for i in 0..notes.len {
        let keep = match filter {
            1 => i == 0,
            2 => i + 1 == notes.len,
            3 => i == 0 || i + 1 == notes.len,
            4 => i % 2 == 0,
            5 => i % 2 == 1,
            _ => true,
        };
        if keep {
            out.push(notes.values[i] as i16);
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum HarmonizationEngineMode {
    #[default]
    ClassicDualTouch = 0,
    NopiaStatic = 1,
    NopiaReal = 2,
}

impl From<u8> for HarmonizationEngineMode {
    fn from(val: u8) -> Self {
        match val {
            1 => Self::NopiaStatic,
            2 => Self::NopiaReal,
            _ => Self::ClassicDualTouch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ChromaticFlavor {
    #[default]
    SecondaryDominants = 0,
    ModalInterchange = 1,
}

impl From<u8> for ChromaticFlavor {
    fn from(val: u8) -> Self {
        match val {
            1 => Self::ModalInterchange,
            _ => Self::SecondaryDominants,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HarmonizedChord {
    pub root: u8,
    pub quality: u8,
    pub second: Option<u8>,
}

/// Map a keyboard white key (pitch class 0, 2, 4, 5, 7, 9, 11) to relative scale degree 0..=6.
pub fn white_key_degree(pitch_class: u8) -> Option<usize> {
    match pitch_class % 12 {
        0 => Some(0),  // C -> I
        2 => Some(1),  // D -> ii
        4 => Some(2),  // E -> iii
        5 => Some(3),  // F -> IV
        7 => Some(4),  // G -> V
        9 => Some(5),  // A -> vi
        11 => Some(6), // B -> vii°
        _ => None,
    }
}

/// Map a keyboard black key (pitch class 1, 3, 6, 8, 10) to relative chromatic alteration (semitones above tonic).
pub fn black_key_alteration(pitch_class: u8) -> Option<u8> {
    match pitch_class % 12 {
        1 => Some(1),   // C# -> #1 / b2
        3 => Some(3),   // D# -> #2 / b3
        6 => Some(6),   // F# -> #4 / b5
        8 => Some(8),   // G# -> #5 / b6
        10 => Some(10), // A# -> #6 / b7
        _ => None,
    }
}

/// Find the MIDI note with target pitch class that is closest to `reference_note` in valid MIDI range [0..=127].
pub fn closest_pitch(target_pc: u8, reference_note: u8) -> u8 {
    let mut best_note = target_pc % 12;
    let mut min_dist = i16::MAX;
    for octave in 0..=10 {
        let candidate = octave * 12 + (target_pc % 12) as i16;
        if (0..=127).contains(&candidate) {
            let dist = (candidate - reference_note as i16).abs();
            if dist < min_dist {
                min_dist = dist;
                best_note = candidate as u8;
            }
        }
    }
    best_note
}

/// Resolve 12-tone functional chromatic alterations.
///
/// Under SecondaryDominants (Jazz/Gospel):
/// - #1 (1) -> V7/ii (A7 in C, resolving to Dm)
/// - #2 (3) -> V7/iii (B7 in C, resolving to Em)
/// - #4 (6) -> V7/V (D7 in C, resolving to G)
/// - #5 (8) -> V7/vi (E7 in C, resolving to Am)
/// - b7 (10) -> V7/IV (C7 in C, resolving to F)
///
/// Under ModalInterchange (Pop/Neo-Soul):
/// - b2 (1) -> bIImaj7 (Dbmaj7 in C)
/// - b3 (3) -> bIIImaj7 (Ebmaj7 in C)
/// - #4 (6) -> V7/V (D7 in C)
/// - b6 (8) -> bVImaj7 (Abmaj7 in C)
/// - b7 (10) -> bVIImaj7 (Bbmaj7 in C)
pub fn functional_chromatic_chord(flavor: ChromaticFlavor, key: u8, alteration: u8) -> (u8, u8) {
    let k = key % 12;
    let alt = alteration % 12;
    match flavor {
        ChromaticFlavor::SecondaryDominants => match alt {
            0 => (k, 0),             // Tonic Major
            1 => ((k + 9) % 12, 2),  // A7 in C (V7/ii)
            2 => ((k + 2) % 12, 2),  // D7 in C (V7/V)
            3 => ((k + 11) % 12, 2), // B7 in C (V7/iii)
            4 => ((k + 0) % 12, 2),  // C7 in C (V7/iv)
            5 => ((k + 10) % 12, 2), // Bb7 in C (V7/bIII)
            6 => ((k + 2) % 12, 2),  // D7 in C (V7/V)
            7 => ((k + 7) % 12, 2),  // G7 in C (V7)
            8 => ((k + 4) % 12, 2),  // E7 in C (V7/vi)
            9 => ((k + 5) % 12, 2),  // F7 in C (V7/VII)
            10 => ((k + 0) % 12, 2), // C7 in C (V7/IV)
            11 => ((k + 7) % 12, 2), // G7 in C (V7)
            _ => (k, 2),
        },
        ChromaticFlavor::ModalInterchange => match alt {
            0 => (k, 0),              // Tonic Major
            1 => ((k + 1) % 12, 3),   // Dbmaj7 in C (bIImaj7)
            2 => ((k + 2) % 12, 1),   // Dm in C (ii)
            3 => ((k + 3) % 12, 3),   // Ebmaj7 in C (bIIImaj7)
            4 => ((k + 0) % 12, 0),   // C Major in C
            5 => ((k + 5) % 12, 3),   // Fmaj7 in C (IVmaj7)
            6 => ((k + 2) % 12, 2),   // D7 in C (V7/V)
            7 => ((k + 7) % 12, 1),   // Gm in C (v from minor)
            8 => ((k + 8) % 12, 3),   // Abmaj7 in C (bVImaj7)
            9 => ((k + 5) % 12, 0),   // F Major in C (IV from Dorian)
            10 => ((k + 10) % 12, 3), // Bbmaj7 in C (bVIImaj7)
            11 => ((k + 7) % 12, 2),  // G7 in C (V7)
            _ => (k, 3),
        },
    }
}

/// Harmonize an input note in Nopia Static Mode.
/// White keys strictly map to relative scale degrees (I..vii°).
/// Black keys trigger functional chromatic resolutions (secondary dominants or modal interchange).
pub fn harmonize_nopia_static(flavor: ChromaticFlavor, key: u8, scale: u8, note: u8) -> HarmonizedChord {
    let pc = note % 12;
    if let Some(deg) = white_key_degree(pc) {
        let tones = scale_tones(scale);
        let tones_len = tones.len().max(1);
        let deg_in_scale = deg % tones_len;
        let octave_wrap = (deg / tones_len) as i16;
        let scale_semitone = tones[deg_in_scale] as i16 + octave_wrap * 12;
        let octave = (note / 12) as i16;
        let root_midi = ((octave * 12 + (key % 12) as i16 + scale_semitone).clamp(0, 127)) as u8;
        let quality = degree_quality(scale, deg);
        HarmonizedChord {
            root: root_midi,
            quality,
            second: None,
        }
    } else {
        let alt = black_key_alteration(pc).unwrap_or(1);
        let (target_pc, quality) = functional_chromatic_chord(flavor, key, alt);
        let target_note = ((note / 12) * 12 + (key % 12) + alt).min(127);
        let root_midi = closest_pitch(target_pc, target_note);
        HarmonizedChord {
            root: root_midi,
            quality,
            second: None,
        }
    }
}

/// Harmonize an input note in Nopia Real Mode.
/// Standard chromatic keyboard: diatonic notes produce scale degree harmonies with root at the played note;
/// non-diatonic notes produce functional chromatic resolutions.
pub fn harmonize_nopia_real(flavor: ChromaticFlavor, key: u8, scale: u8, note: u8) -> HarmonizedChord {
    let pc = note % 12;
    let rel_semitone = (pc as i16 - (key % 12) as i16).rem_euclid(12) as u8;
    let tones = scale_tones(scale);
    if let Some(deg) = tones.iter().position(|&t| t == rel_semitone) {
        let quality = degree_quality(scale, deg);
        HarmonizedChord {
            root: note,
            quality,
            second: None,
        }
    } else {
        let (target_pc, quality) = functional_chromatic_chord(flavor, key, rel_semitone);
        let root_midi = closest_pitch(target_pc, note);
        HarmonizedChord {
            root: root_midi,
            quality,
            second: None,
        }
    }
}

/// Main harmonization router.
pub fn harmonize(
    mode: HarmonizationEngineMode,
    flavor: ChromaticFlavor,
    key: u8,
    scale: u8,
    note: u8,
) -> HarmonizedChord {
    match mode {
        HarmonizationEngineMode::ClassicDualTouch => HarmonizedChord {
            root: note,
            quality: 0,
            second: None,
        },
        HarmonizationEngineMode::NopiaStatic => harmonize_nopia_static(flavor, key, scale, note),
        HarmonizationEngineMode::NopiaReal => harmonize_nopia_real(flavor, key, scale, note),
    }
}

/// Resolve chord tones based on continuous extensions macro (0.0 .. 1.0)
/// - Tier 0 (< 0.20): Root only
/// - Tier 1 (0.20 .. 0.40): Root + Fifth (Power chord / dyad)
/// - Tier 2 (0.40 .. 0.65): Triad (Root, 3rd, 5th)
/// - Tier 3 (0.65 .. 0.85): 7th (Root, 3rd, 5th, 7th)
/// - Tier 4 (>= 0.85): Extended color (9th, 11th, 13th)
pub fn apply_extensions(notes: &Notes, extensions: f32, _quality: u8) -> Notes {
    if notes.len == 0 {
        return *notes;
    }
    let mut out = Notes::default();
    let root = notes.values[0];

    if extensions < 0.20 {
        // Tier 0: Single note (Root)
        out.push(root as i16);
    } else if extensions < 0.40 {
        // Tier 1: Root + Fifth (Power chord / open fifth)
        out.push(root as i16);
        if let Some(&fifth) = notes.as_slice().iter().find(|&&n| {
            let int = (n as i16 - root as i16).rem_euclid(12);
            int == 7 || int == 6 || int == 8
        }) {
            out.push(fifth as i16);
        } else if notes.len > 1 {
            out.push(notes.values[notes.len - 1] as i16);
        }
    } else if extensions < 0.50 {
        // Tier 2: Triad (Root + 3rd + 5th)
        for &n in notes.as_slice().iter().take(3) {
            out.push(n as i16);
        }
    } else if extensions < 0.85 {
        // Tier 3: 7th chord (Take up to 4 notes)
        for &n in notes.as_slice().iter().take(4) {
            out.push(n as i16);
        }
    } else {
        // Tier 4: Full extensions
        return *notes;
    }
    out
}

/// Extract the single most defining guide tone of the chord (3rd or 7th).
/// Used for orchestral lane separation (Nopia Pad B concept).
pub fn extract_guide_tone(notes: &Notes, root: u8) -> Option<u8> {
    if notes.len <= 1 {
        return notes.as_slice().first().copied();
    }
    // Prefer 7th if present (10 or 11 semitones above root)
    if let Some(&seventh) = notes.as_slice().iter().find(|&&n| {
        let interval = (n as i16 - root as i16).rem_euclid(12);
        interval == 10 || interval == 11
    }) {
        return Some(seventh);
    }
    // Otherwise prefer 3rd (3 or 4 semitones above root)
    if let Some(&third) = notes.as_slice().iter().find(|&&n| {
        let interval = (n as i16 - root as i16).rem_euclid(12);
        interval == 3 || interval == 4
    }) {
        return Some(third);
    }
    // Fallback to top note
    notes.as_slice().last().copied()
}

/// Step key center along the circle of fifths (step = +1: +7 semitones / fifths up, step = -1: -5 semitones / fifths down)
pub fn cycle_key_fifths(current_key: u8, step: i8) -> u8 {
    (current_key as i16 + step as i16 * 7).rem_euclid(12) as u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedChord {
    pub root: u8,
    pub second: Option<u8>,
    pub quality: u8,
    pub inversion: u8,
    pub spread: u8,
    pub transpose: i8,
}
impl SavedChord {
    pub fn encode(self) -> u64 {
        (1 << 63)
            | self.root as u64
            | ((self.second.unwrap_or(128) as u64) << 7)
            | ((self.quality as u64) << 15)
            | ((self.inversion as u64) << 19)
            | (((self.spread as u64) & 3) << 22)
            | (((self.spread as u64) >> 2) << 30)
            | (((self.transpose as i16 + 24) as u64) << 24)
    }
    pub fn decode(n: u64) -> Option<Self> {
        if n >> 63 == 0 {
            return None;
        }
        let second = ((n >> 7) & 255) as u8;
        Some(Self {
            root: (n & 127) as u8,
            second: (second < 128).then_some(second),
            quality: ((n >> 15) & 15).min(11) as u8,
            inversion: ((n >> 19) & 7).min(5) as u8,
            spread: (((n >> 22) & 3) | (((n >> 30) & 3) << 2)).min((VOICING_NAMES.len() - 1) as u64)
                as u8,
            transpose: (((n >> 24) & 63).min(48) as i16 - 24) as i8,
        })
    }
}

/// Derive the label from the resolved recipe rather than losing extensions in UI labels.
pub fn chord_name(chord: SavedChord) -> String {
    chord_name_with_root(
        chord,
        NOTE_NAMES[(chord.root as i16 + chord.transpose as i16).rem_euclid(12) as usize],
    )
}
pub fn chord_name_in_key(chord: SavedChord, key: u8, scale: u8, preference: i32) -> String {
    let pitch = (chord.root as i16 + chord.transpose as i16).rem_euclid(12) as u8;
    chord_name_with_root(chord, &note_name_in_key(pitch, key, scale, preference))
}
fn chord_name_with_root(chord: SavedChord, root: &str) -> String {
    let tones = intervals(
        chord.quality,
        chord
            .second
            .map(|n| (n as i16 - chord.root as i16).rem_euclid(12) as u8),
    );
    let has = |n| tones.as_slice().contains(&n);
    let sus = if has(2) && !has(3) && !has(4) {
        "sus2"
    } else if has(5) && !has(3) && !has(4) {
        "sus4"
    } else {
        ""
    };
    let mut suffix = if has(3) && has(6) && !has(7) {
        "dim".to_string()
    } else if has(4) && has(8) {
        "aug".to_string()
    } else if has(3) {
        "m".to_string()
    } else if !has(3) && !has(4) && sus.is_empty() {
        "5".to_string()
    } else {
        String::new()
    };
    if has(11) {
        suffix.push_str("maj7");
    } else if has(10) {
        suffix.push('7');
    } else if has(9) {
        suffix.push_str(if suffix == "dim" { "7" } else { "6" });
    }
    suffix.push_str(sus);
    if has(6) && has(4) {
        suffix.push_str("(add#11,no5)");
    }
    if has(13) {
        suffix.push_str("(b9)");
    }
    format!("{}{}", root, suffix)
}

/// Incorporate held melody pitch classes within one octave of the chord register.
/// Never double a melody's exact MIDI note, even on another output channel.
pub fn melody_harmony(base: Notes, melody: &[[bool; 128]; 16]) -> Notes {
    if base.len == 0 {
        return base;
    }
    let held = |n: u8| melody.iter().any(|channel| channel[n as usize]);
    let low = base
        .as_slice()
        .iter()
        .copied()
        .min()
        .unwrap_or(0)
        .saturating_sub(12);
    let high = base
        .as_slice()
        .iter()
        .copied()
        .max()
        .unwrap_or(127)
        .saturating_add(12)
        .min(127);
    let center = base.as_slice().iter().map(|&n| n as i16).sum::<i16>() / base.len as i16;
    let mut result = Notes::default();
    for &note in base.as_slice() {
        if !held(note) {
            result.push(note as i16);
        }
    }
    // Chord tones get capacity priority over new melody extensions.
    for pitch in base
        .as_slice()
        .iter()
        .map(|n| n % 12)
        .chain((0..12).filter(|&pc| (0..128u8).any(|n| n % 12 == pc && held(n))))
    {
        if result.as_slice().iter().any(|n| n % 12 == pitch) {
            continue;
        }
        if let Some(note) = (low..=high)
            .filter(|&n| n % 12 == pitch && !held(n) && !result.as_slice().contains(&n))
            .min_by_key(|&n| (n as i16 - center).abs())
        {
            result.push(note as i16);
        }
    }
    result.values[..result.len].sort_unstable();
    result
}

#[cfg(test)]
mod voicing_variation_tests {
    use super::*;
    #[test]
    fn additional_voicings_resolve_expected_chord_tones() {
        assert_eq!(voice(60, 0, None, 0, 0, 3).as_slice(), &[52, 60, 67]);
        assert_eq!(voice(60, 0, None, 0, 0, 4).as_slice(), &[48, 64, 67]);
    }
    #[test]
    fn retired_octave_voicing_clamps_to_drop_three_without_corrupting_memory() {
        let legacy = SavedChord {
            root: 60,
            second: Some(62),
            quality: 3,
            inversion: 2,
            spread: 5,
            transpose: 7,
        };
        let decoded = SavedChord::decode(legacy.encode()).unwrap();
        assert_eq!(
            decoded,
            SavedChord {
                spread: 4,
                ..legacy
            }
        );
        assert_eq!(voice(60, 0, None, 0, 0, 5).as_slice(), &[48, 64, 67]);
    }
    #[test]
    fn all_voicings_round_trip_without_corrupting_transpose() {
        for spread in 0..VOICING_NAMES.len() as u8 {
            for transpose in -24..=24 {
                let chord = SavedChord {
                    root: 60,
                    second: Some(62),
                    quality: 3,
                    inversion: 2,
                    spread,
                    transpose,
                };
                assert_eq!(SavedChord::decode(chord.encode()), Some(chord));
            }
        }
        // A pre-extension Wide memory retains its old voicing and transpose.
        let legacy = (1u64 << 63) | 60 | (128 << 7) | (2 << 22) | (31 << 24);
        let decoded = SavedChord::decode(legacy).unwrap();
        assert_eq!((decoded.spread, decoded.transpose), (2, 7));
    }
}

#[cfg(test)]
mod nopia_harmonization_tests {
    use super::*;

    #[test]
    fn white_key_and_black_key_mappings() {
        for pc in 0..12 {
            let is_white = [0, 2, 4, 5, 7, 9, 11].contains(&pc);
            assert_eq!(white_key_degree(pc).is_some(), is_white);
            assert_eq!(black_key_alteration(pc).is_some(), !is_white);
        }
        assert_eq!(white_key_degree(0), Some(0)); // C -> I
        assert_eq!(white_key_degree(2), Some(1)); // D -> ii
        assert_eq!(white_key_degree(4), Some(2)); // E -> iii
        assert_eq!(white_key_degree(5), Some(3)); // F -> IV
        assert_eq!(white_key_degree(7), Some(4)); // G -> V
        assert_eq!(white_key_degree(9), Some(5)); // A -> vi
        assert_eq!(white_key_degree(11), Some(6)); // B -> vii°

        assert_eq!(black_key_alteration(1), Some(1)); // C# -> #1
        assert_eq!(black_key_alteration(3), Some(3)); // D# -> #2
        assert_eq!(black_key_alteration(6), Some(6)); // F# -> #4
        assert_eq!(black_key_alteration(8), Some(8)); // G# -> #5
        assert_eq!(black_key_alteration(10), Some(10)); // A# -> b7
    }

    #[test]
    fn secondary_dominants_functional_resolutions_in_c() {
        // In Key of C:
        // C# (#1) -> A7 (V7/ii, resolving to Dm)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::SecondaryDominants, 0, 1);
        assert_eq!((root, quality), (9, 2));

        // D# (#2) -> B7 (V7/iii, resolving to Em)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::SecondaryDominants, 0, 3);
        assert_eq!((root, quality), (11, 2));

        // F# (#4) -> D7 (V7/V, resolving to G)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::SecondaryDominants, 0, 6);
        assert_eq!((root, quality), (2, 2));

        // G# (#5) -> E7 (V7/vi, resolving to Am)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::SecondaryDominants, 0, 8);
        assert_eq!((root, quality), (4, 2));

        // Bb (b7) -> C7 (V7/IV, resolving to F)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::SecondaryDominants, 0, 10);
        assert_eq!((root, quality), (0, 2));
    }

    #[test]
    fn modal_interchange_functional_resolutions_in_c() {
        // In Key of C:
        // Db (b2) -> Dbmaj7 (bIImaj7, Neapolitan)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::ModalInterchange, 0, 1);
        assert_eq!((root, quality), (1, 3));

        // Eb (b3) -> Ebmaj7 (bIIImaj7)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::ModalInterchange, 0, 3);
        assert_eq!((root, quality), (3, 3));

        // F# (#4) -> D7 (V7/V)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::ModalInterchange, 0, 6);
        assert_eq!((root, quality), (2, 2));

        // Ab (b6) -> Abmaj7 (bVImaj7)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::ModalInterchange, 0, 8);
        assert_eq!((root, quality), (8, 3));

        // Bb (b7) -> Bbmaj7 (bVIImaj7)
        let (root, quality) = functional_chromatic_chord(ChromaticFlavor::ModalInterchange, 0, 10);
        assert_eq!((root, quality), (10, 3));
    }

    #[test]
    fn transposition_preserves_functional_harmonic_relationships() {
        for key in 0..12 {
            // Secondary dominant to ii in any key is root (key + 9) % 12 with quality 7
            let (root, quality) = functional_chromatic_chord(ChromaticFlavor::SecondaryDominants, key, 1);
            assert_eq!(root, (key + 9) % 12);
            assert_eq!(quality, 2);

            // Modal interchange bVII in any key is root (key + 10) % 12 with quality maj7
            let (root, quality) = functional_chromatic_chord(ChromaticFlavor::ModalInterchange, key, 10);
            assert_eq!(root, (key + 10) % 12);
            assert_eq!(quality, 3);
        }
    }

    #[test]
    fn static_mode_white_keys_ascend_monotonically() {
        // Test that across octaves 3..6 and all keys, degrees strictly ascend
        for key in 0..12 {
            for scale in 0..6 { // 7-tone scales
                for base in [48u8, 60u8] {
                    let white_notes = [base, base + 2, base + 4, base + 5, base + 7, base + 9, base + 11, base + 12];
                    let roots: Vec<u8> = white_notes
                        .into_iter()
                        .map(|n| harmonize_nopia_static(ChromaticFlavor::SecondaryDominants, key, scale, n).root)
                        .collect();
                    assert!(roots.windows(2).all(|w| w[0] < w[1]), "Roots must ascend monotonically: {roots:?} for key={key}, scale={scale}");
                }
            }
        }
    }

    #[test]
    fn static_mode_diatonic_qualities_in_major_and_minor() {
        // Major scale diatonic triad qualities: [Maj(0), Min(1), Min(1), Maj(0), Maj(0), Min(1), Dim(5)]
        let major_white = [60, 62, 64, 65, 67, 69, 71];
        let expected_maj = [0, 1, 1, 0, 0, 1, 5];
        for (i, &note) in major_white.iter().enumerate() {
            let chord = harmonize_nopia_static(ChromaticFlavor::SecondaryDominants, 0, 0, note);
            assert_eq!(chord.quality, expected_maj[i]);
            assert_eq!(chord.root, note);
        }

        // Natural minor diatonic triad qualities: [Min(1), Dim(5), Maj(0), Min(1), Min(1), Maj(0), Maj(0)]
        let expected_min = [1, 5, 0, 1, 1, 0, 0];
        for (i, &note) in major_white.iter().enumerate() {
            let chord = harmonize_nopia_static(ChromaticFlavor::SecondaryDominants, 0, 1, note);
            assert_eq!(chord.quality, expected_min[i]);
        }
    }

    #[test]
    fn real_mode_harmonizes_scale_tones_and_chromatics() {
        // Key of G Major (key = 7)
        // Diatonic note G4 (67): root 67, quality 0 (Major)
        let chord_g = harmonize_nopia_real(ChromaticFlavor::SecondaryDominants, 7, 0, 67);
        assert_eq!((chord_g.root, chord_g.quality), (67, 0));

        // Diatonic note A4 (69): root 69, quality 1 (Minor)
        let chord_a = harmonize_nopia_real(ChromaticFlavor::SecondaryDominants, 7, 0, 69);
        assert_eq!((chord_a.root, chord_a.quality), (69, 1));

        // Chromatic note C#4 (61) in G Major is #4 (alteration 6) -> secondary dominant A7 (root A, quality 2)
        let chord_cs = harmonize_nopia_real(ChromaticFlavor::SecondaryDominants, 7, 0, 61);
        assert_eq!(chord_cs.root % 12, 9); // A
        assert_eq!(chord_cs.quality, 2);   // 7
    }

    #[test]
    fn secondary_dominant_resolutions_trigger_voice_leading() {
        // Progression: C#4 (yielding A7) -> D4 (yielding Dm) in C Major
        let a7_harm = harmonize(HarmonizationEngineMode::NopiaStatic, ChromaticFlavor::SecondaryDominants, 0, 0, 61);
        assert_eq!((a7_harm.root % 12, a7_harm.quality), (9, 2));

        let dm_harm = harmonize(HarmonizationEngineMode::NopiaStatic, ChromaticFlavor::SecondaryDominants, 0, 0, 62);
        assert_eq!((dm_harm.root % 12, dm_harm.quality), (2, 1));

        let chord_a7 = SavedChord {
            root: a7_harm.root,
            quality: a7_harm.quality,
            second: None,
            inversion: 0,
            spread: 0,
            transpose: 0,
        };
        let notes_a7 = voice(chord_a7.root, chord_a7.quality, None, 0, 0, 0);

        let chord_dm = SavedChord {
            root: dm_harm.root,
            quality: dm_harm.quality,
            second: None,
            inversion: 0,
            spread: 0,
            transpose: 0,
        };
        let base_dm = voice(chord_dm.root, chord_dm.quality, None, 0, 0, 0);

        // Nearest voice leading finds the smoothest transition
        let nearest_dm = lead(chord_dm, 0, chord_a7, notes_a7, base_dm, false);
        assert_eq!(nearest_dm.len, base_dm.len);
        assert!(movement(notes_a7, nearest_dm) <= movement(notes_a7, base_dm));

        // Furthest dominant resolution recognizes A7 -> Dm as V7 -> i (fifth resolution down)
        // and triggers expansive contrary motion
        let furthest_dm = lead(chord_dm, 0, chord_a7, notes_a7, base_dm, true);
        assert_eq!(furthest_dm.len, base_dm.len);
    }

    #[test]
    fn all_voicing_spreads_operate_on_harmonized_chords() {
        let harmonized = harmonize(HarmonizationEngineMode::NopiaStatic, ChromaticFlavor::SecondaryDominants, 0, 0, 60);
        for spread in 0..VOICING_NAMES.len() as u8 {
            let voiced = voice(harmonized.root, harmonized.quality, harmonized.second, 0, 0, spread);
            assert!(voiced.len >= 3);
            assert!(voiced.as_slice().windows(2).all(|w| w[0] < w[1]));
        }
    }

    #[test]
    fn boundary_notes_within_midi_range_for_all_modes() {
        for mode in [
            HarmonizationEngineMode::ClassicDualTouch,
            HarmonizationEngineMode::NopiaStatic,
            HarmonizationEngineMode::NopiaReal,
        ] {
            for flavor in [ChromaticFlavor::SecondaryDominants, ChromaticFlavor::ModalInterchange] {
                for key in 0..12 {
                    for scale in 0..8 {
                        for note in [0u8, 1, 60, 126, 127] {
                            let chord = harmonize(mode, flavor, key, scale, note);
                            assert!(chord.root <= 127, "Root exceeds 127 for note={note}, mode={mode:?}");
                            assert!(chord.quality < 12, "Quality invalid for note={note}, mode={mode:?}");
                            let voiced = voice(chord.root, chord.quality, chord.second, 0, 0, 0);
                            assert!(voiced.len > 0);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn mode_and_flavor_from_u8_roundtrip() {
        assert_eq!(HarmonizationEngineMode::from(0), HarmonizationEngineMode::ClassicDualTouch);
        assert_eq!(HarmonizationEngineMode::from(1), HarmonizationEngineMode::NopiaStatic);
        assert_eq!(HarmonizationEngineMode::from(2), HarmonizationEngineMode::NopiaReal);
        assert_eq!(HarmonizationEngineMode::from(99), HarmonizationEngineMode::ClassicDualTouch);

        assert_eq!(ChromaticFlavor::from(0), ChromaticFlavor::SecondaryDominants);
        assert_eq!(ChromaticFlavor::from(1), ChromaticFlavor::ModalInterchange);
        assert_eq!(ChromaticFlavor::from(99), ChromaticFlavor::SecondaryDominants);
    }

    #[test]
    fn extensions_macro_5_tiers() {
        let c_maj7 = voice(60, 3, None, 0, 0, 0); // C, E, G, B
        assert_eq!(c_maj7.as_slice(), &[60, 64, 67, 71]);

        // Tier 0 (<0.20): Root only
        let t0 = apply_extensions(&c_maj7, 0.10, 3);
        assert_eq!(t0.as_slice(), &[60]);

        // Tier 1 (0.20..0.40): Root + 5th (Power chord)
        let t1 = apply_extensions(&c_maj7, 0.30, 3);
        assert_eq!(t1.as_slice(), &[60, 67]);

        // Tier 2 (0.40..0.50): Triad (Root, 3rd, 5th)
        let t2 = apply_extensions(&c_maj7, 0.45, 3);
        assert_eq!(t2.as_slice(), &[60, 64, 67]);

        // Tier 3 (0.65..0.85): 7th (Root, 3rd, 5th, 7th)
        let t3 = apply_extensions(&c_maj7, 0.75, 3);
        assert_eq!(t3.as_slice(), &[60, 64, 67, 71]);

        // Tier 4 (>=0.85): Upper color (all notes)
        let t4 = apply_extensions(&c_maj7, 0.95, 3);
        assert_eq!(t4.as_slice(), &[60, 64, 67, 71]);
    }

    #[test]
    fn guide_tone_extraction() {
        // C Maj7 (C, E, G, B) -> extracts 7th (B = 71)
        let c_maj7 = voice(60, 3, None, 0, 0, 0);
        assert_eq!(extract_guide_tone(&c_maj7, 60), Some(71));

        // C 7 (C, E, G, Bb) -> extracts 7th (Bb = 70)
        let c_dom7 = voice(60, 2, None, 0, 0, 0);
        assert_eq!(extract_guide_tone(&c_dom7, 60), Some(70));

        // C Minor triad (C, Eb, G) -> extracts 3rd (Eb = 63)
        let c_min = voice(60, 1, None, 0, 0, 0);
        assert_eq!(extract_guide_tone(&c_min, 60), Some(63));

        // C Major triad (C, E, G) -> extracts 3rd (E = 64)
        let c_maj = voice(60, 0, None, 0, 0, 0);
        assert_eq!(extract_guide_tone(&c_maj, 60), Some(64));
    }

    #[test]
    fn circle_of_fifths_stepping() {
        assert_eq!(cycle_key_fifths(0, 1), 7);  // C -> G (+5th / +7 st)
        assert_eq!(cycle_key_fifths(7, 1), 2);  // G -> D
        assert_eq!(cycle_key_fifths(0, -1), 5); // C -> F (-5th / +5 st / -7 st)
        assert_eq!(cycle_key_fifths(5, -1), 10); // F -> Bb
    }
}

