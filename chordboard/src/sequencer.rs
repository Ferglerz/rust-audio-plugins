//! Preset-owned patterns and bounded, allocation-free audio publication.
use crossbeam_queue::ArrayQueue;
use nih_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

pub const LANES: usize = 4;
pub const PAGES: usize = 8;
pub const STEPS: usize = 32;
pub const CC_TRACKS: usize = 4;
pub const LANE_NAMES: [&str; LANES] = ["Arp", "Chords", "Bass", "Harmony"];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Step {
    pub enabled: bool,
    pub live: bool,
    pub velocity: u8,
    pub gate: u8,
    pub tie: bool,
    pub octave: i8,
    pub tone: i8,
    pub probability: u8,
    pub ratchets: u8,
    pub micro: i16,
    pub root_offset: i8,
    pub quality: i8,
    pub inversion: i8,
    pub spread: i8,
    pub pressure: i16,
    pub timbre: i16,
    pub bend: f32,
    pub cc: [i16; CC_TRACKS],
}

impl Default for Step {
    fn default() -> Self {
        Self {
            enabled: true,
            live: false,
            velocity: 100,
            gate: 65,
            tie: false,
            octave: 0,
            tone: -1,
            probability: 100,
            ratchets: 1,
            micro: 0,
            root_offset: 0,
            quality: -1,
            inversion: -1,
            spread: -1,
            pressure: 0,
            timbre: 0,
            bend: 0.0,
            cc: [0; CC_TRACKS],
        }
    }
}

impl Step {
    fn sanitize(&mut self) {
        self.velocity = self.velocity.min(200);
        self.gate = self.gate.clamp(1, 100);
        self.octave = self.octave.clamp(-4, 4);
        self.tone = self.tone.clamp(-2, 31);
        self.probability = self.probability.min(100);
        self.ratchets = self.ratchets.clamp(1, 8);
        self.micro = self.micro.clamp(-49, 49);
        self.root_offset = self.root_offset.clamp(-48, 48);
        self.quality = self.quality.clamp(-1, 11);
        self.inversion = self.inversion.clamp(-1, 5);
        self.spread = self.spread.clamp(-1, 4);
        self.pressure = self.pressure.clamp(-127, 127);
        self.timbre = self.timbre.clamp(-127, 127);
        self.bend = if self.bend.is_finite() {
            self.bend.clamp(-48.0, 48.0)
        } else {
            0.0
        };
        for offset in &mut self.cc {
            *offset = (*offset).clamp(-127, 127);
        }
    }
}

pub fn valid_cc(number: u8) -> bool {
    !matches!(number, 0 | 6 | 32..=64 | 74 | 96..=127) && number < 128
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub name: String,
    pub harmony_boxes: bool,
    pub harmony_spans: [u8; STEPS],
    pub length: u8,
    pub start: u8,
    pub end: u8,
    pub rate: f32,
    pub enabled: bool,
    pub interlock_override: Option<u8>,
    pub cc_numbers: [u8; CC_TRACKS],
    pub steps: [Step; STEPS],
}

impl Default for Page {
    fn default() -> Self {
        Self {
            name: "Page".into(),
            harmony_boxes: false,
            harmony_spans: [0; STEPS],
            length: 1,
            start: 0,
            end: 0,
            rate: 0.25,
            enabled: true,
            interlock_override: None,
            cc_numbers: [11, 1, 2, 4],
            steps: [Step::default(); STEPS],
        }
    }
}

