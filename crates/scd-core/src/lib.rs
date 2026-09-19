use rkyv::{Archive, Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use memmap2::Mmap;

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
    pub const ALL: [MicChannel; 6] = [
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
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const ALL: [KitPieceId; 13] = [
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
    pub mic_slices: [Option<SampleSlice>; 6],
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
    audio_data_ptr: *const f32,
    audio_data_len_samples: usize,
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

        Ok(Self {
            _mmap: mmap,
            index,
            audio_data_ptr,
            audio_data_len_samples,
        })
    }

    pub fn index(&self) -> &PackIndex {
        &self.index
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
        self.index.strikes.iter().find(|s| {
            s.midi_note == note
                && velocity >= s.lo_vel
                && velocity <= s.hi_vel
                && (s.rr_group == rr_group || s.rr_group == 0)
        })
    }
}
