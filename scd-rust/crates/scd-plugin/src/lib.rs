use nih_plug::prelude::*;
use std::sync::Arc;

pub mod dsp;
pub mod note_map;
pub mod params;
pub mod presets;
pub mod ui;
pub mod vel_map;

use dsp::{HiHatTracker, KickSine, VoicePool};
pub use params::{ScdParams, VuMeters};
use scd_core::{KitPieceId, MicChannel, ScdPack};

pub struct ScdPlugin {
    params: Arc<ScdParams>,
    voice_pool: VoicePool,
    hihat_tracker: HiHatTracker,
    kick_sine: KickSine,
    sample_rate: f32,
    rr_counter: [[u8; 128]; KitPieceId::COUNT],
}

impl Default for ScdPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(ScdParams::default()),
            voice_pool: VoicePool::new(),
            hihat_tracker: HiHatTracker::new(),
            kick_sine: KickSine::default(),
            sample_rate: 44100.0,
            rr_counter: [[0; 128]; KitPieceId::COUNT],
        }
    }
}

impl Plugin for ScdPlugin {
    const NAME: &'static str = "SoundChef Drums";
    const VENDOR: &'static str = "SoundChef";
    const URL: &'static str = "https://soundchef.com";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Stereo main output + multi-out aux outputs for mic channels
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_output_channels: NonZeroU32::new(2),
            aux_output_ports: &[
                new_nonzero_u32(2), // Close
                new_nonzero_u32(2), // XY
                new_nonzero_u32(2), // Mono
                new_nonzero_u32(2), // Wide
                new_nonzero_u32(2), // Front MS
                new_nonzero_u32(2), // Room
            ],
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type BackgroundTask = ();
    type SysExMessage = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.kick_sine.reset(self.sample_rate);
        self.voice_pool.reset();

        // Attempt to load .scdpack from standard location or next to plugin
        if self.voice_pool.pack().is_none() {
            let possible_paths = [
                "/Volumes/Storage/SoundChefDrums.scdpack",
                "SoundChefDrums.scdpack",
                "/Library/Application Support/SoundChef/SoundChefDrums.scdpack",
            ];
            for p in &possible_paths {
                if let Ok(pack) = ScdPack::open(p) {
                    self.voice_pool.set_pack(pack);
                    break;
                }
            }
        }

        true
    }

    fn reset(&mut self) {
        self.voice_pool.reset();
        self.kick_sine.reset(self.sample_rate);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mix_gains = self.params.mix_gain_table();
        let sub_vol = self.params.sub_kick.vol.value();
        let master_gain = self.params.master_gain.value();
        let mut kit_peaks = [0.0f32; KitPieceId::COUNT];
        let mut mic_peaks = [[0.0f32; 2]; MicChannel::COUNT];
        let mut master_peaks = [0.0f32; 2];

        let mut next_event = context.next_event();
        let num_samples = buffer.samples();
        let main_slices = buffer.as_slice();

        for sample_id in 0..num_samples {
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }
                self.handle_midi(event);
                next_event = context.next_event();
            }

            let mut mic_accum = [[0.0f32; 2]; MicChannel::COUNT];
            self.voice_pool
                .process_sample(&mut mic_accum, &mix_gains, &mut kit_peaks);

            let sub_sample = self.kick_sine.process_sample(sub_vol);

            for (mic_idx, aux_buf) in aux.outputs.iter_mut().enumerate() {
                if mic_idx >= MicChannel::COUNT {
                    break;
                }
                let pair = mic_accum[mic_idx];
                let slices = aux_buf.as_slice();
                if !slices.is_empty() {
                    slices[0][sample_id] = pair[0];
                }
                if slices.len() > 1 {
                    slices[1][sample_id] = pair[1];
                }
            }

            for (mic_idx, pair) in mic_accum.iter().enumerate() {
                mic_peaks[mic_idx][0] = mic_peaks[mic_idx][0].max(pair[0].abs());
                mic_peaks[mic_idx][1] = mic_peaks[mic_idx][1].max(pair[1].abs());
            }

            let [mut left, mut right] = dsp::simd::sum_stereo_pairs(&mic_accum);
            left += sub_sample;
            right += sub_sample;
            left *= master_gain;
            right *= master_gain;

            master_peaks[0] = master_peaks[0].max(left.abs());
            master_peaks[1] = master_peaks[1].max(right.abs());

            if !main_slices.is_empty() {
                main_slices[0][sample_id] = left;
            }
            if main_slices.len() > 1 {
                main_slices[1][sample_id] = right;
            }
        }

        for (i, pair) in mic_peaks.iter().enumerate() {
            self.params.vu.update_mic(i, pair[0], pair[1]);
        }
        self.params
            .vu
            .update_master(master_peaks[0], master_peaks[1]);
        for (i, peak) in kit_peaks.iter().enumerate() {
            if *peak > 0.0 {
                self.params.vu.update_kit(i, *peak);
            }
        }

        ProcessStatus::Normal
    }
}