impl Page {
    /// Legacy held recipes become explicit spans when first edited with the box tool.
    pub fn box_spans(&self) -> [u8; STEPS] {
        if self.harmony_boxes {
            return self.harmony_spans;
        }
        let mut spans = [0; STEPS];
        let mut active = None;
        for i in 0..self.length as usize {
            let step = self.steps[i];
            if step.live || step.enabled {
                if let Some(start) = active.take() {
                    spans[start] = (i - start) as u8;
                }
                if step.enabled && !step.live {
                    active = Some(i);
                }
            }
        }
        if let Some(start) = active {
            spans[start] = self.length - start as u8;
        }
        spans
    }
    pub fn box_at(&self, index: usize) -> Option<(usize, usize)> {
        self.box_spans()
            .iter()
            .enumerate()
            .find_map(|(start, &len)| {
                (len > 0 && index >= start && index < start + len as usize)
                    .then_some((start, len as usize))
            })
    }
    pub fn draw_box(&mut self, start: usize, end: usize) {
        self.harmony_spans = self.box_spans();
        self.harmony_boxes = true;
        let end = end.min(self.length as usize - 1);
        let start = start.min(end);
        // Stop at another box rather than silently overwriting its adjustment.
        let end = ((start + 1)..=end)
            .find(|&i| self.harmony_spans[i] > 0)
            .map_or(end, |next| next - 1);
        self.harmony_spans[start] = (end - start + 1) as u8;
        self.steps[start].enabled = true;
        self.steps[start].live = false;
    }
    pub fn delete_box(&mut self, start: usize) {
        self.harmony_spans = self.box_spans();
        self.harmony_boxes = true;
        self.harmony_spans[start] = 0;
    }
    pub fn sanitize(&mut self) {
        self.length = self.length.clamp(1, STEPS as u8);
        self.start = self.start.min(self.length - 1);
        self.end = self.length - 1;
        self.rate = if self.rate.is_finite() {
            self.rate.clamp(1.0 / 64.0, 16.0)
        } else {
            0.25
        };
        self.interlock_override = self.interlock_override.filter(|&v| v <= 2);
        // A controller belongs to one track; invalid/duplicate assignments restore a free default.
        for i in 0..CC_TRACKS {
            if !valid_cc(self.cc_numbers[i]) || self.cc_numbers[..i].contains(&self.cc_numbers[i]) {
                self.cc_numbers[i] = [11, 1, 2, 4, 7, 10, 12, 13]
                    .into_iter()
                    .find(|n| !self.cc_numbers[..i].contains(n))
                    .unwrap_or(11);
            }
        }
        self.name = self.name.chars().take(32).collect();
        let mut occupied_until = 0;
        for i in 0..STEPS {
            if i >= self.length as usize || i < occupied_until {
                self.harmony_spans[i] = 0;
            } else if self.harmony_spans[i] > 0 {
                self.harmony_spans[i] = self.harmony_spans[i].min(self.length - i as u8);
                occupied_until = i + self.harmony_spans[i] as usize;
            }
        }
        for step in &mut self.steps {
            step.sanitize();
        }
    }

    /// Evenly distribute hits in the inclusive loop range without changing other step settings.
    pub fn euclidean(&mut self, hits: u8, rotation: u8) {
        self.sanitize();
        let len = (self.end - self.start + 1) as usize;
        let hits = (hits as usize).min(len);
        let rotation = rotation as usize % len;
        for index in 0..len {
            let phase = (index + len - rotation) % len;
            self.steps[self.start as usize + index].enabled = (phase * hits) % len < hits;
        }
    }

    /// Shape active hits with a regular accent and quieter intervening ghost notes.
    pub fn accents(&mut self, every: u8, accent: u8, ghost: u8) {
        self.sanitize();
        let every = every.max(1) as usize;
        for index in self.start as usize..=self.end as usize {
            self.steps[index].velocity = if (index - self.start as usize).is_multiple_of(every) {
                accent.min(200)
            } else {
                ghost.min(200)
            };
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub version: u8,
    pub lanes: [[Page; PAGES]; LANES],
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            lanes: std::array::from_fn(|lane| {
                std::array::from_fn(|page| {
                    let mut p = Page {
                        name: format!("{} {}", LANE_NAMES[lane], page + 1),
                        ..Page::default()
                    };
                    if lane != 0 {
                        for step in &mut p.steps {
                            step.enabled = false;
                        }
                    }
                    p
                })
            }),
        }
    }
}

impl State {
    pub fn sanitize(&mut self) {
        self.version = 1;
        for (index, lane) in self.lanes.iter_mut().enumerate() {
            for page in lane {
                // Keep the old Bass slot readable in saved states, but disable its playback.
                if index == 2 {
                    page.enabled = false;
                }
                page.sanitize();
            }
        }
    }
}

