//! A bounded MIDI tape head. Trigger input stays live; output follows tape time.
use nih_plug::prelude::NoteEvent;
use std::collections::VecDeque;

const CAPACITY: usize = 16_384;
const BEND_RANGE: f32 = 48.0;

#[derive(Clone, Copy)]
struct Channel {
    used: bool,
    notes: [bool; 128],
    bend: f32,
    range: [u8; 2],
    rpn: [u8; 2],
    nrpn: bool,
    wide: bool,
    last_bend: f32,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            used: false,
            notes: [false; 128],
            bend: 0.5,
            range: [2, 0],
            rpn: [127, 127],
            nrpn: false,
            wide: false,
            last_bend: 0.5,
        }
    }
}

pub(crate) struct MidiTape {
    queue: VecDeque<(u64, NoteEvent<()>)>,
    write: u64,
    read: f64,
    channels: [Channel; 16],
    enabled: bool,
    reset_pending: bool,
    overflow: bool,
    pitch_interval: u32,
    pitch_countdown: u32,
    braking: bool,
}

impl MidiTape {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(CAPACITY),
            write: 0,
            read: 0.0,
            channels: [Channel::default(); 16],
            enabled: true,
            reset_pending: false,
            overflow: false,
            pitch_interval: 192,
            pitch_countdown: 0,
            braking: false,
        }
    }

    pub fn set_sample_rate(&mut self, rate: f32) {
        self.pitch_interval = (rate / 250.0).round().max(1.0) as u32;
    }

    pub fn reset(&mut self) {
        // reset() cannot emit events. Retain ownership until the next process call.
        self.reset_pending = true;
    }

    pub fn begin_block(&mut self, enabled: bool, out: &mut impl FnMut(NoteEvent<()>)) {
        if self.reset_pending || self.enabled != enabled {
            self.cleanup(0, out);
            self.queue.clear();
            self.write = 0;
            self.read = 0.0;
            self.overflow = false;
            self.pitch_countdown = 0;
            self.reset_pending = false;
            self.braking = false;
        }
        self.enabled = enabled;
    }

    pub fn push(&mut self, event: NoteEvent<()>, out: &mut impl FnMut(NoteEvent<()>)) {
        if !self.enabled || self.overflow {
            self.emit(event, 0.0, out);
        } else if self.queue.len() == CAPACITY {
            // A prolonged stop can fill the tape. Release owned voices, discard the
            // backlog and pass through until the tape is live again. Never allocate.
            self.cleanup(event.timing(), out);
            self.queue.clear();
            self.overflow = true;
            self.emit(event, 0.0, out);
        } else {
            self.queue.push_back((self.write, event));
        }
    }

    pub fn sample(
        &mut self,
        timing: u32,
        speed: f32,
        braking: bool,
        live: bool,
        out: &mut impl FnMut(NoteEvent<()>),
    ) {
        let active = self.enabled && !self.overflow;
        let semitones = if active {
            (12.0 * speed.max(1.0 / 16.0).log2()).clamp(-BEND_RANGE, BEND_RANGE)
        } else {
            0.0
        };
        // A new brake re-aligns the audio read head to live input, including
        // retriggers during return. MIDI must make the same jump.
        if live || !active || (braking && !self.braking) {
            self.read = self.write as f64;
        }
        self.braking = braking;
        // Emit at most 250 pitch updates/sec/channel, plus exact note/controller
        // initialization and restoration. Keep this cadence across host blocks.
        if self.pitch_countdown == 0 || semitones == 0.0 {
            for (index, channel) in self.channels.iter_mut().enumerate() {
                if channel.used {
                    channel.pitch(index as u8, timing, semitones, out);
                }
            }
            self.pitch_countdown = self.pitch_interval;
        }
        self.pitch_countdown = self.pitch_countdown.saturating_sub(1);
        while self
            .queue
            .front()
            .is_some_and(|(at, _)| *at as f64 <= self.read + 1e-7)
        {
            if let Some((_, mut event)) = self.queue.pop_front() {
                retime(&mut event, timing);
                self.emit(event, semitones, out);
            }
        }
        self.write += 1;
        self.read = (self.read + speed.max(0.0) as f64).min(self.write as f64);
        if live {
            self.overflow = false;
        }
    }

    fn emit(&mut self, event: NoteEvent<()>, semitones: f32, out: &mut impl FnMut(NoteEvent<()>)) {
        let Some(index) = event.channel().filter(|&c| c < 16) else {
            out(event);
            return;
        };
        let channel = &mut self.channels[index as usize];
        channel.used = true;
        match event {
            NoteEvent::NoteOn { note, velocity, .. } if note < 128 => {
                channel.notes[note as usize] = velocity > 0.0;
            }
            NoteEvent::NoteOff { note, .. } | NoteEvent::Choke { note, .. } if note < 128 => {
                channel.notes[note as usize] = false;
            }
            NoteEvent::MidiPitchBend { value, .. } => {
                channel.bend = value;
                if semitones != 0.0 {
                    channel.pitch(index, event.timing(), semitones, out);
                    return;
                }
                channel.last_bend = value;
            }
            NoteEvent::MidiCC { cc, value, .. } => {
                let value = (value * 127.0).round() as u8;
                match cc {
                    98..=101 => {
                        let nrpn = cc < 100;
                        if channel.nrpn != nrpn {
                            channel.rpn = [127, 127];
                        }
                        channel.nrpn = nrpn;
                        channel.rpn[usize::from(cc == 100 || cc == 98)] = value;
                    }
                    6 | 38 if !channel.nrpn && channel.rpn == [0, 0] => {
                        channel.range[usize::from(cc == 38)] = value;
                        // The incoming range write will overwrite our wide range.
                        // Reassert it after forwarding the controller below.
                        if channel.wide {
                            channel.wide = semitones == 0.0;
                        }
                    }
                    120 | 123 => channel.notes.fill(false),
                    _ => {}
                }
                out(event);
                channel.pitch(index, event.timing(), semitones, out);
                return;
            }
            _ => {}
        }
        channel.pitch(index, event.timing(), semitones, out);
        out(event);
    }

    fn cleanup(&mut self, timing: u32, out: &mut impl FnMut(NoteEvent<()>)) {
        for (index, channel) in self.channels.iter_mut().enumerate() {
            let index = index as u8;
            if !channel.used {
                continue;
            }
            for (note, held) in channel.notes.iter_mut().enumerate() {
                if *held {
                    out(NoteEvent::NoteOff {
                        timing,
                        voice_id: None,
                        channel: index,
                        note: note as u8,
                        velocity: 0.0,
                    });
                    *held = false;
                }
            }
            // Include pedal-held and overlapping voices in emergency cleanup.
            for cc in [64, 66, 123] {
                send_cc(index, timing, cc, 0, out);
            }
            channel.pitch(index, timing, 0.0, out);
        }
    }
}