impl ScdPlugin {
    fn resolve_note(&self, note: u8) -> note_map::ResolvedNote {
        let mapped = self.params.note_maps.resolve(note);
        if mapped.explicit || mapped.silenced {
            mapped
        } else {
            self.params
                .note_maps
                .resolve(self.hihat_tracker.translate_note(note))
        }
    }

    fn trigger_note(
        &mut self,
        note: u8,
        velocity: f32,
        vel_u8: u8,
        piece: Option<KitPieceId>,
        fallback_art: usize,
    ) {
        let Some(pack) = self.voice_pool.pack() else {
            return;
        };
        let art = piece
            .and_then(|piece| {
                scd_core::kit::stonehouse()
                    .arts(piece)
                    .iter()
                    .position(|art| art.note1 == note || art.note2 == Some(note))
            })
            .unwrap_or(fallback_art);
        let depth = pack.max_rr_for_piece(note, piece).max(1);
        let counter =
            &mut self.rr_counter[piece.unwrap_or(KitPieceId::Kick) as usize][note as usize];
        let rr = (*counter % depth) + 1;
        *counter = rr;
        let lookup = piece
            .map(|p| self.params.vel_maps.lookup(p, art, vel_u8))
            .unwrap_or(vel_u8);
        if let Some(strike) = pack.find_strike_for_piece(note, lookup, rr, piece).cloned() {
            let strip = self.params.get_strip(strike.kit_piece);
            if let Some(slot) = self.params.midi_velocities[strike.kit_piece as usize].get(art) {
                slot.store(vel_u8, std::sync::atomic::Ordering::Relaxed);
            }
            self.voice_pool.trigger_strike(
                &strike,
                velocity,
                strip.pitch.value(),
                strip.punch.value(),
                strip.pan.value(),
                self.sample_rate,
            );
        }
    }

