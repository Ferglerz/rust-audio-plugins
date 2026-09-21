use crate::params::ScdParams;
use crate::note_map::NoteMapBank;
use crate::vel_map::VelMapBank;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{ParamPtr, Params};
use scd_core::{KitPieceId, MicChannel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Factory mixer snapshots ported from `SCD/Scripts/BUILD_PRESETS.js`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactoryPreset {
    Init,
    ResetPitch,
    ResetPan,
    ResetPunch,
    ResetVolumes,
    DrummerPerspective,
}

impl FactoryPreset {
    pub const ALL: [Self; 6] = [
        Self::Init,
        Self::ResetPitch,
        Self::ResetPan,
        Self::ResetPunch,
        Self::ResetVolumes,
        Self::DrummerPerspective,
    ];

    pub const RESETS: [Self; 4] = [
        Self::ResetPitch,
        Self::ResetPan,
        Self::ResetPunch,
        Self::ResetVolumes,
    ];

    pub const SNAPSHOTS: [Self; 2] = [Self::Init, Self::DrummerPerspective];

    pub fn name(self) -> &'static str {
        match self {
            Self::Init => "Init",
            Self::ResetPitch => "Reset Pitch",
            Self::ResetPan => "Reset Pan",
            Self::ResetPunch => "Reset Punch",
            Self::ResetVolumes => "Reset Volumes",
            Self::DrummerPerspective => "Drummer's Perspective",
        }
    }
}

/// HISE "Pan: Drummer's Perspective" kit-piece pan percents.
pub const DRUMMER_PANS: [(KitPieceId, f32); 13] = [
    (KitPieceId::Kick, 0.0),
    (KitPieceId::Snare, 0.0),
    (KitPieceId::OpenSnare, 0.0),
    (KitPieceId::Hihat, -20.0),
    (KitPieceId::Tom1, -10.0),
    (KitPieceId::Tom2, 15.0),
    (KitPieceId::FloorTom, 30.0),
    (KitPieceId::Ride, 35.0),
    (KitPieceId::China, 40.0),
    (KitPieceId::Stack, -5.0),
    (KitPieceId::Splash, -5.0),
    (KitPieceId::LCrash, -10.0),
    (KitPieceId::RCrash, 10.0),
];

/// BUILD_DATA StartingVolume: Close + Front MS at 0 dB, others silent.
pub fn default_send_db(mic: MicChannel) -> f32 {
    match mic {
        MicChannel::Close | MicChannel::FrontMS => 0.0,
        _ => -72.0,
    }
}

pub const MAX_USER_PRESETS: usize = 24;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresetScope {
    pub pitch: bool,
    pub pan: bool,
    pub punch: bool,
    pub volumes: bool,
    pub sub_kick: bool,
    pub vel_maps: bool,
}

impl PresetScope {
    pub fn all() -> Self {
        Self {
            pitch: true,
            pan: true,
            punch: true,
            volumes: true,
            sub_kick: true,
            vel_maps: true,
        }
    }

    pub fn any(self) -> bool {
        self.pitch || self.pan || self.punch || self.volumes || self.sub_kick || self.vel_maps
    }

    pub fn names() -> [&'static str; 6] {
        ["Pitch", "Pan", "Punch", "Volumes", "Sub Kick", "Vel Maps"]
    }

    pub fn get(self, i: usize) -> bool {
        match i {
            0 => self.pitch,
            1 => self.pan,
            2 => self.punch,
            3 => self.volumes,
            4 => self.sub_kick,
            _ => self.vel_maps,
        }
    }

    pub fn toggle(&mut self, i: usize) {
        match i {
            0 => self.pitch = !self.pitch,
            1 => self.pan = !self.pan,
            2 => self.punch = !self.punch,
            3 => self.volumes = !self.volumes,
            4 => self.sub_kick = !self.sub_kick,
            _ => self.vel_maps = !self.vel_maps,
        }
    }