/// Strings stay on the control thread. Audio reads only these bounded values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedPage {
    pub length: u8,
    pub start: u8,
    pub end: u8,
    pub rate: f32,
    pub enabled: bool,
    pub interlock_override: Option<u8>,
    pub cc_numbers: [u8; CC_TRACKS],
    pub steps: [Step; STEPS],
}

impl From<&Page> for PreparedPage {
    fn from(p: &Page) -> Self {
        Self {
            length: p.length,
            start: p.start,
            end: p.end,
            rate: p.rate,
            enabled: p.enabled,
            interlock_override: p.interlock_override,
            cc_numbers: p.cc_numbers,
            steps: p.steps,
        }
    }
}

/// Temporary playback range for one page while a step is being edited. Never saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audition {
    pub lane: usize,
    pub page: usize,
    pub start: u8,
    pub end: u8,
}

#[derive(Debug)]
pub struct Prepared {
    pub generation: u64,
    pub restore_epoch: u64,
    pub pages: [[PreparedPage; PAGES]; LANES],
}

impl Prepared {
    fn new(state: &State, generation: u64, restore_epoch: u64, audition: Option<Audition>) -> Self {
        let mut prepared = Self::from_state(state, generation, restore_epoch);
        if let Some(a) = audition.filter(|a| a.lane < LANES && a.page < PAGES) {
            let page = &mut prepared.pages[a.lane][a.page];
            let last = page.length.saturating_sub(1);
            page.start = a.start.min(last);
            page.end = a.end.clamp(page.start, last);
        }
        prepared
    }
    fn from_state(state: &State, generation: u64, restore_epoch: u64) -> Self {
        Self {
            generation,
            restore_epoch,
            pages: std::array::from_fn(|l| {
                std::array::from_fn(|p| {
                    let page = &state.lanes[l][p];
                    let mut prepared = PreparedPage::from(page);
                    if l == 3 && page.harmony_boxes {
                        for index in 0..STEPS {
                            prepared.steps[index] = if let Some((start, _)) = page.box_at(index) {
                                Step {
                                    enabled: true,
                                    live: false,
                                    probability: 100,
                                    micro: 0,
                                    ..page.steps[start]
                                }
                            } else {
                                Step {
                                    enabled: false,
                                    live: true,
                                    ..Step::default()
                                }
                            };
                        }
                    }
                    prepared
                })
            }),
        }
    }
}

struct ControlState {
    state: State,
    generation: u64,
    restore_epoch: u64,
    audition: Option<Audition>,
}

pub struct Store {
    control: RwLock<ControlState>,
    updates: ArrayQueue<Box<Prepared>>,
    retired: ArrayQueue<Box<Prepared>>,
    consuming: AtomicBool,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            control: RwLock::new(ControlState {
                state: State::default(),
                generation: 0,
                restore_epoch: 0,
                audition: None,
            }),
            updates: ArrayQueue::new(2),
            retired: ArrayQueue::new(4),
            consuming: AtomicBool::new(false),
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

    fn edit_inner(&self, edit: impl FnOnce(&mut State), restore: bool) {
        let mut control = self.control.write().unwrap();
        while self.retired.pop().is_some() {}
        edit(&mut control.state);
        control.state.sanitize();
        control.generation = control.generation.wrapping_add(1);
        if restore {
            control.restore_epoch = control.restore_epoch.wrapping_add(1);
            control.audition = None;
        }
        self.publish(&control);
    }

    pub fn audition(&self) -> Option<Audition> {
        self.control.read().unwrap().audition
    }

    /// Control thread only. Loops part of a page without changing the saved pattern.
    pub fn set_audition(&self, audition: Option<Audition>) {
        let mut control = self.control.write().unwrap();
        if control.audition == audition {
            return;
        }
        while self.retired.pop().is_some() {}
        control.audition = audition;
        control.generation = control.generation.wrapping_add(1);
        self.publish(&control);
    }

