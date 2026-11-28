//! MK8 S preamp, equalizer, Vari-Pan and damper pedal controls.
use nih_plug::prelude::*;
use rhodes_mk8::panel::{Controls, Wave};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Params)]
pub struct Mk8Tone {
    #[id = "vol"]
    pub volume: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "env"]
    pub envelope: FloatParam,
    #[id = "low"]
    pub low: FloatParam,
    #[id = "mid"]
    pub mid: FloatParam,
    #[id = "high"]
    pub high: FloatParam,
    #[id = "freq"]
    pub frequency: FloatParam,
    #[id = "res"]
    pub resonance: FloatParam,
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "pan"]
    pub pan_enabled: BoolParam,
    #[id = "wave"]
    pub wave: IntParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    #[nested(id_prefix = "fx", group = "Factory FX board")]
    pub fx: Mk8FxTone,
    midi_hold: AtomicBool,
}
impl Default for Mk8Tone {
    fn default() -> Self {
        let c = Controls::default();
        let percent = |name, x| {
            FloatParam::new(name, x, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        let db = |name| {
            FloatParam::new(
                name,
                0.,
                FloatRange::Linear {
                    min: -15.,
                    max: 15.,
                },
            )
            .with_unit(" dB")
        };
        let hz = |name, x, min, max| {
            FloatParam::new(
                name,
                x,
                FloatRange::Skewed {
                    min,
                    max,
                    factor: FloatRange::skew_factor(-2.8),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
        };
        Self {
            volume: percent("MK8 Volume", c.volume as f32),
            drive: percent("MK8 Drive", 0.),
            envelope: percent("MK8 Envelope", 0.),
            low: db("MK8 Low"),
            mid: db("MK8 Mid"),
            high: db("MK8 High"),
            frequency: hz("MK8 Mid Frequency", c.mid_hz as f32, 100., 2000.),
            resonance: percent("MK8 Mid Resonance", c.resonance as f32),
            rate: hz("MK8 Vari-Pan Rate", c.pan_rate_hz as f32, 0.1, 3000.),
            depth: percent("MK8 Vari-Pan Depth", c.pan_depth as f32),
            pan_enabled: BoolParam::new("MK8 Vari-Pan", false),
            wave: IntParam::new("MK8 Vari-Pan Wave", 0, IntRange::Linear { min: 0, max: 3 })
                .with_value_to_string(std::sync::Arc::new(|n| {
                    ["Square", "Ramp", "Triangle", "Sine"][n.clamp(0, 3) as usize].to_owned()
                })),
            sustain_pedal: BoolParam::new("MK8 Damper Pedal", false),
            fx: Default::default(),
            midi_hold: AtomicBool::new(false),
        }
    }
}
impl Mk8Tone {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value() as f64,
            drive: self.drive.value() as f64,
            envelope: self.envelope.value() as f64,
            low_db: self.low.value() as f64,
            mid_db: self.mid.value() as f64,
            high_db: self.high.value() as f64,
            mid_hz: self.frequency.value() as f64,
            resonance: self.resonance.value() as f64,
            pan_rate_hz: self.rate.value() as f64,
            pan_depth: self.depth.value() as f64,
            pan_enabled: self.pan_enabled.value(),
            wave: [Wave::Square, Wave::Ramp, Wave::Triangle, Wave::Sine]
                [self.wave.value() as usize],
        }
    }
    pub fn sustain(&self, midi: bool) -> bool {
        self.midi_hold.store(midi, Ordering::Relaxed);
        midi || self.sustain_pedal.value()
    }
    pub fn display_state(&self) -> (Controls, bool, rhodes_mk8::fx::Controls, u8) {
        (
            self.controls(),
            self.sustain_pedal.value() || self.midi_hold.load(Ordering::Relaxed),
            self.fx.controls(),
            self.fx.page(),
        )
    }
}

#[derive(Params)]
pub struct Mk8FxTone {
    #[id = "fitted"]
    pub fitted: BoolParam,
    #[id = "comp"]
    pub compressor: BoolParam,
    #[id = "amount"]
    pub amount: FloatParam,
    #[id = "makeup"]
    pub makeup: FloatParam,
    #[id = "ratio"]
    pub ratio: FloatParam,
    #[id = "chorus"]
    pub chorus: BoolParam,
    #[id = "crate"]
    pub chorus_rate: FloatParam,
    #[id = "cdepth"]
    pub chorus_depth: FloatParam,
    #[id = "ctime"]
    pub chorus_time: FloatParam,
    #[id = "phaser"]
    pub phaser: BoolParam,
    #[id = "prate"]
    pub phaser_rate: FloatParam,
    #[id = "pdepth"]
    pub phaser_depth: FloatParam,
    #[id = "pfreq"]
    pub phaser_freq: FloatParam,
    #[id = "delay"]
    pub delay: BoolParam,
    #[id = "time"]
    pub time: FloatParam,
    #[id = "feed"]
    pub feedback: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "spread"]
    pub spread: FloatParam,
    #[id = "jitter"]
    pub jitter: FloatParam,
    #[id = "dout"]
    pub drive_out: FloatParam,
    #[id = "send"]
    pub send: FloatParam,
    page: std::sync::atomic::AtomicU8,
    taps: std::sync::Mutex<Taps>,
    tapped_ms: std::sync::atomic::AtomicU32,
}
#[derive(Default)]
struct Taps {
    last: Option<std::time::Instant>,
    durations: [f32; 4],
    count: usize,
}
impl Taps {
    fn hit(&mut self, now: std::time::Instant) -> Option<f32> {
        let last = self.last.replace(now)?;
        let ms = now.duration_since(last).as_secs_f32() * 1000.;
        if !(60. ..=2000.).contains(&ms) {
            self.count = 0;
            return None;
        }
        self.durations.rotate_right(1);
        self.durations[0] = ms;
        self.count = (self.count + 1).min(4);
        (self.count >= 2)
            .then(|| self.durations[..self.count].iter().sum::<f32>() / self.count as f32)
    }
}
impl Default for Mk8FxTone {
    fn default() -> Self {
        let c = rhodes_mk8::fx::Controls::default();
        let value = |name, default: f64, min, max, unit: &'static str| {
            FloatParam::new(name, default as f32, FloatRange::Linear { min, max })
                .with_unit(unit)
                .with_value_to_string(formatters::v2s_f32_rounded(1))
        };
        let pct = |name, v: f64| {
            FloatParam::new(name, v as f32, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            fitted: BoolParam::new("MK8 Factory FX Board", false),
            compressor: BoolParam::new("MK8 Compressor", false),
            amount: pct("MK8 Compressor Amount", c.amount),
            makeup: value("MK8 Makeup", c.makeup_db, 0., 24., " dB"),
            ratio: value("MK8 Compressor Ratio", c.ratio, 1., 20., ":1"),
            chorus: BoolParam::new("MK8 Chorus", false),
            chorus_rate: value("MK8 Chorus Rate", c.chorus_rate_hz, 0.05, 8., " Hz"),
            chorus_depth: pct("MK8 Chorus Depth", c.chorus_depth),
            chorus_time: value("MK8 Chorus Center", c.chorus_ms, 4., 25., " ms"),
            phaser: BoolParam::new("MK8 Phaser", false),
            phaser_rate: value("MK8 Phaser Rate", c.phaser_rate_hz, 0.05, 8., " Hz"),
            phaser_depth: pct("MK8 Phaser Depth", c.phaser_depth),
            phaser_freq: value("MK8 Phaser Center", c.phaser_hz, 120., 2400., " Hz"),
            delay: BoolParam::new("MK8 Delay", false),
            time: value("MK8 Delay Time / Tap", c.delay_ms, 60., 2000., " ms"),
            feedback: pct("MK8 Delay Feedback", c.feedback),
            mix: pct("MK8 Delay Mix", c.mix),
            spread: pct("MK8 Delay Spread", c.spread),
            jitter: pct("MK8 Delay Jitter", c.jitter),
            drive_out: value("MK8 Drive Output", 0., -12., 12., " dB"),
            send: value("MK8 FX Send Level", 0., -12., 12., " dB"),
            page: 0.into(),
            taps: Default::default(),
            tapped_ms: 0.into(),
        }
    }
}
impl Mk8FxTone {
    pub fn controls(&self) -> rhodes_mk8::fx::Controls {
        rhodes_mk8::fx::Controls {
            fitted: self.fitted.value(),
            compressor: self.compressor.value(),
            amount: self.amount.value().into(),
            makeup_db: self.makeup.value().into(),
            ratio: self.ratio.value().into(),
            chorus: self.chorus.value(),
            chorus_rate_hz: self.chorus_rate.value().into(),
            chorus_depth: self.chorus_depth.value().into(),
            chorus_ms: self.chorus_time.value().into(),
            phaser: self.phaser.value(),
            phaser_rate_hz: self.phaser_rate.value().into(),
            phaser_depth: self.phaser_depth.value().into(),
            phaser_hz: self.phaser_freq.value().into(),
            delay: self.delay.value(),
            delay_ms: self.time.value().into(),
            feedback: self.feedback.value().into(),
            mix: self.mix.value().into(),
            spread: self.spread.value().into(),
            jitter: self.jitter.value().into(),
            drive_out_db: self.drive_out.value().into(),
            fx_send_db: self.send.value().into(),
        }
    }
    pub fn page(&self) -> u8 {
        if self.fitted.value() {
            self.page.load(Ordering::Relaxed)
        } else {
            0
        }
    }
    pub fn set_page(&self, p: u8) {
        self.page.store(p.min(2), Ordering::Relaxed);
    }
    pub fn tap(&self) -> Option<f32> {
        let ms = self.taps.lock().ok()?.hit(std::time::Instant::now())?;
        self.tapped_ms.store(ms.to_bits(), Ordering::Relaxed);
        Some(ms)
    }
    pub fn tap_active(&self) -> bool {
        let ms = f32::from_bits(self.tapped_ms.load(Ordering::Relaxed));
        ms > 0. && (ms - self.time.value()).abs() < 0.02
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn three_taps_average_intervals_and_allow_times_beyond_hardware_dial() {
        let mut taps = Taps::default();
        let start = Instant::now();
        let mut hit = |ms| taps.hit(start + Duration::from_millis(ms));
        assert_eq!(hit(0), None);
        assert_eq!(hit(900), None);
        assert!((hit(1840).unwrap() - 920.).abs() < 0.01);
        assert!((hit(2790).unwrap() - 930.).abs() < 0.01);
        assert_eq!(hit(6000), None);
        assert_eq!(hit(6500), None);
        assert!((hit(7000).unwrap() - 500.).abs() < 0.01);
    }
}
