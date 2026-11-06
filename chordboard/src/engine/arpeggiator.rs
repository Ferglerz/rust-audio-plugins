//! Automatic arp clock, independent of the live Strumfield.
use super::*;
use crate::arpeggiator::{ArpMode, Prepared, Settings, Step, StepState, Store, LOOPS, STEPS};
use routing::{ARP_MOD_SOURCES, ARP_POSITION_SOURCE, PERF_MOD_SOURCES};
pub use super::{ARP_OWNER, PERFORMANCE_ARP_OWNER};
pub fn pass_length(notes: usize, pattern: u8) -> usize {
    Engine::pattern_length(pattern, notes)
}
#[derive(Clone, Copy)]
struct Event {
    at: u64,
    boundary: f64,
    seed: u32,
    phase: i64,
    step: usize,
    note: i16,
    adjustment: Step,
    interval: f64,
    position: f32,
    audible: bool,
    velocity_scale: f32,
    gate_scale: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Timing {
    mode: ArpMode,
    pattern: u8,
    locked: bool,
    steps: u8,
    fill: crate::arpeggiator::Fill,
}
impl Timing {
    fn from(settings: Settings) -> Self {
        Self {
            mode: settings.mode,
            pattern: settings.pattern,
            locked: settings.lock_length,
            steps: settings.steps,
            fill: settings.fill,
        }
    }
    pub(super) fn apply(self, mut settings: Settings) -> Settings {
        settings.mode = self.mode;
        settings.pattern = self.pattern;
        settings.lock_length = self.locked;
        settings.steps = self.steps;
        settings.fill = self.fill;
        settings
    }
}
pub struct Runtime {
    pub(super) timing: Option<Timing>,
    pub(super) timing_commit: bool,
    pub data: Box<Prepared>,
    pub active: u8,
    pub row_index: usize,
    pub row_id: u64,
    pub loops_in_row: u32,
    counted_phase: i64,
    pub step: u8,
    pub display_phase: i64,
    pub running: bool,
    pub(super) paused_at: Option<u64>,
    pub(super) requested_row: Option<usize>,
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
    clock_phase: i64,
    pub(super) next: f64,
    early: Option<i64>,
    events: [Option<Event>; 8],
    restore_epoch: u64,
    aligned: bool,
    last_settings: Settings,
    pub(super) paste_settings: [Option<Settings>; 8],
    paste_seen: [u64; 8],
    pub(super) preserve_paste_clock: bool,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            timing: None,
            timing_commit: false,
            data: Store::new().initial_prepared(),
            active: 0,
            row_index: 0,
            row_id: 1,
            loops_in_row: 0,
            counted_phase: 0,
            step: 0,
            display_phase: 0,
            running: false,
            paused_at: None,
            requested_row: None,
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
            clock_phase: 0,
            next: 0.0,
            early: None,
            events: [None; 8],
            restore_epoch: 0,
            aligned: false,
            last_settings: Settings::default(),
            paste_settings: [None; 8],
            paste_seen: [0; 8],
            preserve_paste_clock: false,
        }
    }
}
/// The displayed cycle is exactly the played cycle, including repeated occurrences.
pub fn cycle_notes(notes: Notes, settings: Settings, mut seed: u32) -> ([u8; STEPS], usize) {
    let mut strings = [0; 96];
    let mut count = 0;
    for i in 0..notes.len * settings.octaves.clamp(1, LOOPS as u8) as usize {
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
        let index = ordering::index(settings.pattern, step, count, &mut seed);
        *slot = strings[index];
    }
    (result, len)
}
/// Each loop traverses the selected ordering at its own octave, with stable step slots.
pub fn loop_cycle_notes(
    notes: Notes,
    settings: Settings,
    octaves: [i8; LOOPS],
    mut seed: u32,
) -> ([i16; STEPS], usize) {
    let mut result = [0; STEPS];
    if notes.len == 0 {
        return (result, 0);
    }
    let pass_len = Engine::pattern_length(settings.pattern, notes.len);
    let len = (pass_len * settings.octaves.clamp(1, LOOPS as u8) as usize).min(STEPS);
    for (step, slot) in result.iter_mut().enumerate().take(len) {
        let index = ordering::index(settings.pattern, step % pass_len, notes.len, &mut seed);
        *slot = notes.values[index] as i16 + octaves[step / pass_len] as i16 * 12;
    }
    (result, len)
}