    fn publish(&self, control: &ControlState) {
        let mut prepared = Box::new(Prepared::new(
            &control.state,
            control.generation,
            control.restore_epoch,
            control.audition,
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
            control.audition,
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
mod tests {
    use super::*;
    #[test]
    fn state_roundtrip_and_old_missing_fields() {
        let mut state = State::default();
        state.lanes[2][1].steps[4].cc[0] = -23;
        assert_eq!(
            serde_json::from_str::<State>(&serde_json::to_string(&state).unwrap()).unwrap(),
            state
        );
        assert_eq!(
            serde_json::from_str::<State>("{}").unwrap(),
            State::default()
        );
        let legacy = serde_json::json!({"memory_pages": [[5, null, 3, 1]]});
        let restored: State = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored, State::default());
        assert!(serde_json::to_value(&restored)
            .unwrap()
            .get("memory_pages")
            .is_none());
        for lane in &state.lanes {
            for page in lane {
                assert_eq!((page.length, page.start, page.end), (1, 0, 0));
            }
        }
        assert!(state.lanes[0][0].steps[0].enabled);
        assert!(!state.lanes[3][0].steps[0].enabled);
    }
    #[test]
    fn legacy_tone_pool_is_ignored_and_loop_end_follows_length() {
        let mut page: Page = serde_json::from_value(serde_json::json!({
            "length": 12, "start": 1, "end": 2,
            "tone_mute": 4294967295u32, "tone_skip": 4294967295u32,
            "tone_velocity": [0, 0]
        }))
        .unwrap();
        page.sanitize();
        assert_eq!((page.length, page.start, page.end), (12, 1, 11));
        let saved = serde_json::to_value(&page).unwrap();
        for removed in ["tone_mute", "tone_skip", "tone_velocity"] {
            assert!(saved.get(removed).is_none());
        }
    }
    #[test]
    fn audition_loops_published_page_without_saving() {
        let store = Store::new();
        store.edit(|s| s.lanes[0][2].length = 8);
        store.set_audition(Some(Audition {
            lane: 0,
            page: 2,
            start: 3,
            end: 20,
        }));
        let prepared = store.initial_prepared();
        assert_eq!((prepared.pages[0][2].start, prepared.pages[0][2].end), (3, 7));
        assert_eq!((prepared.pages[0][1].start, prepared.pages[0][1].end), (0, 0));
        let saved = store.snapshot();
        assert_eq!((saved.lanes[0][2].start, saved.lanes[0][2].end), (0, 7));
        store.set_audition(None);
        let prepared = store.initial_prepared();
        assert_eq!((prepared.pages[0][2].start, prepared.pages[0][2].end), (0, 7));
    }
    #[test]
    fn malformed_bounds_and_reserved_controllers() {
        let mut state = State::default();
        let p = &mut state.lanes[0][0];
        p.length = 0;
        p.start = 255;
        p.end = 255;
        p.rate = f32::NAN;
        p.cc_numbers = [64, 74, 6, 127];
        p.steps[0].bend = f32::INFINITY;
        p.steps[0].cc[0] = -500;
        state.sanitize();
        let p = &state.lanes[0][0];
        assert_eq!((p.length, p.start, p.end), (1, 0, 0));
        assert_eq!(p.rate, 0.25);
        assert_eq!(p.steps[0].bend, 0.0);
        assert_eq!(p.steps[0].cc[0], -127);
        assert!(p.cc_numbers.into_iter().all(valid_cc));
    }
    #[test]
    fn euclidean_rotation_and_preserved_offsets() {
        let mut p = Page::default();
        p.length = 8;
        p.end = 7;
        p.steps[0].cc[0] = -12;
        p.euclidean(3, 0);
        let original: Vec<_> = p.steps[..8].iter().map(|s| s.enabled).collect();
        assert_eq!(original.iter().filter(|&&on| on).count(), 3);
        p.euclidean(3, 2);
        for i in 0..8 {
            assert_eq!(p.steps[(i + 2) % 8].enabled, original[i]);
        }
        assert_eq!(p.steps[0].cc[0], -12);
    }
    #[test]
    fn latest_generation_survives_full_publication_queue() {
        let store = Store::new();
        let mut audio = store.initial_prepared();
        for n in 1..20 {
            store.edit(|s| s.lanes[0][0].steps[0].cc[0] = n);
        }
        while store.apply_pending(&mut audio) {}
        assert_eq!(audio.generation, 19);
        assert_eq!(audio.pages[0][0].steps[0].cc[0], 19);
        for _ in 0..8 {
            store.edit(|s| s.lanes[0][0].steps[0].cc[0] += 1);
            assert!(store.apply_pending(&mut audio));
        }
    }
    #[test]
    fn full_retirement_queue_defers_without_losing_latest_state() {
        let store = Store::new();
        let mut audio = store.initial_prepared();
        store.edit(|s| s.lanes[0][0].steps[0].cc[0] = -37);
        for _ in 0..4 {
            store.retired.push(store.initial_prepared()).unwrap();
        }
        assert!(!store.apply_pending(&mut audio));
        assert_eq!(audio.generation, 0);
        assert_eq!(store.updates.len(), 1);
        while store.retired.pop().is_some() {}
        assert!(store.apply_pending(&mut audio));
        assert_eq!(audio.pages[0][0].steps[0].cc[0], -37);
    }
    #[test]
    fn persistence_load_sanitizes_before_publication() {
        let store = Store::new();
        let mut audio = store.initial_prepared();
        let mut malformed = State::default();
        malformed.lanes[1][7].length = 255;
        malformed.lanes[1][7].steps[31].ratchets = 255;
        malformed.lanes[1][7].steps[31].micro = i16::MIN;
        PersistentField::set(&store, malformed);
        assert!(store.apply_pending(&mut audio));
        assert_eq!(audio.pages[1][7].length, 32);
        assert_eq!(audio.pages[1][7].steps[31].ratchets, 8);
        assert_eq!(audio.pages[1][7].steps[31].micro, -49);
        assert_eq!(store.snapshot().lanes[1][7].steps[31].micro, -49);
        let too_many_steps = format!("{{\"steps\":[{}]}}", vec!["{}"; 33].join(","));
        assert!(serde_json::from_str::<Page>(&too_many_steps).is_err());
    }
    #[test]
    fn restore_epoch_survives_coalesced_editor_generations() {
        let store = Store::new();
        let mut audio = store.initial_prepared();
        store.edit(|s| s.lanes[0][0].name = "Edited".into());
        assert!(store.apply_pending(&mut audio));
        assert_eq!(audio.restore_epoch, 0);
        PersistentField::set(&store, State::default());
        for n in 1..20 {
            store.edit(|s| s.lanes[0][0].steps[0].cc[0] = n);
        }
        while store.apply_pending(&mut audio) {}
        assert_eq!(audio.generation, 21);
        assert_eq!(audio.restore_epoch, 1);
        assert_eq!(audio.pages[0][0].steps[0].cc[0], 19);
        store.edit(|s| s.lanes[0][1].name = "Inactive page".into());
        assert!(store.apply_pending(&mut audio));
        assert_eq!(audio.restore_epoch, 1);
        PersistentField::set(&store, State::default());
        assert!(store.apply_pending(&mut audio));
        assert_eq!(audio.restore_epoch, 2);
    }
    #[test]
    fn harmony_box_resize_preserves_neighbors_and_legacy_recipes() {
        let mut p = Page {
            length: 16,
            end: 15,
            ..Page::default()
        };
        p.steps.iter_mut().for_each(|s| s.enabled = false);
        p.steps[1].enabled = true;
        p.steps[1].root_offset = 7;
        p.steps[4].live = true;
        assert_eq!(p.box_at(3), Some((1, 3)));
        assert_eq!(p.box_at(4), None);
        p.draw_box(6, 7);
        p.steps[6].root_offset = -2;
        p.draw_box(1, 10);
        assert_eq!(p.box_at(5), Some((1, 5)));
        assert_eq!(p.box_at(6), Some((6, 2)));
        assert_eq!(p.steps[1].root_offset, 7);
        assert_eq!(p.steps[6].root_offset, -2);
        p.delete_box(1);
        assert_eq!(p.box_at(2), None);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<Page>(&json).unwrap(), p);
    }
}
