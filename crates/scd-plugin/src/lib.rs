use nih_plug::prelude::*;
use std::sync::Arc;

pub mod dsp;
pub mod params;
pub mod ui;

use dsp::{HiHatTracker, KickSine, VoicePool};
pub use params::ScdParams;
use scd_core::ScdPack;

pub struct ScdPlugin {
    params: Arc<ScdParams>,
    pack: Option<ScdPack>,
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
            pack: None,
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
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

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
        if self.pack.is_none() {
            let possible_paths = [
                "SoundChefDrums.scdpack",
                "/Library/Application Support/SoundChef/SoundChefDrums.scdpack",
            ];
            for p in &possible_paths {
                if let Ok(pack) = ScdPack::open(p) {
                    self.pack = Some(pack);
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
        let mut next_event = context.next_event();
        let num_samples = buffer.samples();

        for sample_id in 0..num_samples {
            // Process sample-accurate MIDI events
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        let vel_u8 = (velocity * 127.0).clamp(1.0, 127.0) as u8;

                        // Check Kick triggers for Sub Kick
                        if note == 35 || note == 36 {
                            self.kick_sine.trigger(
                                velocity,
                                self.params.sub_kick.length.value(),
                                self.params.sub_kick.dive.value(),
                                self.params.sub_kick.speed.value(),
                                self.params.sub_kick.offset.value(),
                            );
                        }

                        // Hihat note translation & choke
                        let play_note = self.hihat_tracker.translate_note(note);
                        if HiHatTracker::is_choke_trigger(play_note) {
                            self.voice_pool.choke_hihat(150.0, self.sample_rate);
                        }

                        // Round robin cycling
                        let rr = (self.rr_counter[play_note as usize] % 3) + 1;
                        self.rr_counter[play_note as usize] = rr;

                        // Look up strike in memory-mapped pack
                        if let Some(ref pack) = self.pack {
                            if let Some(strike) = pack.find_strike(play_note, vel_u8, rr) {
                                let strip = self.params.get_strip(strike.kit_piece);
                                self.voice_pool.trigger_strike(
                                    strike,
                                    velocity,
                                    strip.pitch.value(),
                                    strip.punch.value(),
                                    self.sample_rate,
                                );
                            }
                        }
                    }
                    NoteEvent::MidiCC { cc, value, .. } => {
                        if cc == 4 {
                            let cc_val = (value * 127.0) as u8;
                            self.hihat_tracker.set_cc4(cc_val);
                        }
                    }
                    _ => {}
                }
                next_event = context.next_event();
            }

            // Render active voices for all 6 mic channels
            let mut mic_accum = [0.0f32; 6];
            if let Some(ref pack) = self.pack {
                self.voice_pool.process_sample(pack, &mut mic_accum);
            }

            // Sub kick sample
            let sub_sample = self.kick_sine.process_sample(self.params.sub_kick.vol.value());

            // Multi-out stem write (if enabled in DAW)
            for (mic_idx, aux_buf) in aux.outputs.iter_mut().enumerate() {
                if mic_idx < 6 {
                    let val = mic_accum[mic_idx];
                    for ch in 0..aux_buf.channels() {
                        aux_buf.as_slice()[ch][sample_id] = val;
                    }
                }
            }

            // Master stereo mix summation
            let mut left = 0.0f32;
            let mut right = 0.0f32;

            for mic_sample in mic_accum.iter() {
                left += *mic_sample;
                right += *mic_sample;
            }

            // Add sub-kick to center
            left += sub_sample;
            right += sub_sample;

            let master_gain = self.params.master_gain.value();
            left *= master_gain;
            right *= master_gain;

            let main_slices = buffer.as_slice();
            if main_slices.len() > 0 {
                main_slices[0][sample_id] = left;
            }
            if main_slices.len() > 1 {
                main_slices[1][sample_id] = right;
            }
        }

        ProcessStatus::Normal
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
