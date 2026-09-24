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
    rr_counter: [u8; 128],
}

impl Default for ScdPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(ScdParams::default()),
            voice_pool: VoicePool::new(),
            hihat_tracker: HiHatTracker::new(),
            kick_sine: KickSine::default(),
            sample_rate: 44100.0,
            rr_counter: [0; 128],
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
    fn handle_midi(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } => {
                self.params.note_maps.offer_learn(note);
                let vel_u8 = (velocity * 127.0).clamp(1.0, 127.0) as u8;

                let play_note = self.hihat_tracker.translate_note(note);
                let resolved = self.params.note_maps.resolve(play_note);
                let pack_note = if resolved.silenced {
                    return;
                } else {
                    resolved.pack_note
                };

                if let Some(piece) = resolved.kit_piece.or_else(|| {
                    self.voice_pool
                        .pack()
                        .and_then(|pack| pack.kit_piece_for_note(pack_note))
                }) {
                    if let Some(slot) =
                        self.params.midi_velocities[piece as usize].get(resolved.art)
                    {
                        slot.store(vel_u8, std::sync::atomic::Ordering::Relaxed);
                    }
                }

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

                let depth = self
                    .voice_pool
                    .pack()
                    .map(|pack| pack.max_rr(pack_note))
                    .unwrap_or(3)
                    .max(1);
                let rr = (self.rr_counter[pack_note as usize] % depth) + 1;
                self.rr_counter[pack_note as usize] = rr;

                if let Some(pack) = self.voice_pool.pack() {
                    let lookup = resolved
                        .kit_piece
                        .or_else(|| pack.kit_piece_for_note(pack_note))
                        .map(|kit_piece| {
                            self.params.vel_maps.lookup(kit_piece, resolved.art, vel_u8)
                        })
                        .unwrap_or(vel_u8);
                    if let Some(strike) = pack.find_strike(pack_note, lookup, rr).cloned() {
                        let strip = self.params.get_strip(strike.kit_piece);
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
