//! Editor-only helpers for oblique instrument cutaways. Motion is slowed to show a strike.
use crate::{
    draw::Draw,
    playable_keyboard::PlayableKeyboard,
    point_in_rect,
    theme::{rgb, LINE, MUTED, PANEL, TEAL, TEXT},
};
use nih_plug_vizia::vizia::vg::Color;
use std::{cell::Cell, time::Instant};

pub type Point = (f32, f32);
pub const STEEL: Color = rgb(156, 171, 179);
pub const CASTING: Color = rgb(96, 113, 119);
pub const WOOD: Color = rgb(191, 141, 90);
pub const FELT: Color = rgb(207, 184, 159);
pub const RUBBER: Color = rgb(76, 69, 68);
const STRIKE_SECONDS: f32 = 1.6;

pub struct StrikePreview {
    note: Cell<u8>,
    start: Cell<Option<Instant>>,
}

impl Default for StrikePreview {
    fn default() -> Self {
        Self {
            note: Cell::new(60),
            start: Cell::new(None),
        }
    }
}

impl StrikePreview {
    pub fn note(&self) -> u8 {
        self.note.get()
    }

    /// Consume the same latched attacks as the piano, including repeats on a held pitch.
    /// For simultaneous attacks the illustration follows the highest struck key.
    pub fn poll(&self, keyboard: &PlayableKeyboard) -> bool {
        let (changed, attacks) = keyboard.poll_with_attacks();
        if attacks != 0 {
            self.note.set((127 - attacks.leading_zeros()) as u8);
            self.start.set(Some(Instant::now()));
        }
        if let Some(start) = self.start.get() {
            if start.elapsed().as_secs_f32() >= STRIKE_SECONDS {
                self.start.set(None);
            }
            return true; // Also draw the final frame at rest.
        }
        changed
    }

    pub fn elapsed(&self) -> f32 {
        self.start.get().map_or(STRIKE_SECONDS, |s| {
            s.elapsed().as_secs_f32().min(STRIKE_SECONDS)
        })
    }
}

/// Height of the hammer as a fraction of its travel, with contact at 80 ms.
pub fn hammer_lift(t: f32) -> f32 {
    if t < 0.08 {
        t / 0.08
    } else {
        (1.0 - (t - 0.08) / 0.14).max(0.0)
    }
}

/// Decaying, slowed motion starting at hammer contact and settling exactly at rest.
pub fn vibration(t: f32, decay: f32) -> f32 {
    let age = (t - 0.08).max(0.0);
    let fade = (1.0 - t / STRIKE_SECONDS).max(0.0);
    (age * 24.0).sin() * (-age / decay).exp() * fade
}

pub fn panel(d: &mut Draw, width: f32, title: &str, note: u8) {
    const NAMES: [&str; 12] = [
        "C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B",
    ];
    d.panel((18., 62., width - 36., 194.), PANEL, LINE);
    d.text(34., 84., title, 12., TEXT);
    let name = format!(
        "{}{}",
        NAMES[usize::from(note % 12)],
        i16::from(note / 12) - 1
    );
    d.text_right(width - 34., 84., &name, 12., MUTED);
}

/// Keep material light and shade consistent in both editor themes.
pub fn hardware(d: &mut Draw, paint: impl FnOnce(&mut Draw)) {
    let mut material = Draw::new(d.c, false, d.s, d.ox, d.oy, d.font);
    material.offset_x = d.offset_x;
    paint(&mut material);
}

fn shade(c: Color, factor: f32) -> Color {
    Color {
        r: (c.r * factor).min(1.),
        g: (c.g * factor).min(1.),
        b: (c.b * factor).min(1.),
        ..c
    }
}

/// Extrude a clockwise side profile away from the viewer.
pub fn solid<const N: usize>(d: &mut Draw, face: [Point; N], depth: Point, color: Color) {
    let back = face.map(|(x, y)| (x + depth.0, y + depth.1));
    for i in 0..N {
        let j = (i + 1) % N;
        let (dx, dy) = (face[j].0 - face[i].0, face[j].1 - face[i].1);
        if dx * depth.1 - dy * depth.0 < 0.0 {
            let light = 0.82 + 0.35 * dx / dx.hypot(dy);
            d.fill_poly(&[face[i], back[i], back[j], face[j]], shade(color, light));
        }
    }
    d.fill_poly(&face, shade(color, 0.82));
}

pub fn screw(d: &mut Draw, x: f32, y: f32, r: f32) {
    d.circle(x, y, r, CASTING, true);
    d.circle(x, y, r * 0.76, STEEL, true);
    d.line(
        x - r * 0.52,
        y + r * 0.18,
        x + r * 0.52,
        y - r * 0.18,
        RUBBER,
        1.4,
    );
}

/// Side profile of a pivoted wood shank and a layered striking head.
pub fn hammer(d: &mut Draw, pivot: Point, tip: Point, lift: f32, covering: Color) {
    let angle = -0.14 * lift;
    let (sin, cos) = angle.sin_cos();
    let rotate = |(x, y): Point| {
        let (dx, dy) = (x - pivot.0, y - pivot.1);
        (pivot.0 + cos * dx - sin * dy, pivot.1 + sin * dx + cos * dy)
    };
    let (x, y) = tip;
    let (px, py) = pivot;
    solid(
        d,
        [
            (px - 4., py - 3.),
            (x + 3., y + 19.),
            (x + 6., y + 26.),
            (px - 3., py + 5.),
        ]
        .map(rotate),
        (5., -4.),
        WOOD,
    );
    let core = [
        (x - 14., y + 12.),
        (x - 6., y + 4.),
        (x + 8., y + 4.),
        (x + 14., y + 10.),
        (x + 8., y + 25.),
        (x - 5., y + 27.),
    ];
    solid(d, core.map(rotate), (6., -4.), WOOD);
    let head = [
        (x - 16., y + 12.),
        (x - 13., y + 3.),
        (x - 6., y - 2.),
        (x + 3., y - 3.),
        (x + 12., y + 1.),
        (x + 16., y + 8.),
        (x + 12., y + 15.),
        (x + 3., y + 9.),
        (x - 5., y + 10.),
        (x - 12., y + 18.),
    ];
    solid(d, head.map(rotate), (6., -4.), covering);
    screw(d, px, py, 4.5);
}

pub struct Callout {
    pub label: &'static str,
    pub origin: Point,
    pub width: f32,
    pub elbow: Point,
}

impl Callout {
    pub fn hit(&self, x: f32, y: f32) -> bool {
        point_in_rect(
            x,
            y,
            (
                self.origin.0 - 4.,
                self.origin.1 - 14.,
                self.width + 8.,
                20.,
            ),
        )
    }

    pub fn draw(&self, d: &mut Draw, label: &str, anchor: Point, active: bool) {
        let ink = if active { TEAL } else { MUTED };
        let (x, y) = self.origin;
        d.poly(&[(x, y + 4.), self.elbow, anchor], ink, 1.);
        d.circle(anchor.0, anchor.1, 2.4, ink, true);
        d.text(x, y, label, 11., if active || d.light { TEXT } else { ink });
    }
}
