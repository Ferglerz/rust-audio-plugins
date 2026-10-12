//! Presentation only: harmony names and saved state retain their ASCII spelling.
pub(super) const FONT: &[u8] = include_bytes!("../../assets/fonts/BravuraText.otf");

/// SMuFL chord symbols, paired with the existing text face using Bravura Text.
/// Only pass musical labels, never arbitrary UI prose (where `b` is a letter).
pub(super) fn label(text: &str) -> String {
    text.replace("m7b5", "\u{e871}7")
        .replace("maj7", "\u{e873}7")
        .replace("dim", "\u{e870}")
        .replace("aug", "\u{e872}")
        .replace("bb", "\u{e264}")
        .replace("##", "\u{e263}")
        .replace(['b', '♭'], "\u{e260}")
        .replace(['#', '♯'], "\u{e262}")
        .replace('♮', "\u{e261}")
}