/// A locked loop measures clock slots, independently of harmony or ordering.
pub fn settings_pass_length(notes: usize, settings: Settings) -> usize {
    if settings.lock_length {
        settings.steps.clamp(1, 24) as usize
    } else if settings.pattern == 7 {
        7
    } else {
        pass_length(notes, settings.pattern).max(1)
    }
}
/// Absolute sequence position, shared by the scheduler and editor.
pub fn program_index(settings: Settings, slot: usize, phase: i64, pass_len: usize) -> usize {
    if settings.lock_length && settings.fill == crate::arpeggiator::Fill::Evolve {
        let group = pass_len * settings.octaves.clamp(1, LOOPS as u8) as usize;
        phase.max(0) as usize / group * group + slot
    } else {
        slot % pass_len
    }
}
pub fn program_cycle(
    notes: Notes,
    recipe: Option<crate::harmony::SavedChord>,
    globals: crate::harmony::GlobalHarmony,
    settings: Settings,
    page: &crate::arpeggiator::Page,
    mut seed: u32,
    phase: i64,
) -> ([i16; STEPS], usize) {
    let mut result = [i16::MIN; STEPS];
    // An inaudible voicing must not stop a locked row's clock.
    if notes.len == 0 && (recipe.is_none() || (!settings.lock_length && settings.pattern != 7)) {
        return (result, 0);
    }
    let pass_len = settings_pass_length(notes.len, settings);
    let len = pass_len * settings.octaves.clamp(1, LOOPS as u8) as usize;
    for loop_index in 0..settings.octaves.clamp(1, LOOPS as u8) as usize {
        let variation = &page.loops[loop_index];
        let voiced = if variation.inversion == 0 {
            notes
        } else {
            recipe.map_or_else(
                || crate::harmony::rotate_voicing(notes, variation.inversion),
                |chord| crate::harmony::resolve_loop_chord(chord, globals, variation.inversion),
            )
        };
        for local in 0..pass_len {
            let slot = loop_index * pass_len + local;
            let sequence = program_index(settings, slot, phase, pass_len);
            let note = if settings.pattern == 7 {
                let custom = settings.custom[sequence % 7];
                recipe
                    .and_then(|chord| {
                        crate::harmony::custom_degree_pitch(
                            chord,
                            globals,
                            variation.inversion,
                            custom.degree,
                        )
                    })
                    .map(|n| n + custom.octave as i16 * 12)
            } else if voiced.len > 0 {
                let index = ordering::index(settings.pattern, sequence, voiced.len, &mut seed);
                Some(voiced.values[index] as i16)
            } else {
                None
            };
            if let Some(note) = note {
                result[slot] = note + variation.octave as i16 * 12;
            }
        }
    }
    (result, len)
}

