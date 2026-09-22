use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq)]
pub struct ValueEdit<T> {
    pub target: T,
    pub rect: (f32, f32, f32, f32),
    pub text: String,
    pub original: String,
    pub cursor: usize,
    pub anchor: usize,
    pub invalid: bool,
}

impl<T> ValueEdit<T> {
    pub fn new(target: T, rect: (f32, f32, f32, f32), initial: String) -> Self {
        let len = initial.len();
        Self {
            target,
            rect,
            text: initial.clone(),
            original: initial,
            cursor: len,
            anchor: 0,
            invalid: false,
        }
    }

    pub fn selection(&self) -> Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }

    pub fn insert(&mut self, text: &str) {
        let text = text
            .trim_matches(['\r', '\n'])
            .replace('−', "-")
            .replace('\u{a0}', " ");
        if !text.is_ascii() || text.chars().any(|c| c.is_control()) {
            return;
        }
        let selection = self.selection();
        if self.text.len() - selection.len() + text.len() > 24 {
            return;
        }
        self.cursor = selection.start + text.len();
        self.anchor = self.cursor;
        self.text.replace_range(selection, &text);
        self.invalid = false;
    }

    pub fn erase(&mut self, backwards: bool) {
        if self.cursor == self.anchor {
            if backwards {
                self.anchor = self.cursor.saturating_sub(1);
            } else {
                self.anchor = (self.cursor + 1).min(self.text.len());
            }
        }
        self.insert("");
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.text.len();
    }

    pub fn handle_mouse_down(&mut self, x: f32) {
        let capacity = ((self.rect.2 - 8.0) / 6.6) as usize;
        let start = self.cursor.saturating_sub(capacity);
        self.cursor = (start + (((x - self.rect.0 - 4.0) / 6.6).round().max(0.0) as usize))
            .min(self.text.len());
        self.anchor = self.cursor;
    }

    pub fn handle_key(&mut self, cx: &mut EventContext, code: Code) {
        let command = cx.modifiers().command();
        match code {
            Code::KeyA if command => {
                self.select_all();
            }
            Code::KeyC | Code::KeyX if command => {
                let _ = cx.set_clipboard(self.text[self.selection()].to_string());
                if code == Code::KeyX {
                    self.insert("");
                }
            }
            Code::KeyV if command => {
                if let Ok(text) = cx.get_clipboard() {
                    self.insert(&text);
                }
            }
            Code::Backspace => self.erase(true),
            Code::Delete => self.erase(false),
            Code::ArrowLeft | Code::ArrowRight | Code::Home | Code::End => {
                let selecting = cx.modifiers().shift();
                self.cursor = match code {
                    Code::Home => 0,
                    Code::End => self.text.len(),
                    Code::ArrowLeft if !selecting && self.cursor != self.anchor => {
                        self.selection().start
                    }
                    Code::ArrowRight if !selecting && self.cursor != self.anchor => {
                        self.selection().end
                    }
                    Code::ArrowLeft => self.cursor.saturating_sub(1),
                    _ => (self.cursor + 1).min(self.text.len()),
                };
                if !selecting {
                    self.anchor = self.cursor;
                }
            }
            _ => {}
        }
    }
}

pub fn slider_value_rect(r: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (r.0 + r.2 - 80.0, r.1 + 4.0, 72.0, 20.0)
}

pub fn typed_char(code: Code, shift: bool) -> Option<char> {
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

pub fn parse_number_with_units(text: &str, suffixes: &[(&str, f64)]) -> Option<f64> {
    let text = text.trim().to_ascii_lowercase();
    for &(suffix, mult) in suffixes {
        if let Some(num_str) = text.strip_suffix(suffix) {
            if let Ok(val) = num_str.trim().parse::<f64>() {
                return Some(val * mult);
            }
        }
    }
    text.parse::<f64>().ok()
}
