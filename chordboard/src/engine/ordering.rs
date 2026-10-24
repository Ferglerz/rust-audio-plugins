//! Shared ordering after each caller has prepared its playable strings.

pub(super) fn index(pattern: u8, step: usize, total: usize, seed: &mut u32) -> usize {
    debug_assert!(total > 0);
    match pattern {
        1 => total - 1 - step % total,
        2 => {
            let span = (total * 2).saturating_sub(2).max(1);
            let p = step % span;
            if p < total {
                p
            } else {
                span - p
            }
        }
        3 => {
            let k = step / 2;
            if step.is_multiple_of(2) {
                k % total
            } else {
                (k + 2) % total
            }
        }
        4 => {
            let start = total - 1 - (step / 2) % total;
            if step.is_multiple_of(2) {
                start
            } else {
                (start + total - 2 % total) % total
            }
        }
        6 => random(seed) as usize % total,
        // Played order is prepared by the caller, before ordinary ascending traversal.
        _ => step % total,
    }
}

pub(super) fn random(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}
