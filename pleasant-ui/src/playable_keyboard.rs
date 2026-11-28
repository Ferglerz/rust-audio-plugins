//! Communication between an instrument's on-screen piano and its audio thread.
use crate::piano::Piano;
use crossbeam_queue::ArrayQueue;
use nih_plug_vizia::vizia::prelude::*;
use std::{
    cell::Cell,
    ops::RangeInclusive,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

const VELOCITY: f32 = 0.8;
pub const HELP: &str = "Click and hold a key to play. Drag across keys to change notes.";

pub struct KeyboardLink {
    // Each command is the entire desired pointer state. Under overload the
    // newest state wins, including the final release.
    auditions: ArrayQueue<Option<u8>>,
    visible: AtomicBool,
    playing: [AtomicU64; 2],
    attacks: [AtomicU64; 2],
}

impl Default for KeyboardLink {
    fn default() -> Self {
        Self {
            auditions: ArrayQueue::new(32),
            visible: AtomicBool::new(false),
            playing: Default::default(),
            attacks: Default::default(),
        }
    }
}

impl KeyboardLink {
    pub fn audition(&self, note: Option<u8>) {
        self.auditions.force_push(note);
    }

    pub fn drain_auditions(
        &self,
        range: RangeInclusive<u8>,
        mut apply: impl FnMut(Option<u8>, f32),
    ) {
        for _ in 0..self.auditions.len() {
            if let Some(note) = self.auditions.pop() {
                let note = note.filter(|key| range.contains(key));
                if let Some(key) = note {
                    self.attack(key);
                }
                apply(note, VELOCITY);
            }
        }
    }

    pub fn publish(&self, playing: impl FnOnce() -> u128) {
        if !self.visible.load(Ordering::Acquire) {
            return;
        }
        let notes = playing();
        for (i, word) in self.playing.iter().enumerate() {
            word.store((notes >> (64 * i)) as u64, Ordering::Relaxed);
        }
    }

    // Latch brief strikes that start and finish between two editor frames.
    pub fn attack(&self, note: u8) {
        if self.visible.load(Ordering::Acquire) {
            self.attacks[usize::from(note / 64)].fetch_or(1 << (note % 64), Ordering::Relaxed);
        }
    }

    fn playing(&self) -> u128 {
        u128::from(self.playing[0].load(Ordering::Relaxed))
            | (u128::from(self.playing[1].load(Ordering::Relaxed)) << 64)
    }

    fn take_attacks(&self) -> u128 {
        u128::from(self.attacks[0].swap(0, Ordering::Relaxed))
            | (u128::from(self.attacks[1].swap(0, Ordering::Relaxed)) << 64)
    }
}

pub struct PlayableKeyboard {
    link: Arc<KeyboardLink>,
    wake: Box<dyn Fn()>,
    pressed: Cell<Option<u8>>,
    frame: Cell<u128>,
}

impl PlayableKeyboard {
    pub fn new(link: Arc<KeyboardLink>, wake: impl Fn() + 'static) -> Self {
        link.take_attacks();
        for word in &link.playing {
            word.store(0, Ordering::Relaxed);
        }
        link.visible.store(true, Ordering::Release);
        wake();
        Self {
            link,
            wake: Box::new(wake),
            pressed: Cell::new(None),
            frame: Cell::new(0),
        }
    }

    pub fn poll(&self) -> bool {
        self.poll_with_attacks().0
    }

    /// Update the piano and return both a highlight change and the fresh note-on mask.
    pub(crate) fn poll_with_attacks(&self) -> (bool, u128) {
        let attacks = self.link.take_attacks();
        let notes = attacks | self.link.playing();
        (self.frame.replace(notes) != notes, attacks)
    }

    pub fn is_down(&self, note: u8) -> bool {
        note < 128 && self.frame.get() & (1u128 << note) != 0
    }

    pub fn draw(&self, d: &mut crate::Draw, piano: Piano) {
        piano.draw_instrument(d, self.frame.get());
    }

    fn set(&self, note: Option<u8>) {
        if self.pressed.replace(note) != note {
            self.link.audition(note);
            (self.wake)();
        }
    }

    pub fn event(&self, cx: &mut EventContext, e: &WindowEvent, piano: Piano, x: f32, y: f32) {
        self.event_for_note(cx, e, || piano.note_at(x, y));
    }

    /// Handle pointer auditions from a piano key or another note-bearing surface.
    pub fn event_for_note(
        &self,
        cx: &mut EventContext,
        e: &WindowEvent,
        note_at_pointer: impl FnOnce() -> Option<u8>,
    ) {
        match e {
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                if let Some(note) = note_at_pointer() {
                    cx.capture();
                    self.set(Some(note));
                }
            }
            WindowEvent::MouseMove(..) if self.pressed.get().is_some() => {
                let note = note_at_pointer();
                self.set(note);
                if note.is_none() {
                    cx.release();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left)
            | WindowEvent::FocusOut
            | WindowEvent::MouseLeave
                if self.pressed.get().is_some() =>
            {
                self.set(None);
                cx.release();
            }
            _ => {}
        }
    }
}

impl Drop for PlayableKeyboard {
    fn drop(&mut self) {
        self.link.visible.store(false, Ordering::Relaxed);
        self.set(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piano_gestures_hit_accidentals_and_release_on_cancel_or_close() {
        let link = Arc::new(KeyboardLink::default());
        let keyboard = PlayableKeyboard::new(link.clone(), || {});
        let mut context = Context::default();
        let entity = Element::new(&mut context).entity();
        let mut cx = EventContext::new_with_current(&mut context, entity);
        for (low, high) in [(28, 100), (33, 96), (0, 127)] {
            let piano = Piano::new((22.0, 440.0, 636.0, 60.0), low, high);
            for n in low..=high {
                let (x, y, w, h) = piano.key_rect(n);
                assert_eq!(piano.note_at(x + w * 0.5, y + h - 1.0), Some(n));
            }
        }

        let piano = Piano::new((0.0, 0.0, 140.0, 60.0), 60, 71);
        let press = WindowEvent::MouseDown(MouseButton::Left);
        let queued = || {
            let mut notes = Vec::new();
            link.drain_auditions(60..=71, |note, _| notes.push(note));
            notes
        };
        keyboard.event(&mut cx, &press, piano, 18.0, 55.0);
        let moved = WindowEvent::MouseMove(18.0, 12.0);
        keyboard.event(&mut cx, &moved, piano, 18.0, 12.0);
        keyboard.event(&mut cx, &moved, piano, 18.0, 12.0);
        keyboard.event(
            &mut cx,
            &WindowEvent::MouseUp(MouseButton::Left),
            piano,
            -1.0,
            -1.0,
        );
        assert_eq!(queued(), [Some(60), Some(61), None]);
        for repeated in [
            WindowEvent::MouseDoubleClick(MouseButton::Left),
            WindowEvent::MouseTripleClick(MouseButton::Left),
        ] {
            keyboard.event(&mut cx, &repeated, piano, 18.0, 12.0);
            keyboard.event(
                &mut cx,
                &WindowEvent::MouseUp(MouseButton::Left),
                piano,
                18.0,
                12.0,
            );
            assert_eq!(queued(), [Some(61), None]);
        }
        for cancel in [
            WindowEvent::FocusOut,
            WindowEvent::MouseLeave,
            WindowEvent::MouseMove(-1.0, -1.0),
        ] {
            keyboard.event(&mut cx, &press, piano, 10.0, 55.0);
            keyboard.event(&mut cx, &cancel, piano, -1.0, -1.0);
            assert_eq!(queued(), [Some(60), None]);
        }
        keyboard.event(&mut cx, &press, piano, 10.0, 55.0);
        drop(keyboard);
        assert_eq!(queued(), [Some(60), None]);
    }

    #[test]
    fn piano_frame_shows_short_notes_and_both_ends_of_a_chord() {
        let link = Arc::new(KeyboardLink::default());
        let view = PlayableKeyboard::new(link.clone(), || {});
        let chord = (1 << 28) | (1 << 100);
        link.publish(|| chord);
        link.attack(63);
        link.attack(64);
        assert!(view.poll());
        assert_eq!(view.frame.get(), chord | (1 << 63) | (1 << 64));
        assert!(view.poll());
        assert_eq!(view.frame.get(), chord);
        assert!(!view.poll());
        link.publish(|| 0);
        assert!(view.poll());
        assert_eq!(view.frame.get(), 0);
        drop(view);
        link.publish(|| panic!("closed keyboards must skip the voice scan"));
    }
}
