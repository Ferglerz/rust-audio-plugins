//! Tracker rows and legacy arp editing state; allocation and serialization stay on the control thread.
use crossbeam_queue::ArrayQueue;
use nih_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, RwLock,
};

pub const PAGES: usize = 8;
pub const LOOPS: usize = 4;
pub const LOOP_STEPS: usize = 24;
pub const STEPS: usize = LOOP_STEPS * LOOPS;
pub const TAPS: usize = 8;
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
pub mod source {
    pub const ARP: u8 = 1;
    pub const STRUM: u8 = 2;
    pub const CHORDS: u8 = 4;
    pub const BASS: u8 = 8;
    pub const MELODY: u8 = 16;
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DelayTap {
    pub enabled: bool,
    /// Beat interval in sync mode, milliseconds otherwise.
    pub time: f32,
    pub velocity: f32,
    pub feedback: f32,
    pub pitch: i8,
    pub fine: f32,
    /// Bipolar MIDI pan. Ping pong replaces this with alternating hard pan.
    pub pan: f32,
    /// Zero = automatic, 1..16 = explicit MIDI channel.
    pub channel: u8,
}
impl Default for DelayTap {
    fn default() -> Self {
        Self {
            enabled: true,
            time: 0.5,
            velocity: 0.75,
            feedback: 0.35,
            pitch: 0,
            fine: 0.0,
            pan: 0.0,
            channel: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DelaySettings {
    pub enabled: bool,
    pub sync: bool,
    pub ping_pong: bool,
    /// Join the main voice-leading mode, including sounding echo references.
    pub voice_leading: bool,
    /// Resolve each new echo against the chord active at playback time.
    pub follow_chords: bool,
    pub sources: u8,
    pub count: u8,
    pub taps: [DelayTap; TAPS],
}
impl Default for DelaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sync: true,
            ping_pong: false,
            voice_leading: false,
            follow_chords: false,
            sources: source::ARP,
            count: 1,
            taps: [DelayTap::default(); TAPS],
        }
    }
}
impl DelaySettings {
    pub fn sanitize(&mut self) {
        self.count = self.count.clamp(1, TAPS as u8);
        self.sources &= 31;
        for tap in &mut self.taps {
            tap.time = finite(tap.time, if self.sync { 0.5 } else { 250.0 }).clamp(
                if self.sync { 1.0 / 64.0 } else { 1.0 },
                if self.sync { 16.0 } else { 8000.0 },
            );
            tap.velocity = finite(tap.velocity, 0.75).clamp(0.0, 2.0);
            tap.feedback = finite(tap.feedback, 0.35).clamp(0.0, 0.95);
            tap.pitch = tap.pitch.clamp(-24, 24);
            tap.fine = finite(tap.fine, 0.0).clamp(-100.0, 100.0);
            tap.pan = finite(tap.pan, 0.0).clamp(-1.0, 1.0);
            tap.channel = tap.channel.min(16);
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
    pub steps: [Step; LOOP_STEPS],
}
impl Default for Loop {
    fn default() -> Self {
        Self {
            octave: 0,
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
    pub delay: DelaySettings,
    #[serde(skip)]
    pub replacement: Option<Replacement>,
}
impl Default for Page {
    fn default() -> Self {
        Self {
            loops: [Loop::default(); LOOPS],
            initialized: 1,
            delay: DelaySettings::default(),
            replacement: None,
        }
    }
}
impl Page {
    pub fn sanitize(&mut self) {
        self.initialized = self.initialized.clamp(1, LOOPS as u8);
        for variation in &mut self.loops {
            variation.octave = variation.octave.clamp(-4, 4);
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
        self.delay.sanitize();
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
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackerRow {
    pub id: u64,
    pub chord: Option<crate::harmony::SavedChord>,
    pub duration: u32,
    pub settings: Settings,
    pub page: Page,
}
impl Default for TrackerRow {
    fn default() -> Self {
        Self {
            id: 1,
            chord: None,
            duration: 1,
            settings: Settings::default(),
            page: Page::default(),
        }
    }
}
impl TrackerRow {
    fn sanitize(&mut self) {
        self.duration = self.duration.max(1);
        self.settings.pattern = self.settings.pattern.min(6);
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
    /// Dynamic composition rows. Audio processing only reads the prepared copy.
    pub rows: Vec<TrackerRow>,
    /// Retained for the old editor's unit fixtures; deliberately omitted from persistence.
    #[serde(skip)]
    pub pages: [Page; PAGES],
}
impl Default for State {
    fn default() -> Self {
        Self {
            rows: vec![TrackerRow::default()],
            pages: [Page::default(); PAGES],
        }
    }
}
impl State {
    fn sanitize(&mut self) {
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
        row.duration = 1;
        let index = selected + 1;
        self.rows.insert(index, row);
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
    pub generation: u64,
    pub restore_epoch: u64,
    pub rows: Vec<TrackerRow>,
    pub pages: [PreparedPage; PAGES],
}
impl Prepared {
    fn new(state: &State, generation: u64, restore_epoch: u64) -> Self {
        Self {
            generation,
            restore_epoch,
            rows: state.rows.clone(),
            pages: std::array::from_fn(|p| state.pages[p]),
        }
    }
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
    pub initialization_pending: AtomicBool,
    pub replacement_serials: [AtomicU64; PAGES],
}

impl Default for Store {
    fn default() -> Self {
        Self {
            control: RwLock::new(ControlState {
                state: State::default(),
                generation: 0,
                restore_epoch: 0,
            }),
            updates: ArrayQueue::new(2),
            retired: ArrayQueue::new(4),
            consuming: AtomicBool::new(false),
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
