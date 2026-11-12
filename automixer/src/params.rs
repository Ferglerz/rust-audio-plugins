use crate::network::{Network, Slot, CAPACITY};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
    Arc, Mutex,
};

pub fn float(name: &str, default: f32, min: f32, max: f32, unit: &'static str) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_unit(unit)
        .with_value_to_string(formatters::v2s_f32_rounded(1))
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicGroup {
    #[name = "Voice"]
    #[id = "voice"]
    Voice,
    #[name = "Other"]
    #[id = "other"]
    Other,
}

#[derive(Params)]
pub struct MicParams {
    #[id = "weight"]
    pub weight: FloatParam,
    #[id = "midich"]
    pub midi_channel: IntParam,
    #[id = "midicc"]
    pub midi_cc: IntParam,
    #[id = "group"]
    pub group: EnumParam<MicGroup>,
    #[persist = "midi-weight"]
    pub midi_value: Arc<AtomicI32>,
    #[persist = "mic-name"]
    pub name: Mutex<String>,
}

impl MicParams {
    fn new(index: usize) -> Self {
        let midi_value = Arc::new(AtomicI32::new(-1));
        let reset = midi_value.clone();
        Self {
            weight: float(
                &format!("Mic {} weight", index + 1),
                0.0,
                -12.0,
                12.0,
                " dB",
            )
            .with_callback(Arc::new(move |_| {
                reset.store(-1, Ordering::Relaxed);
            })),
            midi_channel: IntParam::new(
                "MIDI channel",
                1,
                IntRange::Linear { min: 0, max: 16 },
            )
            .with_value_to_string(Arc::new(|value| {
                if value == 0 {
                    "Off".into()
                } else {
                    value.to_string()
                }
            }))
            .with_string_to_value(Arc::new(|value| {
                let value = value.trim();
                if value.eq_ignore_ascii_case("off") {
                    Some(0)
                } else {
                    value.parse().ok()
                }
            })),
            midi_cc: IntParam::new(
                "Weight CC",
                20 + index as i32,
                IntRange::Linear { min: 0, max: 119 },
            ),
            group: EnumParam::new(&format!("Mic {} group", index + 1), MicGroup::Voice),
            midi_value,
            name: Mutex::new(format!("MIC {:02}", index + 1)),
        }
    }

    pub fn is_voice(&self) -> bool {
        self.group.value() == MicGroup::Voice
    }

    pub fn effective_weight(&self) -> f32 {
        match self.midi_value.load(Ordering::Relaxed) {
            value @ 0..=127 if self.midi_channel.value() != 0 => cc_weight(value as u8),
            _ => self.weight.value(),
        }
    }

    pub fn manual_edit(&self) {
        self.midi_value.store(-1, Ordering::Relaxed);
    }
}

/// MIDI 64 is exactly neutral; both endpoints reach the full priority range.
pub fn cc_weight(value: u8) -> f32 {
    if value <= 64 {
        (value as f32 - 64.0) * (12.0 / 64.0)
    } else {
        (value.min(127) as f32 - 64.0) * (12.0 / 63.0)
    }
}

#[derive(Params)]
pub struct SharedParams {
    #[id = "amount"]
    pub amount: FloatParam,
    #[id = "cap"]
    pub cap: FloatParam,
    #[id = "ocap"]
    pub other_cap: FloatParam,
    #[id = "voices"]
    pub voices: IntParam,
    #[id = "thresh"]
    pub threshold: FloatParam,
    #[id = "hold"]
    pub hold: FloatParam,
    #[id = "fade"]
    pub fade: FloatParam,
    #[id = "assist"]
    pub assist: FloatParam,
    #[nested(array, group = "Microphones")]
    pub mics: [MicParams; CAPACITY],
}

impl Default for SharedParams {
    fn default() -> Self {
        Self {
            amount: float("Automix amount", 100.0, 0.0, 100.0, " %"),
            cap: float("Voice max gain reduction", 18.0, 0.0, 60.0, " dB"),
            other_cap: float("Other max gain reduction", 18.0, 0.0, 60.0, " dB"),
            voices: IntParam::new(
                "Chorus above N (0 = off)",
                3,
                IntRange::Linear {
                    min: 0,
                    max: CAPACITY as i32,
                },
            ),
            threshold: float("Active microphone threshold", -40.0, -80.0, 0.0, " dBFS"),
            hold: float("Activity / chorus hold", 300.0, 50.0, 2000.0, " ms"),
            fade: float("Amount / chorus fade", 400.0, 20.0, 3000.0, " ms"),
            assist: float("Voice detection assist", 0.0, 0.0, 100.0, " %"),
            mics: std::array::from_fn(MicParams::new),
        }
    }
}

impl SharedParams {
    pub fn midi(&self, channel: u8, cc: u8, value: f32) {
        if channel >= 16 || cc >= 120 || !value.is_finite() {
            return;
        }
        let value = (value.clamp(0.0, 1.0) * 127.0).round() as i32;
        for mic in &self.mics {
            if mic.midi_channel.value() == channel as i32 + 1 && mic.midi_cc.value() == cc as i32 {
                mic.midi_value.store(value, Ordering::Relaxed);
            }
        }
    }
}

#[derive(Params)]
pub struct AutomixerParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[id = "mic"]
    pub mic: IntParam,
    // Low 7 bits hold the one-based mixer number. The upper bits hold the
    // one-based hardware input it was assigned against, or zero if unknown.
    #[persist = "auto-mic"]
    pub auto_mic: AtomicU32,
    slot: Option<Arc<Slot>>,
    #[id = "bypass"]
    pub bypass: BoolParam,
    #[persist = "strip-bypass"]
    strip_bypass: AtomicBool,
    #[nested(group = "Linked mixer")]
    pub shared: Arc<SharedParams>,
}

