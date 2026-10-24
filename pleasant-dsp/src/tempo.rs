//! Musical durations, independent of host transport and parameter storage.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Division {
    pub label: &'static str,
    /// Quarter-note beats.
    pub beats: f64,
}

// Sorted by duration. The longest dotted value is deliberately capped at 4/1.
pub const DIVISIONS: [Division; 26] = [
    Division {
        label: "1/64T",
        beats: 1.0 / 24.0,
    },
    Division {
        label: "1/64",
        beats: 1.0 / 16.0,
    },
    Division {
        label: "1/32T",
        beats: 1.0 / 12.0,
    },
    Division {
        label: "1/64D",
        beats: 3.0 / 32.0,
    },
    Division {
        label: "1/32",
        beats: 1.0 / 8.0,
    },
    Division {
        label: "1/16T",
        beats: 1.0 / 6.0,
    },
    Division {
        label: "1/32D",
        beats: 3.0 / 16.0,
    },
    Division {
        label: "1/16",
        beats: 1.0 / 4.0,
    },
    Division {
        label: "1/8T",
        beats: 1.0 / 3.0,
    },
    Division {
        label: "1/16D",
        beats: 3.0 / 8.0,
    },
    Division {
        label: "1/8",
        beats: 1.0 / 2.0,
    },
    Division {
        label: "1/4T",
        beats: 2.0 / 3.0,
    },
    Division {
        label: "1/8D",
        beats: 3.0 / 4.0,
    },
    Division {
        label: "1/4",
        beats: 1.0,
    },
    Division {
        label: "1/2T",
        beats: 4.0 / 3.0,
    },
    Division {
        label: "1/4D",
        beats: 3.0 / 2.0,
    },
    Division {
        label: "1/2",
        beats: 2.0,
    },
    Division {
        label: "1/1T",
        beats: 8.0 / 3.0,
    },
    Division {
        label: "1/2D",
        beats: 3.0,
    },
    Division {
        label: "1/1",
        beats: 4.0,
    },
    Division {
        label: "2/1T",
        beats: 16.0 / 3.0,
    },
    Division {
        label: "1/1D",
        beats: 6.0,
    },
    Division {
        label: "2/1",
        beats: 8.0,
    },
    Division {
        label: "4/1T",
        beats: 32.0 / 3.0,
    },
    Division {
        label: "2/1D",
        beats: 12.0,
    },
    Division {
        label: "4/1",
        beats: 16.0,
    },
];

pub const QUARTER_NOTE: i32 = 13;

pub fn valid_bpm(bpm: f64) -> f64 {
    if bpm.is_finite() && bpm > 0.0 {
        bpm
    } else {
        120.0
    }
}

pub fn resolve_bpm(host: bool, host_bpm: Option<f64>, manual_bpm: f64) -> f64 {
    valid_bpm(if host {
        host_bpm.unwrap_or(120.0)
    } else {
        manual_bpm
    })
}

pub fn beats_to_seconds(beats: f64, bpm: f64) -> f64 {
    beats * 60.0 / valid_bpm(bpm)
}

pub fn division(index: i32) -> Division {
    DIVISIONS[index.clamp(0, DIVISIONS.len() as i32 - 1) as usize]
}

pub fn parse_division(text: &str) -> Option<i32> {
    DIVISIONS
        .iter()
        .position(|d| d.label.eq_ignore_ascii_case(text.trim()))
        .map(|i| i as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisions_are_ordered_and_convert_at_multiple_tempos() {
        assert!(DIVISIONS.windows(2).all(|w| w[0].beats < w[1].beats));
        assert_eq!(division(QUARTER_NOTE).label, "1/4");
        assert_eq!(beats_to_seconds(1.0, 120.0), 0.5);
        assert_eq!(beats_to_seconds(1.0, 60.0), 1.0);
        assert_eq!(parse_division(" 1/8t "), Some(8));
    }

    #[test]
    fn invalid_host_tempos_fall_back_without_overriding_manual_tempo() {
        for bpm in [
            None,
            Some(0.0),
            Some(-1.0),
            Some(f64::NAN),
            Some(f64::INFINITY),
        ] {
            assert_eq!(resolve_bpm(true, bpm, 87.0), 120.0);
            assert_eq!(resolve_bpm(false, bpm, 87.0), 87.0);
        }
    }
}
