use memmap2::Mmap;
use rkyv::{Archive, Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

pub mod kit;

pub const SCD_MAGIC: &[u8; 8] = b"SCDPACK1";

#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[archive(check_bytes)]
#[repr(u8)]
pub enum MicChannel {
    Close = 0,
    XY = 1,
    Mono = 2,
    Wide = 3,
    FrontMS = 4,
    Room = 5,
}

impl MicChannel {
    pub const COUNT: usize = 6;
    pub const ALL: [MicChannel; Self::COUNT] = [
        MicChannel::Close,
        MicChannel::XY,
        MicChannel::Mono,
        MicChannel::Wide,
        MicChannel::FrontMS,
        MicChannel::Room,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::XY => "XY",
            Self::Mono => "Mono",
            Self::Wide => "Wide",
            Self::FrontMS => "Front MS",
            Self::Room => "Room",
        }
    }

    /// Infer mic position from a WAV filename (HISE naming convention).
    pub fn from_filename(filename: &str) -> Option<Self> {
        if filename.contains("Close") {
            Some(Self::Close)
        } else if filename.contains("XY") {
            Some(Self::XY)
        } else if filename.contains("Mono") {
            Some(Self::Mono)
        } else if filename.contains("Wide") {
            Some(Self::Wide)
        } else if filename.contains("Front MS") {
            Some(Self::FrontMS)
        } else if filename.contains("Room") {
            Some(Self::Room)
        } else {
            None
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
#[repr(u8)]
pub enum KitPieceId {
    Kick = 0,
    Snare = 1,
    OpenSnare = 2,
    Hihat = 3,
    Tom1 = 4,
    Tom2 = 5,
    FloorTom = 6,
    Ride = 7,
    China = 8,
    Stack = 9,
    Splash = 10,
    LCrash = 11,
    RCrash = 12,
}

impl KitPieceId {
    pub const COUNT: usize = 13;
    pub const ALL: [KitPieceId; Self::COUNT] = [
        KitPieceId::Kick,
        KitPieceId::Snare,
        KitPieceId::OpenSnare,
        KitPieceId::Hihat,
        KitPieceId::Tom1,
        KitPieceId::Tom2,
        KitPieceId::FloorTom,
        KitPieceId::Ride,
        KitPieceId::China,
        KitPieceId::Stack,
        KitPieceId::Splash,
        KitPieceId::LCrash,
        KitPieceId::RCrash,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Kick => "Kick",
            Self::Snare => "Snare",
            Self::OpenSnare => "Open Snare",
            Self::Hihat => "Hihat",
            Self::Tom1 => "Tom 1",
            Self::Tom2 => "Tom 2",
            Self::FloorTom => "Floor Tom",
            Self::Ride => "Ride",
            Self::China => "China",
            Self::Stack => "Stack",
            Self::Splash => "Splash",
            Self::LCrash => "L Crash",
            Self::RCrash => "R Crash",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "Kick" => Self::Kick,
            "Snare" => Self::Snare,
            "Open Snare" => Self::OpenSnare,
            "Hihat" => Self::Hihat,
            "Tom 1" => Self::Tom1,
            "Tom 2" => Self::Tom2,
            "Floor Tom" => Self::FloorTom,
            "Ride" => Self::Ride,
            "China" => Self::China,
            "Stack" => Self::Stack,
            "Splash" => Self::Splash,
            "L Crash" => Self::LCrash,
            "R Crash" => Self::RCrash,
            _ => return None,
        })
    }

    /// Close mic has no L/R crash capsules in the original kit.
    pub fn omits_mic(self, mic: MicChannel) -> bool {
        matches!(
            (self, mic),
            (Self::LCrash | Self::RCrash, MicChannel::Close)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crashes_omit_close_mic_only() {
        assert!(KitPieceId::LCrash.omits_mic(MicChannel::Close));
        assert!(KitPieceId::RCrash.omits_mic(MicChannel::Close));
        assert!(!KitPieceId::LCrash.omits_mic(MicChannel::Room));
        assert!(!KitPieceId::Kick.omits_mic(MicChannel::Close));
        assert!(!KitPieceId::China.omits_mic(MicChannel::Close));
    }
}

/// Metadata for a single audio stream slice in the pack
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[archive(check_bytes)]
pub struct SampleSlice {
    pub offset_bytes: u64,
    pub length_frames: u32,
    pub sample_rate: u32,
    pub channels: u8,
}

/// Strike entry: 1 drum hit mapped to velocity & round robin, containing slices for each mic
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct StrikeEntry {
    pub midi_note: u8,
    pub lo_vel: u8,
    pub hi_vel: u8,
    pub rr_group: u8,
    pub volume_db: i8,
    pub kit_piece: KitPieceId,
    /// Up to 6 mic slices: [Close, XY, Mono, Wide, FrontMS, Room]
    pub mic_slices: [Option<SampleSlice>; MicChannel::COUNT],
}

impl StrikeEntry {
    pub fn new(
        midi_note: u8,
        lo_vel: u8,
        hi_vel: u8,
        rr_group: u8,
        volume_db: i8,
        kit_piece: KitPieceId,
        mic_slices: [Option<SampleSlice>; MicChannel::COUNT],
    ) -> Self {
        Self {
            midi_note,
            lo_vel,
            hi_vel,
            rr_group,
            volume_db,
            kit_piece,
            mic_slices,
        }
    }

    /// Replace packed mic slices. WAV paths never live on this type.
    pub fn with_mic_slices(mut self, mic_slices: [Option<SampleSlice>; MicChannel::COUNT]) -> Self {
        self.mic_slices = mic_slices;
        self
    }
}

/// Root index serialized into the header of .scdpack
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct PackIndex {
    pub strikes: Vec<StrikeEntry>,
}

/// Live memory-mapped reader for zero-latency sample playback
pub struct ScdPack {
    _mmap: Mmap,
    index: PackIndex,
    note_index: NoteIndex,
    audio_data_ptr: *const f32,
    audio_data_len_samples: usize,
}

struct NoteIndex {
    by_note: [Vec<u16>; 128],
    max_rr: [u8; 128],
    kick_notes: [bool; 128],
    kit_pieces: [Option<KitPieceId>; 128],
}

impl NoteIndex {
    fn build(strikes: &[StrikeEntry]) -> Self {
        let mut by_note: [Vec<u16>; 128] = std::array::from_fn(|_| Vec::new());
        let mut max_rr = [0u8; 128];
        let mut kick_notes = [false; 128];
        let mut kit_pieces = [None; 128];
        for (i, strike) in strikes.iter().enumerate() {
            let note = strike.midi_note as usize;
            if note >= 128 {
                continue;
            }
            by_note[note].push(i as u16);
            max_rr[note] = max_rr[note].max(strike.rr_group);
            if kit_pieces[note].is_none() {
                kit_pieces[note] = Some(strike.kit_piece);
            }
            if strike.kit_piece == KitPieceId::Kick {
                kick_notes[note] = true;
            }
        }
        Self {
            by_note,
            max_rr,
            kick_notes,
            kit_pieces,
        }
    }
}

unsafe impl Send for ScdPack {}
unsafe impl Sync for ScdPack {}

impl ScdPack {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < 16 || &mmap[0..8] != SCD_MAGIC {
            return Err("invalid magic or corrupted scdpack".into());
        }

        let index_len = u64::from_le_bytes(mmap[8..16].try_into().unwrap()) as usize;
        let index_bytes = &mmap[16..16 + index_len];

        let archived = unsafe { rkyv::archived_root::<PackIndex>(index_bytes) };
        let index: PackIndex = archived.deserialize(&mut rkyv::Infallible)?;

        let audio_offset = 16 + index_len;
        let audio_bytes = &mmap[audio_offset..];
        let audio_data_ptr = audio_bytes.as_ptr() as *const f32;
        let audio_data_len_samples = audio_bytes.len() / std::mem::size_of::<f32>();
        let note_index = NoteIndex::build(&index.strikes);

        Ok(Self {
            _mmap: mmap,
            index,
            note_index,
            audio_data_ptr,
            audio_data_len_samples,
        })
    }

    /// Audio thread safe slice access (zero allocations, bounds checked)
    #[inline(always)]
    pub fn get_sample_slice(&self, slice: &SampleSlice) -> Option<&'static [f32]> {
        let start = (slice.offset_bytes / 4) as usize;
        let end = start + (slice.length_frames as usize * slice.channels as usize);
        if end <= self.audio_data_len_samples {
            unsafe {
                Some(std::slice::from_raw_parts(
                    self.audio_data_ptr.add(start),
                    end - start,
                ))
            }
        } else {
            None
        }
    }

    /// Match a MIDI note + velocity + RR group to strike entry
    pub fn find_strike(&self, note: u8, velocity: u8, rr_group: u8) -> Option<&StrikeEntry> {
        let note = note as usize;
        if note >= 128 {
            return None;
        }
        self.note_index.by_note[note].iter().find_map(|&idx| {
            let strike = &self.index.strikes[idx as usize];
            if velocity >= strike.lo_vel
                && velocity <= strike.hi_vel
                && (strike.rr_group == rr_group || strike.rr_group == 0)
            {
                Some(strike)
            } else {
                None
            }
        })
    }

    /// Highest `rr_group` packed for this MIDI note. `0` if the note is unused.
    pub fn max_rr(&self, note: u8) -> u8 {
        self.note_index
            .max_rr
            .get(note as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn is_kick_note(&self, note: u8) -> bool {
        self.note_index
            .kick_notes
            .get(note as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn kit_piece_for_note(&self, note: u8) -> Option<KitPieceId> {
        self.note_index
            .kit_pieces
            .get(note as usize)
            .copied()
            .flatten()
    }
}