impl AutomixerParams {
    pub fn new(shared: Arc<SharedParams>, slot: Option<Arc<Slot>>, token: u64) -> Self {
        let callback_slot = slot.clone();
        Self {
            auto_mic: AtomicU32::new(0),
            slot,
            editor_state: ViziaState::new_screen_sized("Automixer", || (1120, 760)),
            mic: IntParam::new(
                "This microphone (0 = Auto)",
                0,
                IntRange::Linear {
                    min: 0,
                    max: CAPACITY as i32,
                },
            )
            .non_automatable()
            .with_value_to_string(Arc::new(|id| {
                if id == 0 {
                    "Auto".into()
                } else {
                    id.to_string()
                }
            }))
            .with_string_to_value(Arc::new(|value| {
                if value.trim().eq_ignore_ascii_case("auto") {
                    Some(0)
                } else {
                    value.trim().parse().ok()
                }
            }))
            .with_callback(Arc::new(move |id| {
                if let Some(slot) = &callback_slot {
                    if id > 0 && slot.owner.load(Ordering::Acquire) == token {
                        slot.withdraw();
                        slot.mic.store(id as usize - 1, Ordering::Release);
                        slot.assigned.store(true, Ordering::Release);
                    }
                }
            })),
            bypass: BoolParam::new("Bypass this microphone", false).make_bypass(),
            strip_bypass: AtomicBool::new(false),
            shared,
        }
    }

    pub fn strip_bypassed(&self) -> bool {
        self.strip_bypass.load(Ordering::Relaxed)
    }

    pub fn toggle_strip_bypass(&self) {
        self.strip_bypass.fetch_xor(true, Ordering::Relaxed);
    }

    pub fn bypassed(&self) -> bool {
        self.bypass.value() || self.strip_bypassed()
    }

    pub fn prepare_mic_assignment(&self, network: &Network, index: usize, token: u64) {
        let Some(slot) = &self.slot else { return };
        if self.mic.value() != 0 {
            return;
        }
        let preferred = slot
            .labels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .input
            .as_ref()
            .map(|input| input.first_channel);
        let saved = self.auto_mic.load(Ordering::Relaxed);
        let saved_mic = ((saved & 127) as usize).checked_sub(1);
        let saved_input = ((saved >> 7) as usize).checked_sub(1);
        let retained = if preferred.is_none() || preferred == saved_input {
            saved_mic
        } else {
            None
        };
        if let Some(assigned) = network.assign_automatic(index, token, retained, preferred) {
            let input = preferred
                .or(saved_input)
                .map_or(0, |input| input as u32 + 1);
            self.auto_mic
                .store((input << 7) | (assigned as u32 + 1), Ordering::Relaxed);
        }
    }

    pub fn mic_index(&self) -> usize {
        self.slot
            .as_ref()
            .map_or(0, |slot| slot.mic.load(Ordering::Acquire))
    }
}