impl Engine {
    pub(super) fn active_arp_page(&self) -> &crate::arpeggiator::Page {
        if self.in_performance_arp {
            return &self.arp.data.performance.page;
        }
        if self.config.tracker_enabled {
            self.arp
                .data
                .rows
                .get(self.arp.row_index)
                .map(|row| if row.enabled { &row.page } else { &self.arp.data.performance.page })
                .unwrap_or(&self.arp.data.pages[0])
        } else {
            &self.arp.data.pages[self.arp.active as usize]
        }
    }
    fn advance_tracker_row(&mut self, out: &mut impl FnMut(Out)) {
        if self.arp.data.rows.is_empty() {
            return;
        }
        self.arp.row_index = (self.arp.row_index + 1) % self.arp.data.rows.len();
        self.arp.row_id = self.arp.data.rows[self.arp.row_index].id;
        self.arp.active = self.arp.row_index.min(u8::MAX as usize) as u8;
        self.arp.loops_in_row = 0;
        self.arp.counted_phase = 0;
        self.arp.timing = None;
        self.arp.events.fill(None);
        self.arp.early = None;
        self.arp.phase = 0;
        self.arp.done = false;
        self.arp.finished = false;
        self.configure(self.host_config, out);
        self.arp.next = self.now as f64;
    }
    pub(super) fn retime_arp(&mut self, old_tempo: f64, new_tempo: f64) {
        if !self.config.arp_sync || old_tempo == new_tempo {
            return;
        }
        let factor = old_tempo / new_tempo;
        let now = self.arp.paused_at.unwrap_or(self.now) as f64;
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
        // A stopped host repeats its cursor position every callback. Keep the
        // continuous performance clock running for synced echoes during live play.
        if let Some(beat) = beats.filter(|b| self.playing && b.is_finite()) {
            self.beat = beat;
        }
    }
    pub fn accept_arp_generation(&mut self, out: &mut impl FnMut(Out)) {
        if self.arp.restore_epoch != self.arp.data.restore_epoch {
            self.arp.restore_epoch = self.arp.data.restore_epoch;
            self.arp.paste_settings.fill(None);
            self.arp.paste_seen.fill(0);
            self.arp.preserve_paste_clock = false;
            self.reset_automatic_arp(out);
            self.reset_delay(out);
            let ctx = self.fx_context();
            self.fx.clear(ctx, out);
            self.arp.active = self.host_config.arp_page.min(7);
            self.configure(self.host_config, out);
        }
        if self.host_config.tracker_enabled && !self.arp.data.rows.is_empty() {
            if let Some(index) = self
                .arp
                .data
                .rows
                .iter()
                .position(|row| row.id == self.arp.row_id)
            {
                self.arp.row_index = index;
            } else {
                if self.arp.row_index >= self.arp.data.rows.len() {
                    self.arp.row_index = 0;
                }
                self.arp.row_id = self.arp.data.rows[self.arp.row_index].id;
                self.arp.loops_in_row = 0;
                self.arp.events.fill(None);
            }
            self.arp.active = self.arp.row_index.min(u8::MAX as usize) as u8;
            self.configure(self.host_config, out);
        }
        for page in 0..8 {
            if let Some(replacement) = self.arp.data.pages[page].replacement {
                if replacement.serial != self.arp.paste_seen[page] {
                    self.arp.paste_seen[page] = replacement.serial;
                    self.arp.paste_settings[page] = Some(replacement.settings);
                    if page == self.arp.active as usize {
                        self.arp.preserve_paste_clock = true;
                        self.arp.timing = Some(Timing::from(replacement.settings));
                        if !self.arp.finished {
                            self.arp.done = false;
                        }
                    }
                }
            }
        }
        let previous_sources = self.sources;
        self.with_performance_arp(|e| {
            if e.arp.restore_epoch != e.arp.data.restore_epoch {
                e.arp.restore_epoch = e.arp.data.restore_epoch;
                e.reset_automatic_arp(out);
            }
            if !e.arp_gated() { e.reset_automatic_arp(out); }
        });
        self.refresh_perf_mod_routes(previous_sources, out);
        self.sync_layers(out);
        self.update_delay_power(out);
        self.configure_fx(out);
    }
    pub(super) fn arp_live_settings(&self) -> Settings {
        let mut settings = self.arp_requested_settings();
        settings.swing = self.config.swing;
        settings.humanize = self.config.humanize;
        self.arp
            .timing
            .map_or(settings, |timing| timing.apply(settings))
    }
    fn arp_requested_settings(&self) -> Settings {
        if self.in_performance_arp { return self.arp.data.performance.settings; }
        if self.config.tracker_enabled {
            return self
                .arp
                .data
                .rows
                .get(self.arp.row_index)
                .map_or(Settings::default(), |row| if row.enabled { row.settings } else { self.arp.data.performance.settings });
        }
        let requested = self.arp.paste_settings[self.arp.active.min(7) as usize]
            .unwrap_or(self.host_config.arp_settings[self.arp.active.min(7) as usize]);
        Settings {
            mode: requested.mode,
            pattern: requested.pattern,
            octaves: self.config.arp_octaves,
            sync: self.config.arp_sync,
            rate: self.config.rate,
            free_ms: self.config.arp_free_ms,
            swing: self.config.swing,
            humanize: self.config.humanize,
            ..requested
        }
    }
    pub(super) fn arp_alignment_available(&self) -> bool {
        self.config.daw_alignment
            && self.config.arp_sync
            && self.config.host_sync
            && self.playing
            && self.host_beat_available
            && self.host_tempo_available
    }
    pub(super) fn arp_gated(&self) -> bool {
        if self.config.tracker_enabled {
            self.config.arp_enabled
                && self
                    .arp
                    .data
                    .rows
                    .get(self.arp.row_index)
                    .is_some_and(|row| row.chord.is_some())
        } else {
            self.config.arp_enabled
                && self.notes.len > 0
                && ((!self.in_performance_arp && self.playing)
                    || self.pedal
                    || (self.config.latch && self.memory.is_some())
                    || self.root.is_some_and(|s| s.held)
                    || self.held_inputs.iter().any(Option::is_some)
                    || self.melody.iter().any(|n| n.iter().any(|&v| v)))
        }
    }
    pub(super) fn tracker_play(&mut self, selected: usize, out: &mut impl FnMut(Out)) {
        if !self.config.tracker_enabled || self.arp.data.rows.is_empty() {
            return;
        }
        // The host can apply the Play button's power parameter after its UI command.
        if !self.config.arp_enabled {
            self.arp.requested_row = Some(selected);
            return;
        }
        self.arp.requested_row = None;
        if let Some(paused_at) = self.arp.paused_at.take() {
            let elapsed = self.now.saturating_sub(paused_at);
            self.arp.next += elapsed as f64;
            for event in self.arp.events.iter_mut().flatten() {
                event.at = event.at.saturating_add(elapsed);
                event.boundary += elapsed as f64;
            }
            return;
        }
        if self.arp.running && !self.arp.finished {
            return;
        }
        self.reset_automatic_arp(out);
        self.arp.row_index = selected.min(self.arp.data.rows.len() - 1);
        self.arp.row_id = self.arp.data.rows[self.arp.row_index].id;
        self.arp.active = self.arp.row_index.min(u8::MAX as usize) as u8;
        self.arp.loops_in_row = 0;
        self.configure(self.host_config, out);
    }

