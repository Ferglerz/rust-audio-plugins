use crate::models::Instrument;
use pleasant_ui::page_slide;
use std::time::Instant;

/// Back-to-front model panels. Offsets are measured in panel widths.
struct Layer {
    piano: Instrument,
    from: f32,
    to: f32,
    x: f32,
}
impl Layer {
    fn at(piano: Instrument, x: f32) -> Self {
        Self {
            piano,
            from: x,
            to: x,
            x,
        }
    }
}

pub(super) struct Slide {
    selected: Instrument,
    layers: Vec<Layer>,
    started: Option<Instant>,
}

impl Slide {
    pub fn new(selected: impl Into<Instrument>) -> Self {
        let selected = selected.into();
        let mut layers = Vec::with_capacity(6);
        layers.push(Layer::at(selected, 0.));
        Self {
            selected,
            layers,
            started: None,
        }
    }

    pub fn moving(&self) -> bool {
        self.started.is_some()
    }

    pub fn panels(&self) -> impl Iterator<Item = (Instrument, f32)> + '_ {
        self.layers.iter().map(|l| (l.piano, l.x))
    }

    pub fn select(&mut self, next: impl Into<Instrument>, now: Instant) -> bool {
        let next = next.into();
        if next == self.selected {
            return false;
        }
        // Retarget from the last painted positions, including any partially revealed page.
        for l in &mut self.layers {
            l.from = l.x;
            l.to = l.x;
        }
        let side = |a: Instrument, b: Instrument| if a.index() > b.index() { 1. } else { -1. };
        if let Some(i) = self.layers.iter().position(|l| l.piano == next) {
            self.layers[i].to = 0.;
            // Uncover a previous tab smoothly when the user reverses a slide.
            for l in &mut self.layers[i + 1..] {
                l.to = side(l.piano, next);
            }
        } else {
            let mut incoming = Layer::at(next, side(next, self.selected));
            incoming.to = 0.;
            self.layers.push(incoming);
        }
        self.selected = next;
        self.started = Some(now);
        true
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        let Some(started) = self.started else {
            return false;
        };
        let elapsed = now.saturating_duration_since(started).as_secs_f32();
        if elapsed >= page_slide::DURATION {
            self.started = None;
            self.layers.clear();
            self.layers.push(Layer::at(self.selected, 0.));
        } else {
            for layer in &mut self.layers {
                layer.x = page_slide::position(layer.from, layer.to, elapsed);
            }
        }
        true
    }
}
