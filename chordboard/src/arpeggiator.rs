//! Tracker rows and legacy arp editing state; allocation and serialization stay on the control thread.
use crossbeam_queue::ArrayQueue;
use nih_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, RwLock,
};

pub const PAGES: usize = 8;
pub const LOOPS: usize = 4;
pub const LOOP_STEPS: usize = 24;
pub const STEPS: usize = LOOP_STEPS * LOOPS;
pub const CUSTOM_STEPS: usize = 7;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fill {
    Repeat,
    #[default]
    Evolve,
}

fn legacy_fill() -> Fill {
    Fill::Repeat
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomStep {
    pub degree: u8,
    pub octave: i8,
    pub rest: bool,
    pub velocity: u8,
    pub gate: u8,
}
impl Default for CustomStep {
    fn default() -> Self {
        Self {
            degree: 1,
            octave: 0,
            rest: false,
            velocity: 100,
            gate: 100,
        }
    }
}
pub fn default_custom() -> [CustomStep; CUSTOM_STEPS] {
    std::array::from_fn(|i| CustomStep {
        degree: i as u8 + 1,
        ..CustomStep::default()
    })
}
pub const MODE_NAMES: [&str; 4] = ["Once", "Loop", "Sync Loop", "Aligned Loop"];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArpMode {
    Once,
    Loop,
    #[default]
    SyncLoop,
    AlignedLoop,
}
impl ArpMode {
    pub fn from_index(index: u8) -> Self {
        [Self::Once, Self::Loop, Self::SyncLoop, Self::AlignedLoop][index.min(3) as usize]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: ArpMode,
    pub pattern: u8,
    pub lock_length: bool,
    pub steps: u8,
    #[serde(default = "legacy_fill")]
    pub fill: Fill,
    pub custom: [CustomStep; CUSTOM_STEPS],
    /// Number of pattern passes (historical parameter ID: arp_octaves).
    pub octaves: u8,
    pub sync: bool,
    pub rate: f32,
    pub free_ms: f32,
    pub swing: f32,
    pub humanize: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: ArpMode::SyncLoop,
            pattern: 0,
            lock_length: false,
            steps: 4,
            fill: Fill::default(),
            custom: default_custom(),
            octaves: 1,
            sync: true,
            rate: 0.25,
            free_ms: 125.0,
            swing: 0.0,
            humanize: 0.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepState {
    #[default]
    Play,
    Mute,
    Skip,
}
impl StepState {
    pub fn next(self) -> Self {
        match self {
            Self::Play => Self::Mute,
            Self::Mute => Self::Skip,
            Self::Skip => Self::Play,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Step {
    pub state: StepState,
    pub velocity: u8,
    pub gate: u8,
    pub probability: u8,
    pub octave: i8,
    pub micro: i16,
    pub mods: [i16; 2],
}
impl Default for Step {
    fn default() -> Self {
        Self {
            state: StepState::Play,
            velocity: 100,
            gate: 65,
            probability: 100,
            octave: 0,
            micro: 0,
            mods: [0; 2],
        }
    }
}
fn finite(value: f32, default: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        default
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Loop {
    pub octave: i8,
    pub inversion: i8,
    pub steps: [Step; LOOP_STEPS],
}
impl Default for Loop {
    fn default() -> Self {
        Self {
            octave: 0,
            inversion: 0,
            steps: [Step::default(); LOOP_STEPS],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Replacement {
    pub serial: u64,
    pub settings: Settings,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub loops: [Loop; LOOPS],
    pub initialized: u8,
    #[serde(skip)]
    pub replacement: Option<Replacement>,
}
impl Default for Page {
    fn default() -> Self {
        Self {
            loops: [Loop::default(); LOOPS],
            initialized: 1,
            replacement: None,
        }
    }
}
impl Page {
    pub fn sanitize(&mut self) {
        self.initialized = self.initialized.clamp(1, LOOPS as u8);
        for variation in &mut self.loops {
            variation.octave = variation.octave.clamp(-4, 4);
            variation.inversion = variation.inversion.clamp(-12, 12);
            for step in &mut variation.steps {
                step.velocity = step.velocity.min(200);
                step.gate = step.gate.clamp(1, 100);
                step.probability = step.probability.min(100);
                step.octave = step.octave.clamp(-4, 4);
                step.micro = step.micro.clamp(-49, 49);
                for v in &mut step.mods {
                    *v = (*v).clamp(-100, 100);
                }
            }
        }
    }
    pub fn ensure_loops(&mut self, count: usize) -> bool {
        let count = count.clamp(1, LOOPS);
        let before = self.initialized;
        while (self.initialized as usize) < count {
            let i = self.initialized as usize;
            self.loops[i] = self.loops[i - 1];
            self.initialized += 1;
        }
        before != self.initialized
    }
    pub fn step(&self, occurrence: usize, pass_len: usize) -> &Step {
        &self.loops[occurrence / pass_len.max(1)].steps[occurrence % pass_len.max(1)]
    }
    pub fn step_mut(&mut self, occurrence: usize, pass_len: usize) -> &mut Step {
        &mut self.loops[occurrence / pass_len.max(1)].steps[occurrence % pass_len.max(1)]
    }
    pub fn flattened_steps(&self, pass_len: usize) -> [Step; STEPS] {
        let pass_len = pass_len.clamp(1, LOOP_STEPS);
        std::array::from_fn(|i| {
            if i < pass_len * LOOPS {
                *self.step(i, pass_len)
            } else {
                Step::default()
            }
        })
    }
    pub fn octaves(&self) -> [i8; LOOPS] {
        self.loops.map(|l| l.octave)
    }

    pub fn offset_voicing(&mut self, octave: i8, inversion: i8) {
        for variation in &mut self.loops {
            variation.octave += octave;
            variation.inversion += inversion;
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackerRow {
    pub enabled: bool,
    pub inherit_arp: bool,
    pub id: u64,
    pub chord: Option<crate::harmony::SavedChord>,
    pub duration: u32,
    pub arp_octave: i8,
    pub arp_inversion: i8,
    /// Display origin only; audio occurrence indices stay unchanged.
    pub view_step_origin: usize,
    pub settings: Settings,
    pub page: Page,
}
impl Default for TrackerRow {
    fn default() -> Self {
        Self {
            enabled: true,
            inherit_arp: false,
            id: 1,
            chord: None,
            duration: 1,
            arp_octave: 0,
            arp_inversion: 0,
            view_step_origin: 0,
            settings: Settings::default(),
            page: Page::default(),
        }
    }
}
impl TrackerRow {
    fn sanitize(&mut self) {
        self.duration = self.duration.max(1);
        self.arp_octave = self.arp_octave.clamp(-4, 4);
        self.arp_inversion = self.arp_inversion.clamp(-12, 12);
        self.settings.pattern = self.settings.pattern.min(7);
        self.settings.steps = self.settings.steps.clamp(1, LOOP_STEPS as u8);
        for step in &mut self.settings.custom {
            step.degree = step.degree.clamp(1, 7);
            step.octave = step.octave.clamp(-4, 4);
            step.velocity = step.velocity.min(200);
            step.gate = step.gate.clamp(1, 200);
        }
        self.settings.octaves = self.settings.octaves.clamp(1, LOOPS as u8);
        self.settings.rate = finite(self.settings.rate, 0.25).clamp(1.0 / 64.0, 2.0);
        self.settings.free_ms = finite(self.settings.free_ms, 125.0).clamp(1.0, 1500.0);
        self.settings.swing = finite(self.settings.swing, 0.0).clamp(-0.75, 0.75);
        self.settings.humanize = finite(self.settings.humanize, 0.0).clamp(0.0, 1.0);
        self.page.sanitize();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub performance: TrackerRow,
    /// Stable row identity shown at the left edge of the tracker.
    pub view_row_origin: u64,
    pub midi_fx: crate::midi_fx::MidiFxState,
    /// Dynamic composition rows. Audio processing only reads the prepared copy.
    pub rows: Vec<TrackerRow>,
    /// Retained for the old editor's unit fixtures; deliberately omitted from persistence.
    #[serde(skip)]
    pub pages: [Page; PAGES],
}
impl Default for State {
    fn default() -> Self {
        Self {
            performance: TrackerRow { enabled: false, id: 0, ..TrackerRow::default() },
            view_row_origin: 0,
            midi_fx: crate::midi_fx::MidiFxState::default(),
            rows: vec![TrackerRow::default()],
            pages: [Page::default(); PAGES],
        }
    }
}
impl State {
    pub fn view_row_start(&self) -> usize {
        self.rows.iter().position(|r| r.id == self.view_row_origin).unwrap_or(0)
    }
    pub fn editor_row(&self, index: usize, performance: bool) -> &TrackerRow {
        if performance { &self.performance } else { &self.rows[index.min(self.rows.len() - 1)] }
    }
    pub fn editor_row_mut(&mut self, index: usize, performance: bool) -> &mut TrackerRow {
        let index = index.min(self.rows.len() - 1);
        if performance { &mut self.performance } else { &mut self.rows[index] }
    }

    fn sanitize(&mut self) {
        self.performance.sanitize();
        self.performance.chord = None;
        self.midi_fx.sanitize();
        if self.rows.is_empty() {
            self.rows.push(TrackerRow::default());
        }
        let mut used_ids = Vec::with_capacity(self.rows.len());
        let mut next_id = 1u64;
        for row in &mut self.rows {
            if row.id == 0 || used_ids.contains(&row.id) {
                while used_ids.contains(&next_id) {
                    next_id = next_id.saturating_add(1);
                }
                row.id = next_id;
            }
            used_ids.push(row.id);
            next_id = next_id.max(row.id.saturating_add(1));
            row.sanitize();
        }
        for page in &mut self.pages {
            page.sanitize();
        }
    }

    pub fn add_row_after(&mut self, selected: usize) -> usize {
        let selected = selected.min(self.rows.len().saturating_sub(1));
        let mut row = self.rows[selected].clone();
        row.id = self
            .rows
            .iter()
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
            .max(1);
        let index = selected + 1;
        self.rows.insert(index, row);
        index
    }
    pub fn insert_blank_row_after(&mut self, previous: usize) -> usize {
        let index = self.add_row_after(previous);
        let row = &mut self.rows[index];
        row.chord = None;
        row.duration = 1;
        row.arp_octave = 0;
        row.arp_inversion = 0;
        row.inherit_arp = true;
        index
    }
    pub fn delete_row(&mut self, selected: usize) -> usize {
        if self.rows.len() <= 1 {
            self.rows[0] = TrackerRow::default();
            return 0;
        }
        let selected = selected.min(self.rows.len() - 1);
        self.rows.remove(selected);
        selected.min(self.rows.len() - 1)
    }
    pub fn move_row(&mut self, selected: usize, offset: isize) -> usize {
        if selected >= self.rows.len() {
            return self.rows.len().saturating_sub(1);
        }
        let destination =
            (selected as isize + offset).clamp(0, self.rows.len() as isize - 1) as usize;
        let row = self.rows.remove(selected);
        self.rows.insert(destination, row);
        destination
    }
    pub fn paste_row(&mut self, selected: usize, copied: &TrackerRow) {
        if let Some(row) = self.rows.get_mut(selected) {
            let id = row.id;
            *row = copied.clone();
            row.id = id;
        }
    }
}
pub type PreparedPage = Page;
#[derive(Debug)]
pub struct Prepared {
    pub performance: TrackerRow,
    pub midi_fx: crate::midi_fx::MidiFxState,
    saved_powers: u8,
    pub generation: u64,
    pub restore_epoch: u64,
    pub rows: Vec<TrackerRow>,
    pub pages: [PreparedPage; PAGES],
}
impl Prepared {
    pub(crate) fn apply_midi_powers(&mut self, params: &crate::params::ChordboardParams) {
        use crate::midi_learn::{fx_power_target, power_override, PERFORMANCE_ARP};
        let enabled = |target| {
            power_override(params, target)
                .unwrap_or(self.saved_powers & (1 << (target - PERFORMANCE_ARP)) != 0)
        };
        self.performance.enabled = enabled(PERFORMANCE_ARP);
        for effect in crate::midi_fx::EFFECTS {
            self.midi_fx.set_enabled(effect, enabled(fx_power_target(effect)));
        }
    }

    fn new(state: &State, generation: u64, restore_epoch: u64) -> Self {
        // Resolve the source in tracker order on the control thread. Keep each
        // entry's own chord and duration, including when starting mid-tracker.
        let mut previous_arp = None;
        let rows = state
            .rows
            .iter()
            .map(|row| {
                let arp = if row.inherit_arp {
                    previous_arp.unwrap_or(row)
                } else {
                    row
                };
                previous_arp = Some(arp);
                TrackerRow {
                    enabled: arp.enabled,
                    settings: arp.settings,
                    page: arp.page,
                    view_step_origin: 0,
                    ..row.clone()
                }
            })
            .collect();
        Self {
            performance: TrackerRow { view_step_origin: 0, ..state.performance.clone() },
            generation,
            restore_epoch,
            midi_fx: state.midi_fx.clone(),
            saved_powers: power_bits(state),
            rows,
            pages: std::array::from_fn(|p| state.pages[p]),
        }
    }
}
fn power_bits(state: &State) -> u8 {
    use crate::midi_learn::{fx_power_target, PERFORMANCE_ARP};
    crate::midi_fx::EFFECTS.into_iter().fold(u8::from(state.performance.enabled), |bits, effect| {
        bits | (u8::from(state.midi_fx.enabled(effect)) << (fx_power_target(effect) - PERFORMANCE_ARP))
    })
}

struct ControlState {
    state: State,
    generation: u64,
    restore_epoch: u64,
}

pub struct Store {
    control: RwLock<ControlState>,
    updates: ArrayQueue<Box<Prepared>>,
    retired: ArrayQueue<Box<Prepared>>,
    consuming: AtomicBool,
    powers: AtomicU8,
    pub initialization_pending: AtomicBool,
    pub replacement_serials: [AtomicU64; PAGES],
}

impl Default for Store {
    fn default() -> Self {
        let state = State::default();
        let powers = AtomicU8::new(power_bits(&state));
        Self {
            control: RwLock::new(ControlState {
                state,
                generation: 0,
                restore_epoch: 0,
            }),
            updates: ArrayQueue::new(2),
            retired: ArrayQueue::new(4),
            consuming: AtomicBool::new(false),
            powers,
            initialization_pending: AtomicBool::new(false),
            replacement_serials: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}

impl Store {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn snapshot(&self) -> State {
        self.control.read().unwrap().state.clone()
    }

    pub(crate) fn power(&self, target: usize) -> bool {
        self.powers.load(Ordering::Acquire) & (1 << (target - crate::midi_learn::PERFORMANCE_ARP)) != 0
    }

    pub fn tracker_arp_inheritance(&self, index: usize) -> Option<bool> {
        if index == 0 {
            return None;
        }
        self.control
            .read()
            .unwrap()
            .state
            .rows
            .get(index)
            .map(|row| row.inherit_arp)
    }

    /// UI metadata never publishes a generation to the audio thread.
    pub fn set_view_row_origin(&self, id: u64) {
        self.control.write().unwrap().state.view_row_origin = id;
    }
    pub fn set_view_step_origin(&self, index: usize, performance: bool, origin: usize) {
        self.control.write().unwrap().state.editor_row_mut(index, performance).view_step_origin = origin;
    }

    /// Control thread only. Publication is serialized with edits and obsolete boxes reclaimed here.
    pub fn edit(&self, edit: impl FnOnce(&mut State)) {
        self.edit_inner(edit, false);
    }

    /// Control/background thread only; already initialized variations are never overwritten.
    pub fn ensure_loops(&self, counts: [u8; PAGES]) {
        let mut control = self.control.write().unwrap();
        let mut changed = false;
        for (page, count) in control.state.pages.iter_mut().zip(counts) {
            changed |= page.ensure_loops(count as usize);
        }
        if changed {
            while self.retired.pop().is_some() {}
            control.generation = control.generation.wrapping_add(1);
            self.publish(&control);
        }
    }
    pub fn ensure_row_loops(&self, index: usize, count: usize) {
        let mut control = self.control.write().unwrap();
        let Some(row) = control.state.rows.get_mut(index) else {
            return;
        };
        if row.page.ensure_loops(count) {
            while self.retired.pop().is_some() {}
            control.generation = control.generation.wrapping_add(1);
            self.publish(&control);
        }
    }
    pub fn replace_page(&self, index: usize, mut page: Page, settings: Settings) {
        let mut control = self.control.write().unwrap();
        while self.retired.pop().is_some() {}
        control.generation = control.generation.wrapping_add(1);
        // Advertise the pending transaction before sending any host parameter changes.
        self.replacement_serials[index].store(control.generation, Ordering::Release);
        page.replacement = Some(Replacement {
            serial: control.generation,
            settings,
        });
        control.state.pages[index] = page;
        control.state.sanitize();
        self.publish(&control);
    }
    fn edit_inner(&self, edit: impl FnOnce(&mut State), restore: bool) {
        let mut control = self.control.write().unwrap();
        while self.retired.pop().is_some() {}
        edit(&mut control.state);
        control.state.sanitize();
        control.generation = control.generation.wrapping_add(1);
        if restore {
            for serial in &self.replacement_serials {
                serial.store(0, Ordering::Release);
            }
            control.restore_epoch = control.restore_epoch.wrapping_add(1);
        }
        self.publish(&control);
    }

    fn publish(&self, control: &ControlState) {
        self.powers.store(power_bits(&control.state), Ordering::Release);
        let mut prepared = Box::new(Prepared::new(
            &control.state,
            control.generation,
            control.restore_epoch,
        ));
        loop {
            match self.updates.push(prepared) {
                Ok(()) => break,
                Err(value) => {
                    prepared = value;
                    let _ = self.updates.pop();
                }
            }
        }
    }

    /// Initialization only; never call from process().
    pub fn initial_prepared(&self) -> Box<Prepared> {
        let control = self.control.read().unwrap();
        Box::new(Prepared::new(
            &control.state,
            control.generation,
            control.restore_epoch,
        ))
    }

    /// Single audio consumer. Exchanges at most one generation per callback. Old boxes are
    /// retained until the control thread reclaims them; a full retire queue delays updates.
    pub fn apply_pending(&self, current: &mut Box<Prepared>) -> bool {
        // Never wait if another caller is consuming. This also protects ownership while a
        // host is transitioning processing instances during state restoration.
        if self
            .consuming
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }
        struct ConsumerGuard<'a>(&'a AtomicBool);
        impl Drop for ConsumerGuard<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = ConsumerGuard(&self.consuming);
        if self.retired.is_full() {
            return false;
        }
        if let Some(next) = self.updates.pop() {
            let old = std::mem::replace(current, next);
            // Guarded sole producer; the control thread can only make room after is_full().
            self.retired
                .push(old)
                .expect("retirement space reserved by sole consumer");
            true
        } else {
            false
        }
    }
}

impl<'a> PersistentField<'a, State> for Arc<Store> {
    fn set(&self, value: State) {
        self.edit_inner(|state| *state = value, true);
    }
    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&State) -> R,
    {
        f(&self.control.read().unwrap().state)
    }
}

#[cfg(test)]
#[path = "arpeggiator_tests.rs"]
mod arpeggiator_tests;