    pub fn matches_id(self, id: &str) -> bool {
        if id == "fader_lock" || id == "cc_num" || id == "cc_inv" {
            return false;
        }
        if id.starts_with("sub_") {
            return self.sub_kick;
        }
        if id.ends_with("_pitch") {
            return self.pitch;
        }
        if id.ends_with("_pan") {
            return self.pan;
        }
        if id.ends_with("_punch") {
            return self.punch;
        }
        if id == "master_gain" || id.ends_with("_gain") || id.contains("_send_") {
            return self.volumes;
        }
        false
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserPreset {
    pub name: String,
    pub scope: PresetScope,
    pub values: HashMap<String, f32>,
    pub vel_maps: Option<VelMapBank>,
    #[serde(default)]
    pub note_maps: Option<NoteMapBank>,
}

impl UserPreset {
    pub fn capture(name: String, scope: PresetScope, params: &ScdParams) -> Self {
        let mut values = HashMap::new();
        for (id, ptr, _) in params.param_map() {
            if scope.matches_id(&id) {
                values.insert(id, unsafe { ptr.unmodulated_normalized_value() });
            }
        }
        let vel_maps = scope.vel_maps.then(|| params.vel_maps.snapshot());
        let note_maps = scope.vel_maps.then(|| params.note_maps.snapshot());
        Self {
            name,
            scope,
            values,
            vel_maps,
            note_maps,
        }
    }

    pub fn apply_values(&self, params: &ScdParams) -> Vec<(ParamPtr, f32)> {
        params
            .param_map()
            .into_iter()
            .filter_map(|(id, ptr, _)| self.values.get(&id).copied().map(|v| (ptr, v)))
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct UserPresetBank {
    pub presets: Vec<UserPreset>,
}

pub struct UserPresetState {
    presets: Mutex<Vec<UserPreset>>,
}

impl UserPresetState {
    pub fn empty() -> Self {
        Self {
            presets: Mutex::new(Vec::new()),
        }
    }

    pub fn list(&self) -> Vec<UserPreset> {
        self.lock().clone()
    }

    pub fn get(&self, index: usize) -> Option<UserPreset> {
        self.lock().get(index).cloned()
    }

    pub fn push(&self, preset: UserPreset) -> Option<usize> {
        let mut presets = self.lock();
        if presets.len() >= MAX_USER_PRESETS {
            return None;
        }
        presets.push(preset);
        Some(presets.len() - 1)
    }

    pub fn clear(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<UserPreset>> {
        self.presets.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Default for UserPresetState {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a> PersistentField<'a, UserPresetBank> for Arc<UserPresetState> {
    fn set(&self, new_value: UserPresetBank) {
        *self.lock() = new_value.presets;
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&UserPresetBank) -> R,
    {
        f(&UserPresetBank {
            presets: self.lock().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_and_resets_partition_factory_list() {
        let mut all: Vec<_> = FactoryPreset::ALL.to_vec();
        all.sort_by_key(|p| *p as u8);
        let mut split: Vec<_> = FactoryPreset::RESETS
            .iter()
            .chain(FactoryPreset::SNAPSHOTS.iter())
            .copied()
            .collect();
        split.sort_by_key(|p| *p as u8);
        assert_eq!(all, split);
    }

    #[test]
    fn drummer_pans_cover_every_kit_piece() {
        let ids: Vec<_> = DRUMMER_PANS.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, KitPieceId::ALL.to_vec());
    }

    #[test]
    fn scope_matches_strip_param_ids() {
        let pitch = PresetScope {
            pitch: true,
            pan: false,
            punch: false,
            volumes: false,
            sub_kick: false,
            vel_maps: false,
        };
        assert!(pitch.matches_id("kick_pitch"));
        assert!(!pitch.matches_id("kick_pan"));
        assert!(!pitch.matches_id("kick_gain"));
        assert!(!pitch.matches_id("sub_vol"));

        let volumes = PresetScope {
            pitch: false,
            pan: false,
            punch: false,
            volumes: true,
            sub_kick: false,
            vel_maps: false,
        };
        assert!(volumes.matches_id("master_gain"));
        assert!(volumes.matches_id("snare_gain"));
        assert!(volumes.matches_id("kick_send_close"));
        assert!(!volumes.matches_id("kick_pitch"));
        assert!(!volumes.matches_id("cc_inv"));
    }

    #[test]
    fn user_preset_capture_respects_scope() {
        let params = ScdParams::default();
        let scope = PresetScope {
            pitch: true,
            pan: false,
            punch: false,
            volumes: false,
            sub_kick: false,
            vel_maps: false,
        };
        let preset = UserPreset::capture("Test".into(), scope, &params);
        assert!(preset.values.keys().any(|id| id.ends_with("_pitch")));
        assert!(preset.values.keys().all(|id| id.ends_with("_pitch")));
        assert!(preset.vel_maps.is_none());
    }
}
