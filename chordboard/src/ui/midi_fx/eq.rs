use super::*;
use crate::midi_fx::{note_frequency, MidiEqBand, MidiEqSettings, MIDI_EQ_GAIN_DB, MIDI_EQ_RATE};
use pleasant_eq::ResponseCache;
use pleasant_ui::graph::Viewport;
use std::sync::OnceLock;

const CURVE_POINTS: usize = 257;

pub(super) fn graph() -> Viewport {
    let x = DELAY_SURFACE.0 + 55.0;
    Viewport::new(
        x,
        DELAY_SURFACE.1 + 55.0,
        source_rect(0).0 - 65.0 - x,
        212.0,
    )
}

fn position(note: f32, gain_db: f32) -> (f32, f32) {
    graph().from_normalized(
        note / 127.0,
        0.5 + 0.5 * (gain_db / MIDI_EQ_GAIN_DB).clamp(-1.0, 1.0),
    )
}

fn band_values(x: f32, y: f32) -> (f32, f32) {
    let (note, gain) = graph().to_normalized_clamped(x, y);
    ((127.0 * note).round(), (2.0 * gain - 1.0) * MIDI_EQ_GAIN_DB)
}

fn band_at(eq: &MidiEqSettings, x: f32, y: f32) -> Option<usize> {
    eq.bands.iter().enumerate().rev().find_map(|(i, band)| {
        let band = band.as_ref()?;
        let (bx, by) = position(band.note, band.gain_db);
        ((x - bx).hypot(y - by) <= 12.0).then_some(i)
    })
}

#[derive(Clone, Copy)]
pub(super) struct Drag {
    index: usize,
    offset: (f32, f32),
}

pub(super) struct Editor {
    pub(super) drag: Option<Drag>,
    selected: Option<usize>,
    cache: RefCell<ResponseCache>,
    pub(super) levels: [f32; 128],
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            drag: None,
            selected: None,
            cache: RefCell::default(),
            levels: [0.0; 128],
        }
    }
}

impl Editor {
    pub(super) fn draw(&self, d: &mut Draw, eq: &MidiEqSettings) {
        let g = graph();
        let right = g.x + g.width;
        let bottom = g.y + g.height;
        d.rect(g.x, g.y, g.width, g.height, alpha(LINE, 0.16));
        for db in [-24, -12, 0, 12, 24] {
            let (_, y) = position(0.0, db as f32);
            d.line(g.x, y, right, y, if db == 0 { MUTED } else { LINE }, 1.0);
            let label = if db == 0 {
                "0 dB".to_owned()
            } else {
                format!("{db:+}")
            };
            d.text_centered(g.x - 24.0, y + 4.0, &label, 13.0, MUTED);
        }
        for note in (0..=120).step_by(12) {
            let (x, _) = position(note as f32, 0.0);
            d.line(x, g.y, x, bottom, LINE, 1.0);
            d.text_centered(
                x,
                bottom + 19.0,
                &format!("C{}", note / 12 - 1),
                13.0,
                MUTED,
            );
        }

        let dimmed = d.dimmed;
        d.dimmed = false;
        d.text_centered(right + 24.0, g.y + 4.0, "127", 13.0, GOLD);
        d.text_centered(
            right + 24.0,
            g.y + g.height * 0.5 + 4.0,
            "MIDI In",
            12.0,
            GOLD,
        );
        d.text_centered(right + 24.0, bottom + 4.0, "0", 13.0, GOLD);
        for (note, &level) in self
            .levels
            .iter()
            .enumerate()
            .filter(|(_, level)| **level > 0.0)
        {
            let (x, y) = g.from_normalized(note as f32 / 127.0, level);
            d.line(x, bottom, x, y, alpha(GOLD, 0.52), 3.0);
            d.circle(x, y, 3.0, GOLD, true);
        }
        d.dimmed = dimmed;

        static FREQUENCIES: OnceLock<[f64; CURVE_POINTS]> = OnceLock::new();
        let frequencies = FREQUENCIES.get_or_init(|| {
            std::array::from_fn(|i| note_frequency(i as f64 * 127.0 / (CURVE_POINTS - 1) as f64))
        });
        let coefficients: Vec<_> = eq
            .bands
            .iter()
            .flatten()
            .map(|band| band.coefficients())
            .collect();
        let mut cache = self.cache.borrow_mut();
        cache.update(&coefficients, frequencies, MIDI_EQ_RATE);
        let points = |values: &[f64]| {
            std::array::from_fn::<_, CURVE_POINTS, _>(|i| {
                position(
                    i as f32 * 127.0 / (CURVE_POINTS - 1) as f32,
                    values[i] as f32,
                )
            })
        };
        let mut curve_index = 0;
        for (i, band) in eq.bands.iter().enumerate() {
            if band.is_some() {
                let curve = points(cache.band(curve_index));
                let selected = self.selected == Some(i);
                if selected {
                    d.area(&curve, position(0.0, 0.0).1, alpha(TEAL, 0.14));
                }
                d.poly(&curve, alpha(TEAL, if selected { 0.60 } else { 0.24 }), 1.0);
                curve_index += 1;
            }
        }
        d.poly(&points(cache.sum()), TEAL, 2.0);
        for (i, band) in eq
            .bands
            .iter()
            .enumerate()
            .filter_map(|(i, band)| band.map(|b| (i, b)))
        {
            let (x, y) = position(band.note, band.gain_db);
            d.eq_node(x, y, TEAL, self.selected == Some(i));
        }
        if let Some((x, y)) = d.hover.filter(|&(x, y)| g.contains(x, y)) {
            if band_at(eq, x, y).is_none() && self.drag.is_none() && eq.bands.contains(&None) {
                let (note, db) = band_values(x, y);
                let (x, y) = position(note, db);
                d.circle(x, y, 6.0, alpha(TEAL, 0.65), false);
            }
        }
    }

