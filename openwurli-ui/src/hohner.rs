//! The D6 and Pianets have direct key actions and ignore the damper pedal.
use nih_plug::{midi::control_change, prelude::NoteEvent};

pub(super) enum Action {
    Down(f32),
    Up,
    Choke,
}

pub(super) fn dispatch(
    event: &NoteEvent<()>,
    range: std::ops::RangeInclusive<u8>,
    mut apply: impl FnMut(u8, Action),
) {
    use Action::*;
    match *event {
        NoteEvent::NoteOn {
            note, velocity: 0., ..
        } => apply(note, Up),
        NoteEvent::NoteOn { note, velocity, .. } if range.contains(&note) => {
            apply(note, Down(velocity))
        }
        NoteEvent::NoteOff { note, .. } if note > 127 => {
            for n in 0..=127 {
                apply(n, Up)
            }
        }
        NoteEvent::Choke { note, .. } if note > 127 => {
            for n in 0..=127 {
                apply(n, Choke)
            }
        }
        NoteEvent::NoteOff { note, .. } => apply(note, Up),
        NoteEvent::Choke { note, .. } => apply(note, Choke),
        NoteEvent::MidiCC {
            cc: control_change::ALL_SOUND_OFF,
            ..
        } => {
            for n in 0..=127 {
                apply(n, Choke)
            }
        }
        NoteEvent::MidiCC { cc, .. }
            if cc == control_change::ALL_NOTES_OFF || (124..=127).contains(&cc) =>
        {
            for n in 0..=127 {
                apply(n, Up)
            }
        }
        _ => {}
    }
}
