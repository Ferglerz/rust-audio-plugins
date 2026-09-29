//! Composure — a curve-based dynamic processor.
//!
//! Cross-platform CLAP/VST3 plugin built with [nih-plug](https://github.com/robbert-vdh/nih-plug)
//! and [nih-plug-vizia](https://github.com/robbert-vdh/nih-plug/tree/master/nih_plug_vizia).

use std::sync::Arc;

use nih_plug::prelude::*;

pub(crate) mod detector_eq;
pub mod dsp;
pub mod graph_store;
mod parameter_adapter;
pub mod params;
pub mod ui;

use dsp::ProcessingChain;
use params::ComposureParams;
use ui::UiDisplay;

/// Main plugin struct implementing the nih-plug `Plugin` trait.
pub struct Composure {
    params: Arc<ComposureParams>,
    chain: ProcessingChain,
    ui_display: Arc<UiDisplay>,
}

impl Default for Composure {
    fn default() -> Self {
        Self {
            params: Arc::new(ComposureParams::default()),
            chain: ProcessingChain::new(44100.0),
            ui_display: Arc::new(UiDisplay::default()),
        }
    }
}

impl Plugin for Composure {
    const NAME: &'static str = "Composure";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[new_nonzero_u32(2)],
            names: PortNames {
                aux_inputs: &["Sidechain"],
                ..PortNames::const_default()
            },
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type BackgroundTask = ();
    type SysExMessage = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.chain = ProcessingChain::new(buffer_config.sample_rate as f64);
        self.ui_display.sample_rate.store(
            buffer_config.sample_rate,
            std::sync::atomic::Ordering::Relaxed,
        );
        let graph_range_mode = self.params.graph_range_mode.value();
        self.params.graph_store.select_range(graph_range_mode);
        self.chain.force_update_settings(
            parameter_adapter::capture_block_state(&self.params),
            self.params.graph_store.load_for_range(graph_range_mode),
        );
        self.chain.snap_runtime_params();
        context.set_latency_samples(self.chain.latency_samples());
        true
    }

    fn reset(&mut self) {
        self.chain.reset();
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.ui_display.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let editor_open = self.params.editor_state.is_open();
        let graph_range_mode = self.params.graph_range_mode.value();
        self.params.graph_store.select_range(graph_range_mode);
        self.chain.update_settings(
            parameter_adapter::capture_block_state(&self.params),
            self.params.graph_store.load_for_range(graph_range_mode),
        );
        self.chain
            .set_filter_preview_drag(self.ui_display.filter_preview_active());
        if let Some(latency) = self.chain.latency_changed() {
            context.set_latency_samples(latency);
        }

        let use_sc = self.params.use_sidechain.value();
        let (sc_l, sc_r): (Option<&[f32]>, Option<&[f32]>) = if use_sc && !aux.inputs.is_empty() {
            let slices = aux.inputs[0].as_slice_immutable();
            match slices.len() {
                n if n >= 2 => (Some(slices[0]), Some(slices[1])),
                1 => (Some(slices[0]), Some(slices[0])),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        self.chain.begin_process_block();

        let channels = buffer.as_slice();
        if channels.is_empty() {
            return ProcessStatus::Normal;
        }
        self.chain.process_block(channels, sc_l, sc_r);

        if editor_open {
            let block_detector_max = self.chain.block_meter_detector_db() as f32;
            let block_gr_max = self.chain.block_meter_gr_db() as f32;
            self.ui_display.update_block(
                block_detector_max,
                block_gr_max,
                self.chain.block_meter_input_db() as f32,
            );
            let rate_activity = self.chain.block_meter_input_rate_activity() as f32;
            self.ui_display.program_activity.store(
                self.chain.meter_program_activity() as f32,
                std::sync::atomic::Ordering::Relaxed,
            );
            let previous = self
                .ui_display
                .input_rate_activity
                .load(std::sync::atomic::Ordering::Relaxed);
            let duration = channels[0].len() as f32
                / self
                    .ui_display
                    .sample_rate
                    .load(std::sync::atomic::Ordering::Relaxed);
            let faded = previous * (-duration / 0.12).exp();
            let activity = if rate_activity * previous < 0.0 || rate_activity.abs() >= faded.abs() {
                rate_activity
            } else {
                faded
            };
            self.ui_display
                .input_rate_activity
                .store(activity, std::sync::atomic::Ordering::Relaxed);
            self.ui_display.update_debug_telemetry(
                block_detector_max,
                block_gr_max,
                self.chain.meter_target_gr_db() as f32,
                self.chain.meter_lut_threshold_db() as f32,
                self.chain.latency_samples(),
            );
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Composure {
    const CLAP_ID: &'static str = "com.fergler.composure";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Curve-based dynamic processor");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Compressor,
    ];
}

impl Vst3Plugin for Composure {
    const VST3_CLASS_ID: [u8; 16] = *b"FergComposureRst";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(Composure);
nih_export_vst3!(Composure);