impl Channel {
    fn pitch(
        &mut self,
        channel: u8,
        timing: u32,
        semitones: f32,
        out: &mut impl FnMut(NoteEvent<()>),
    ) {
        let wide = semitones != 0.0;
        if wide != self.wide {
            let range = if wide { [48, 0] } else { self.range };
            let selectors = if self.nrpn { [99, 98] } else { [101, 100] };
            for (cc, value) in [
                (101, 0),
                (100, 0),
                (6, range[0]),
                (38, range[1]),
                (selectors[0], self.rpn[0]),
                (selectors[1], self.rpn[1]),
            ] {
                send_cc(channel, timing, cc, value, out);
            }
            self.wide = wide;
            self.last_bend = -1.0;
        }
        let range = self.range[0] as f32 + self.range[1] as f32 / 100.0;
        let value = if wide {
            (0.5 + ((self.bend - 0.5) * 2.0 * range + semitones) / (2.0 * BEND_RANGE))
                .clamp(0.0, 1.0)
        } else {
            self.bend
        };
        // MIDI pitch bend has 14-bit precision; do not resend identical values.
        if (value * 16383.0).round() != (self.last_bend * 16383.0).round() {
            out(NoteEvent::MidiPitchBend {
                timing,
                channel,
                value,
            });
            self.last_bend = value;
        }
    }
}

fn send_cc(channel: u8, timing: u32, cc: u8, value: u8, out: &mut impl FnMut(NoteEvent<()>)) {
    out(NoteEvent::MidiCC {
        timing,
        channel,
        cc,
        value: value as f32 / 127.0,
    });
}

fn retime(event: &mut NoteEvent<()>, at: u32) {
    match event {
        NoteEvent::NoteOn { timing, .. }
        | NoteEvent::NoteOff { timing, .. }
        | NoteEvent::Choke { timing, .. }
        | NoteEvent::VoiceTerminated { timing, .. }
        | NoteEvent::PolyModulation { timing, .. }
        | NoteEvent::MonoAutomation { timing, .. }
        | NoteEvent::PolyPressure { timing, .. }
        | NoteEvent::PolyVolume { timing, .. }
        | NoteEvent::PolyPan { timing, .. }
        | NoteEvent::PolyTuning { timing, .. }
        | NoteEvent::PolyVibrato { timing, .. }
        | NoteEvent::PolyExpression { timing, .. }
        | NoteEvent::PolyBrightness { timing, .. }
        | NoteEvent::MidiChannelPressure { timing, .. }
        | NoteEvent::MidiPitchBend { timing, .. }
        | NoteEvent::MidiCC { timing, .. }
        | NoteEvent::MidiProgramChange { timing, .. }
        | NoteEvent::MidiSysEx { timing, .. } => *timing = at,
        _ => {}
    }
}

#[cfg(test)]
#[path = "midi_tests.rs"]
mod tests;
