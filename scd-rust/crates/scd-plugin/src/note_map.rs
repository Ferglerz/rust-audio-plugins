use crate::vel_map::MAX_ARTS;
use nih_plug::params::persist::PersistentField;
use scd_core::kit::{stonehouse, ArticulationDef};
use scd_core::KitPieceId;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU16, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

const UNMAPPED: u8 = 255;
const ART_NONE: u16 = u16::MAX;
const N2_OFF: u8 = 254;
const DEFAULT_SLOT: u8 = 255;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtNotes {
    /// `None` = factory note 1.
    pub n1: Option<u8>,
    /// `Some(254)` is not used. `None` = factory note 2. `Some(n)` override.
    /// Use `n2_off` to disable a factory note 2.
    pub n2: Option<u8>,
    #[serde(default)]
    pub n2_off: bool,
}

impl Default for ArtNotes {
    fn default() -> Self {
        Self {
            n1: None,
            n2: None,
            n2_off: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteMapBank {
    #[serde(default)]
    pub notes: [[ArtNotes; MAX_ARTS]; KitPieceId::COUNT],
}

impl Default for NoteMapBank {
    fn default() -> Self {
        Self {
            notes: std::array::from_fn(|_| std::array::from_fn(|_| ArtNotes::default())),
        }
    }
}

pub struct ResolvedNote {
    pub pack_note: u8,
    pub kit_piece: Option<KitPieceId>,
    pub art: usize,
    pub silenced: bool,
    pub explicit: bool,
}

pub struct NoteMapState {
    bank: Mutex<NoteMapBank>,
    pack_of: [AtomicU8; 128],
    silenced: [AtomicU8; 128],
    explicit: [AtomicU8; 128],
    art_of: [AtomicU16; 128],
    learn_armed: AtomicU8,
    learn_note: AtomicU8,
}

impl NoteMapState {
    pub fn identity() -> Self {
        let state = Self {
            bank: Mutex::new(NoteMapBank::default()),
            pack_of: std::array::from_fn(|i| AtomicU8::new(i as u8)),
            silenced: std::array::from_fn(|_| AtomicU8::new(0)),
            explicit: std::array::from_fn(|_| AtomicU8::new(0)),
            art_of: std::array::from_fn(|_| AtomicU16::new(ART_NONE)),
            learn_armed: AtomicU8::new(0),
            learn_note: AtomicU8::new(255),
        };
        state.rebuild();
        state
    }

    pub fn arm_learn(&self) {
        self.learn_note.store(255, Ordering::Relaxed);
        self.learn_armed.store(1, Ordering::Relaxed);
    }

    pub fn disarm_learn(&self) {
        self.learn_armed.store(0, Ordering::Relaxed);
        self.learn_note.store(255, Ordering::Relaxed);
    }

    pub fn offer_learn(&self, note: u8) {
        if self.learn_armed.load(Ordering::Relaxed) != 0 && note <= 127 {
            self.learn_note.store(note, Ordering::Relaxed);
        }
    }

    pub fn take_learn(&self) -> Option<u8> {
        if self.learn_armed.load(Ordering::Relaxed) == 0 {
            return None;
        }
        let n = self.learn_note.load(Ordering::Relaxed);
        if n > 127 {
            return None;
        }
        self.disarm_learn();
        Some(n)
    }

    pub fn resolve(&self, incoming: u8) -> ResolvedNote {
        let i = incoming.min(127) as usize;
        let key = self.art_of[i].load(Ordering::Relaxed);
        let (kit_piece, art) = decode_art(key);
        ResolvedNote {
            pack_note: self.pack_of[i].load(Ordering::Relaxed),
            kit_piece,
            art,
            silenced: self.silenced[i].load(Ordering::Relaxed) != 0,
            explicit: self.explicit[i].load(Ordering::Relaxed) != 0,
        }
    }

    pub fn snapshot(&self) -> NoteMapBank {
        self.lock().clone()
    }

    pub fn load_bank(&self, bank: NoteMapBank) {
        *self.lock() = bank;
        self.rebuild();
    }

    pub fn reset_all(&self) {
        self.load_bank(NoteMapBank::default());
    }

    pub fn art_notes(&self, kit_piece: KitPieceId, art: usize) -> ArtNotes {
        self.lock().notes[kit_piece as usize][art.min(MAX_ARTS - 1)]
    }

    pub fn set_n1(&self, kit_piece: KitPieceId, art: usize, value: Option<u8>) {
        let art = art.min(MAX_ARTS - 1);
        self.lock().notes[kit_piece as usize][art].n1 = value;
        self.rebuild();
    }

    pub fn set_n2(&self, kit_piece: KitPieceId, art: usize, value: Option<u8>, off: bool) {
        let art = art.min(MAX_ARTS - 1);
        {
            let mut bank = self.lock();
            let slot = &mut bank.notes[kit_piece as usize][art];
            slot.n2 = value;
            slot.n2_off = off;
        }
        self.rebuild();
    }

    pub fn reset_slot(&self, kit_piece: KitPieceId, art: usize, note2: bool) {
        if note2 {
            self.set_n2(kit_piece, art, None, false);
        } else {
            self.set_n1(kit_piece, art, None);
        }
    }

    pub fn effective_n1(kit_piece: KitPieceId, art: usize, over: ArtNotes) -> u8 {
        over.n1.unwrap_or_else(|| factory_art(kit_piece, art).note1)
    }

    pub fn effective_n2(kit_piece: KitPieceId, art: usize, over: ArtNotes) -> Option<u8> {
        if over.n2_off {
            return None;
        }
        over.n2.or_else(|| factory_art(kit_piece, art).note2)
    }

    pub fn n1_altered(kit_piece: KitPieceId, art: usize, over: ArtNotes) -> bool {
        over.n1
            .is_some_and(|n| n != factory_art(kit_piece, art).note1)
    }

    pub fn n2_altered(kit_piece: KitPieceId, art: usize, over: ArtNotes) -> bool {
        if over.n2_off {
            return factory_art(kit_piece, art).note2.is_some();
        }
        match (over.n2, factory_art(kit_piece, art).note2) {
            (Some(n), Some(d)) => n != d,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }

    fn rebuild(&self) {
        let kit = stonehouse();
        let bank = self.lock().clone();
        let mut pack_of: [u8; 128] = std::array::from_fn(|i| i as u8);
        let mut silenced = [0u8; 128];
        let mut explicit = [0u8; 128];
        let mut art_of = [ART_NONE; 128];
        let mut claimed = [false; 128];
        let mut vacated = [false; 128];

        // Explicit assignments win collisions with factory notes, regardless
        // of the kit-piece iteration order.
        for explicit_pass in [false, true] {
            for kit_piece in KitPieceId::ALL {
                for (ai, art) in kit.arts(kit_piece).iter().enumerate().take(MAX_ARTS) {
                    let over = bank.notes[kit_piece as usize][ai];
                    let n1 = over.n1.unwrap_or(art.note1);
                    if over.n1.is_some() == explicit_pass && n1 < 128 {
                        claim(
                            &mut pack_of,
                            &mut art_of,
                            &mut claimed,
                            n1,
                            art.note1,
                            kit_piece,
                            ai,
                        );
                        explicit[n1 as usize] = u8::from(explicit_pass);
                    }
                    if n1 != art.note1 {
                        vacated[art.note1 as usize] = true;
                    }
                    let n2 = if over.n2_off {
                        None
                    } else {
                        over.n2.or(art.note2)
                    };
                    match (n2, art.note2) {
                        (Some(n), Some(d)) => {
                            let pack = d;
                            if over.n2.is_some() == explicit_pass && n < 128 {
                                claim(
                                    &mut pack_of,
                                    &mut art_of,
                                    &mut claimed,
                                    n,
                                    pack,
                                    kit_piece,
                                    ai,
                                );
                                explicit[n as usize] = u8::from(explicit_pass);
                            }
                            if n != d {
                                vacated[d as usize] = true;
                            }
                        }
                        (Some(n), None) => {
                            if over.n2.is_some() == explicit_pass && n < 128 {
                                claim(
                                    &mut pack_of,
                                    &mut art_of,
                                    &mut claimed,
                                    n,
                                    art.note1,
                                    kit_piece,
                                    ai,
                                );
                                explicit[n as usize] = u8::from(explicit_pass);
                            }
                        }
                        (None, Some(d)) => {
                            vacated[d as usize] = true;
                        }
                        (None, None) => {}
                    }
                }
            }
        }

        for i in 0..128 {
            if vacated[i] && !claimed[i] {
                silenced[i] = 1;
            }
        }

        for i in 0..128 {
            self.pack_of[i].store(pack_of[i], Ordering::Relaxed);
            self.silenced[i].store(silenced[i], Ordering::Relaxed);
            self.art_of[i].store(art_of[i], Ordering::Relaxed);
            self.explicit[i].store(explicit[i], Ordering::Relaxed);
        }
        let _ = (UNMAPPED, N2_OFF, DEFAULT_SLOT);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, NoteMapBank> {
        self.bank.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn factory_art(kit_piece: KitPieceId, art: usize) -> ArticulationDef {
    stonehouse()
        .arts(kit_piece)
        .get(art)
        .cloned()
        .unwrap_or(ArticulationDef {
            name: String::new(),
            layers: 0,
            rem_vol: 0,
            note1: 0,
            note2: None,
        })
}

fn claim(
    pack_of: &mut [u8; 128],
    art_of: &mut [u16; 128],
    claimed: &mut [bool; 128],
    incoming: u8,
    pack: u8,
    kit_piece: KitPieceId,
    art: usize,
) {
    let i = incoming as usize;
    if i >= 128 {
        return;
    }
    pack_of[i] = pack;
    art_of[i] = encode_art(kit_piece, art);
    claimed[i] = true;
}

fn encode_art(kit_piece: KitPieceId, art: usize) -> u16 {
    ((kit_piece as u16) << 8) | (art.min(MAX_ARTS - 1) as u16)
}

fn decode_art(key: u16) -> (Option<KitPieceId>, usize) {
    if key == ART_NONE {
        return (None, 0);
    }
    let piece = KitPieceId::ALL.get((key >> 8) as usize).copied();
    (piece, (key & 0xFF) as usize)
}

impl Default for NoteMapState {
    fn default() -> Self {
        Self::identity()
    }
}

impl<'a> PersistentField<'a, NoteMapBank> for Arc<NoteMapState> {
    fn set(&self, new_value: NoteMapBank) {
        self.load_bank(new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&NoteMapBank) -> R,
    {
        f(&self.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_keeps_factory_notes() {
        let state = NoteMapState::identity();
        let kick = state.resolve(36);
        assert!(!kick.silenced);
        assert_eq!(kick.pack_note, 36);
        assert_eq!(kick.kit_piece, Some(KitPieceId::Kick));
    }

    #[test]
    fn remap_n1_vacates_old_note() {
        let state = NoteMapState::identity();
        state.set_n1(KitPieceId::Kick, 1, Some(5));
        let neu = state.resolve(5);
        assert!(!neu.silenced);
        assert_eq!(neu.pack_note, 36);
        let old = state.resolve(36);
        assert!(old.silenced);
    }

    #[test]
    fn remap_n2_vacates_old_note() {
        let state = NoteMapState::identity();
        state.set_n2(KitPieceId::Kick, 1, Some(5), false);
        let neu = state.resolve(5);
        assert!(!neu.silenced);
        assert_eq!(neu.pack_note, 35);
        let old = state.resolve(35);
        assert!(old.silenced);
        state.set_n2(KitPieceId::Kick, 1, None, true);
        assert!(state.resolve(35).silenced);
        assert_eq!(state.resolve(5).pack_note, 5);
    }

    #[test]
    fn learn_only_captures_while_armed() {
        let state = NoteMapState::identity();
        state.offer_learn(40);
        assert_eq!(state.take_learn(), None);
        state.arm_learn();
        state.offer_learn(40);
        assert_eq!(state.take_learn(), Some(40));
        assert_eq!(state.take_learn(), None);
    }

    #[test]
    fn alt_reset_restores_factory() {
        let state = NoteMapState::identity();
        state.set_n1(KitPieceId::Kick, 1, Some(40));
        state.reset_slot(KitPieceId::Kick, 1, false);
        assert!(!state.resolve(36).silenced);
        assert_eq!(state.resolve(36).pack_note, 36);
    }
}
