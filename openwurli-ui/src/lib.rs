//! OpenWurli UI: an independent nih-plug host shell for upstream OpenWurli DSP.

use nih_plug::{midi::control_change, prelude::*};
use openwurli_dsp::{CircuitMode, WurliEngine};
use std::sync::Arc;

pub mod params;
pub mod ui;

use params::{CpuMode, OpenWurliUiParams};

// Use upstream v0.9's nominal physical volume-pot position. The wrapper's
// Volume remains a linear output control with its existing 50% default.
const ENGINE_VOLUME: f64 = 0.8;
const OUTPUT_VOLUME_REFERENCE: f32 = 0.5;
// Upstream's plugin-level alignment, applied after the physical signal chain.
const OUTPUT_ALIGNMENT_DB: f32 = 14.0;

pub struct OpenWurliUi {
    params: Arc<OpenWurliUiParams>,
    engine: WurliEngine,
    finish: FastFinish,
}

/// Optional post-engine extra sag, shared by both circuit modes.
/// The Heavy circuit also includes its native physical rail behavior.
struct FastFinish {
    sag_envelope: f32,
    sag_attack: f32,
    sag_release: f32,
}

impl FastFinish {
    fn new(sample_rate: f32) -> Self {
        let mut finish = Self {
            sag_envelope: 0.0,
            sag_attack: 0.0,
            sag_release: 0.0,
        };
        finish.set_sample_rate(sample_rate);
        finish
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let sample_rate = sample_rate.max(1.0);
        self.sag_attack = 1.0 - (-1.0 / (0.008 * sample_rate)).exp();
        self.sag_release = 1.0 - (-1.0 / (0.150 * sample_rate)).exp();
    }

    fn reset(&mut self) {
        self.sag_envelope = 0.0;
    }

    fn process(&mut self, output: &mut [f32], sag_enabled: bool) {
        for sample in output {
            if sag_enabled {
                let magnitude = sample.abs();
                let coefficient = if magnitude > self.sag_envelope {
                    self.sag_attack
                } else {
                    self.sag_release
                };
                self.sag_envelope += coefficient * (magnitude - self.sag_envelope);
                *sample /= 1.0 + 12.0 * self.sag_envelope;
            }
        }
        if !sag_enabled {
            self.sag_envelope = 0.0;
        }
    }
}

impl Default for OpenWurliUi {
    fn default() -> Self {
        Self {
            params: Arc::new(OpenWurliUiParams::default()),
            engine: WurliEngine::new(44_100.0),
            finish: FastFinish::new(44_100.0),
        }
    }
}

impl OpenWurliUi {
    fn sync_params(&mut self) {
        self.engine
            .set_circuit_mode(match self.params.cpu_mode.value() {
                CpuMode::Fast => CircuitMode::Fast,
                CpuMode::Heavy => CircuitMode::Heavy,
            });
        self.engine.set_volume(ENGINE_VOLUME);
        self.engine
            .set_tremolo_depth(self.params.tremolo_depth.value() as f64);
        self.engine
            .set_speaker_character(self.params.speaker_character.value() as f64);
        // The legacy mlp parameter is hidden and inert, including old saved false values.
        self.engine.set_mlp_enabled(true);
        // Hiss is removed, including native preamp noise in both circuit modes.
        self.engine.set_noise_enabled(false);
        self.engine
            .set_reed_decay(self.params.reed_decay.value() as f64);
        self.engine
            .set_hammer_hardness(self.params.hammer_hardness.value() as f64);
        self.engine
            .set_pickup_drive(self.params.pickup_drive.value() as f64);
        self.engine
            .set_tremolo_response(self.params.tremolo_response.value() as f64);
    }

