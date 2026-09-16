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
