//! Shared key ownership, damper pedal and click-free voice retirement for struck actions.
/// A physical action owns resonator state; the keyboard only controls excitation and release.
pub trait StruckAction {
    fn strike_key(&mut self, velocity: f32);
    fn release(&mut self);
    fn sustain(&mut self, down: bool);
    fn reposition(&mut self, _gap_m: f64) {}
    fn step_output(&mut self, gap_m: f64) -> f64;
    fn clear(&mut self);
    fn quiet(&self) -> bool;
}
struct Voice<A> {
    action: A,
    fade: u32,
}
pub struct StrikeKeys<A> {
    voices: Vec<Voice<A>>,
    low: u8,
    high: u8,
    active: u128,
    held: u128,
    audition: Option<u8>,
    sustain: bool,
    extended: bool,
    gap: f64,
    phase: u8,
    fade_frames: u32,
}
impl<A: StruckAction> StrikeKeys<A> {
    pub fn new(rate: f64, low: u8, high: u8, mut action: impl FnMut(u8) -> A) -> Self {
        assert!(low <= high && high <= 127);
        let voices = (0..=127)
            .map(|n| Voice {
                action: action(n),
                fade: 0,
            })
            .collect();
        Self {
            voices,
            active: 0,
            held: 0,
            audition: None,
            sustain: false,
            extended: false,
            gap: 0.,
            low,
            high,
            phase: 0,
            fade_frames: (0.004 * rate).ceil() as u32,
        }
    }
    pub fn set_extended(&mut self, enabled: bool) {
        if self.extended && !enabled {
            for n in (0..self.low).chain(self.high + 1..=127) {
                if (self.active | self.held_notes()) & (1 << n) != 0 {
                    self.choke(n);
                }
            }
        }
        self.extended = enabled;
    }
    fn accepts(&self, n: u8) -> bool {
        n < 128 && (self.extended || (self.low..=self.high).contains(&n))
    }
    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        if !offset.is_finite() {
            return;
        }
        let gap = 0.001 * offset.clamp(-0.25, 2.);
        if self.gap != gap {
            self.gap = gap;
            let mut keys = self.active;
            while keys != 0 {
                let voice = &mut self.voices[keys.trailing_zeros() as usize];
                keys &= keys - 1;
                voice.action.reposition(gap);
            }
        }
    }
    pub fn set_sustain(&mut self, down: bool) {
        if self.sustain != down {
            self.sustain = down;
            for v in &mut self.voices {
                v.action.sustain(down);
            }
        }
    }
    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.action.clear();
            v.fade = 0;
        }
        self.held = 0;
        self.active = 0;
        self.sustain = false;
        self.audition = None;
        self.phase = 0;
    }
    fn strike(&mut self, n: u8, v: f32) {
        let voice = &mut self.voices[usize::from(n)];
        voice.action.reposition(self.gap);
        voice.action.sustain(self.sustain);
        voice.action.strike_key(v);
        voice.fade = 0;
        self.active |= 1 << n;
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        if !self.accepts(n) || !v.is_finite() {
            return;
        }
        if v <= 0. {
            self.note_off(n);
            return;
        }
        self.held |= 1 << n;
        self.strike(n, v);
    }
    pub fn note_off(&mut self, n: u8) {
        if n >= 128 {
            return;
        }
        self.held &= !(1 << n);
        if self.audition != Some(n) {
            self.voices[usize::from(n)].action.release();
        }
    }
    pub fn audition(&mut self, n: u8, v: f32) {
        if !self.accepts(n) || !v.is_finite() || v <= 0. {
            return;
        }
        self.audition_off();
        self.audition = Some(n);
        self.strike(n, v);
    }
    pub fn audition_off(&mut self) {
        if let Some(n) = self.audition.take() {
            if self.held & (1 << n) == 0 {
                self.voices[usize::from(n)].action.release();
            }
        }
    }
    pub fn choke(&mut self, n: u8) {
        if n >= 128 {
            return;
        }
        if self.audition == Some(n) {
            self.audition = None;
        }
        self.note_off(n);
        self.voices[usize::from(n)].fade = self.fade_frames;
    }
    pub fn held_notes(&self) -> u128 {
        self.held | self.audition.map_or(0, |n| 1 << n)
    }
    pub fn active(&self) -> bool {
        self.active != 0
    }
    pub fn begin_frame(&mut self) {
        self.phase = self.phase.wrapping_add(1);
    }
    pub fn step_sources(&mut self, mut emit: impl FnMut(u8, f64)) {
        let mut mask = self.active;
        while mask != 0 {
            let n = mask.trailing_zeros();
            mask &= mask - 1;
            let v = &mut self.voices[n as usize];
            let sample = v.action.step_output(self.gap);
            let output = if v.fade == 0 {
                sample
            } else {
                sample * (f64::from(v.fade) / f64::from(self.fade_frames))
            };
            emit(n as u8, output);
        }
    }
    pub fn step(&mut self) -> f64 {
        let mut sum = 0.;
        self.step_sources(|_, x| sum += x);
        sum
    }
    pub fn finish_frame(&mut self) {
        let mut mask = self.active;
        while mask != 0 {
            let n = mask.trailing_zeros();
            mask &= mask - 1;
            let v = &mut self.voices[n as usize];
            if v.fade > 0 {
                v.fade -= 1;
                if v.fade > 0 {
                    continue;
                }
            } else if self.phase != 0 || !v.action.quiet() {
                continue;
            }
            self.active &= !(1 << n);
            v.action.clear();
        }
    }
}
