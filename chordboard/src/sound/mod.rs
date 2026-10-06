//! Multi-engine sound subsystem for Chordboard.
//!
//! Provides 4 isolated sound lanes (Bass, Comp, Arp, Lead) each capable of
//! hosting independent instances of Karplus-Strong physical modeling strings
//! or OpenWurli electric piano.

pub mod karplus;
pub mod wurli;

use crate::engine::{Lane, LaneEvent};
pub use karplus::KarplusEngine;
pub use wurli::WurliEngineWrapper;

pub trait SoundEngine: Send {
    fn set_sample_rate(&mut self, sample_rate: f32);
    fn reset(&mut self);
    fn note_on(&mut self, channel: u8, note: u8, velocity: f32);
    fn note_off(&mut self, channel: u8, note: u8, velocity: f32);
    fn bend(&mut self, channel: u8, value: f32);
    fn pressure(&mut self, channel: u8, value: f32);
    fn cc(&mut self, channel: u8, cc: u8, value: f32);
    fn set_param(&mut self, index: usize, value: f32);
    fn render(&mut self, left: &mut [f32], right: &mut [f32]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneEngineType {
    Off = 0,
    Karplus = 1,
    OpenWurli = 2,
}

impl LaneEngineType {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => LaneEngineType::Karplus,
            2 => LaneEngineType::OpenWurli,
            _ => LaneEngineType::Off,
        }
    }

    pub fn to_index(self) -> i32 {
        self as i32
    }

    pub fn name(self) -> &'static str {
        match self {
            LaneEngineType::Off => "Off",
            LaneEngineType::Karplus => "Karplus String",
            LaneEngineType::OpenWurli => "OpenWurli",
        }
    }
}

pub struct LaneStrip {
    pub engine_type: LaneEngineType,
    pub karplus: Option<Box<KarplusEngine>>,
    pub wurli: Option<Box<WurliEngineWrapper>>,
    pub level: f32,
    pub pan: f32,
    pub mute: bool,
    pub auto_mute: bool,
    pub peak: f32,
}

impl LaneStrip {
    fn new(engine_type: LaneEngineType) -> Self {
        Self {
            engine_type,
            karplus: None,
            wurli: None,
            level: 0.8,
            pan: 0.0,
            mute: false,
            auto_mute: false,
            peak: 0.0,
        }
    }

    pub fn set_engine(&mut self, engine_type: LaneEngineType, sample_rate: f32) {
        if self.engine_type != engine_type {
            match self.engine_type {
                LaneEngineType::Karplus => {
                    if let Some(k) = self.karplus.as_mut() {
                        k.reset();
                    }
                }
                LaneEngineType::OpenWurli => {
                    if let Some(w) = self.wurli.as_mut() {
                        w.reset();
                    }
                }
                LaneEngineType::Off => {}
            }
            self.engine_type = engine_type;
        }
        match self.engine_type {
            LaneEngineType::Karplus => {
                if self.karplus.is_none() {
                    self.karplus = Some(Box::new(KarplusEngine::new(sample_rate)));
                }
            }
            LaneEngineType::OpenWurli => {
                if self.wurli.is_none() {
                    self.wurli = Some(Box::new(WurliEngineWrapper::new(sample_rate)));
                }
            }
            LaneEngineType::Off => {}
        }
    }

    pub fn ensure_engine(&mut self, sample_rate: f32) {
        match self.engine_type {
            LaneEngineType::Karplus => {
                if self.karplus.is_none() {
                    self.karplus = Some(Box::new(KarplusEngine::new(sample_rate)));
                }
            }
            LaneEngineType::OpenWurli => {
                if self.wurli.is_none() {
                    self.wurli = Some(Box::new(WurliEngineWrapper::new(sample_rate)));
                }
            }
            LaneEngineType::Off => {}
        }
    }

    pub fn active_engine_mut(&mut self) -> Option<&mut dyn SoundEngine> {
        match self.engine_type {
            LaneEngineType::Off => None,
            LaneEngineType::Karplus => self.karplus.as_mut().map(|k| k.as_mut() as &mut dyn SoundEngine),
            LaneEngineType::OpenWurli => self.wurli.as_mut().map(|w| w.as_mut() as &mut dyn SoundEngine),
        }
    }
}

pub struct LaneDispatcher {
    pub lanes: [LaneStrip; Lane::COUNT],
    pub pad_swell_source: u8,
    pub pad_swell: f32,
    sample_rate: f32,
    temp_l: Vec<f32>,
    temp_r: Vec<f32>,
}