    fn handle_event(&mut self, event: &NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } if *velocity == 0.0 => {
                self.engine.note_off_extended(*note)
            }
            NoteEvent::NoteOn { note, velocity, .. }
                if self.params.extended_notes.value()
                    || (openwurli_dsp::tables::MIDI_LO..=openwurli_dsp::tables::MIDI_HI)
                        .contains(note) =>
            {
                self.engine.note_on_extended(*note, *velocity)
            }
            // Always release the exact pitch, even if Extended Notes was
            // disabled while it was held. Ignored pitches cannot release an end key.
            NoteEvent::NoteOff { note, .. } => self.engine.note_off_extended(*note),
            NoteEvent::MidiCC { cc, value, .. } if *cc == control_change::DAMPER_PEDAL => {
                self.engine.set_sustain(*value >= 0.5)
            }
            _ => {}
        }
    }

    fn render_block(
        &mut self,
        output: &mut [f32],
        mut next_event: impl FnMut() -> Option<NoteEvent<()>>,
    ) {
        let num_samples = output.len();
        let mut block_start = 0;
        let mut event = next_event();
        while block_start < num_samples {
            while let Some(ref current) = event {
                if current.timing() as usize > block_start {
                    break;
                }
                self.handle_event(current);
                event = next_event();
            }
            let block_end = event.as_ref().map_or(num_samples, |current| {
                (current.timing() as usize).min(num_samples)
            });
            if block_end > block_start {
                self.engine.render(&mut output[block_start..block_end]);
            }
            block_start = block_end;
        }
        while let Some(current) = event {
            self.handle_event(&current);
            event = next_event();
        }
    }

    fn finish_output(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample *= util::db_to_gain(OUTPUT_ALIGNMENT_DB);
        }
        self.finish.process(output, self.params.rail_sag.value());
        for sample in output {
            *sample *= self.params.volume.smoothed.next() / OUTPUT_VOLUME_REFERENCE;
        }
    }
}

impl Plugin for OpenWurliUi {
    const NAME: &'static str = "OpenWurli UI";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "https://github.com/Ferglerz/rust-audio-plugins";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn filter_state(state: &mut PluginState) {
        // NIH-plug restores plain floats without clamping them to the range.
        if let Some(nih_plug::wrapper::state::ParamValue::F32(drive)) =
            state.params.get_mut("pickup_drive")
        {
            *drive = drive.clamp(0.5, params::PICKUP_DRIVE_MAX);
        }
        // Missing parameters retain their current value in NIH-plug. Explicitly
        // restore Fast selection even when loading into a Heavy instance.
        state
            .params
            .entry("extended_notes".into())
            .or_insert(nih_plug::wrapper::state::ParamValue::Bool(false));
        state
            .params
            .entry("cpu_mode".into())
            .or_insert_with(|| nih_plug::wrapper::state::ParamValue::String("fast".into()));
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Restore the selected circuit before preparation so a saved Heavy
        // session starts in Heavy without an initial transition from Fast.
        self.engine
            .set_circuit_mode(match self.params.cpu_mode.value() {
                CpuMode::Fast => CircuitMode::Fast,
                CpuMode::Heavy => CircuitMode::Heavy,
            });
        self.engine
            .set_sample_rate(buffer_config.sample_rate as f64);
        self.finish.set_sample_rate(buffer_config.sample_rate);
        self.engine
            .ensure_buffer_capacity(buffer_config.max_buffer_size as usize);
        self.sync_params();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.finish.reset();
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.sync_params();
        let num_samples = buffer.samples();
        if num_samples == 0 {
            return ProcessStatus::Normal;
        }

        // OpenWurli renders mono. Split at MIDI events, then duplicate the
        // finished signal to the second output channel.
        let channels = buffer.as_slice();
        self.render_block(&mut channels[0][..num_samples], || context.next_event());
        self.finish_output(&mut channels[0][..num_samples]);
        let (left, rest) = channels.split_at_mut(1);
        for channel in rest {
            channel[..num_samples].copy_from_slice(&left[0][..num_samples]);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for OpenWurliUi {
    const CLAP_ID: &'static str = "com.fergler.openwurli-ui";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("OpenWurli electric piano with Pleasant UI");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Custom("electric-piano"),
    ];
}

impl Vst3Plugin for OpenWurliUi {
    const VST3_CLASS_ID: [u8; 16] = *b"FergOpenWurliUI_";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(OpenWurliUi);
nih_export_vst3!(OpenWurliUi);

#[cfg(test)]
mod tests;