    pub(super) fn hint(&self, x: f32, y: f32, eq: &MidiEqSettings) -> Option<String> {
        if !graph().contains(x, y) {
            return None;
        }
        if let Some(band) = band_at(eq, x, y).and_then(|i| eq.bands[i]) {
            let note = band.note.round() as u8;
            let name = harmony::midi_note_name_in_key(note, 0, 0, 0);
            return Some(format!("{name} ({note}) · {:+.1} dB · Q {:.2} · Drag to move · Scroll for width · Right-click to remove", band.gain_db, band.q));
        }
        Some(if eq.bands.contains(&None) {
            "MIDI EQ · Click to add a bell band · Gold bars show incoming MIDI velocity before processing"
        } else {
            "MIDI EQ · All 16 bands are in use · Right-click a band to remove it"
        }.into())
    }
}

impl ChordboardView {
    pub(in crate::ui) fn tick_midi_eq(&mut self, old: &Snapshot, dt: f32) -> bool {
        if !self.fx_ui.open || self.fx_ui.collapsed.contains(&MidiFxId::MidiEq) {
            self.fx_ui.eq.levels.fill(0.0);
            return false;
        }
        let mut changed = false;
        for (note, level) in self.fx_ui.eq.levels.iter_mut().enumerate() {
            let input = self.snapshot.midi_input[note];
            let next = if input.held || input.strike != old.midi_input[note].strike {
                input.velocity
            } else {
                (*level - dt * 2.7).max(0.0)
            };
            changed |= next != *level;
            *level = next;
        }
        changed
    }

    pub(super) fn midi_eq_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if self.fx_ui.collapsed.contains(&MidiFxId::MidiEq) {
            return false;
        }
        let local_y = y - self.effect_stack_offset(MidiFxId::MidiEq);
        if let Some(drag) = self.fx_ui.eq.drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let (note, gain_db) = band_values(x + drag.offset.0, local_y + drag.offset.1);
                    self.params.arpeggiator.edit(|state| {
                        if let Some(band) = &mut state.midi_fx.midi_eq.bands[drag.index] {
                            band.note = note;
                            band.gain_db = gain_db;
                        }
                    });
                }
                WindowEvent::MouseUp(MouseButton::Left)
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::FocusOut => {
                    self.fx_ui.eq.drag = None;
                    cx.release();
                }
                WindowEvent::MouseScroll(_, _)
                | WindowEvent::MouseDown(_)
                | WindowEvent::MouseDoubleClick(_)
                | WindowEvent::MouseTripleClick(_) => return true,
                _ => return false,
            }
            cx.needs_redraw();
            return true;
        }
        if !hit(VIEWPORT, x, y) || !graph().contains(x, local_y) {
            return false;
        }
        let state = self.arp_state();
        let eq = &state.midi_fx.midi_eq;
        let hit = band_at(eq, x, local_y);
        match *event {
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                let (index, offset) = if let Some(i) = hit {
                    let band = eq.bands[i].expect("band_at only returns occupied band indices");
                    let (bx, by) = position(band.note, band.gain_db);
                    (i, (bx - x, by - local_y))
                } else {
                    let Some(i) = eq.bands.iter().position(Option::is_none) else {
                        return true;
                    };
                    let (note, gain_db) = band_values(x, local_y);
                    self.params.arpeggiator.edit(|s| {
                        s.midi_fx.midi_eq.bands[i] = Some(MidiEqBand {
                            note,
                            gain_db,
                            ..MidiEqBand::default()
                        });
                    });
                    (i, (0.0, 0.0))
                };
                self.cancel_arp_editor();
                self.fx_ui.eq.selected = Some(index);
                self.fx_ui.eq.drag = Some(Drag { index, offset });
                cx.capture();
                cx.focus();
                self.focused = true;
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                let Some(i) = hit else {
                    return false;
                };
                self.params
                    .arpeggiator
                    .edit(|s| s.midi_fx.midi_eq.bands[i] = None);
                self.fx_ui.eq.selected = None;
            }
            WindowEvent::MouseScroll(_, dy) if dy != 0.0 => {
                let Some(i) = hit else {
                    return false;
                };
                self.params.arpeggiator.edit(|s| {
                    if let Some(band) = &mut s.midi_fx.midi_eq.bands[i] {
                        band.q *= (dy * 0.2).exp();
                    }
                });
                self.fx_ui.eq.selected = Some(i);
            }
            _ => return false,
        }
        cx.needs_redraw();
        true
    }
}
