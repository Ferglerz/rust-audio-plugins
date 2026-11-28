use crate::{
    engine::{Engine, HIGH, LATENCY, LOW},
    panel::Controls,
    ui,
};
use nih_plug::{midi::control_change, prelude::*};
use nih_plug_vizia::ViziaState;
use pleasant_ui::playable_keyboard::KeyboardLink;
use std::sync::Arc;

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Brilliance {
    Low,
    Medium,
    High,
}

#[derive(Params)]
pub struct Parameters {
    pub keyboard: Arc<KeyboardLink>,
    #[persist = "editor-state"]
    pub editor: Arc<ViziaState>,
    #[id = "volume"]
    pub volume: FloatParam,
    #[nested]
    pub tone: GrandTone,
}
impl std::ops::Deref for Parameters {
    type Target = GrandTone;
    fn deref(&self) -> &Self::Target {
        &self.tone
    }
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            keyboard: Arc::default(),
            editor: ViziaState::new_screen_sized("Electric Grand 80", || {
                (ui::SIZE.0 as u32, ui::SIZE.1 as u32)
            }),
            volume: pot("Volume", 1.),
            tone: Default::default(),
        }
    }
}
impl Parameters {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value() as f64,
            ..self.tone.controls()
        }
    }
}
fn pot(name: &'static str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0., max: 1. })
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
#[derive(Params)]
pub struct GrandTone {
    #[id = "bass"]
    pub bass: FloatParam,
    #[id = "middle"]
    pub middle: FloatParam,
    #[id = "treble"]
    pub treble: FloatParam,
    #[id = "frame_ring"]
    pub frame_ring: BoolParam,
    #[id = "brilliance"]
    pub brilliance: EnumParam<Brilliance>,
    #[id = "tremolo"]
    pub tremolo: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "depth"]
    pub depth: FloatParam,
}
impl Default for GrandTone {
    fn default() -> Self {
        Self {
            bass: pot("Bass", 0.5),
            middle: pot("Middle", 0.5),
            treble: pot("Treble", 0.5),
            frame_ring: BoolParam::new("Frame Ring", false),
            brilliance: EnumParam::new("Brilliance", Brilliance::Medium),
            tremolo: BoolParam::new("Tremolo", false),
            speed: pot("Tremolo speed", 0.75),
            depth: pot("Tremolo depth", 0.6),
        }
    }
}
impl GrandTone {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: 1.,
            bass: self.bass.value() as f64,
            middle: self.middle.value() as f64,
            treble: self.treble.value() as f64,
            frame_ring: self.frame_ring.value(),
            brilliance: self.brilliance.value().to_index() as u8,
            tremolo: self.tremolo.value(),
            speed: self.speed.value() as f64,
            depth: self.depth.value() as f64,
        }
    }
}

pub struct ElectricGrand {
    params: Arc<Parameters>,
    engine: Engine,
}
impl Default for ElectricGrand {
    fn default() -> Self {
        Self {
            params: Arc::default(),
            engine: Engine::new(44100.),
        }
    }
}

pub fn dispatch(engine: &mut Engine, e: NoteEvent<()>) {
    match e {
        NoteEvent::NoteOn { note, velocity, .. } => {
            engine.note_on(note, velocity);
        }
        NoteEvent::NoteOff { note, .. } if note > 127 => engine.release_all(),
        NoteEvent::NoteOff { note, .. } => engine.note_off(note),
        NoteEvent::Choke { note, .. } if note > 127 => {
            engine.choke_all();
        }
        NoteEvent::Choke { note, .. } => engine.choke(note),
        NoteEvent::MidiCC {
            cc: control_change::DAMPER_PEDAL,
            value,
            ..
        } => engine.sustain(value >= 0.5),
        NoteEvent::MidiCC {
            cc: control_change::RESET_ALL_CONTROLLERS,
            ..
        } => engine.sustain(false),
        NoteEvent::MidiCC {
            cc: control_change::ALL_SOUND_OFF,
            ..
        } => {
            engine.choke_all();
        }
        NoteEvent::MidiCC { cc: 123..=127, .. } => engine.release_all(),
        _ => {}
    }
}
impl Plugin for ElectricGrand {
    const NAME: &'static str = "Electric Grand 80";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "https://github.com/Ferglerz/rust-audio-plugins";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
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
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone())
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        ctx: &mut impl InitContext<Self>,
    ) -> bool {
        if !config.sample_rate.is_finite()
            || config.sample_rate < 22050.
            || config.sample_rate > 384000.
        {
            return false;
        }
        self.engine = Engine::new(config.sample_rate as f64);
        self.engine.set(self.params.controls());
        ctx.set_latency_samples(LATENCY);
        true
    }
    fn reset(&mut self) {
        self.engine.reset();
        self.params.keyboard.publish(|| 0);
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine.set(self.params.controls());
        self.params
            .keyboard
            .drain_auditions(LOW..=HIGH, |note, v| self.engine.audition(note, v));
        let frames = buffer.samples();
        let (left, right) = buffer.as_slice().split_at_mut(1);
        let mut pos = 0;
        loop {
            let event = ctx.next_event();
            let end = event
                .as_ref()
                .map_or(frames, |e| (e.timing() as usize).clamp(pos, frames));
            for (l, r) in left[0][pos..end].iter_mut().zip(&mut right[0][pos..end]) {
                [*l, *r] = self.engine.sample();
            }
            pos = end;
            if let Some(e) = event {
                if let NoteEvent::NoteOn { note, velocity, .. } = e {
                    if velocity > 0. && (LOW..=HIGH).contains(&note) {
                        self.params.keyboard.attack(note);
                    }
                }
                dispatch(&mut self.engine, e);
            } else {
                break;
            }
        }
        self.params.keyboard.publish(|| self.engine.pressed());
        if self.engine.sounding() {
            ProcessStatus::KeepAlive
        } else {
            ProcessStatus::Tail(16384)
        }
    }
}
impl ClapPlugin for ElectricGrand {
    const CLAP_ID: &'static str = "com.fergler.electricgrand80";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("An 88-key physically modeled electric grand piano");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer];
}
impl Vst3Plugin for ElectricGrand {
    const VST3_CLASS_ID: [u8; 16] = *b"FergElecGrand080";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