impl LaneDispatcher {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.max(1.0);
        let lanes = [
            LaneStrip::new(LaneEngineType::Karplus),   // Bass
            LaneStrip::new(LaneEngineType::OpenWurli), // Comp
            LaneStrip::new(LaneEngineType::Karplus),   // Arp
            LaneStrip::new(LaneEngineType::Karplus),   // Lead
        ];
        Self {
            lanes,
            pad_swell_source: 0,
            pad_swell: 1.0,
            sample_rate,
            temp_l: vec![0.0; 2048],
            temp_r: vec![0.0; 2048],
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        for lane in &mut self.lanes {
            if let Some(k) = lane.karplus.as_mut() {
                k.set_sample_rate(self.sample_rate);
            }
            if let Some(w) = lane.wurli.as_mut() {
                w.set_sample_rate(self.sample_rate);
            }
        }
    }

    pub fn reset(&mut self) {
        for lane in &mut self.lanes {
            if let Some(k) = lane.karplus.as_mut() {
                k.reset();
            }
            if let Some(w) = lane.wurli.as_mut() {
                w.reset();
            }
            lane.peak = 0.0;
        }
    }

    pub fn set_lane_engine(&mut self, lane: Lane, engine_type: LaneEngineType) {
        self.lanes[lane.index()].set_engine(engine_type, self.sample_rate);
    }

    pub fn set_lane_level(&mut self, lane: Lane, level: f32) {
        self.lanes[lane.index()].level = level.clamp(0.0, 2.0);
    }

    pub fn set_lane_pan(&mut self, lane: Lane, pan: f32) {
        self.lanes[lane.index()].pan = pan.clamp(-1.0, 1.0);
    }

    pub fn set_lane_mute(&mut self, lane: Lane, mute: bool) {
        self.lanes[lane.index()].mute = mute;
    }

    pub fn set_lane_auto_mute(&mut self, lane: Lane, auto_mute: bool) {
        self.lanes[lane.index()].auto_mute = auto_mute;
    }

    pub fn set_pad_swell_source(&mut self, source: u8) {
        self.pad_swell_source = source;
    }

    pub fn set_lane_opt_a(&mut self, lane: Lane, opt_a: bool) {
        let strip = &mut self.lanes[lane.index()];
        if let Some(w) = strip.wurli.as_mut() {
            w.cpu_heavy = opt_a;
            w.set_param(7, if opt_a { 1.0 } else { 0.0 });
        }
    }

    pub fn set_lane_opt_b(&mut self, lane: Lane, opt_b: bool) {
        let strip = &mut self.lanes[lane.index()];
        if let Some(w) = strip.wurli.as_mut() {
            w.rail_sag = opt_b;
            w.set_param(8, if opt_b { 1.0 } else { 0.0 });
        }
    }

    pub fn set_lane_param(&mut self, lane: Lane, index: usize, value: f32) {
        let strip = &mut self.lanes[lane.index()];
        if let Some(k) = strip.karplus.as_mut() {
            k.set_param(index, value);
        }
        if let Some(w) = strip.wurli.as_mut() {
            w.set_param(index, value);
        }
    }

    pub fn handle_lane_event(&mut self, event: LaneEvent) {
        let sample_rate = self.sample_rate;
        let strip = &mut self.lanes[event.lane().index()];
        if strip.mute {
            return;
        }
        strip.ensure_engine(sample_rate);
        if let Some(engine) = strip.active_engine_mut() {
            match event {
                LaneEvent::On {
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    engine.note_on(channel, note, velocity);
                }
                LaneEvent::Off {
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    engine.note_off(channel, note, velocity);
                }
            }
        }
    }

    pub fn handle_bend(&mut self, channel: u8, value: f32) {
        for lane in &mut self.lanes {
            if let Some(engine) = lane.active_engine_mut() {
                engine.bend(channel, value);
            }
        }
    }

    pub fn handle_pressure(&mut self, channel: u8, value: f32) {
        for lane in &mut self.lanes {
            if let Some(engine) = lane.active_engine_mut() {
                engine.pressure(channel, value);
            }
        }
    }

    pub fn handle_cc(&mut self, channel: u8, cc: u8, value: f32) {
        if (self.pad_swell_source == 1 && cc == 11) || (self.pad_swell_source == 2 && cc == 1) {
            self.pad_swell = value.clamp(0.0, 1.0);
        }
        for lane in &mut self.lanes {
            if let Some(engine) = lane.active_engine_mut() {
                engine.cc(channel, cc, value);
            }
        }
    }

