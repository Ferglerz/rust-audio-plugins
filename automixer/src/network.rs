//! A fixed, atomic meter bus. No audio-thread allocation, locking, or waiting.
//! Shared parameter objects also belong to every wrapper's parameter map, so
//! editing any strip from any editor uses a valid host automation gesture.
use crate::params::SharedParams;
use atomic_float::AtomicF32;
use nih_plug::context::track::HardwareInput;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
    Arc, Mutex, OnceLock, Weak,
};
use std::time::Instant;

pub const CAPACITY: usize = 64;
pub const WAITING: u32 = 0;
pub const MIXING: u32 = 1;
pub const CHORUS: u32 = 2;
pub const QUIET: u32 = 3;
pub const BYPASSED: u32 = 4;
pub const CONFLICT: u32 = 5;

#[derive(Clone, Default)]
pub struct TrackLabels {
    pub name: Option<String>,
    pub input: Option<HardwareInput>,
}

pub struct Slot {
    pub owner: AtomicU64,
    pub mic: AtomicUsize,
    pub labels: Mutex<TrackLabels>,
    pub assigned: AtomicBool,
    // Sequence in high word; active bit + positive weighted RMS in low word.
    pub signal: AtomicU64,
    sequence: AtomicU32,
    pub updated_ms: AtomicU64,
    pub input_db: AtomicF32,
    pub output_db: AtomicF32,
    pub reduction_db: AtomicF32,
    pub chorus_mix: AtomicF32,
    pub speech: AtomicF32,
    pub vad_ready: AtomicBool,
    pub active: AtomicBool,
    pub state: AtomicU32,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            owner: AtomicU64::new(0),
            mic: AtomicUsize::new(0),
            labels: Mutex::default(),
            assigned: AtomicBool::new(false),
            signal: AtomicU64::new(0),
            sequence: AtomicU32::new(1),
            updated_ms: AtomicU64::new(0),
            input_db: AtomicF32::new(-120.0),
            output_db: AtomicF32::new(-120.0),
            reduction_db: AtomicF32::new(0.0),
            chorus_mix: AtomicF32::new(0.0),
            speech: AtomicF32::new(0.0),
            vad_ready: AtomicBool::new(false),
            active: AtomicBool::new(false),
            state: AtomicU32::new(WAITING),
        }
    }
}

impl Slot {
    pub fn publish(&self, weighted_rms: f32, active: bool) {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed).max(1);
        let value = weighted_rms.clamp(1e-12, 1e12).to_bits() | ((active as u32) << 31);
        self.signal
            .store(((seq as u64) << 32) | value as u64, Ordering::Release);
    }
    pub fn withdraw(&self) {
        self.signal.store(0, Ordering::Release);
        self.active.store(false, Ordering::Relaxed);
    }
}

pub struct Network {
    pub params: Arc<SharedParams>,
    pub slots: [Arc<Slot>; CAPACITY],
    token: AtomicU64,
    clock: Instant,
    registration: Mutex<()>,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            params: Arc::default(),
            slots: std::array::from_fn(|_| Arc::default()),
            token: AtomicU64::new(1),
            clock: Instant::now(),
            registration: Mutex::new(()),
        }
    }
}

impl Network {
    pub fn now_ms(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64 + 1
    }

    /// Only called during construction/prepare, never from process or reset.
    pub fn register(self: &Arc<Self>) -> Registration {
        let _guard = self.registration.lock().unwrap_or_else(|e| e.into_inner());
        let token = self.token.fetch_add(1, Ordering::Relaxed);
        let index = self
            .slots
            .iter()
            .position(|slot| slot.owner.load(Ordering::Acquire) == 0);
        let _initial_mic = index.map(|index| {
            let mut used = 0u64;
            for (i, slot) in self.slots.iter().enumerate() {
                if i != index && slot.owner.load(Ordering::Acquire) != 0 {
                    used |= 1 << slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
                }
            }
            let mic = (!used).trailing_zeros().min(63) as usize;
            let slot = &self.slots[index];
            slot.withdraw();
            slot.mic.store(mic, Ordering::Release);
            slot.assigned.store(false, Ordering::Release);
            *slot.labels.lock().unwrap_or_else(|e| e.into_inner()) = TrackLabels::default();
            slot.updated_ms.store(0, Ordering::Relaxed);
            slot.input_db.store(-120.0, Ordering::Relaxed);
            slot.output_db.store(-120.0, Ordering::Relaxed);
            slot.reduction_db.store(0.0, Ordering::Relaxed);
            slot.chorus_mix.store(0.0, Ordering::Relaxed);
            slot.speech.store(0.0, Ordering::Relaxed);
            slot.vad_ready.store(false, Ordering::Relaxed);
            slot.state.store(WAITING, Ordering::Relaxed);
            // Publish membership last. Audio readers must never observe a newly
            // claimed slot with the previous/default microphone identity.
            slot.owner.store(token, Ordering::Release);
            mic
        });
        Registration {
            network: self.clone(),
            index,
            token,
            #[cfg(test)]
            initial_mic: _initial_mic.unwrap_or(0),
        }
    }

