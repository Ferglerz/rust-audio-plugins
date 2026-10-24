//! Explanations of the existing Degrees mapping; never changes playback.
use super::*;

pub(super) struct DegreeTheory {
    pub summary: String,
    pub detail: String,
    /// Pitch class before performance transpose; only marked in the actual voicing.
    pub changed_pitch: Option<u8>,
}

impl ChordboardView {
    pub(super) fn degree_theory(&self, index: usize) -> Option<DegreeTheory> {
        if self.keyboard_layout() != 2 {
            return None;
        }
        let chord = self.keyboard_chord(index)?;
        let column = index % KEY_COLUMNS;
        let baseline_index = KEY_COLUMNS + column;
        let baseline = self.keyboard_chord(baseline_index)?;
        let key = self.params.key.value() as u8;
        let scale = self.params.scale.value() as u8;
        let spelling_scale = match scale {
            6 => 0,
            7 => 1,
            _ => scale,
        };
        let preference = self.params.key_spelling.value();
        let roman = |root, quality| harmony::roman_in_key(root, quality, key, scale, preference);
        let name = |c: harmony::KeyboardChord| {
            format!(
                "{}{}",
                self.note_name(c.root),
                harmony::QUALITY_SUFFIX[c.quality as usize]
            )
        };
        let base_tones = harmony::intervals(baseline.quality, None);
        let tones = harmony::intervals(chord.quality, None);
        let changed = base_tones
            .as_slice()
            .iter()
            .zip(tones.as_slice())
            .enumerate()
            .find(|(_, (from, to))| from != to);
        let Some((tone, (&from, &to))) = changed else {
            let edge = if index / KEY_COLUMNS != 1 {
                "Same triad · quality limit reached"
            } else if matches!(scale, 6 | 7) {
                "Parent-scale triad"
            } else {
                "Scale triad"
            };
            let notes = tones
                .as_slice()
                .iter()
                .map(|interval| {
                    harmony::note_name_in_key(
                        chord.root + interval,
                        key,
                        spelling_scale,
                        preference,
                    )
                })
                .collect::<Vec<_>>()
                .join("–");
            let summary = format!(
                "{} · Diatonic {} · {edge}",
                name(chord),
                roman(chord.root, chord.quality)
            );
            return Some(DegreeTheory {
                detail: format!(
                    "{summary} · {notes} · Rows keep the root and alter one chord tone"
                ),
                summary,
                changed_pitch: None,
            });
        };
        let shift = to as i8 - from as i8;
        let original = (chord.root + from) % 12;
        let altered = (chord.root + to) % 12;
        let from_name = harmony::note_name_in_key(original, key, spelling_scale, preference);
        let to_name =
            harmony::altered_note_name_in_key(original, shift, key, spelling_scale, preference);
        let movement = format!(
            "{from_name} → {to_name} ({} {})",
            if shift > 0 { "raised" } else { "lowered" },
            if tone == 1 { "third" } else { "fifth" }
        );
        let relative = (chord.root + 12 - key) % 12;
        let mut function = "Chromatic colour".to_string();
        let mut try_keys = format!(
            "{} → {} → {}",
            harmony::HINTS[baseline_index],
            harmony::HINTS[index],
            harmony::HINTS[baseline_index]
        );

        // A major triad a fifth above another scale chord can tonicize it.
        // Say "can lead" because harmonic function depends on the progression.
        let target = (relative + 5) % 12;
        let target_quality = harmony::scale_quality(scale, target);
        if chord.quality == 0 && matches!(target_quality, Some(0 | 1)) {
            let target_quality = target_quality.unwrap_or(0);
            let target_chord = harmony::KeyboardChord {
                root: key + target,
                quality: target_quality,
            };
            let target_roman = roman(target_chord.root, target_chord.quality);
            let label = if target == 0 {
                "Dominant V".to_string()
            } else {
                format!("Secondary dominant V/{target_roman}")
            };
            function = format!("{label} · Can lead to {}", name(target_chord));
            if let Some(target_column) = harmony::scale_tones(scale)
                .iter()
                .position(|&n| n == target)
            {
                try_keys = format!(
                    "{} → {} → {}",
                    harmony::HINTS[baseline_index],
                    harmony::HINTS[index],
                    harmony::HINTS[KEY_COLUMNS + target_column]
                );
            }
        } else if let Some(source) = [0, 1].into_iter().find(|&source| {
            harmony::scale_quality(source, relative) == Some(chord.quality)
                && harmony::scale_quality(scale, relative) != Some(chord.quality)
        }) {
            function = format!(
                "Borrowed {} · Parallel {}",
                roman(chord.root, chord.quality),
                if source == 0 { "major" } else { "minor" }
            );
            if relative == 5 && chord.quality == 1 && harmony::scale_quality(scale, 0) == Some(0) {
                try_keys = format!(
                    "{} → {} → Q",
                    harmony::HINTS[baseline_index],
                    harmony::HINTS[index]
                );
            }
        }
        let summary = format!("{} · {function} · {movement}", name(chord));
        Some(DegreeTheory {
            detail: format!("{summary} · Try {try_keys} · Same root, two shared tones"),
            summary,
            changed_pitch: Some(altered),
        })
    }

    pub(super) fn selected_degree_theory(&self) -> Option<DegreeTheory> {
        let saved = SavedChord::decode(self.snapshot.captured)?;
        // Do not explain a different triad when MIDI input has added/altered notes.
        if saved.second.is_some() || saved.additions != 0 {
            return None;
        }
        let index = (KEY_COLUMNS..KEY_COLUMNS * 2)
            .chain(0..KEY_COUNT)
            .find(|&index| {
                self.keyboard_chord(index).is_some_and(|chord| {
                    saved.quality == chord.quality && saved.root % 12 == chord.root % 12
                })
            })?;
        let mut theory = self.degree_theory(index)?;
        if saved.transpose != 0 {
            let suffix = format!(" · Voicing transpose {:+}", saved.transpose);
            theory.summary.push_str(&suffix);
            theory.detail.push_str(&suffix);
        }
        Some(theory)
    }

    pub(super) fn theory_changed_pitch(&self) -> Option<u8> {
        let saved = SavedChord::decode(self.snapshot.captured)?;
        self.selected_degree_theory()?
            .changed_pitch
            .map(|pitch| (pitch as i16 + saved.transpose as i16).rem_euclid(12) as u8)
    }
}