    pub fn any_active(&self) -> bool {
        self.lanes
            .iter()
            .any(|strip| !strip.mute && !strip.auto_mute && strip.engine_type != LaneEngineType::Off)
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let len = left.len().min(right.len());
        if len == 0 {
            return;
        }

        left[..len].fill(0.0);
        right[..len].fill(0.0);

        if self.temp_l.len() < len {
            self.temp_l.resize(len, 0.0);
            self.temp_r.resize(len, 0.0);
        }

        let sample_rate = self.sample_rate;
        for (i, strip) in self.lanes.iter_mut().enumerate() {
            if strip.mute || strip.auto_mute || strip.engine_type == LaneEngineType::Off {
                strip.peak = 0.0;
                continue;
            }

            strip.ensure_engine(sample_rate);
            let lane_l = &mut self.temp_l[..len];
            let lane_r = &mut self.temp_r[..len];
            lane_l.fill(0.0);
            lane_r.fill(0.0);

            if let Some(engine) = strip.active_engine_mut() {
                engine.render(lane_l, lane_r);
            }

            // Constant-power pan:
            let pan_normalized = (strip.pan + 1.0) * 0.5; // 0.0 .. 1.0
            let angle = pan_normalized * (std::f32::consts::PI * 0.5);
            let swell = if i == Lane::Comp.index() && self.pad_swell_source != 0 {
                self.pad_swell
            } else {
                1.0
            };
            let gain_l = strip.level * swell * angle.cos();
            let gain_r = strip.level * swell * angle.sin();

            let mut max_peak = 0.0f32;
            for s in 0..len {
                let sl = lane_l[s] * gain_l;
                let sr = lane_r[s] * gain_r;
                left[s] += sl;
                right[s] += sr;
                max_peak = max_peak.max(sl.abs()).max(sr.abs());
            }
            strip.peak = max_peak;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_karplus_renders_audio() {
        let mut k = karplus::KarplusEngine::new(44100.0);
        k.note_on(0, 60, 0.8);
        let mut l = vec![0.0f32; 512];
        let mut r = vec![0.0f32; 512];
        k.render(&mut l, &mut r);
        let max_l = l.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let max_r = r.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_l > 0.01, "Karplus left output is silent: {max_l}");
        assert!(max_r > 0.01, "Karplus right output is silent: {max_r}");
    }

    #[test]
    fn test_wurli_renders_audio() {
        std::thread::Builder::new()
            .name("test_wurli".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let mut w = WurliEngineWrapper::new(44100.0);
                w.note_on(0, 60, 0.8);
                let mut l = vec![0.0f32; 512];
                let mut r = vec![0.0f32; 512];
                w.render(&mut l, &mut r);
                let max_l = l.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let max_r = r.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                assert!(max_l > 0.0001, "Wurli left output is silent: {max_l}");
                assert!(max_r > 0.0001, "Wurli right output is silent: {max_r}");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn test_dispatcher_handles_lane_events_and_renders() {
        std::thread::Builder::new()
            .name("test_disp".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let mut disp = LaneDispatcher::new(44100.0);
                disp.handle_lane_event(LaneEvent::On {
                    lane: Lane::Bass,
                    channel: 0,
                    note: 36,
                    velocity: 0.9,
                });
                disp.handle_lane_event(LaneEvent::On {
                    lane: Lane::Comp,
                    channel: 0,
                    note: 60,
                    velocity: 0.8,
                });

                let mut l = vec![0.0f32; 512];
                let mut r = vec![0.0f32; 512];
                disp.render(&mut l, &mut r);

                let max_l = l.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let max_r = r.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                assert!(max_l > 0.01, "Dispatcher left output is silent: {max_l}");
                assert!(max_r > 0.01, "Dispatcher right output is silent: {max_r}");
                assert!(disp.lanes[0].peak > 0.0, "Bass peak meter is zero");
                assert!(disp.lanes[1].peak > 0.0, "Comp peak meter is zero");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn test_pad_swell_and_auto_mute() {
        let mut disp = LaneDispatcher::new(44100.0);
        disp.set_pad_swell_source(1); // CC 11 Expression
        assert_eq!(disp.pad_swell, 1.0);

        // Turn down expression
        disp.handle_cc(0, 11, 0.25);
        assert!((disp.pad_swell - 0.25).abs() < 1e-4);

        // Turn auto_mute on for Comp lane
        disp.set_lane_auto_mute(Lane::Comp, true);
        assert!(disp.lanes[Lane::Comp.index()].auto_mute);

        disp.set_lane_auto_mute(Lane::Comp, false);
        assert!(!disp.lanes[Lane::Comp.index()].auto_mute);
    }
}
