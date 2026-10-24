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

pub const MIN_STRINGS: usize = 3;
pub const MAX_STRINGS: usize = 12;

/// Strumfield and once-sweep strings: every chord tone in every arp octave.
pub fn string_count(notes: usize, octaves: u8) -> usize {
    (notes * octaves.clamp(1, 4) as usize).clamp(MIN_STRINGS, MAX_STRINGS)
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
    voice_recipe(SavedChord { root, quality, second, additions: 0, transpose, inversion, spread }, spread)
}

/// Resolve every consumer from the same root-relative recipe, deduplicating pitch classes.
pub fn recipe_intervals(chord: SavedChord) -> Notes {
    let mut tones = intervals(chord.quality, chord.second.map(|n| (n as i16 - chord.root as i16).rem_euclid(12) as u8));
    // A flat ninth alters a suspended second rather than forming a flat-second
    // suspension. Keep the recipe intact so releasing the addition restores sus2.
    if chord.additions & (1 << 1) != 0 && tones.as_slice().contains(&2) {
        let mut altered = Notes::default();
        for &tone in tones.as_slice() {
            if tone != 2 { altered.push(tone as i16); }
        }
        tones = altered;
    }
    for pc in 0..12u8 {
        if chord.additions & (1 << pc) != 0 && !tones.as_slice().iter().any(|n| n % 12 == pc) {
            let interval = match pc { 1..=3 | 5..=6 | 8..=9 => pc + 12, _ => pc };
            tones.push(interval as i16);
        }
    }
    tones.values[..tones.len].sort_unstable();
    tones
}