    /// Resolve an automatic identity before audio starts. A provisional ID
    /// held by an instance that has not prepared can be exchanged safely.
    pub fn assign_automatic(
        &self,
        index: usize,
        token: u64,
        saved: Option<usize>,
        preferred: Option<usize>,
    ) -> Option<usize> {
        let _guard = self.registration.lock().unwrap_or_else(|e| e.into_inner());
        let slot = &self.slots[index];
        if slot.owner.load(Ordering::Acquire) != token {
            return None;
        }
        let current = slot.mic.load(Ordering::Acquire);
        let owner_of = |mic| {
            self.slots
                .iter()
                .enumerate()
                .filter(move |(i, peer)| {
                    *i != index
                        && peer.owner.load(Ordering::Acquire) != 0
                        && peer.mic.load(Ordering::Acquire) == mic
                })
                .map(|(_, slot)| slot)
        };

        for mic in saved
            .into_iter()
            .chain(preferred)
            .chain(std::iter::once(current))
            .chain(0..CAPACITY)
        {
            if mic >= CAPACITY {
                continue;
            }
            let mut occupants = owner_of(mic);
            if let Some(peer) = occupants.next() {
                if peer.assigned.load(Ordering::Acquire)
                    || occupants.next().is_some()
                    || owner_of(current).next().is_some()
                {
                    continue;
                }
                peer.mic.store(current, Ordering::Release);
            }
            slot.withdraw();
            slot.mic.store(mic, Ordering::Release);
            slot.assigned.store(true, Ordering::Release);
            return Some(mic);
        }
        None
    }

    pub fn conflict_mask(&self) -> u64 {
        let mut seen = 0;
        let mut conflicts = 0;
        for slot in &self.slots {
            if slot.owner.load(Ordering::Acquire) != 0 {
                let bit = 1u64 << slot.mic.load(Ordering::Acquire).min(63);
                conflicts |= seen & bit;
                seen |= bit;
            }
        }
        conflicts
    }

    /// GUI aggregation reads the live bus itself, so even an idle/bypassed
    /// instance's editor continues monitoring the other microphones.
    pub fn monitor(&self, now_ms: u64) -> Monitor {
        let conflicts = self.conflict_mask();
        let mut result = Monitor::default();
        for slot in &self.slots {
            let updated = slot.updated_ms.load(Ordering::Relaxed);
            let mic = slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
            if slot.owner.load(Ordering::Acquire) == 0
                || slot.signal.load(Ordering::Acquire) == 0
                || updated == 0
                || now_ms.saturating_sub(updated) >= 750
                || conflicts & (1 << mic) != 0
            {
                continue;
            }
            result.members += 1;
            result.active += slot.active.load(Ordering::Relaxed) as usize;
            result.chorus = result.chorus.max(slot.chorus_mix.load(Ordering::Relaxed));
        }
        result
    }
}

#[derive(Default)]
pub struct Monitor {
    pub members: usize,
    pub active: usize,
    pub chorus: f32,
}

pub struct Registration {
    pub network: Arc<Network>,
    pub index: Option<usize>,
    pub token: u64,
    #[cfg(test)]
    pub initial_mic: usize,
}
impl Registration {
    pub fn slot(&self) -> Option<&Arc<Slot>> {
        self.index.map(|i| &self.network.slots[i])
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(slot) = self.slot() {
            slot.withdraw();
            slot.owner.store(0, Ordering::Release);
        }
    }
}

pub fn shared_network() -> Arc<Network> {
    static NETWORK: OnceLock<Mutex<Weak<Network>>> = OnceLock::new();
    let mut weak = NETWORK
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(network) = weak.upgrade() {
        return network;
    }
    let network = Arc::new(Network::default());
    *weak = Arc::downgrade(&network);
    network
}

#[derive(Clone, Copy, Default)]
struct Peer {
    sequence: u32,
    owner: u64,
    age_samples: u64,
}

pub struct Observer {
    peers: [Peer; CAPACITY],
}
impl Default for Observer {
    fn default() -> Self {
        Self {
            peers: [Peer::default(); CAPACITY],
        }
    }
}

#[derive(Default, Debug)]
pub struct MixSnapshot {
    pub total: f32,
    pub active: usize,
    pub members: usize,
    pub conflicts: u64,
}

impl Observer {
    pub fn read(&mut self, network: &Network, elapsed: u32, stale_after: u64) -> MixSnapshot {
        let mut result = MixSnapshot {
            conflicts: network.conflict_mask(),
            ..Default::default()
        };
        for (slot, peer) in network.slots.iter().zip(self.peers.iter_mut()) {
            let owner = slot.owner.load(Ordering::Acquire);
            let signal = slot.signal.load(Ordering::Acquire);
            let sequence = (signal >> 32) as u32;
            if owner != peer.owner || sequence != peer.sequence {
                *peer = Peer {
                    owner,
                    sequence,
                    age_samples: 0,
                };
            } else {
                peer.age_samples = peer.age_samples.saturating_add(elapsed as u64);
            }
            let mic = slot.mic.load(Ordering::Acquire).min(63);
            if owner == 0
                || signal == 0
                || peer.age_samples > stale_after
                || result.conflicts & (1 << mic) != 0
            {
                continue;
            }
            let word = signal as u32;
            result.total += f32::from_bits(word & 0x7fff_ffff);
            result.active += (word >> 31) as usize;
            result.members += 1;
        }
        result
    }
}
