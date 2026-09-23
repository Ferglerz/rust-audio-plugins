use nih_plug_vizia::vizia::prelude::Code;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ValueTarget {
    Global(usize),
    // Frequency, gain, Q, threshold, ratio, attack, release, range.
    Band(usize),
    Lift(usize),
}

pub(super) fn typed_char(code: Code, shift: bool) -> Option<char> {
    Some(match code {
        Code::Digit0 | Code::Numpad0 => '0',
        Code::Digit1 | Code::Numpad1 => '1',
        Code::Digit2 | Code::Numpad2 => '2',
        Code::Digit3 | Code::Numpad3 => '3',
        Code::Digit4 | Code::Numpad4 => '4',
        Code::Digit5 | Code::Numpad5 => '5',
        Code::Digit6 | Code::Numpad6 => '6',
        Code::Digit7 | Code::Numpad7 => '7',
        Code::Digit8 | Code::Numpad8 => '8',
        Code::Digit9 | Code::Numpad9 => '9',
        Code::Period | Code::NumpadDecimal => '.',
        Code::Comma => ',',
        Code::Minus | Code::NumpadSubtract => '-',
        Code::Equal => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        Code::NumpadAdd => '+',
        Code::Slash | Code::NumpadDivide => '/',
        Code::Space => ' ',
        Code::KeyA => letter(shift, 'a'),
        Code::KeyB => letter(shift, 'b'),
        Code::KeyC => letter(shift, 'c'),
        Code::KeyD => letter(shift, 'd'),
        Code::KeyE => letter(shift, 'e'),
        Code::KeyF => letter(shift, 'f'),
        Code::KeyG => letter(shift, 'g'),
        Code::KeyH => letter(shift, 'h'),
        Code::KeyI => letter(shift, 'i'),
        Code::KeyJ => letter(shift, 'j'),
        Code::KeyK => letter(shift, 'k'),
        Code::KeyL => letter(shift, 'l'),
        Code::KeyM => letter(shift, 'm'),
        Code::KeyN => letter(shift, 'n'),
        Code::KeyO => letter(shift, 'o'),
        Code::KeyP => letter(shift, 'p'),
        Code::KeyQ => letter(shift, 'q'),
        Code::KeyR => letter(shift, 'r'),
        Code::KeyS => letter(shift, 's'),
        Code::KeyT => letter(shift, 't'),
        Code::KeyU => letter(shift, 'u'),
        Code::KeyV => letter(shift, 'v'),
        Code::KeyW => letter(shift, 'w'),
        Code::KeyX => letter(shift, 'x'),
        Code::KeyY => letter(shift, 'y'),
        Code::KeyZ => letter(shift, 'z'),
        _ => return None,
    })
}

fn letter(shift: bool, c: char) -> char {
    if shift {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

pub(super) fn parse_value(text: &str, target: ValueTarget) -> Option<f64> {
    let text = text.trim().to_ascii_lowercase();
    if target == ValueTarget::Global(1) && text == "off" {
        return Some(-80.0);
    }
    if target == ValueTarget::Global(10) {
        match text.as_str() {
            "a" => return Some(0.0),
            "b" => return Some(1.0),
            "c" => return Some(2.0),
            "d" => return Some(3.0),
            "e" => return Some(4.0),
            "f" => return Some(5.0),
            _ => {}
        }
        if let Some(s) = text.strip_suffix("ms") {
            if let Ok(ms) = s.trim().parse::<f64>() {
                return Some(crate::dsp::seconds_to_pse_time_pos(ms * 0.001));
            }
        }
        if let Some(s) = text.strip_suffix("s") {
            if let Ok(sec) = s.trim().parse::<f64>() {
                return Some(crate::dsp::seconds_to_pse_time_pos(sec));
            }
        }
        if let Ok(val) = text.parse::<f64>() {
            if (0.0..=5.0).contains(&val) {
                return Some(val);
            }
            if val > 5.0 {
                return Some(crate::dsp::seconds_to_pse_time_pos(val * 0.001));
            }
        }
        return None;
    }
    let suffixes: &[(&str, f64)] = match target {
        ValueTarget::Band(0) | ValueTarget::Lift(0) => {
            &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0)]
        }
        ValueTarget::Band(5 | 6) | ValueTarget::Lift(5 | 6) | ValueTarget::Global(5 | 11) => {
            &[("ms", 1.0), ("s", 1000.0)]
        }
        ValueTarget::Band(4) | ValueTarget::Lift(4) | ValueTarget::Global(6) => &[(":1", 1.0)],
        ValueTarget::Global(2 | 3 | 4 | 16 | 17) => &[("%", 1.0)],
        ValueTarget::Global(0 | 18) => &[("db", 1.0)],
        ValueTarget::Band(2) | ValueTarget::Lift(2) => &[],
        _ => &[("db", 1.0)],
    };
    let (number, multiplier) = suffixes
        .iter()
        .find_map(|(suffix, multiplier)| {
            text.strip_suffix(suffix)
                .map(|number| (number.trim(), *multiplier))
        })
        .unwrap_or((&text, 1.0));
    let value = number.parse::<f64>().ok()? * multiplier;
    value.is_finite().then_some(value)
}