pub fn voice_recipe(chord: SavedChord, spread: u8) -> Notes {
    let SavedChord { root, transpose, inversion, .. } = chord;
    let tones = recipe_intervals(chord);
    let inv = inversion as usize % tones.len;
    let mut out = Notes::default();
    let mut previous = -1;
    for i in 0..tones.len {
        let source = (i + inv) % tones.len;
        // Compound extensions span two octaves: rotate above the preceding tone,
        // before spreading, so the selected inversion remains the Close bass.
        let mut interval = tones.values[source] as i16;
        while interval <= previous { interval += 12; }
        previous = interval;
        let open = match spread.min((VOICING_NAMES.len() - 1) as u8) {
            1 if i % 2 == 1 => 12,
            2 => (i / 2) as i16 * 12,
            3 if tones.len >= 3 && i == tones.len - 2 => -12,
            4 if tones.len >= 3 && i == tones.len - 3 => -12,
            _ => 0,
        };
        out.push(root as i16 + transpose as i16 + interval + open);
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
/// Sounding delay voices join the movement score without changing chord tones.
#[allow(clippy::too_many_arguments)]
pub fn lead(
    chord: SavedChord,
    spread: u8,
    previous: SavedChord,
    previous_notes: Notes,
    base: Notes,
    furthest_dominant: bool,
    echoes: &[u8],
) -> Notes {
    if previous_notes.len == 0 || base.len == 0 {
        return base;
    }
    let old_tones = recipe_intervals(previous);
    let tones = recipe_intervals(chord);
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
    let score = |notes: Notes| {
        let echo_movement: i32 = echoes
            .iter()
            .map(|&echo| {
                notes.as_slice().iter()
                    .map(|&note| i32::from(echo.abs_diff(note)))
                    .min().unwrap_or(0)
            })
            .sum();
        let distance = movement(previous_notes, notes) + echo_movement;
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
            let candidate = voice_recipe(SavedChord { transpose: transpose as i8, inversion: inversion as u8, ..chord }, spread);
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
            additions: 0,
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
                        let notes = lead(chord, spread, previous, previous_notes, base, far, &[]);
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedChord {
    pub root: u8,
    pub second: Option<u8>,
    pub additions: u16,
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
            | (((self.inversion as u64) & 7) << 19)
            | (((self.inversion as u64) >> 3) << 44)
            | (((self.additions as u64) & 0xfff) << 32)
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
            inversion: (((n >> 19) & 7) | (((n >> 44) & 1) << 3)).min(11) as u8,
            additions: ((n >> 32) & 0xfff) as u16,
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
    let base = intervals(chord.quality, chord.second.map(|n| (n as i16 - chord.root as i16).rem_euclid(12) as u8));
    let tones = recipe_intervals(chord);
    let has = |n| base.as_slice().contains(&n) && tones.as_slice().contains(&n);
    let resolved_has = |n| tones.as_slice().contains(&n);
    let sus = if has(2) && !has(3) && !has(4) {
        "sus2"
    } else if has(5) && !has(3) && !has(4) {
        "sus4"
    } else {
        ""
    };
    let half_diminished = has(3) && has(6) && !has(7) && has(10);
    let no_third = !has(3) && !has(4) && sus.is_empty();
    let mut suffix = if half_diminished {
        "m7b5".to_string()
    } else if has(3) && has(6) && !has(7) {
        "dim".to_string()
    } else if has(4) && has(8) {
        "aug".to_string()
    } else if has(3) {
        "m".to_string()
    } else if no_third && has(7) && !has(9) && !has(10) && !has(11) {
        "5".to_string()
    } else {
        String::new()
    };
    if has(11) {
        suffix.push_str("maj7");
    } else if has(10) && !half_diminished {
        suffix.push('7');
    } else if has(9) {
        suffix.push_str(if suffix == "dim" { "7" } else { "6" });
    }
    suffix.push_str(sus);
    if has(6) && has(4) && !resolved_has(7) {
        suffix.push_str("(add#11,no5)");
    }
    if resolved_has(14) && (has(10) || has(11)) && has(7) && (has(3) || has(4)) && suffix.ends_with('7') {
        suffix.pop();
        suffix.push('9');
    }
    let mut additions = Vec::new();
    if no_third && !suffix.starts_with('5') {
        additions.push("no3".to_string());
    }
    if no_third && has(6) { additions.push("addb5".to_string()); }
    if no_third && has(8) { additions.push("add#5".to_string()); }
    for &interval in tones.as_slice() {
        // The base b9 is an addition too; all other base tones are named above.
        if interval != 13 && base.as_slice().contains(&interval) { continue; }
        if interval == 14 && suffix.ends_with('9') { continue; }
        let label = match interval { 0 => "1", 4 => "3", 6 => "b5", 7 => "5", 8 => "#5", 9 => "6", 10 => "b7", 11 => "maj7", 13 => "b9", 14 => "9", 15 => "#9", 17 => "11", 18 => "#11", 20 => "b13", 21 => "13", _ => "?" };
        additions.push(format!("add{label}"));
    }
    if !additions.is_empty() { suffix.push('('); suffix.push_str(&additions.join(",")); suffix.push(')'); }
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
            additions: 0,
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
                    additions: 0,
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
mod recipe_tests {
    use super::*;
    fn chord(quality: u8, additions: u16) -> SavedChord {
        SavedChord { root: 36, second: None, quality, additions, inversion: 0, spread: 0, transpose: 0 }
    }
    #[test]
    fn flat_ninth_alters_suspended_second_across_roots_and_qualities() {
        for root in 48..60 {
            for (quality, expected, suffix) in [
                (0, vec![0, 7, 13], "5(addb9)"),
                (1, vec![0, 7, 13], "5(addb9)"),
                (2, vec![0, 7, 10, 13], "7(no3,addb9)"),
                (3, vec![0, 7, 11, 13], "maj7(no3,addb9)"),
                (4, vec![0, 7, 10, 13], "7(no3,addb9)"),
                (5, vec![0, 6, 13], "(no3,addb5,addb9)"),
                (6, vec![0, 8, 13], "(no3,add#5,addb9)"),
                (7, vec![0, 7, 9, 13], "6(no3,addb9)"),
                (8, vec![0, 7, 9, 13], "6(no3,addb9)"),
                (9, vec![0, 6, 9, 13], "6(no3,addb5,addb9)"),
                (10, vec![0, 6, 10, 13], "7(no3,addb5,addb9)"),
                (11, vec![0, 7, 13], "5(addb9)"),
            ] {
                let c = SavedChord { root, second: Some(root + 2), ..chord(quality, 1 << 1) };
                assert_eq!(recipe_intervals(c).as_slice(), expected.as_slice());
                assert_eq!(chord_name(c), format!("{}{suffix}", NOTE_NAMES[(root % 12) as usize]));
                assert_eq!(SavedChord::decode(c.encode()), Some(c));
                let restored = SavedChord { additions: 0, ..c };
                assert!(recipe_intervals(restored).as_slice().contains(&2));
                assert!(chord_name(restored).contains("sus2"));
            }
        }
    }
    #[test]
    fn flat_ninth_preserves_fourth_suspensions_and_thirds() {
        for (second, expected, label) in [
            (None, vec![0, 4, 7, 13], "C(addb9)"),
            (Some(41), vec![0, 5, 7, 13], "Csus4(addb9)"),
            (Some(43), vec![0, 7, 13], "C5(addb9)"),
        ] {
            let c = SavedChord { second, ..chord(0, 1 << 1) };
            assert_eq!(recipe_intervals(c).as_slice(), expected.as_slice());
            assert_eq!(chord_name(c), label);
        }
        assert_eq!(recipe_intervals(SavedChord { second: Some(38), ..chord(0, 1 << 3) }).as_slice(), &[0, 2, 7, 15]);
    }
    #[test]
    fn extension_variants_coexist_with_every_base_tone() {
        let c = chord(2, (1 << 1) | (1 << 2) | (1 << 3) | (1 << 5) | (1 << 6) | (1 << 8) | (1 << 9) | (1 << 11));
        assert_eq!(recipe_intervals(c).as_slice(), &[0, 4, 7, 10, 11, 13, 14, 15, 17, 18, 20, 21]);
        let label = chord_name(c);
        for name in ["C9", "addb9", "add#9", "add11", "add#11", "addb13", "add13", "addmaj7"] { assert!(label.contains(name), "{label} lacks {name}"); }
        assert_eq!(chord_name(chord(10, 0)), "Cm7b5");
        assert_eq!(chord_name(chord(3, 1 << 2)), "Cmaj9");
        assert_eq!(chord_name(chord(4, 1 << 2)), "Cm9");
        assert_eq!(chord_name(chord(1, 1 << 4)), "Cm(add3)");
        assert_eq!(chord_name(chord(3, 1 << 10)), "Cmaj7(addb7)");
    }
    #[test]
    fn dense_recipes_keep_all_pitch_classes_in_every_inversion_and_spread() {
        for spread in 0..5 { for inversion in 0..12 {
            let c = SavedChord { root: 12, spread, inversion, ..chord(0, 0xfff) };
            let notes = voice_recipe(c, spread);
            assert_eq!(notes.len, 12, "spread={spread} inversion={inversion}");
            if spread == 0 { assert_eq!(notes.values[0] % 12, (c.root + recipe_intervals(c).values[inversion as usize]) % 12); }
            let mut pcs: Vec<_> = notes.as_slice().iter().map(|n| n % 12).collect();
            pcs.sort_unstable(); assert_eq!(pcs, (0..12).collect::<Vec<_>>());
            assert_eq!(SavedChord::decode(c.encode()), Some(c));
        }}
        assert_eq!(recipe_intervals(chord(0, (1 << 0) | (1 << 4) | (1 << 7))).as_slice(), &[0,4,7]);
    }
}