    pub(super) fn tracker_pause(&mut self, out: &mut impl FnMut(Out)) {
        if !self.config.tracker_enabled || !self.config.arp_enabled || self.arp.paused_at.is_some() {
            return;
        }
        self.arp.paused_at = Some(self.now);
        self.clear_arp_sources();
        // Release only the tracker voices; live keyboard/Strumfield notes remain playable.
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|voice| voice.owner == self.automatic_arp_owner()) {
                self.end_voice(i, 0.0, out);
            }
        }
    }

    pub(super) fn restart_automatic_arp(&mut self) {
        if self.arp.paused_at.is_some() { self.arp.paused_at = Some(self.now); }
        self.arp.phase = 0;
        self.arp.clock_phase = 0;
        self.arp.counted_phase = 0;
        self.arp.timing = None;
        self.arp.next = self.now as f64;
        self.arp.done = false;
        self.arp.finished = false;
        self.arp.early = None;
        self.arp.events.fill(None);
        self.arp.running = false;
    }
    pub(super) fn reset_automatic_arp(&mut self, out: &mut impl FnMut(Out)) {
        self.restart_automatic_arp();
        self.clear_arp_sources();
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| v.owner == self.automatic_arp_owner()) {
                self.end_voice(i, 0.0, out);
            }
        }
    }
    fn arp_mod_sources(&self) -> std::ops::Range<usize> {
        if self.in_performance_arp {
            PERF_MOD_SOURCES
        } else {
            ARP_MOD_SOURCES
        }
    }
    fn clear_arp_sources(&mut self) {
        let lanes = self.arp_mod_sources();
        self.sources[lanes].fill(0.0);
        self.sources[ARP_POSITION_SOURCE] = 0.0;
    }
    pub(super) fn arp_cycle_chord(&self) -> Notes {
        let mut notes = if self.config.tracker_enabled {
            self.arp
                .data
                .rows
                .get(self.arp.row_index)
                .and_then(|row| row.chord)
                .map(|chord| crate::harmony::resolve_chord(chord, self.config.global_harmony()))
                .unwrap_or_default()
        } else {
            self.notes
        };
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
    pub(super) fn arp_pass_len(&self) -> usize {
        settings_pass_length(self.arp_cycle_chord().len, self.arp_live_settings())
    }
    #[cfg(test)]
    pub(super) fn effective_arp_gate(&self, gate: u8) -> f32 {
        self.effective_arp_gate_scaled(gate, 1.0)
    }
    pub(super) fn effective_arp_gate_scaled(&self, gate: u8, custom: f32) -> f32 {
        let mut config = self.config;
        config.gate = gate as f32 / 100.0;
        self.apply_routes(&mut config);
        (config.gate * self.config.gate_scale * custom).clamp(0.01, custom.max(1.0))
    }
    pub(super) fn arp_recipe(&self) -> Option<crate::harmony::SavedChord> {
        if self.config.tracker_enabled {
            self.arp
                .data
                .rows
                .get(self.arp.row_index)
                .and_then(|r| r.chord)
        } else {
            self.memory.map(|mut recipe| {
                recipe.transpose = 0;
                recipe
            })
        }
    }
    fn effective_arp_page(&self) -> crate::arpeggiator::Page {
        let mut page = *self.active_arp_page();
        for (i, variation) in page.loops.iter_mut().enumerate() {
            variation.octave = self.config.arp_loop_octaves[i];
        }
        if self.config.tracker_enabled {
            if let Some(row) = self.arp.data.rows.get(self.arp.row_index) {
                page.offset_voicing(row.arp_octave, row.arp_inversion);
            }
        }
        page
    }
    pub(super) fn resolved_arp_cycle(&self, phase: i64, seed: u32) -> ([i16; STEPS], usize) {
        program_cycle(
            self.arp_cycle_chord(),
            self.arp_recipe(),
            self.config.global_harmony(),
            self.arp_live_settings(),
            &self.effective_arp_page(),
            seed,
            phase,
        )
    }
    pub(super) fn arp_slots(&self) -> ([usize; STEPS], usize, [i16; STEPS]) {
        self.arp_slots_at(self.arp.phase)
    }
    fn arp_slots_at(&self, phase: i64) -> ([usize; STEPS], usize, [i16; STEPS]) {
        let (notes, len) = self.resolved_arp_cycle(phase, self.arp.plan_seed);
        let mut slots = [0; STEPS];
        let mut count = 0;
        let pass_len = self.arp_pass_len();
        for i in 0..len {
            if self.active_arp_page().step(i, pass_len).state != StepState::Skip
            {
                slots[count] = i;
                count += 1;
            }
        }
        (slots, count, notes)
    }
    pub(super) fn arp_deadline(&self) -> Option<u64> {
        if self.arp.paused_at.is_some() { return None; }
        if !self.arp_gated() {
            return self.arp.running.then_some(self.now);
        }
        if self.arp_alignment_available() != self.arp.aligned {
            return Some(self.now);
        }
        let count = self.arp_slots().1;
        let requested =
            !self.config.tracker_enabled && self.config.arp_page.min(7) != self.arp.active;
        if !self.arp.running {
            return (count > 0 || requested).then_some(self.now);
        }
        let boundary =
            (!self.arp.finished || requested || self.config.tracker_enabled && self.arp.done)
                .then_some(self.arp.next.round().max(0.0) as u64);
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
        self.arp.clock_phase = phase;
        self.arp.next = self.now as f64
            + (boundary - self.beat).max(0.0) * self.sample_rate as f64 * 60.0 / self.tempo;
        self.arp.early = None;
        self.arp.events.fill(None);
    }
    fn prepare_arp_event(&mut self, phase: i64, boundary: f64, floor: u64) -> Option<Event> {
        let (slots, count, notes) = self.arp_slots_at(phase);
        if count == 0 {
            return None;
        }
        let ordinal = phase.rem_euclid(count as i64) as usize;
        let index = slots[ordinal];
        let step = *self.active_arp_page().step(index, self.arp_pass_len());
        let interval = self.arp_interval(self.arp.clock_phase + phase - self.arp.phase);
        let jitter = self.humanize_timing(interval);
        let at = (boundary + interval * step.micro as f64 / 100.0 + jitter)
            .round()
            .max(floor as f64) as u64;
        let settings = self.arp_live_settings();
        let custom = if settings.pattern == 7 {
            settings.custom[program_index(settings, index, phase, self.arp_pass_len()) % 7]
        } else {
            crate::arpeggiator::CustomStep::default()
        };
        let audible = !custom.rest
            && step.state == StepState::Play
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
            velocity_scale: custom.velocity as f32 / 100.0,
            gate_scale: custom.gate as f32 / 100.0,
        })
    }
    fn timing_pending(&self) -> bool {
        self.arp
            .timing
            .is_some_and(|t| t != Timing::from(self.arp_requested_settings()))
    }
    fn commit_arp_timing(&mut self, out: &mut impl FnMut(Out)) {
        let requested = self.arp_requested_settings();
        if self.arp.timing.is_none() {
            self.arp.timing = Some(Timing::from(requested));
            return;
        }
        let old_len = self.arp_pass_len() as i64;
        if self.timing_pending()
            && self.arp.phase.rem_euclid(old_len) == 0
            && self.now >= self.arp.next.round().max(0.0) as u64
        {
            self.arp.timing = Some(Timing::from(requested));
            let new_len = self.arp_pass_len() as i64;
            self.arp.phase = self.arp.phase.div_euclid(old_len) * new_len;
            self.arp.events.fill(None);
            self.arp.early = None;
            self.arp.done = false;
            self.arp.finished = false;
            self.arp.timing_commit = true;
            self.configure(self.host_config, out);
            self.arp.timing_commit = false;
        }
    }
    pub(super) fn refresh_arp_events(&mut self) {
        let settings = self.arp_live_settings();
        let notes = self.arp_cycle_chord();
        let request = if self.config.tracker_enabled {
            self.arp.row_index.min(u8::MAX as usize) as u8
        } else {
            self.config.arp_page
        };
        if self.arp.cached_notes == notes
            && self.arp.cached_settings == settings
            && self.arp.cached_generation == self.arp.data.generation
            && self.arp.cached_request == request
        {
            return;
        }
        if !self.arp.preserve_paste_clock
            && self.arp_alignment_available()
            && (settings.rate != self.arp.cached_settings.rate
                || settings.swing != self.arp.cached_settings.swing)
        {
            self.arp.running = false;
        }
        self.arp.cached_notes = notes;
        self.arp.cached_settings = settings;
        self.arp.cached_generation = self.arp.data.generation;
        self.arp.cached_request = request;
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
        if self.arp.paused_at.is_some() { return; }
        self.refresh_arp_events();
        if !self.arp_gated() {
            if self.arp.running {
                self.restart_automatic_arp();
                self.clear_arp_sources();
                self.configure(self.host_config, out);
            }
            return;
        }
        self.commit_arp_timing(out);
        let aligned = self.arp_alignment_available();
        let settings = self.arp_live_settings();
        if self.arp.preserve_paste_clock {
            self.arp.aligned = aligned;
            self.arp.last_settings = settings;
            self.arp.preserve_paste_clock = false;
        }
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
            let completed_count = self.arp_slots().1.max(1) as i64;
            if self.config.tracker_enabled
                && self.arp.phase != self.arp.counted_phase
                && self.arp.phase.rem_euclid(completed_count) == 0
            {
                self.arp.counted_phase = self.arp.phase;
                self.arp.loops_in_row = self.arp.loops_in_row.saturating_add(1);
                let duration = self.arp.data.rows[self.arp.row_index].duration;
                if self.arp.loops_in_row >= duration {
                    let legacy_once = self.arp.done && !self.arp_live_settings().lock_length;
                    self.advance_tracker_row(out);
                    if legacy_once {
                        self.arp.next += self.arp_interval(self.arp.clock_phase);
                        return;
                    }
                } else if self.arp.done {
                    self.arp.finished = true;
                    self.clear_arp_sources();
                    self.arp.next += (0..completed_count)
                        .map(|i| self.arp_interval(self.arp.clock_phase + i))
                        .sum::<f64>();
                    self.arp.phase += completed_count;
                    self.arp.clock_phase += completed_count;
                    self.configure(self.host_config, out);
                }
            }
            let requested = if self.config.tracker_enabled {
                self.arp.active
            } else {
                self.config.arp_page.min(7)
            };
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
                    || self.arp.paste_settings[requested as usize]
                        .unwrap_or(self.host_config.arp_settings[requested as usize])
                        .mode
                        == ArpMode::Once
                {
                    self.arp.phase = 0;
                    self.arp.clock_phase = 0;
                    self.arp.timing = None;
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
                self.clear_arp_sources();
                return;
            }
            if self.arp.done && !self.arp.finished {
                self.arp.finished = true;
                self.clear_arp_sources();
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
                self.arp.next += self.arp_interval(self.arp.clock_phase);
                self.arp.clock_phase = self.arp.clock_phase.wrapping_add(1);
                self.arp.phase = self.arp.phase.wrapping_add(1);
                self.arp.done = self.config.arp_mode == ArpMode::Once
                    && self.arp.phase.rem_euclid(count as i64) == 0;
                if self.config.arp_pattern == 6 && self.arp.phase.rem_euclid(count as i64) == 0 {
                    self.arp.plan_seed = self.random();
                }
                let row_ends = self.config.tracker_enabled
                    && self.arp.phase.rem_euclid(count as i64) == 0
                    && self.arp.loops_in_row + 1 >= self.arp.data.rows[self.arp.row_index].duration;
                if !self.arp.done
                    && !row_ends
                    && !(self.timing_pending()
                        && self.arp.phase.rem_euclid(self.arp_pass_len() as i64) == 0)
                {
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
            let lanes = self.arp_mod_sources();
            self.sources[lanes]
                .copy_from_slice(&event.adjustment.mods.map(|value| value as f32 / 100.0));
            self.sources[ARP_POSITION_SOURCE] = event.position;
            self.configure(self.host_config, out);
            if event.audible {
                let (notes, len) = self.resolved_arp_cycle(event.phase, event.seed);
                let note = if len > 0 {
                    notes[event.step % len]
                } else {
                    event.note
                };
                if note == i16::MIN {
                    continue;
                }
                let pitch = note + event.adjustment.octave as i16 * 12;
                if (0..=127).contains(&pitch) {
                    let velocity = self.base_strike_velocity() * event.adjustment.velocity as f32
                        / 100.0
                        * event.velocity_scale
                        * self.humanize_velocity_factor();
                    // Gate is the cell's absolute percentage, with routed Gate replacing or
                    // offsetting that base through the existing modulation matrix.
                    let duration = (event.interval
                        * self.effective_arp_gate_scaled(event.adjustment.gate, event.gate_scale)
                            as f64)
                        .round()
                        .max(1.0) as u64;
                    if velocity > 0.0 {
                        self.strike_prepared(pitch as u8, velocity, duration, self.automatic_arp_owner(), out);
                    }
                }
            }
        }
    }
}
