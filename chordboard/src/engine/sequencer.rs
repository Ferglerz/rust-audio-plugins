//! Independent, bounded MIDI timelines. Names and serialization never enter processing.
use super::*;
use crate::sequencer::{Prepared, PreparedPage, Step, Store};

#[derive(Clone, Copy, Debug, Default)]
pub struct StepExpression {
    pub pressure: i16,
    pub timbre: i16,
    pub bend: f32,
    pub cc: [i16; 4],
    pub numbers: [u8; 4],
}
impl StepExpression {
    fn new(step: Step, page: EventSettings) -> Self {
        Self {
            pressure: step.pressure,
            timbre: step.timbre,
            bend: step.bend,
            cc: step.cc,
            numbers: page.cc_numbers,
        }
    }
}
#[derive(Clone, Copy)]
struct EventSettings {
    enabled: bool,
    interlock_override: Option<u8>,
    length: u8,
    cc_numbers: [u8; 4],
}
impl From<PreparedPage> for EventSettings {
    fn from(page: PreparedPage) -> Self {
        Self {
            enabled: page.enabled,
            interlock_override: page.interlock_override,
            length: page.length,
            cc_numbers: page.cc_numbers,
        }
    }
}
#[derive(Clone, Copy)]
struct Event {
    at: u64,
    lane: usize,
    phase: u64,
    page: u8,
    step: Step,
    settings: EventSettings,
    duration: u64,
    first: bool,
}
pub struct Runtime {
    pub data: Box<Prepared>,
    pub active: [u8; 4],
    pub base: [u8; 4],
    pub steps: [u8; 4],
    pub running: bool,
    arp_done: bool,
    restore_epoch: u64,
    next: [u64; 4],
    phase: [u64; 4],
    fraction: [f64; 4],
    early: [Option<(u64, u8)>; 4],
    early_fired: [bool; 4],
    early_settings: [Option<PreparedPage>; 4],
    events: [Option<Event>; 256],
    harmony: Option<Step>,
    comp_at: Option<u64>,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            data: Store::new().initial_prepared(),
            active: [0; 4],
            base: [0; 4],
            steps: [0; 4],
            running: false,
            arp_done: false,
            restore_epoch: 0,
            next: [0; 4],
            phase: [0; 4],
            fraction: [0.0; 4],
            early: [None; 4],
            early_fired: [false; 4],
            early_settings: [None; 4],
            events: [None; 256],
            harmony: None,
            comp_at: None,
        }
    }
}
impl Engine {
    pub(super) fn manual_active(&self) -> bool {
        self.config.strum_enabled && self.config.mode == MANUAL
    }
    pub fn accept_sequence_generation(&mut self, out: &mut impl FnMut(Out)) {
        if self.seq.restore_epoch != self.seq.data.restore_epoch {
            self.seq.restore_epoch = self.seq.data.restore_epoch;
            self.reset_sequencer(out);
            self.seq.base = self.host_config.seq_pages;
        } else {
            self.refresh_sequence_lookahead();
        }
    }
    pub fn reset_sequencer(&mut self, out: &mut impl FnMut(Out)) {
        let changed = self.sources[9..].iter().any(|&value| value != 0.0);
        self.sources[9..].fill(0.0);
        self.reset_sequencer_runtime(out);
        if changed { self.configure_inner(self.host_config, false, out); }
    }
    // Routing topology changes release owned voices but retain this step's sources.
    pub(super) fn reset_sequencer_runtime(&mut self, out: &mut impl FnMut(Out)) {
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| matches!(v.owner, 1..=4)) {
                self.end_voice(i, 0.0, out);
            }
        }
        self.seq.events.fill(None);
        self.seq.running = false;
        self.seq.arp_done = false;
        self.seq.harmony = None;
        self.seq.phase = [0; 4];
        self.seq.fraction = [0.0; 4];
        self.seq.early = [None; 4];
        self.seq.early_fired = [false; 4];
        self.seq.early_settings = [None; 4];
        self.seq.comp_at = None;
        if self.memory.is_some() {
            self.rebuild_harmony(false, out);
        }
    }
    fn sequence_gated(&self) -> bool {
        self.config.seq_enabled
            && (self.playing
                || (self.config.latch && self.memory.is_some())
                || self.root.is_some_and(|s| s.held)
                || self.held_inputs.iter().any(Option::is_some)
                || self.melody.iter().any(|notes| notes.iter().any(|&n| n)))
    }
    pub(super) fn sequence_requests(&self) -> [u8; 4] {
        let mut config = self.host_config;
        config.seq_pages = self.seq.base;
        self.apply_routes(&mut config);
        config.seq_pages.map(|p| p.min(7))
    }
    pub(super) fn sequencer_deadline(&self) -> Option<u64> {
        if !self.sequence_gated() {
            return self.seq.running.then_some(self.now);
        }
        if !self.seq.running {
            return Some(self.now);
        }
        [0, 1]
            .into_iter()
            .filter(|&lane| lane != 0 || !self.seq.arp_done)
            .map(|lane| self.seq.next[lane])
            .chain(self.seq.events.iter().flatten().map(|e| e.at))
            .min()
    }
    /// A new selection retriggers Once without restarting the Chords timeline.
    pub(super) fn restart_sequence_arp(&mut self) {
        self.seq.arp_done = false;
        self.seq.next[0] = self.now;
        self.seq.phase[0] = 0;
        self.seq.fraction[0] = 0.0;
        self.seq.early[0] = None;
        self.seq.early_fired[0] = false;
        self.seq.early_settings[0] = None;
        for event in &mut self.seq.events {
            if event.is_some_and(|e| e.lane == 0) {
                *event = None;
            }
        }
    }
    pub(super) fn apply_sequenced_harmony(&mut self) {
        let Some(recipe) = self.memory else {
            return;
        };
        let Some(step) = self.seq.harmony else {
            return;
        };
        let root = recipe.root as i16 + step.root_offset as i16;
        if !(0..=127).contains(&root) {
            self.notes = Notes::default();
            return;
        }
        let quality = if step.quality < 0 {
            recipe.quality
        } else {
            step.quality as u8
        };
        let inversion = if step.inversion < 0 {
            recipe.inversion
        } else {
            step.inversion as u8
        };
        let spread = if step.spread < 0 {
            recipe.spread
        } else {
            step.spread as u8
        };
        let second = recipe.second.and_then(|n| {
            let n = n as i16 + step.root_offset as i16;
            (0..=127).contains(&n).then_some(n as u8)
        });
        self.full_notes = harmony::voice_recipe(harmony::SavedChord { root: root as u8, quality, second, inversion, spread, ..recipe }, spread);
        if self.config.affect_chords {
            self.full_notes = harmony::melody_harmony(self.full_notes, &self.melody);
        }
        self.notes = harmony::filter_notes(self.full_notes, self.config.filter);
    }
    /// The arp lane shares the arpeggiator's rate so one control sets both.
    fn sequence_period(&self, lane: usize, page: PreparedPage, phase: u64) -> f64 {
        let swing = if phase.is_multiple_of(2) {
            1.0 + self.config.swing as f64
        } else {
            1.0 - self.config.swing as f64
        };
        let step = if lane == 0 {
            self.rate_samples()
        } else {
            self.sample_rate as f64 * 60.0 / self.tempo * page.rate as f64
        };
        let span = (page.end - page.start + 1) as u64;
        let index = page.start as usize + (phase % span) as usize;
        let speed = page.steps[index].speed_factor().max(0.01);
        (step * swing / speed).max(1.0)
    }
    fn enqueue_step(&mut self, lane: usize, page_id: u8, phase: u64, boundary: u64, horizon: u64) {
        let page = self.seq.data.pages[lane][page_id as usize];
        let span = (page.end - page.start + 1) as u64;
        let index = page.start as usize + (phase % span) as usize;
        let step = page.steps[index];
        let period = self.sequence_period(lane, page, phase);
        let shifted = (boundary as i128 + (period * step.micro as f64 / 100.0) as i128)
            .clamp(horizon as i128, u64::MAX as i128) as u64;
        let ratchets = if lane == 3 { 1 } else { step.ratchets as usize };
        // Humanize applies to the whole ratchet group, so ratchets stay ordered.
        let mut step = step;
        let mut shifted = shifted;
        if lane != 3 {
            shifted = self.humanized_at(shifted, period, horizon);
            if step.velocity > 0 {
                let factor = self.humanize_velocity_factor();
                step.velocity = (step.velocity as f32 * factor).round().clamp(1.0, 200.0) as u8;
            }
        }
        // Reserve the entire group, so capacity pressure cannot make a partial chord or ratchet.
        if self.seq.events.iter().filter(|e| e.is_none()).count() < ratchets {
            return;
        }
        let hit = step.enabled
            && page.enabled
            && (step.probability == 100 || (self.random() % 100) < step.probability as u32);
        for ratchet in 0..ratchets {
            let mut step = step;
            step.enabled = hit;
            let event = Event {
                at: shifted.saturating_add((period * ratchet as f64 / ratchets as f64) as u64),
                lane,
                phase,
                page: page_id,
                step,
                settings: page.into(),
                duration: (period / ratchets as f64 * step.gate as f64 / 100.0).max(1.0) as u64,
                first: ratchet == 0,
            };
            if let Some(slot) = self.seq.events.iter_mut().find(|e| e.is_none()) {
                *slot = Some(event);
            }
        }
    }
    fn refresh_sequence_lookahead(&mut self) {
        if !self.seq.running {
            return;
        }
        let requests = self.sequence_requests();
        for lane in [0, 1] {
            let requested = requests[lane];
            if lane == 0 && self.seq.arp_done {
                continue;
            }
            if let Some((phase, page)) = self.seq.early[lane] {
                let changed = page != requested
                    || self.seq.early_settings[lane]
                        != Some(self.seq.data.pages[lane][page as usize]);
                if changed && !self.seq.early_fired[lane] {
                    for event in &mut self.seq.events {
                        if event.is_some_and(|e| e.lane == lane && e.phase == phase) {
                            *event = None;
                        }
                    }
                    self.seq.early[lane] = None;
                    self.seq.early_settings[lane] = None;
                }
            }
            if self.seq.early[lane].is_none() {
                let page = self.seq.data.pages[lane][requested as usize];
                let phase = self.seq.phase[lane];
                let index =
                    page.start as usize + (phase % (page.end - page.start + 1) as u64) as usize;
                if page.steps[index].micro < 0 {
                    self.enqueue_step(lane, requested, phase, self.seq.next[lane], self.now);
                    self.seq.early[lane] = Some((phase, requested));
                    self.seq.early_settings[lane] = Some(page);
                    self.seq.early_fired[lane] = false;
                }
            }
        }
    }
    pub(super) fn tick_sequencer(&mut self, out: &mut impl FnMut(Out)) {
        if !self.sequence_gated() {
            if self.seq.running {
                self.reset_sequencer(out);
            }
            return;
        }
        if !self.seq.running {
            self.seq.running = true;
            self.seq.next = [self.now; 4];
        }
        let requests = self.sequence_requests();
        if self.seq.arp_done && requests[0] != self.seq.active[0] {
            self.restart_sequence_arp();
        }
        self.refresh_sequence_lookahead();
        // Chords precede Arp so Interlock sees coincident chord strikes.
        for lane in [1, 0] {
            if lane == 0 && self.seq.arp_done {
                continue;
            }
            if self.now < self.seq.next[lane] {
                continue;
            }
            let page_id = requests[lane];
            let phase = self.seq.phase[lane];
            let boundary = self.seq.next[lane];
            self.seq.active[lane] = page_id;
            let page = self.seq.data.pages[lane][page_id as usize];
            self.seq.steps[lane] = page.start + (phase % (page.end - page.start + 1) as u64) as u8;
            let consumed =
                self.seq.early[lane].is_some_and(|(p, _)| p == phase) && self.seq.early_fired[lane];
            if self.seq.early[lane] != Some((phase, page_id)) && !consumed {
                // Invalidate lookahead from an obsolete request without cancelling existing tails.
                for event in &mut self.seq.events {
                    if event.is_some_and(|e| e.lane == lane && e.phase == phase) {
                        *event = None;
                    }
                }
                self.enqueue_step(lane, page_id, phase, boundary, self.now);
            }
            let exact = self.sequence_period(lane, page, phase) + self.seq.fraction[lane];
            let period = exact.floor().max(1.0) as u64;
            self.seq.fraction[lane] = exact - period as f64;
            self.seq.phase[lane] = phase.wrapping_add(1);
            if lane == 0 && !self.config.seq_loop {
                self.seq.arp_done = self.seq.phase[0] >= (page.end - page.start + 1) as u64;
            }
            self.seq.next[lane] = boundary.saturating_add(period);
            self.seq.early[lane] = None;
            self.seq.early_fired[lane] = false;
            self.seq.early_settings[lane] = None;
        }
        self.refresh_sequence_lookahead();
        for lane in [1, 0] {
            for slot in 0..self.seq.events.len() {
                if self.seq.events[slot].is_some_and(|e| e.lane == lane && e.at <= self.now) {
                    let event = self.seq.events[slot].take().unwrap();
                    self.play_sequence_event(event, out);
                }
            }
        }
    }
    fn play_sequence_event(&mut self, event: Event, out: &mut impl FnMut(Out)) {
        if event.first && self.seq.early[event.lane] == Some((event.phase, event.page)) {
            self.seq.early_fired[event.lane] = true;
        }
        if event.first && event.lane <= 1 && event.settings.enabled {
            let values = event.step.mods.map(|value| value.clamp(-100, 100) as f32 / 100.0);
            if self.sources[9..] != values {
                self.sources[9..].copy_from_slice(&values);
                self.configure(self.host_config, out);
            }
        }
        let mut step = event.step;
        step.tie = false;
        if event.lane == 3 {
            if !event.settings.enabled || step.live {
                self.seq.harmony = None;
            } else if step.enabled {
                self.seq.harmony = Some(step);
            }
            self.rebuild_harmony(false, out);
            return;
        }
        let owner = event.lane as u8 + 1;
        let mut notes = Notes::default();
        if step.enabled && step.velocity > 0 {
            match event.lane {
                1 => {
                    for &note in self.notes.as_slice() {
                        notes.push(note as i16 + step.octave as i16 * 12);
                    }
                }
                2 => {
                    let root = self.memory.map(|m| {
                        m.root as i16
                            + m.transpose as i16
                            + self.seq.harmony.map_or(0, |s| s.root_offset as i16)
                    });
                    if let Some(root) = root {
                        let pitch = if step.tone >= 0 {
                            self.notes.string(step.tone as usize).map(|n| n as i16)
                        } else {
                            Some(root + if step.tone == -2 { 7 } else { 0 })
                        };
                        if let Some(pitch) = pitch {
                            notes.push(pitch + step.octave as i16 * 12);
                        }
                    }
                }
                _ => {
                    let total = (self.notes.len * self.config.octaves.clamp(1, 4) as usize).min(32);
                    if total > 0 {
                        let index = if step.tone < 0 {
                            self.arp_index(
                                self.config.arp_pattern,
                                event.phase as usize,
                                total,
                            )
                        } else {
                            step.tone as usize
                        };
                        if index < total {
                            if let Some(note) = self.notes.string(index) {
                                notes.push(note as i16 + step.octave as i16 * 12);
                            }
                        }
                    }
                }
            }
        }
        if event.first {
            for i in 0..self.voices.len() {
                if self.voices[i].is_some_and(|v| {
                    v.owner == owner
                        && v.off == u64::MAX
                        && (!step.tie || !notes.as_slice().contains(&v.note))
                }) {
                    self.end_voice(i, 0.0, out);
                }
            }
        }
        if event.lane == 1 && notes.len > 0 {
            self.seq.comp_at = Some(self.now);
        }
        let mut velocity = self.base_strike_velocity() * step.velocity as f32 / 100.0;
        let mut duration = event.duration;
        if event.lane == 0 {
            velocity *= contour_factor(
                self.seq.steps[0] as f32 / event.settings.length.saturating_sub(1).max(1) as f32,
                self.config.contour,
                self.config.contour_curve,
            );
            if self.seq.comp_at == Some(self.now) {
                match event
                    .settings
                    .interlock_override
                    .unwrap_or(self.config.interlock)
                {
                    1 => {
                        velocity *= 0.5;
                        duration = (duration as f32 * 0.6).max(1.0) as u64;
                    }
                    2 => {
                        velocity *= 1.35;
                        duration = (duration as f32 * 1.2).max(1.0) as u64;
                    }
                    _ => {}
                }
            }
        }
        if velocity <= 0.0 {
            return;
        }
        for &note in notes.as_slice() {
            let off = if step.tie {
                u64::MAX
            } else {
                self.now.saturating_add(duration)
            };
            if step.tie {
                if let Some(v) = self
                    .voices
                    .iter_mut()
                    .flatten()
                    .find(|v| v.owner == owner && v.note == note && v.off == u64::MAX)
                {
                    let expression = StepExpression::new(step, event.settings);
                    let old = v.seq_expression;
                    v.seq_expression = Some(expression);
                    let channel = v.channel;
                    if !self.melody_channel(channel)
                        && self
                            .newest_voice(channel)
                            .is_some_and(|voice| voice.owner == owner && voice.note == note)
                    {
                        if let Some(old) = old {
                            self.restore_step_cc(channel, old, Some(expression), out);
                        }
                        self.emit_step_expression(channel, expression, out);
                    }
                    continue;
                }
            }
            for i in 0..self.voices.len() {
                if self.voices[i].is_some_and(|v| v.owner == owner && v.note == note) {
                    self.end_voice(i, 0.0, out);
                }
            }
            self.allocate_owned_voice(
                note,
                velocity.clamp(0.01, 1.0),
                off,
                false,
                owner,
                Some(StepExpression::new(step, event.settings)),
                out,
            );
            self.note_strikes[note as usize] = self.note_strikes[note as usize].wrapping_add(1);
        }
    }
    pub(super) fn controller_source(&self) -> usize {
        self.root
            .filter(|s| s.channel < 16)
            .map_or(if self.input_upper { 15 } else { 0 }, |s| s.channel) as usize
    }
    pub(super) fn controller_baseline(&self, cc: u8) -> u8 {
        let index = match cc { 1 => Some(1), 11 => Some(2), 2 => Some(3), 74 => Some(5), _ => None };
        if let Some(value) = index.and_then(|index| self.source_overrides[index]) {
            return (value * 127.0).round() as u8;
        }
        let source = self.controller_source();
        if self.cc_known[source][cc as usize] {
            self.cc[source][cc as usize]
        } else {
            match cc {
                7 | 11 => 127,
                10 => 64,
                _ => 0,
            }
        }
    }
    pub(super) fn restore_step_cc(
        &self,
        channel: u8,
        old: StepExpression,
        next: Option<StepExpression>,
        out: &mut impl FnMut(Out),
    ) {
        let source = self.controller_source();
        for i in 0..4 {
            let cc = old.numbers[i];
            let emitted = next.is_some_and(|next| {
                next.numbers
                    .iter()
                    .position(|&n| n == cc)
                    .is_some_and(|j| next.cc[j] != 0 || self.cc_known[source][cc as usize])
            });
            if old.cc[i] != 0 && !emitted {
                out(Out::Cc(
                    channel,
                    cc,
                    self.controller_baseline(cc) as f32 / 127.0,
                ));
            }
        }
    }
    pub(super) fn emit_step_expression(
        &self,
        channel: u8,
        step: StepExpression,
        out: &mut impl FnMut(Out),
    ) {
        let e = self.routed_expression();
        let pressure = if self.y_active && self.config.y_target == 2 {
            self.y
        } else {
            e.pressure
        };
        let timbre = if self.y_active && self.config.y_target == 3 {
            self.y
        } else {
            e.timbre
        };
        let bend = if self.y_active && self.config.y_target == 4 {
            self.y
        } else {
            e.bend
        };
        out(Out::Pressure(
            channel,
            (pressure + step.pressure as f32 / 127.0).clamp(0.0, 1.0),
        ));
        out(Out::Cc(
            channel,
            74,
            (timbre + step.timbre as f32 / 127.0).clamp(0.0, 1.0),
        ));
        out(Out::Bend(
            channel,
            (bend
                + step.bend
                    / (if self.config.mpe {
                        self.config.bend_range
                    } else {
                        self.config.master_range
                    }
                    .max(1.0)
                        * 2.0))
                .clamp(0.0, 1.0),
        ));
        self.emit_routed_controllers(channel, out);
        let source =
            self.root
                .filter(|s| s.channel < 16)
                .map_or(if self.input_upper { 15 } else { 0 }, |s| s.channel) as usize;
        for i in 0..4 {
            let number = step.numbers[i];
            if crate::sequencer::valid_cc(number)
                && (step.cc[i] != 0 || self.cc_known[source][number as usize]
                    || matches!(number, 1 | 2 | 11) && self.host_config.routes.iter().any(|r| r.active() && routing::TARGETS[r.target as usize].source_index() == Some(match number { 1 => 1, 11 => 2, _ => 3 })))
            {
                let baseline = self.controller_baseline(number);
                out(Out::Cc(
                    channel,
                    number,
                    ((baseline as i16 + step.cc[i]) as f32 / 127.0).clamp(0.0, 1.0),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn engine() -> Engine {
        let mut engine = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        engine.configure(
            Config {
                mode: MANUAL,
                seq_enabled: true,
                ..Config::default()
            },
            &mut |_| {},
        );
        engine.note_on(2048, 60, 16, 0.8, Some(0), &mut |_| {});
        engine
    }
    fn prepare(e: &mut Engine, edit: impl FnOnce(&mut crate::sequencer::State)) {
        let store = Store::new();
        store.edit(|state| {
            for lane in &mut state.lanes {
                for page in lane {
                    page.length = 16;
                    page.end = 15;
                }
            }
            edit(state);
        });
        assert!(store.apply_pending(&mut e.seq.data));
    }
    #[test]
    fn step_modulator_revoices_parameter_routes_before_the_strike() {
        for (value, transpose) in [(50, 12), (-50, -12), (0, 0)] {
            let mut e = engine();
            let before = e.full_notes;
            let mut c = e.host_config;
            c.routes[0] = routing::Route {
                source: 10,
                target: routing::TARGETS.iter().position(|t| t.id == "transpose").unwrap() as u8,
                min: 0.5, max: 1.0, ..routing::Route::default()
            };
            e.configure(c, &mut |_| {});
            prepare(&mut e, |state| {
                state.lanes[1][0].enabled = false;
                state.lanes[0][0].steps[0].mods = [value, 0];
            });
            e.tick_sequencer(&mut |_| {});
            assert_eq!(e.config.transpose, transpose);
            assert_eq!(e.full_notes.as_slice(), before.as_slice().iter().map(|&n| (n as i16 + transpose as i16) as u8).collect::<Vec<_>>());
        }
    }

    #[test]
    fn dedicated_step_modulators_follow_rests_and_reset_with_playback() {
        let mut e = engine();
        prepare(&mut e, |state| {
            state.lanes[1][0].enabled = false;
            state.lanes[0][0].steps[0].mods = [25, 75];
            state.lanes[0][0].steps[0].enabled = false;
        });
        e.tick_sequencer(&mut |_| {});
        assert_eq!(e.sources[9..], [0.25, 0.75]);
        e.reset_sequencer(&mut |_| {});
        assert_eq!(e.sources[9..], [0.0, 0.0]);
    }

    #[test]
    fn sequencer_power_preserves_manual_voices_and_selected_voicing() {
        let mut e = engine();
        e.configure(
            Config {
                voice_leading: 0,
                ..e.host_config
            },
            &mut |_| {},
        );
        e.command(Command::BeginGesture(1.0, 0.8), &mut |_| {});
        e.tick(&mut |_| {});
        let manual: Vec<_> = e
            .voices
            .iter()
            .flatten()
            .filter(|v| v.owner == 0)
            .map(|v| (v.note, v.off))
            .collect();
        let voiced = e.full_notes;
        assert!(!manual.is_empty());
        e.configure(
            Config {
                seq_enabled: false,
                ..e.host_config
            },
            &mut |_| {},
        );
        assert!(!e.seq.running);
        assert!(e.voices.iter().flatten().all(|v| v.owner == 0));
        assert_eq!(e.full_notes, voiced);
        assert_eq!(
            e.voices
                .iter()
                .flatten()
                .map(|v| (v.note, v.off))
                .collect::<Vec<_>>(),
            manual
        );
        let mut events = Vec::new();
        e.advance(1000, &mut |_, event| events.push(event));
        assert!(!events.iter().any(|event| matches!(event, Out::On(..))));
    }

    #[test]
    fn selection_playback_is_independent_of_strum_and_sequence_power() {
        for seq_enabled in [false, true] {
            for strum_enabled in [false, true] {
                for selection in 0..3 {
                    let mut e = Engine::default();
                    e.configure(
                        Config {
                            mode: MANUAL,
                            seq_enabled,
                            strum_enabled,
                            root_on_select: selection == 1,
                            always_chord: selection == 2,
                            ..Config::default()
                        },
                        &mut |_| {},
                    );
                    let mut events = Vec::new();
                    e.note_on(2048, 60, 16, 0.8, Some(0), &mut |v| events.push(v));
                    let ons: Vec<_> = events
                        .iter()
                        .filter_map(|event| match event {
                            Out::On(_, note, _) => Some(*note),
                            _ => None,
                        })
                        .collect();
                    let expected = match selection {
                        1 => vec![60],
                        2 => vec![60, 64, 67],
                        _ => vec![],
                    };
                    assert_eq!(
                        ons, expected,
                        "selection {selection}, seq {seq_enabled}, strum {strum_enabled}"
                    );
                    e.tick(&mut |_| {});
                    e.configure(
                        Config {
                            seq_enabled: false,
                            ..e.host_config
                        },
                        &mut |_| {},
                    );
                    if selection == 2 {
                        assert_eq!(e.voices.iter().flatten().filter(|v| v.layer).count(), 3);
                        e.advance(500, &mut |_, _| {});
                        assert_eq!(e.voices.iter().flatten().filter(|v| v.layer).count(), 3);
                    }
                }
            }
        }
    }

    #[test]
    fn power_changes_preserve_root_on_select_deadline() {
        let mut e = engine();
        e.configure(Config { root_on_select: true, ..e.host_config }, &mut |_| {});
        let root = *e.voices.iter().flatten().find(|v| v.owner == SELECT_ROOT_OWNER).unwrap();
        e.tick(&mut |_| {});
        e.configure(Config { strum_enabled: false, seq_enabled: false, ..e.host_config }, &mut |_| {});
        assert!(e.voices.iter().flatten().any(|v| v.owner == SELECT_ROOT_OWNER && v.off == root.off));
        e.advance(root.off as usize, &mut |_, _| {});
        assert!(!e.voices.iter().flatten().any(|v| v.owner == SELECT_ROOT_OWNER));
    }

    #[test]
    fn selected_chord_never_borrows_sequencer_ownership() {
        let mut e = engine();
        e.tick(&mut |_| {});
        e.configure(
            Config {
                always_chord: true,
                ..e.host_config
            },
            &mut |_| {},
        );
        assert!(e
            .voices
            .iter()
            .flatten()
            .filter(|v| v.owner != 0)
            .all(|v| !v.layer));
        e.configure(
            Config {
                seq_enabled: false,
                ..e.host_config
            },
            &mut |_| {},
        );
        assert_eq!(e.voices.iter().flatten().filter(|v| v.layer).count(), 3);
    }

    #[test]
    fn interlock_never_changes_voicing_in_any_leading_mode() {
        for leading in 0..3 {
            let mut expected = None;
            for interlock in 0..3 {
                let mut e = engine();
                e.configure(
                    Config {
                        voice_leading: leading,
                        interlock,
                        ..e.host_config
                    },
                    &mut |_| {},
                );
                e.command(Command::KeyUp(0), &mut |_| {});
                e.command(Command::KeyDown(1, 67, 0), &mut |_| {});
                e.tick(&mut |_| {});
                if let Some(notes) = expected {
                    assert_eq!(e.full_notes, notes);
                }
                expected = Some(e.full_notes);
                if leading == 2 {
                    assert_eq!(e.full_notes.as_slice(), &[67, 71, 74]);
                }
            }
        }
    }

    #[test]
    fn once_finishes_arp_page_but_chords_keep_running_and_new_selection_retriggers() {
        let mut e = engine();
        e.configure(
            Config {
                seq_loop: false,
                ..e.host_config
            },
            &mut |_| {},
        );
        prepare(&mut e, |s| {
            s.lanes[0][0].length = 2;
            s.lanes[0][0].end = 1;
            s.lanes[0][0].steps[0].tone = 0;
            s.lanes[0][0].steps[1].tone = 1;
            s.lanes[1][0].steps[0].enabled = true;
        });
        e.advance(1000, &mut |_, _| {});
        assert!(e.seq.arp_done);
        assert_eq!(e.seq.phase[0], 2);
        assert!(e.seq.phase[1] > 2);
        e.command(Command::KeyUp(0), &mut |_| {});
        e.command(Command::KeyDown(1, 67, 0), &mut |_| {});
        assert!(!e.seq.arp_done);
        e.tick(&mut |_| {});
        assert_eq!(e.seq.phase[0], 1);
        e.configure(
            Config {
                seq_loop: true,
                ..e.host_config
            },
            &mut |_| {},
        );
        e.advance(1000, &mut |_, _| {});
        assert!(!e.seq.arp_done);
        assert!(e.seq.phase[0] > 2);
    }

    #[test]
    fn manual_pad_velocity_does_not_change_arp_or_chords() {
        let sequence = |y| {
            let mut e = engine();
            prepare(&mut e, |s| {
                s.lanes[0][0].steps[0].enabled = true;
                s.lanes[0][0].steps[0].octave = 1;
                s.lanes[1][0].steps[0].enabled = true;
            });
            let mut manual = Vec::new();
            e.command(Command::BeginGesture(0.0, y), &mut |out| {
                if let Out::On(_, _, v) = out { manual.push(v); }
            });
            let mut sequence = Vec::new();
            e.advance(4, &mut |_, out| {
                if let Out::On(_, note, v) = out { sequence.push((note, v)); }
            });
            (manual, sequence)
        };
        let (soft, a) = sequence(0.2);
        let (hard, b) = sequence(0.9);
        assert!(!soft.is_empty() && !hard.is_empty());
        assert!(soft[0] < hard[0]);
        assert!(!a.is_empty());
        assert_eq!(a, b);
    }

    #[test]
    fn sequencer_always_accepts_manual_strumming() {
        let mut e = engine();
        assert!(e.manual_active());
        e.config.mode = MANUAL;
        assert!(e.manual_active());
        e.config.mode = 1;
        assert!(!e.manual_active());
        e.config.mode = 3;
        assert!(!e.manual_active());
    }
    #[test]
    fn strum_bypass_releases_manual_voices_and_preserves_sequencer_voices() {
        let mut e = engine();
        e.strike(60, 0.8, u64::MAX, &mut |_| {});
        e.allocate_owned_voice(67, 0.8, u64::MAX, false, 1, None, &mut |_| {});
        let config = Config {
            strum_enabled: false,
            ..e.host_config
        };
        e.configure(config, &mut |_| {});
        assert!(!e.manual_active());
        assert!(!e.voices.iter().flatten().any(|v| v.owner == 0 && !v.layer));
        assert!(e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == 1 && v.note == 67));
        e.configure(
            Config {
                strum_enabled: true,
                ..config
            },
            &mut |_| {},
        );
        assert!(e.manual_active());
    }
    #[test]
    fn removed_bass_lane_cannot_play_restored_enabled_patterns() {
        let mut e = engine();
        prepare(&mut e, |state| {
            for page in &mut state.lanes[2] {
                page.enabled = true;
                for step in &mut page.steps {
                    step.enabled = true;
                }
            }
        });
        assert!(e.seq.data.pages[2].iter().all(|page| !page.enabled));
        e.advance(500, &mut |_, _| {});
        assert!(!e.voices.iter().flatten().any(|voice| voice.owner == 3));
    }
    #[test]
    fn independent_pages_lengths_and_phase_switch() {
        let mut e = engine();
        prepare(&mut e, |s| {
            s.lanes[0][0].length = 3;
            s.lanes[0][0].end = 2;
            s.lanes[1][0].length = 5;
            s.lanes[1][0].end = 4;
            s.lanes[1][0].rate = 0.5;
            s.lanes[0][1].length = 2;
            s.lanes[0][1].end = 1;
        });
        e.advance(126, &mut |_, _| {});
        assert_eq!(e.seq.steps[..2], [1, 0]);
        e.seq.base[0] = 1;
        e.advance(125, &mut |_, _| {});
        assert_eq!(e.seq.active[0], 1);
        assert_eq!(e.seq.steps[0], 0);
        assert_eq!(e.seq.steps[1], 1);
    }
    #[test]
    fn arp_lane_steps_at_the_arpeggiator_rate() {
        let mut e = engine();
        e.config.rate = 0.5;
        prepare(&mut e, |s| {
            s.lanes[0][0].rate = 4.0;
            s.lanes[1][0].rate = 0.25;
        });
        e.advance(251, &mut |_, _| {});
        assert_eq!(e.seq.steps[..2], [1, 2]);
    }
    #[test]
    fn global_interlock_applies_unless_the_arp_page_overrides_it() {
        let arp_velocity = |global: u8, page: Option<u8>| {
            let mut e = engine();
            e.config.interlock = global;
            prepare(&mut e, |s| {
                s.lanes[0][0].steps[0].enabled = true;
                s.lanes[0][0].steps[0].octave = 1;
                s.lanes[0][0].interlock_override = page;
                s.lanes[1][0].steps[0].enabled = true;
            });
            let mut velocity = None;
            e.advance(4, &mut |_, out| {
                if let Out::On(_, 72, v) = out {
                    velocity = Some(v);
                }
            });
            velocity.expect("arp strike")
        };
        let plain = arp_velocity(0, None);
        assert!(arp_velocity(1, None) < plain);
        assert_eq!(arp_velocity(1, Some(0)), plain);
        assert!(arp_velocity(0, Some(2)) > plain);
    }
    #[test]
    fn sample_spans_match_single_sample_processing() {
        let mut a = engine();
        let mut b = engine();
        let edit = |s: &mut crate::sequencer::State| {
            s.lanes[0][0].steps[1].micro = -25;
            s.lanes[0][0].steps[0].ratchets = 3;
        };
        prepare(&mut a, edit);
        prepare(&mut b, edit);
        let mut expected = Vec::new();
        for _ in 0..800 {
            let at = b.now;
            b.tick(&mut |e| expected.push((at, e)));
        }
        let mut actual = Vec::new();
        for n in [3, 127, 1, 269, 400] {
            let origin = a.now;
            a.advance(n, &mut |offset, e| actual.push((origin + offset as u64, e)));
        }
        assert_eq!(actual, expected);
        assert!(actual
            .iter()
            .any(|(at, e)| *at == 94 && matches!(e, Out::On(..))));
    }
    #[test]
    fn harmony_lane_does_not_change_the_live_chord() {
        let mut e = engine();
        prepare(&mut e, |s| {
            let p = &mut s.lanes[3][0];
            p.steps[0].enabled = true;
            p.steps[0].root_offset = 2;
            p.steps[2].live = true;
        });
        e.tick(&mut |_| {});
        assert_eq!(e.notes.values[0], 60);
        e.advance(250, &mut |_, _| {});
        assert_eq!(e.notes.values[0], 60);
        assert_eq!(e.memory.unwrap().root, 60);
    }
    #[test]
    fn modulation_overrides_memory_base_without_feedback() {
        let mut e = engine();
        e.seq.base[0] = 3;
        let index = routing::TARGETS
            .iter()
            .position(|t| t.id == "seq_page_0")
            .unwrap();
        let mut c = e.host_config;
        c.routes[1] = routing::Route {
            source: 2,
            target: index as u8,
            min: 1.0,
            max: 1.0,
            enabled: true,
            curve: 0.0,
            smoothing_ms: 0.0,
        };
        e.configure(c, &mut |_| {});
        e.tick(&mut |_| {});
        assert_eq!(e.seq.active[0], 7);
        assert_eq!(e.seq.base[0], 3);
    }
    #[test]
    fn expression_offsets_clamp_and_never_accumulate() {
        let mut e = engine();
        e.expression = Expression {
            pressure: 0.5,
            timbre: 0.1,
            bend: 0.5,
        };
        let step = StepExpression {
            pressure: 127,
            timbre: -127,
            bend: 12.0,
            numbers: [11, 1, 2, 4],
            cc: [-127, 127, 0, 0],
        };
        let mut first = Vec::new();
        e.emit_step_expression(1, step, &mut |v| first.push(v));
        let mut second = Vec::new();
        e.emit_step_expression(1, step, &mut |v| second.push(v));
        assert_eq!(first, second);
        assert!(first.contains(&Out::Pressure(1, 1.0)));
        assert!(first.contains(&Out::Cc(1, 74, 0.0)));
        assert!(first.contains(&Out::Bend(1, 1.0)));
        assert_eq!(e.expression.pressure, 0.5);
    }
    #[test]
    fn ties_and_shared_strum_ownership_release_on_stop() {
        let mut e = engine();
        prepare(&mut e, |s| {
            s.lanes[0][0].steps[0].tie = true;
            s.lanes[0][0].steps[0].tone = 0;
        });
        e.tick(&mut |_| {});
        e.strike(60, 0.8, 500, &mut |_| {});
        assert!(e.voices.iter().flatten().any(|v| v.owner == 1));
        assert!(e.voices.iter().flatten().any(|v| v.owner == 0));
        let mut output = Vec::new();
        e.transport(false, 120.0, true, &mut |v| output.push(v));
        assert!(!e.voices.iter().any(Option::is_some));
        assert!(output.contains(&Out::Off(0, 60, 0.0)));
    }
    #[test]
    fn neutral_steps_do_not_send_unknown_zero_expression_cc() {
        let mut e = engine();
        let mut events = Vec::new();
        e.tick(&mut |event| events.push(event));
        assert!(!events
            .iter()
            .any(|event| matches!(event, Out::Cc(_, 11, _))));
    }
    #[test]
    fn inactive_edits_preserve_clocks_and_ties_but_restore_resets() {
        use nih_plug::params::persist::PersistentField;
        let store = Store::new();
        let mut e = engine();
        store.edit(|s| s.lanes[0][0].steps[0].tie = true);
        store.apply_pending(&mut e.seq.data);
        e.accept_sequence_generation(&mut |_| {});
        e.tick(&mut |_| {});
        let phase = e.seq.phase;
        let next = e.seq.next;
        store.edit(|s| s.lanes[2][7].name = "Inactive".into());
        store.apply_pending(&mut e.seq.data);
        e.accept_sequence_generation(&mut |_| {});
        assert_eq!(e.seq.phase, phase);
        assert_eq!(e.seq.next, next);
        assert!(e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner > 0 && v.off != u64::MAX));
        store.set(crate::sequencer::State::default());
        store.apply_pending(&mut e.seq.data);
        e.accept_sequence_generation(&mut |_| {});
        assert_eq!(e.seq.phase, [0; 4]);
        assert!(!e.voices.iter().flatten().any(|v| v.owner > 0));
    }
    #[test]
    fn tied_steps_emit_new_expression_and_live_cc_keeps_offset() {
        let mut e = engine();
        prepare(&mut e, |s| {
            let p = &mut s.lanes[0][0];
            for step in &mut p.steps {
                step.tie = true;
                step.tone = 0;
            }
            p.steps[1].pressure = 20;
            p.steps[1].cc[0] = -10;
        });
        e.tick(&mut |_| {});
        let mut events = Vec::new();
        e.advance(125, &mut |_, event| events.push(event));
        assert!(events.iter().any(
            |event| matches!(event,Out::Pressure(_,value) if (*value - 20.0/127.0).abs()<0.0001)
        ));
        assert!(events.iter().any(|event| matches!(event, Out::On(..))));
        events.clear();
        e.control(0, 11, 100.0 / 127.0, &mut |event| events.push(event));
        assert!(events.contains(&Out::Cc(0, 11, 90.0 / 127.0)));
        assert_eq!(e.cc[0][11], 100);
    }
    #[test]
    fn chord_memory_recall_preserves_sequencer_pages() {
        let mut e = engine();
        prepare(&mut e, |_| {});
        e.seq.base = [0, 4, 0, 0];
        e.command(
            Command::RecallMemory(
                2,
                harmony::SavedChord {
                    additions: 0,
                    root: 60,
                    second: None,
                    quality: 0,
                    inversion: 0,
                    transpose: 0,
                    spread: 0,
                }
                .encode(),
            ),
            &mut |_| {},
        );
        assert_eq!(e.seq.base, [0, 4, 0, 0]);
        e.tick(&mut |_| {});
        assert_eq!(e.seq.active, [0, 4, 0, 0]);
    }
    #[test]
    fn inactive_edit_after_anticipated_hit_does_not_retrigger_at_boundary() {
        let store = Store::new();
        let mut e = engine();
        store.edit(|s| {
            let p = &mut s.lanes[0][0];
            p.length = 16;
            p.end = 15;
            p.steps[1].micro = -25;
        });
        store.apply_pending(&mut e.seq.data);
        e.accept_sequence_generation(&mut |_| {});
        e.advance(100, &mut |_, _| {});
        store.edit(|s| s.lanes[2][7].name = "Rename".into());
        store.apply_pending(&mut e.seq.data);
        e.accept_sequence_generation(&mut |_| {});
        let mut hits = Vec::new();
        e.advance(26, &mut |at, event| {
            if matches!(event, Out::On(..)) {
                hits.push(at);
            }
        });
        assert!(hits.is_empty(), "anticipated hit played twice: {hits:?}");
    }
    #[test]
    fn page_request_cancels_unplayed_obsolete_lookahead() {
        let mut e = engine();
        prepare(&mut e, |s| {
            s.lanes[0][0].steps[1].micro = -25;
            for step in &mut s.lanes[0][1].steps {
                step.enabled = false;
            }
        });
        e.advance(10, &mut |_, _| {});
        e.seq.base[0] = 1;
        let mut hits = 0;
        e.advance(116, &mut |_, event| {
            if matches!(event, Out::On(..)) {
                hits += 1;
            }
        });
        assert_eq!(hits, 0);
        assert_eq!(e.seq.active[0], 1);
    }
    #[test]
    fn tied_delta_to_unknown_neutral_restores_controller_baseline() {
        let mut e = engine();
        prepare(&mut e, |s| {
            for step in &mut s.lanes[0][0].steps {
                step.tie = true;
                step.tone = 0;
            }
            s.lanes[0][0].steps[0].cc[0] = -20;
        });
        e.tick(&mut |_| {});
        let mut events = Vec::new();
        e.advance(125, &mut |_, event| events.push(event));
        assert!(events.contains(&Out::Cc(0, 11, 1.0)));
    }
    #[test]
    fn newest_shared_channel_voice_owns_live_controller_offsets() {
        let mut e = engine();
        let a = StepExpression {
            numbers: [11, 1, 2, 4],
            cc: [-20, 0, 0, 0],
            ..Default::default()
        };
        let b = StepExpression {
            cc: [-10, 0, 0, 0],
            pressure: 10,
            ..a
        };
        e.allocate_owned_voice(60, 0.8, 500, false, 1, Some(a), &mut |_| {});
        e.now = 1;
        e.allocate_owned_voice(64, 0.8, 500, false, 2, Some(b), &mut |_| {});
        let mut events = Vec::new();
        e.control(0, 11, 100.0 / 127.0, &mut |event| events.push(event));
        assert!(events.contains(&Out::Cc(0, 11, 90.0 / 127.0)));
        assert!(!events.contains(&Out::Cc(0, 11, 80.0 / 127.0)));
        events.clear();
        e.fan_expression(&mut |event| events.push(event));
        assert!(events.contains(&Out::Pressure(0, 10.0 / 127.0)));
    }
    #[test]
    fn all_rest_zero_probability_and_skipped_pool_are_silent() {
        for variant in 0..2 {
            let mut e = engine();
            prepare(&mut e, |s| {
                let p = &mut s.lanes[0][0];
                for step in &mut p.steps {
                    if variant == 0 {
                        step.enabled = false;
                    }
                    if variant == 1 {
                        step.probability = 0;
                    }
                }
            });
            let mut notes = 0;
            e.advance(1000, &mut |_, v| {
                if matches!(v, Out::On(..)) {
                    notes += 1;
                }
            });
            assert_eq!(notes, 0);
        }
    }
    #[test]
    fn harmony_boxes_do_not_change_the_live_chord() {
        let mut e = engine();
        prepare(&mut e, |s| {
            let p = &mut s.lanes[3][0];
            p.draw_box(1, 2);
            p.steps[1].root_offset = 2;
            p.draw_box(4, 4);
            p.steps[4].root_offset = -2;
        });
        e.tick(&mut |_| {});
        assert_eq!(e.notes.values[0], 60);
        for _ in 0..5 {
            e.advance(125, &mut |_, _| {});
            assert_eq!(e.notes.values[0], 60);
            assert_eq!(e.memory.unwrap().root, 60);
        }
    }
    fn arp_hits(humanize: f32, samples: usize) -> Vec<(u64, f32)> {
        let mut e = engine();
        e.config.rate = 0.5;
        e.config.humanize = humanize;
        prepare(&mut e, |s| {
            for step in &mut s.lanes[0][0].steps {
                step.enabled = true;
                step.velocity = 100;
            }
        });
        let mut hits = Vec::new();
        for _ in 0..samples {
            let at = e.now;
            e.tick(&mut |o| {
                if let Out::On(_, _, v) = o {
                    hits.push((at, v));
                }
            });
        }
        hits
    }
    #[test]
    fn double_speed_step_halves_interval_to_next_step() {
        let next_after_first = |speed: u8, fine: i8| {
            let mut e = engine();
            e.config.rate = 0.5;
            prepare(&mut e, |s| {
                s.lanes[0][0].steps[0].speed = speed;
                s.lanes[0][0].steps[0].fine = fine;
            });
            e.tick(&mut |_| {});
            e.seq.next[0]
        };
        let base = next_after_first(2, 0);
        assert_eq!(next_after_first(4, 0) * 2, base);
        assert_eq!(next_after_first(1, 0), base * 2);
        assert!((next_after_first(2, 50) * 3).abs_diff(base * 2) <= 3);
    }
    #[test]
    fn humanize_zero_is_exact_and_full_humanize_spreads_time_and_velocity() {
        let exact = arp_hits(0.0, 3000);
        assert!(exact.len() >= 8);
        assert!(exact.windows(2).all(|w| w[1].0 - w[0].0 == 250));
        assert!(exact.iter().all(|h| h.1 == exact[0].1));
        let loose = arp_hits(1.0, 3000);
        assert!(loose.len() >= 8);
        let lo = loose.iter().map(|h| h.1).fold(f32::MAX, f32::min);
        let hi = loose.iter().map(|h| h.1).fold(0.0, f32::max);
        assert!(hi - lo > 0.1 * exact[0].1, "velocity spread {lo}..{hi}");
        let mut late = 0;
        let mut offsets = std::collections::BTreeSet::new();
        for (i, h) in loose.iter().enumerate() {
            let off = h.0 as i64 - 250 * i as i64;
            offsets.insert(off);
            late += (off > 0) as usize;
        }
        assert!(offsets.len() >= 4 && late > 0, "offsets {offsets:?}");
        assert!(offsets.iter().all(|o| (0..=63).contains(o)), "{offsets:?}");
    }
}

#[cfg(test)]
mod extension_recipe_tests {
    use super::*;
    #[test]
    fn sequencer_transforms_full_recipe_and_melody_never_changes_capture() {
        let mut e = Engine::default();
        e.command(Command::Recall(harmony::SavedChord {
            root: 48, second: Some(58), quality: 0, additions: (1 << 1) | (1 << 2) | (1 << 5),
            inversion: 0, spread: 0, transpose: 0,
        }.encode()), &mut |_|{});
        let captured = e.memory.unwrap();
        e.seq.harmony = Some(Step { root_offset: 5, quality: 3, inversion: 1, ..Step::default() });
        e.apply_sequenced_harmony();
        let expected = harmony::voice_recipe(harmony::SavedChord { root:53, second:Some(63), quality:3, inversion:1, ..captured },0);
        assert_eq!(e.full_notes, expected);
        assert_eq!(e.memory,Some(captured));
        e.config.affect_chords = true;
        e.melody[2][74] = true;
        e.apply_sequenced_harmony();
        assert_eq!(e.memory,Some(captured));
        e.command(Command::Capture(3),&mut |_|{});
        assert_eq!(e.saved,Some((3,captured.encode())));
        assert_eq!(harmony::SavedChord::decode(captured.encode()),Some(captured));
    }
}
