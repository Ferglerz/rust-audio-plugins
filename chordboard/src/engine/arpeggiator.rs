//! Automatic arp clock, independent of the live Strumfield.
use super::*;
use crate::arpeggiator::{ArpMode, Prepared, Settings, Step, StepState, Store, STEPS};
pub const ARP_OWNER: u8 = 1;
#[derive(Clone, Copy)]
struct Event {
    at: u64,
    boundary: f64,
    seed: u32,
    phase: i64,
    step: usize,
    note: u8,
    adjustment: Step,
    interval: f64,
    position: f32,
    audible: bool,
}
pub struct Runtime {
    pub data: Box<Prepared>,
    pub active: u8,
    pub step: u8,
    pub display_phase: i64,
    pub running: bool,
    pub done: bool,
    pub(super) finished: bool,
    pub(super) applied_page: u8,
    pub seed: u32,
    plan_seed: u32,
    cached_notes: Notes,
    cached_generation: u64,
    cached_settings: Settings,
    cached_request: u8,
    pub(super) phase: i64,
    pub(super) next: f64,
    early: Option<i64>,
    events: [Option<Event>; 8],
    restore_epoch: u64,
    aligned: bool,
    last_settings: Settings,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            data: Store::new().initial_prepared(),
            active: 0,
            step: 0,
            display_phase: 0,
            running: false,
            done: false,
            finished: false,
            applied_page: 0,
            seed: 0x43484f52,
            plan_seed: 0x43484f52,
            cached_notes: Notes::default(),
            cached_generation: u64::MAX,
            cached_settings: Settings::default(),
            cached_request: 0,
            phase: 0,
            next: 0.0,
            early: None,
            events: [None; 8],
            restore_epoch: 0,
            aligned: false,
            last_settings: Settings::default(),
        }
    }
}
/// The displayed cycle is exactly the played cycle, including repeated occurrences.
pub fn cycle_notes(notes: Notes, settings: Settings, mut seed: u32) -> ([u8; STEPS], usize) {
    let mut strings = [0; 48];
    let mut count = 0;
    for i in 0..notes.len * settings.octaves.clamp(1, 4) as usize {
        if let Some(note) = notes.string(i) {
            strings[count] = note;
            count += 1;
        }
    }
    let mut result = [0; STEPS];
    if count == 0 {
        return (result, 0);
    }
    let len = Engine::pattern_length(settings.pattern, count).min(STEPS);
    for (step, slot) in result.iter_mut().enumerate().take(len) {
        let index = match settings.pattern {
            1 => count - 1 - step % count,
            2 => {
                let span = (count * 2).saturating_sub(2).max(1);
                let p = step % span;
                if p < count {
                    p
                } else {
                    span - p
                }
            }
            3 => {
                let k = step / 2;
                if step % 2 == 0 {
                    k % count
                } else {
                    (k + 2) % count
                }
            }
            4 => {
                let start = count - 1 - (step / 2) % count;
                if step % 2 == 0 {
                    start
                } else {
                    (start + count - 2 % count) % count
                }
            }
            6 => {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as usize % count
            }
            _ => step % count,
        };
        *slot = strings[index];
    }
    (result, len)
}
impl Engine {
    pub(super) fn retime_arp(&mut self, old_tempo: f64, new_tempo: f64) {
        if !self.config.arp_sync || old_tempo == new_tempo {
            return;
        }
        let factor = old_tempo / new_tempo;
        let now = self.now as f64;
        self.arp.next = now + (self.arp.next - now) * factor;
        for event in self.arp.events.iter_mut().flatten() {
            event.at = (now + (event.at as f64 - now) * factor).round().max(now) as u64;
            event.boundary = now + (event.boundary - now) * factor;
            event.interval *= factor;
        }
    }
    pub fn set_host_position(&mut self, beats: Option<f64>, tempo_available: bool) {
        self.host_beat_available = beats.is_some_and(f64::is_finite);
        self.host_tempo_available = tempo_available;
        if let Some(beat) = beats.filter(|b| b.is_finite()) {
            self.beat = beat;
        }
    }
    pub fn accept_arp_generation(&mut self, out: &mut impl FnMut(Out)) {
        if self.arp.restore_epoch != self.arp.data.restore_epoch {
            self.arp.restore_epoch = self.arp.data.restore_epoch;
            self.reset_automatic_arp(out);
            self.reset_delay(out);
            self.arp.active = self.host_config.arp_page.min(7);
            self.configure(self.host_config, out);
        }
        self.update_delay_power(out);
    }
    pub(super) fn arp_live_settings(&self) -> Settings {
        Settings {
            mode: self.config.arp_mode,
            pattern: self.config.arp_pattern,
            octaves: self.config.arp_octaves,
            sync: self.config.arp_sync,
            rate: self.config.rate,
            free_ms: self.config.arp_free_ms,
            swing: self.config.swing,
            humanize: self.config.humanize,
        }
    }
    pub(super) fn arp_alignment_available(&self) -> bool {
        self.config.arp_mode == ArpMode::AlignedLoop
            && self.config.arp_sync
            && self.config.host_sync
            && self.playing
            && self.host_beat_available
            && self.host_tempo_available
    }
    fn arp_gated(&self) -> bool {
        self.config.arp_enabled
            && self.notes.len > 0
            && (self.playing
                || (self.config.latch && self.memory.is_some())
                || self.root.is_some_and(|s| s.held)
                || self.held_inputs.iter().any(Option::is_some)
                || self.melody.iter().any(|n| n.iter().any(|&v| v)))
    }
    pub(super) fn restart_automatic_arp(&mut self) {
        self.arp.phase = 0;
        self.arp.next = self.now as f64;
        self.arp.done = false;
        self.arp.finished = false;
        self.arp.early = None;
        self.arp.events.fill(None);
        self.arp.running = false;
    }
    pub(super) fn reset_automatic_arp(&mut self, out: &mut impl FnMut(Out)) {
        self.restart_automatic_arp();
        self.sources[9..].fill(0.0);
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| v.owner == ARP_OWNER) {
                self.end_voice(i, 0.0, out);
            }
        }
    }
    pub(super) fn arp_cycle_chord(&self) -> Notes {
        let mut notes = self.notes;
        if self.config.arp_pattern == 5 && notes.len > 1 {
            if let Some(index) = self
                .second
                .and_then(|s| notes.as_slice().iter().position(|n| n % 12 == s.note % 12))
            {
                notes.values.swap(1, index);
            }
        }
        notes
    }
    fn arp_slots(&self) -> ([usize; STEPS], usize, [u8; STEPS]) {
        let (notes, len) = cycle_notes(
            self.arp_cycle_chord(),
            self.arp_live_settings(),
            self.arp.plan_seed,
        );
        let mut slots = [0; STEPS];
        let mut count = 0;
        for (i, step) in self.arp.data.pages[self.arp.active as usize]
            .steps
            .iter()
            .enumerate()
            .take(len)
        {
            if step.state != StepState::Skip {
                slots[count] = i;
                count += 1;
            }
        }
        (slots, count, notes)
    }
    pub(super) fn arp_deadline(&self) -> Option<u64> {
        if !self.arp_gated() {
            return self.arp.running.then_some(self.now);
        }
        if self.arp_alignment_available() != self.arp.aligned {
            return Some(self.now);
        }
        let count = self.arp_slots().1;
        let requested = self.config.arp_page.min(7) != self.arp.active;
        if !self.arp.running {
            return (count > 0 || requested).then_some(self.now);
        }
        let boundary =
            (!self.arp.finished || requested).then_some(self.arp.next.round().max(0.0) as u64);
        boundary
            .into_iter()
            .chain(self.arp.events.iter().flatten().map(|e| e.at))
            .min()
    }
    fn arp_interval(&self, phase: i64) -> f64 {
        let base = if self.config.arp_sync {
            self.sample_rate as f64 * 60.0 / self.tempo * self.config.rate.max(0.0001) as f64
        } else {
            self.sample_rate as f64 * self.config.arp_free_ms.max(1.0) as f64 / 1000.0
        };
        (base
            * (1.0
                + if phase.rem_euclid(2) == 0 {
                    self.config.swing as f64
                } else {
                    -self.config.swing as f64
                }))
        .max(1.0)
    }
    fn align_arp(&mut self) {
        let rate = self.config.rate.max(0.0001) as f64;
        let pair = (self.beat / (2.0 * rate)).floor();
        let within = self.beat - pair * 2.0 * rate;
        let odd = within >= rate * (1.0 + self.config.swing as f64) - 1e-9;
        let mut phase = pair as i64 * 2 + i64::from(odd);
        let mut boundary = pair * 2.0 * rate
            + if odd {
                rate * (1.0 + self.config.swing as f64)
            } else {
                0.0
            };
        if boundary < self.beat - 1e-8 {
            boundary += rate
                * (1.0
                    + if phase.rem_euclid(2) == 0 {
                        self.config.swing as f64
                    } else {
                        -self.config.swing as f64
                    });
            phase += 1;
        }
        self.arp.phase = phase;
        self.arp.next = self.now as f64
            + (boundary - self.beat).max(0.0) * self.sample_rate as f64 * 60.0 / self.tempo;
        self.arp.early = None;
        self.arp.events.fill(None);
    }
    fn prepare_arp_event(&mut self, phase: i64, boundary: f64, floor: u64) -> Option<Event> {
        let (slots, count, notes) = self.arp_slots();
        if count == 0 {
            return None;
        }
        let ordinal = phase.rem_euclid(count as i64) as usize;
        let index = slots[ordinal];
        let step = self.arp.data.pages[self.arp.active as usize].steps[index];
        let interval = self.arp_interval(phase);
        let jitter = self.humanize_timing(interval);
        let at = (boundary + interval * step.micro as f64 / 100.0 + jitter)
            .round()
            .max(floor as f64) as u64;
        let audible = step.state == StepState::Play
            && step.velocity > 0
            && (step.probability == 100 || (self.random() % 100) < step.probability as u32);
        Some(Event {
            at,
            boundary,
            seed: self.arp.plan_seed,
            phase,
            step: index,
            note: notes[index],
            adjustment: step,
            interval,
            position: if count > 1 {
                ordinal as f32 / (count - 1) as f32
            } else {
                0.0
            },
            audible,
        })
    }
    pub(super) fn refresh_arp_events(&mut self) {
        let settings = self.arp_live_settings();
        if self.arp.cached_notes == self.notes
            && self.arp.cached_settings == settings
            && self.arp.cached_generation == self.arp.data.generation
            && self.arp.cached_request == self.config.arp_page
        {
            return;
        }
        if self.arp_alignment_available()
            && (settings.rate != self.arp.cached_settings.rate
                || settings.swing != self.arp.cached_settings.swing)
        {
            self.arp.running = false;
        }
        self.arp.cached_notes = self.notes;
        self.arp.cached_settings = settings;
        self.arp.cached_generation = self.arp.data.generation;
        self.arp.cached_request = self.config.arp_page;
        for i in 0..self.arp.events.len() {
            if let Some(event) = self.arp.events[i].take() {
                if self.config.arp_page != self.arp.active && event.phase == self.arp.phase {
                    self.arp.early = None;
                } else {
                    self.arp.events[i] =
                        self.prepare_arp_event(event.phase, event.boundary, self.now);
                }
            }
        }
    }
    fn enqueue_arp(&mut self, event: Event) {
        if let Some(slot) = self.arp.events.iter_mut().find(|e| e.is_none()) {
            *slot = Some(event);
        }
    }
    pub(super) fn tick_automatic_arp(&mut self, out: &mut impl FnMut(Out)) {
        self.refresh_arp_events();
        if !self.arp_gated() {
            if self.arp.running {
                self.restart_automatic_arp();
                self.sources[9..].fill(0.0);
                self.configure(self.host_config, out);
            }
            return;
        }
        let aligned = self.arp_alignment_available();
        let settings = self.arp_live_settings();
        if aligned
            && (!self.arp.aligned
                || settings.rate != self.arp.last_settings.rate
                || settings.swing != self.arp.last_settings.swing)
        {
            self.align_arp();
        }
        self.arp.aligned = aligned;
        self.arp.last_settings = settings;
        if !self.arp.running {
            self.arp.running = true;
            if aligned {
                self.align_arp();
            } else {
                self.arp.next = self.now as f64;
            }
        }
        if self.now >= self.arp.next.round().max(0.0) as u64 {
            let requested = self.config.arp_page.min(7);
            if requested != self.arp.active {
                let consumed = self.arp.early == Some(self.arp.phase)
                    && !self
                        .arp
                        .events
                        .iter()
                        .flatten()
                        .any(|e| e.phase == self.arp.phase);
                self.arp.active = requested;
                self.arp.events.fill(None);
                self.arp.early = if consumed { Some(self.arp.phase) } else { None };
                if self.arp.done
                    || self.host_config.arp_settings[requested as usize].mode == ArpMode::Once
                {
                    self.arp.phase = 0;
                    self.arp.next = self.now as f64;
                    self.arp.done = false;
                    self.arp.finished = false;
                }
                self.configure(self.host_config, out);
                if self.arp_alignment_available() {
                    self.align_arp();
                }
            }
            let (_, count, _) = self.arp_slots();
            if count == 0 {
                self.arp.running = false;
                self.arp.events.fill(None);
                self.sources[9..].fill(0.0);
                return;
            }
            if self.arp.done && !self.arp.finished {
                self.arp.finished = true;
                self.sources[9..].fill(0.0);
                self.configure(self.host_config, out);
            }
            if !self.arp.done {
                let phase = self.arp.phase;
                if self.arp.early != Some(phase) {
                    if let Some(event) = self.prepare_arp_event(phase, self.arp.next, self.now) {
                        self.enqueue_arp(event);
                    }
                }
                self.arp.early = None;
                self.arp.next += self.arp_interval(phase);
                self.arp.phase = self.arp.phase.wrapping_add(1);
                self.arp.done =
                    self.config.arp_mode == ArpMode::Once && self.arp.phase >= count as i64;
                if self.config.arp_pattern == 6 && self.arp.phase.rem_euclid(count as i64) == 0 {
                    self.arp.plan_seed = self.random();
                }
                if !self.arp.done {
                    // Prepare the next event once, retaining it even if it is late. This makes
                    // negative microtiming and humanize independent of callback boundaries.
                    if let Some(event) =
                        self.prepare_arp_event(self.arp.phase, self.arp.next, self.now)
                    {
                        self.enqueue_arp(event);
                        self.arp.early = Some(self.arp.phase);
                    }
                }
            }
        }
        loop {
            let index = self
                .arp
                .events
                .iter()
                .enumerate()
                .filter_map(|(i, e)| e.filter(|e| e.at <= self.now).map(|e| (i, e)))
                .min_by_key(|(_, e)| (e.at, e.phase))
                .map(|(i, _)| i);
            let Some(index) = index else {
                break;
            };
            let Some(event) = self.arp.events[index].take() else {
                break;
            };
            self.arp.step = event.step as u8;
            self.arp.display_phase = event.phase;
            self.arp.seed = event.seed;
            self.sources[9] = event.adjustment.mods[0] as f32 / 100.0;
            self.sources[10] = event.adjustment.mods[1] as f32 / 100.0;
            self.sources[11] = event.position;
            self.configure(self.host_config, out);
            if event.audible {
                let (notes, len) =
                    cycle_notes(self.arp_cycle_chord(), self.arp_live_settings(), event.seed);
                let note = if len > 0 {
                    notes[event.step % len]
                } else {
                    event.note
                };
                let pitch = note as i16 + event.adjustment.octave as i16 * 12;
                if (0..=127).contains(&pitch) {
                    let velocity = self.base_strike_velocity() * event.adjustment.velocity as f32
                        / 100.0
                        * self.humanize_velocity_factor();
                    // Gate is the cell's absolute percentage, with routed Gate replacing or
                    // offsetting that base through the existing modulation matrix.
                    let mut gate_config = self.config;
                    gate_config.gate = event.adjustment.gate as f32 / 100.0;
                    self.apply_routes(&mut gate_config);
                    let duration =
                        (event.interval * gate_config.gate as f64).round().max(1.0) as u64;
                    if velocity > 0.0 {
                        self.strike_prepared(pitch as u8, velocity, duration, ARP_OWNER, out);
                    }
                }
            }
        }
    }
}