    fn handle_midi(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } => {
                self.params.note_maps.offer_learn(note);
                let vel_u8 = (velocity * 127.0).clamp(1.0, 127.0) as u8;

                let resolved = self.resolve_note(note);
                let pack_note = if resolved.silenced {
                    return;
                } else {
                    resolved.pack_note
                };

                if self
                    .voice_pool
                    .pack()
                    .map(|pack| pack.is_kick_note(pack_note))
                    .unwrap_or(pack_note == 35 || pack_note == 36)
                {
                    self.kick_sine.trigger(
                        self.params.sub_kick.length.value(),
                        self.params.sub_kick.dive.value(),
                        self.params.sub_kick.speed.value(),
                        self.params.sub_kick.offset.value(),
                    );
                }

                if HiHatTracker::is_choke_trigger(pack_note) {
                    self.voice_pool.choke_hihat(150.0, self.sample_rate);
                }

                let piece = resolved.kit_piece.or_else(|| {
                    self.voice_pool
                        .pack()
                        .and_then(|pack| pack.kit_piece_for_note(pack_note))
                });
                if matches!(piece, Some(KitPieceId::Snare | KitPieceId::OpenSnare)) {
                    let selected = if self.params.snare_wires_off.value() {
                        KitPieceId::OpenSnare
                    } else {
                        KitPieceId::Snare
                    };
                    for candidate in [KitPieceId::Snare, KitPieceId::OpenSnare] {
                        if self.params.snare_mixed.value() || candidate == selected {
                            self.trigger_note(
                                pack_note,
                                velocity,
                                vel_u8,
                                Some(candidate),
                                resolved.art,
                            );
                        }
                    }
                } else {
                    self.trigger_note(pack_note, velocity, vel_u8, piece, resolved.art);
                }
            }
            NoteEvent::MidiCC { cc, value, .. } if cc as i32 == self.params.cc_number.value() => {
                let raw = (value * 127.0).round().clamp(0.0, 127.0) as u8;
                let processed = dsp::HiHatTracker::process_cc(raw, self.params.invert_cc.value());
                self.hihat_tracker.set_cc(processed);
                self.params
                    .cc_display
                    .store(processed, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

impl ClapPlugin for ScdPlugin {
    const CLAP_ID: &'static str = "com.soundchef.drums";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("SoundChef Drums sampler plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Drum,
        ClapFeature::Sampler,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for ScdPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"SoundChefDrumsPl";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Drum,
        Vst3SubCategory::Sampler,
    ];
}

nih_export_clap!(ScdPlugin);
nih_export_vst3!(ScdPlugin);

#[cfg(test)]
mod note_resolution_tests {
    use super::*;

    #[test]
    fn explicit_hihat_trigger_mappings_take_precedence() {
        let mut plugin = ScdPlugin::default();
        for note in [42, 46, 22, 26] {
            plugin.params.note_maps.reset_all();
            plugin
                .params
                .note_maps
                .set_n1(KitPieceId::Kick, 1, Some(note));
            for cc in [0, 50, 127] {
                plugin.hihat_tracker.set_cc(cc);
                let resolved = plugin.resolve_note(note);
                assert_eq!(resolved.pack_note, 36, "note {note}, CC {cc}");
                assert_eq!(resolved.kit_piece, Some(KitPieceId::Kick));
                assert!(resolved.explicit);
                assert!(!resolved.silenced);
            }
        }
    }

    #[test]
    fn factory_hihat_triggers_still_follow_cc() {
        let mut plugin = ScdPlugin::default();
        for (cc, tip, shoulder) in [(0, 66, 60), (50, 68, 62), (127, 70, 64)] {
            plugin.hihat_tracker.set_cc(cc);
            for (note, expected) in [(42, tip), (46, tip), (22, shoulder), (26, shoulder)] {
                let resolved = plugin.resolve_note(note);
                assert_eq!(resolved.pack_note, expected, "note {note}, CC {cc}");
                assert!(!resolved.silenced);
            }
        }
    }

    #[test]
    fn explicit_factory_hihat_note_bypasses_cc_translation() {
        let mut plugin = ScdPlugin::default();
        plugin
            .params
            .note_maps
            .set_n1(KitPieceId::Hihat, 7, Some(42));
        plugin.hihat_tracker.set_cc(127);
        assert_eq!(plugin.resolve_note(42).pack_note, 42);
        plugin
            .params
            .note_maps
            .reset_slot(KitPieceId::Hihat, 7, false);
        assert_eq!(plugin.resolve_note(42).pack_note, 70);
    }

    #[test]
    fn moving_factory_hihat_trigger_keeps_old_note_silent() {
        let plugin = ScdPlugin::default();
        plugin
            .params
            .note_maps
            .set_n1(KitPieceId::Hihat, 7, Some(5));
        assert!(plugin.resolve_note(42).silenced);
        let mapped = plugin.resolve_note(5);
        assert!(!mapped.silenced);
        assert_eq!(mapped.pack_note, 42);
    }
}

#[cfg(test)]
mod snare_tests {
    use super::*;
    use scd_core::{PackIndex, SampleSlice, StrikeEntry, SCD_MAGIC};
    use std::io::Write;

    #[test]
    fn snare_modes_select_and_layer_separate_voices() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snare.scdpack");
        let strikes = [KitPieceId::Snare, KitPieceId::OpenSnare]
            .into_iter()
            .map(|piece| {
                StrikeEntry::new(
                    38,
                    1,
                    127,
                    1,
                    0,
                    piece,
                    [
                        Some(SampleSlice {
                            offset_bytes: 0,
                            length_frames: 16,
                            sample_rate: 44100,
                            channels: 1,
                        }),
                        None,
                        None,
                        None,
                        None,
                        None,
                    ],
                )
            })
            .collect();
        let bytes = rkyv::to_bytes::<_, 256>(&PackIndex { strikes }).unwrap();
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(SCD_MAGIC).unwrap();
        file.write_all(&(bytes.len() as u64).to_le_bytes()).unwrap();
        file.write_all(&bytes).unwrap();
        for _ in 0..16 {
            file.write_all(&8192i16.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();
        for (off, mixed, expected) in [
            (false, false, [true, false]),
            (true, false, [false, true]),
            (false, true, [true, true]),
            (true, true, [true, true]),
        ] {
            let mut plugin = ScdPlugin::default();
            let params = Arc::get_mut(&mut plugin.params).unwrap();
            params.snare_wires_off = BoolParam::new("Wires Off", off);
            params.snare_mixed = BoolParam::new("Mixed", mixed);
            plugin.voice_pool.set_pack(ScdPack::open(&path).unwrap());
            plugin.handle_midi(NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 38,
                velocity: 0.5,
            });
            let mut peaks = [0.0; KitPieceId::COUNT];
            plugin.voice_pool.process_sample(
                &mut [[0.0; 2]; MicChannel::COUNT],
                &plugin.params.mix_gain_table(),
                &mut peaks,
            );
            for (piece, expected) in [KitPieceId::Snare, KitPieceId::OpenSnare]
                .into_iter()
                .zip(expected)
            {
                let art = scd_core::kit::stonehouse()
                    .arts(piece)
                    .iter()
                    .position(|art| art.note1 == 38)
                    .unwrap();
                assert_eq!(
                    plugin.params.midi_velocities[piece as usize][art]
                        .load(std::sync::atomic::Ordering::Relaxed)
                        > 0,
                    expected,
                    "off={off}, mixed={mixed}, piece={piece:?}"
                );
                assert_eq!(plugin.rr_counter[piece as usize][38] > 0, expected);
                assert_eq!(
                    peaks[piece as usize] > 0.0,
                    expected,
                    "audible voice for {piece:?}"
                );
            }
        }
    }
}
