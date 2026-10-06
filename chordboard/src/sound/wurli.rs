//! OpenWurli electric piano engine wrapper for Chordboard.
//!
//! Wraps `openwurli_dsp::WurliEngine` with fast/heavy circuit modeling,
//! modal reed synthesis, and dynamic power-rail sag simulation.

use super::SoundEngine;
use openwurli_dsp::{CircuitMode, WurliEngine};

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
                *sample /= 1.0 + 1.2 * self.sag_envelope;
            }
        }
        if !sag_enabled {
            self.sag_envelope = 0.0;
        }
    }
}

pub struct WurliEngineWrapper {
    engine: Box<WurliEngine>,
    finish: FastFinish,
    mono_buf: Vec<f32>,

    // Parameters
    pub volume: f32,
    pub tremolo_depth: f32,
    pub tremolo_response: f32,
    pub speaker_character: f32,
    pub reed_decay: f32,
    pub hammer_hardness: f32,
    pub pickup_drive: f32,
    pub cpu_heavy: bool,
    pub rail_sag: bool,
}

impl WurliEngineWrapper {
    pub fn new(sample_rate: f32) -> Self {
        let mut engine = Box::new(WurliEngine::new(sample_rate.max(1.0) as f64));
        engine.set_circuit_mode(CircuitMode::Fast);
        engine.set_mlp_enabled(true);
        engine.set_noise_enabled(false);

        let mut w = Self {
            engine,
            finish: FastFinish::new(sample_rate),
            mono_buf: vec![0.0; 2048],
            volume: 0.7,
            tremolo_depth: 0.4,
            tremolo_response: 1.0,
            speaker_character: 0.2,
            reed_decay: 1.0,
            hammer_hardness: 1.0,
            pickup_drive: 1.0,
            cpu_heavy: false,
            rail_sag: false,
        };
        w.sync_params();
        w
    }

    fn sync_params(&mut self) {
        self.engine.set_circuit_mode(if self.cpu_heavy {
            CircuitMode::Heavy
        } else {
            CircuitMode::Fast
        });
        self.engine.set_volume(self.volume as f64);
        self.engine.set_tremolo_depth(self.tremolo_depth as f64);
        self.engine.set_speaker_character(self.speaker_character as f64);
        self.engine.set_reed_decay(self.reed_decay as f64);
        self.engine.set_hammer_hardness(self.hammer_hardness as f64);
        self.engine.set_pickup_drive(self.pickup_drive as f64);
        self.engine.set_tremolo_response(self.tremolo_response as f64);
    }
}

impl SoundEngine for WurliEngineWrapper {
    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.engine.set_sample_rate(sample_rate as f64);
        self.finish.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.finish.reset();
    }

    fn note_on(&mut self, _channel: u8, note: u8, velocity: f32) {
        self.engine.note_on_extended(note, velocity);
    }

    fn note_off(&mut self, _channel: u8, note: u8, _velocity: f32) {
        self.engine.note_off_extended(note);
    }

    fn bend(&mut self, _channel: u8, _value: f32) {
        // Physical Wurlitzer reeds are fixed metal tines; bend is ignored.
    }

    fn pressure(&mut self, _channel: u8, _value: f32) {}

    fn cc(&mut self, _channel: u8, cc: u8, value: f32) {
        if cc == 64 {
            self.engine.set_sustain(value >= 0.5);
        }
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.volume = value.clamp(0.0, 1.0),
            1 => self.tremolo_depth = value.clamp(0.0, 1.0),
            2 => self.tremolo_response = value.clamp(0.1, 4.0),
            3 => self.speaker_character = value.clamp(0.0, 1.0),
            4 => self.reed_decay = (0.5 + 4.5 * value).clamp(0.5, 10.0),
            5 => self.hammer_hardness = (0.5 + 2.0 * value).clamp(0.5, 3.0),
            6 => self.pickup_drive = (0.5 + 2.5 * value).clamp(0.5, 3.0),
            7 => self.cpu_heavy = value >= 0.5,
            8 => self.rail_sag = value >= 0.5,
            _ => {}
        }
        self.sync_params();
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let len = left.len().min(right.len());
        if len == 0 {
            return;
        }

        if self.mono_buf.len() < len {
            self.mono_buf.resize(len, 0.0);
        }

        let slice = &mut self.mono_buf[..len];
        slice.fill(0.0);

        self.engine.render(slice);
        self.finish.process(slice, self.rail_sag);

        for (s, &sample) in slice.iter().enumerate() {
            left[s] += sample;
            right[s] += sample;
        }
    }
}
