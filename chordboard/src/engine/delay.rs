//! Bounded MIDI echo scheduling. The caller owns MIDI channels and emitted voices.
//!
//! `beat` is the engine's continuous quarter-note position, including when the host
//! does not supply a transport position. Clear this runtime when seeking. Every
//! emitted On reserves an echo slot until its Off is returned; capacity pressure
//! can therefore drop attacks, but never loses a note-off that is already owed.
use super::types::Expression;
use crate::arpeggiator::DelaySettings;

pub const MAX_TRAINS: usize = 256;
pub const MAX_ECHOES: usize = 512;
pub const MAX_GENERATIONS: u8 = 64;
// MIDI velocity is delivered as f32. Preserve its exact representation of 1,
// which rounds slightly below the mathematical f64 value of 1 / 127.
const MIN_VELOCITY: f64 = (1.0_f32 / 127.0) as f64;
const SAMPLE_EPSILON: f64 = 0.000_000_1;
const BEAT_EPSILON: f64 = 0.000_000_001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    On {
        token: u64,
        note: u8,
        velocity: f32,
        expression: Expression,
        /// Cents, added to the captured expression bend by the channel allocator.
        fine: f32,
        /// Bipolar MIDI pan: -1 left, +1 right.
        pan: f32,
        /// Zero selects automatic allocation; 1..16 overrides the output channel.
        channel: u8,
    },
    Off {
        token: u64,
    },
}

#[derive(Clone, Copy, Debug)]
enum When {
    Samples { origin: u64, offset: f64 },
    Beats(f64),
}

impl When {
    fn remaining(self, now: u64, beat: f64, samples_per_beat: f64) -> f64 {
        match self {
            Self::Samples { origin, offset } => {
                if origin >= now {
                    (origin - now) as f64 + offset
                } else {
                    offset - (now - origin) as f64
                }
            }
            Self::Beats(at) => (at - beat) * samples_per_beat,
        }
    }

    fn due(self, now: u64, beat: f64) -> bool {
        match self {
            Self::Samples { .. } => self.remaining(now, beat, 1.0) <= SAMPLE_EPSILON,
            Self::Beats(at) => beat.is_finite() && at <= beat + BEAT_EPSILON,
        }
    }

    fn deadline(self, now: u64, beat: f64, samples_per_beat: f64) -> u64 {
        if self.due(now, beat) {
            return now;
        }
        let remaining = self.remaining(now, beat, samples_per_beat);
        if !remaining.is_finite() {
            return u64::MAX;
        }
        now.saturating_add((remaining - SAMPLE_EPSILON).ceil().max(1.0) as u64)
    }
}

#[derive(Clone, Copy, Debug)]
struct Timing {
    source_at: u64,
    source_beat: f64,
    /// Beats in synced mode, fractional samples otherwise.
    interval: f64,
    samples_per_beat: f64,
    sync: bool,
}

impl Timing {
    fn at(self, offset: f64) -> When {
        if self.sync {
            When::Beats(self.source_beat + offset)
        } else {
            When::Samples {
                origin: self.source_at,
                offset,
            }
        }
    }

    fn duration(self, now: u64, beat: f64) -> f64 {
        if self.sync && beat.is_finite() {
            (beat - self.source_beat).max(0.0)
        } else {
            let samples = now.saturating_sub(self.source_at) as f64;
            if self.sync {
                samples / self.samples_per_beat
            } else {
                samples
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Train {
    source_token: u64,
    timing: Timing,
    duration: Option<f64>,
    released: bool,
    generation: u8,
    note: u8,
    velocity: f64,
    feedback: f64,
    expression: Expression,
    fine: f32,
    pan: f32,
    channel: u8,
    ping_pong: bool,
    tap_index: u8,
}

impl Train {
    fn offset(self) -> f64 {
        self.timing.interval * f64::from(self.generation)
    }
    fn next_on(self) -> When {
        self.timing.at(self.offset())
    }
}

#[derive(Clone, Copy, Debug)]
struct Echo {
    token: u64,
    source_token: u64,
    off: Option<When>,
    offset: f64,
    sync: bool,
    samples_per_beat: f64,
    released: bool,
}

pub struct Runtime {
    trains: [Option<Train>; MAX_TRAINS],
    echoes: [Option<Echo>; MAX_ECHOES],
    next_token: u64,
    // Used only to order overdue events from different clocks. Normal scheduling
    // calls deadline() and visits each exact deadline, so the events are not late.
    samples_per_beat: f64,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            trains: [None; MAX_TRAINS],
            echoes: [None; MAX_ECHOES],
            next_token: 0,
            samples_per_beat: 24_000.0,
        }
    }
}

impl Runtime {
    /// Capture one dry attack. Tokens identify individual source voices and must
    /// remain unique until released. `off` is an absolute sample deadline, when
    /// known; held notes use None and must subsequently call release().
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        &mut self,
        mut config: DelaySettings,
        source: u8,
        source_token: u64,
        note: u8,
        velocity: f32,
        expression: Expression,
        off: Option<u64>,
        now: u64,
        beat: f64,
        sample_rate: f64,
        tempo: f64,
    ) {
        if !config.enabled || config.sources & source & 31 == 0 || note > 127 {
            return;
        }
        if !velocity.is_finite() || velocity <= 0.0 {
            return;
        }
        config.sanitize();
        let sample_rate = positive(sample_rate, 48_000.0);
        let samples_per_beat = sample_rate * 60.0 / positive(tempo, 120.0);
        self.samples_per_beat = samples_per_beat;
        for (tap_index, tap) in config.taps[..usize::from(config.count)].iter().enumerate() {
            let velocity = f64::from(velocity.min(1.0)) * f64::from(tap.velocity);
            let transposed = i16::from(note) + i16::from(tap.pitch);
            if !tap.enabled || velocity < MIN_VELOCITY || !(0..=127).contains(&transposed) {
                continue;
            }
            let Some(slot) = self.trains.iter_mut().find(|slot| slot.is_none()) else {
                break;
            };
            // Invalid external beat positions fall back to a captured-tempo sample
            // clock, so malformed transport information cannot strand an echo.
            let sync = config.sync && beat.is_finite();
            let interval = if sync {
                f64::from(tap.time)
            } else if config.sync {
                f64::from(tap.time) * samples_per_beat
            } else {
                f64::from(tap.time) * sample_rate / 1000.0
            };
            let timing = Timing {
                source_at: now,
                source_beat: beat,
                interval,
                samples_per_beat,
                sync,
            };
            let duration = off.map(|off| {
                let samples = off.saturating_sub(now) as f64;
                if sync {
                    samples / samples_per_beat
                } else {
                    samples
                }
            });
            *slot = Some(Train {
                source_token,
                timing,
                duration,
                released: false,
                generation: 1,
                note: transposed as u8,
                velocity,
                feedback: f64::from(tap.feedback),
                expression,
                fine: tap.fine,
                pan: tap.pan,
                channel: tap.channel,
                ping_pong: config.ping_pong,
                tap_index: tap_index as u8,
            });
        }
    }

    /// Mirror an actual source release onto every pending and sounding echo.
    /// The first release wins, making duplicate dry note-off notifications safe.
    pub fn release(&mut self, source_token: u64, now: u64, beat: f64) {
        for train in self.trains.iter_mut().flatten() {
            if train.source_token == source_token && !train.released {
                train.duration = Some(train.timing.duration(now, beat));
                train.released = true;
            }
        }
        for echo in self.echoes.iter_mut().flatten() {
            if echo.source_token == source_token && !echo.released {
                echo.off = Some(if echo.sync && beat.is_finite() {
                    When::Beats(beat + echo.offset)
                } else {
                    When::Samples {
                        origin: now,
                        offset: if echo.sync {
                            echo.offset * echo.samples_per_beat
                        } else {
                            echo.offset
                        },
                    }
                });
                echo.released = true;
            }
        }
    }

    /// Return the next absolute sample deadline, recalculating synced deadlines
    /// from the current beat and tempo on every call.
    pub fn deadline(&self, now: u64, beat: f64, samples_per_beat: f64) -> Option<u64> {
        let samples_per_beat = positive(samples_per_beat, self.samples_per_beat);
        self.trains
            .iter()
            .flatten()
            .map(|train| train.next_on())
            .chain(self.echoes.iter().flatten().filter_map(|echo| echo.off))
            .map(|at| at.deadline(now, beat, samples_per_beat))
            .min()
    }

    /// Drain this at each deadline until None. Note-offs win ties. An attack is
    /// returned only after reserving the slot for its eventual note-off.
    pub fn pop_due(&mut self, now: u64, beat: f64) -> Option<Action> {
        loop {
            let next_off = self
                .echoes
                .iter()
                .enumerate()
                .filter_map(|(index, echo)| {
                    let at = echo.as_ref()?.off?;
                    at.due(now, beat)
                        .then_some((index, at.remaining(now, beat, self.samples_per_beat)))
                })
                .min_by(|left, right| left.1.total_cmp(&right.1));
            let next_on = self
                .trains
                .iter()
                .enumerate()
                .filter_map(|(index, train)| {
                    let at = train.as_ref()?.next_on();
                    at.due(now, beat)
                        .then_some((index, at.remaining(now, beat, self.samples_per_beat)))
                })
                .min_by(|left, right| left.1.total_cmp(&right.1));

            if let Some((index, at)) = next_off {
                if next_on.is_none_or(|(_, on)| at <= on + SAMPLE_EPSILON) {
                    let echo = self.echoes[index].take()?;
                    return Some(Action::Off { token: echo.token });
                }
            }
            let (index, _) = next_on?;
            let train = self.trains[index].take()?;
            let next_velocity = train.velocity * train.feedback;
            if train.generation < MAX_GENERATIONS && next_velocity >= MIN_VELOCITY {
                self.trains[index] = Some(Train {
                    generation: train.generation + 1,
                    velocity: next_velocity,
                    ..train
                });
            }
            let Some(echo_index) = self.echoes.iter().position(Option::is_none) else {
                // Dropping an attack still advances that train. The loop has a
                // hard bound of MAX_TRAINS * MAX_GENERATIONS, even after a jump.
                continue;
            };
            self.next_token = self.next_token.wrapping_add(1).max(1);
            let token = self.next_token;
            let offset = train.offset();
            self.echoes[echo_index] = Some(Echo {
                token,
                source_token: train.source_token,
                off: train
                    .duration
                    .map(|duration| train.timing.at(offset + duration)),
                offset,
                sync: train.timing.sync,
                samples_per_beat: train.timing.samples_per_beat,
                released: train.released,
            });
            let pan = if train.ping_pong {
                if (train.generation + train.tap_index) % 2 == 1 {
                    -1.0
                } else {
                    1.0
                }
            } else {
                train.pan
            };
            return Some(Action::On {
                token,
                note: train.note,
                velocity: train.velocity.min(1.0) as f32,
                expression: train.expression,
                fine: train.fine,
                pan,
                channel: train.channel,
            });
        }
    }

    /// Discard scheduling only. The caller must release its sounding echo voices.
    pub fn clear(&mut self) {
        self.trains.fill(None);
        self.echoes.fill(None);
    }
}

fn positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
#[path = "delay_tests.rs"]
mod tests;
