//! Common damper rail, key ownership and preallocated MIDI bank. Each instrument
//! supplies the motion at a key and the circuit fed by the sum of the pickups.
use pleasant_dsp::instrument::Decimator;
pub trait Action {
    type Observation: Copy;
    fn reset(&mut self);
    fn press(&mut self, velocity: f64);
    fn release(&mut self);
    fn set_pickup_distance_mm(&mut self, offset: f32);
    fn induced_voltage(&mut self, rail_lift: f64) -> f64;
    fn support_velocity(&self) -> f64 {
        0.
    }
    fn observation(&self) -> Self::Observation;
}
pub trait Board {
    fn reset(&mut self);
    fn step(&mut self, emf: f64) -> f64;
    fn hold(&mut self, _note: u8, _down: bool) {}
    fn pedal(&mut self, _down: bool) {}
    fn choke(&mut self, _note: u8) {}
    fn drive(&mut self, _note: u8, _velocity: f64) {}
    fn set_pickup_distance_mm(&mut self, _offset: f32) {}
    fn ringing(&self) -> bool {
        false
    }
}
struct Slot<K> {
    in_audio_band: bool,
    key: K,
    tail: usize,
    choke: usize,
}
pub struct Keyboard<K, P> {
    native: std::ops::RangeInclusive<u8>,
    supports_pedal: bool,
    keys: Vec<Slot<K>>,
    pub(crate) panel: P,
    decimator: Decimator<1>,
    active: u128,
    extended: bool,
    gap_mm: f32,
    held: u128,
    audition: Option<u8>,
    pedal: bool,
    lift: f64,
    bass_mask: u128,
    bass_pedal: bool,
    bass_lift: f64,
    substeps: usize,
    rate: f64,
    panel_tail: usize,
    tail_ticks: usize,
    choke_ticks: usize,
}
impl<K: Action, P: Board> Keyboard<K, P> {
    pub(crate) fn from_parts(
        native: std::ops::RangeInclusive<u8>,
        supports_pedal: bool,
        latency: u32,
        host_rate: f64,
        make_key: impl Fn(u8, f64) -> K,
        make_panel: impl FnOnce(f64) -> P,
    ) -> Self {
        assert!(host_rate.is_finite() && host_rate >= 8000.);
        let substeps = (88200. / host_rate).ceil() as usize;
        let rate = host_rate * substeps as f64;
        Self {
            native,
            supports_pedal,
            keys: (0..=127)
                .map(|m| Slot {
                    in_audio_band: crate::beam::midi_hz(m) < 0.5 * host_rate,
                    key: make_key(m, rate),
                    tail: 0,
                    choke: 0,
                })
                .collect(),
            panel: make_panel(rate),
            decimator: Decimator::new(substeps, host_rate, latency as usize),
            active: 0,
            extended: false,
            gap_mm: 0.,
            held: 0,
            audition: None,
            pedal: false,
            lift: 0.,
            bass_mask: 0,
            bass_pedal: false,
            bass_lift: 0.,
            substeps,
            rate,
            panel_tail: 0,
            tail_ticks: (0.5 * rate).ceil() as usize,
            choke_ticks: (0.004 * rate).ceil() as usize,
        }
    }
    pub fn reset(&mut self) {
        let mut bits = self.active;
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            self.keys[i].key.reset();
            self.keys[i].tail = 0;
            self.keys[i].choke = 0;
        }
        self.active = 0;
        self.held = 0;
        self.audition = None;
        self.pedal = false;
        self.lift = 0.;
        self.bass_pedal = false;
        self.bass_lift = 0.;
        self.panel_tail = 0;
        self.panel.reset();
        self.decimator.reset();
    }
    pub fn key_range(&self) -> std::ops::RangeInclusive<u8> {
        if self.extended {
            0..=127
        } else {
            self.native.clone()
        }
    }
    pub fn set_extended(&mut self, extended: bool) {
        if self.extended && !extended {
            for n in 0..=127 {
                if !self.native.contains(&n) {
                    self.choke(n);
                }
            }
        }
        self.extended = extended;
    }
    pub(crate) fn keys_mut(&mut self) -> impl Iterator<Item = &mut K> {
        self.keys.iter_mut().map(|slot| &mut slot.key)
    }
    pub fn set_pickup_distance_mm(&mut self, offset: f32) {
        if self.gap_mm == offset {
            return;
        }
        self.gap_mm = offset;
        self.panel.set_pickup_distance_mm(offset);
        for slot in &mut self.keys {
            slot.key.set_pickup_distance_mm(offset);
        }
    }
    pub fn set_sustain(&mut self, down: bool) {
        if !self.supports_pedal || self.pedal == down {
            return;
        }
        self.pedal = down;
        self.panel.pedal(down);
        self.refresh_dampers(u128::MAX);
    }
    pub(crate) fn set_bass_rail(&mut self, down: bool, highest: u8) {
        if self.bass_pedal == down {
            return;
        }
        self.bass_mask = u128::MAX >> (127 - highest);
        self.bass_pedal = down;
        self.refresh_dampers(self.bass_mask);
    }
    fn refresh_dampers(&mut self, mask: u128) {
        let mut bits = self.active & !self.pressed() & mask;
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            if self.keys[i].choke == 0 {
                self.keys[i].tail =
                    if self.pedal || (self.bass_pedal && self.bass_mask & (1 << i) != 0) {
                        0
                    } else {
                        self.tail_ticks
                    };
            }
        }
    }
    pub fn pressed(&self) -> u128 {
        self.held | self.audition.map_or(0, |n| 1u128 << n)
    }
    pub fn sounding(&self) -> bool {
        self.active != 0 || self.panel_tail > 0 || self.panel.ringing()
    }
    pub fn observation(&self, n: u8) -> Option<K::Observation> {
        self.key_range()
            .contains(&n)
            .then(|| self.keys[usize::from(n)].key.observation())
    }
    fn strike(&mut self, n: u8, v: f32) {
        let i = usize::from(n);
        self.active |= 1u128 << i;
        self.panel.hold(n, true);
        let s = &mut self.keys[i];
        s.choke = 0;
        s.tail = 0;
        s.key.press(f64::from(v.clamp(0., 1.)));
    }
    fn lift_key(&mut self, n: u8) {
        self.panel.hold(n, false);
        let s = &mut self.keys[usize::from(n)];
        s.key.release();
        s.tail = if self.pedal || (self.bass_pedal && self.bass_mask & (1u128 << n) != 0) {
            0
        } else {
            self.tail_ticks
        };
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        if !self.key_range().contains(&n) {
            return;
        }
        if v <= 0. {
            self.note_off(n);
            return;
        }
        self.held |= 1u128 << n;
        self.strike(n, v);
    }
    pub fn note_off(&mut self, n: u8) {
        if !self.key_range().contains(&n) {
            return;
        }
        let bit = 1u128 << n;
        if self.held & bit == 0 {
            return;
        }
        self.held &= !bit;
        if self.audition != Some(n) {
            self.lift_key(n);
        }
    }
    pub fn audition(&mut self, n: Option<u8>, v: f32) {
        let next = n.filter(|k| self.key_range().contains(k) && v > 0.);
        if next == self.audition {
            return;
        }
        if let Some(old) = self.audition.take() {
            if self.held & (1u128 << old) == 0 {
                self.lift_key(old);
            }
        }
        self.audition = next;
        if let Some(k) = next {
            self.strike(k, v);
        }
    }
    pub fn choke(&mut self, n: u8) {
        if !self.key_range().contains(&n) {
            return;
        }
        let i = usize::from(n);
        let bit = 1u128 << i;
        if self.active & bit == 0 {
            return;
        }
        self.held &= !bit;
        if self.audition == Some(n) {
            self.audition = None;
        }
        self.panel.choke(n);
        self.keys[i].key.release();
        self.keys[i].tail = self.choke_ticks;
        self.keys[i].choke = self.choke_ticks;
    }
    fn tick(&mut self) -> f64 {
        let target = f64::from(self.pedal);
        self.lift += (target - self.lift).clamp(-100. / self.rate, 100. / self.rate);
        self.bass_lift += (f64::from(self.bass_pedal) - self.bass_lift)
            .clamp(-100. / self.rate, 100. / self.rate);
        let mut sum = 0.;
        let mut bits = self.active;
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let s = &mut self.keys[i];
            let rail = if self.bass_mask & (1u128 << i) != 0 {
                self.lift.max(self.bass_lift)
            } else {
                self.lift
            };
            let emf = s.key.induced_voltage(rail);
            let gain = if s.choke > 0 {
                let g = s.choke as f64 / self.choke_ticks as f64;
                s.choke -= 1;
                g
            } else {
                1.
            };
            if s.in_audio_band {
                sum += gain * emf;
                self.panel.drive(i as u8, gain * s.key.support_velocity());
            }
            if s.tail > 0 {
                s.tail -= 1;
                if s.tail == 0 {
                    self.active &= !(1u128 << i);
                    s.key.reset();
                }
            }
        }
        self.panel.step(sum)
    }
    pub fn sample(&mut self) -> f32 {
        if !self.sounding() {
            self.lift = f64::from(self.pedal);
            self.bass_lift = f64::from(self.bass_pedal);
            return 0.;
        }
        if self.active != 0 || self.panel.ringing() {
            self.panel_tail = (0.25 * self.rate / self.substeps as f64) as usize;
        } else {
            self.panel_tail -= 1;
        }
        let mut out = 0.;
        for s in 0..self.substeps {
            let x = self.tick();
            self.decimator.push([x]);
            if s == 0 {
                out = self.decimator.read::<1>()[0];
            }
        }
        out as f32
    }
    pub fn render(&mut self, buffer: &mut [f32]) {
        for x in buffer {
            *x = self.sample();
        }
    }
}

#[cfg(test)]
pub(crate) fn check_register<K: Action, P: Board>(make: impl Fn(f64) -> Keyboard<K, P>) {
    use std::f64::consts::TAU;
    let energy = |y: &[f32]| y.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    for sr in [44100, 48000, 96000] {
        let mut e = make(f64::from(sr));
        e.note_on(0, 0.7);
        e.note_on(127, 0.7);
        assert_eq!(e.pressed(), 0);
        e.note_on(64, 0.67);
        let mut native = vec![0.; (sr / 8) as usize];
        e.render(&mut native);
        e.reset();
        e.set_extended(true);
        e.note_on(64, 0.67);
        let mut adjusted = native.clone();
        e.render(&mut adjusted);
        assert_eq!(adjusted, native);
        e.reset();
        e.set_pickup_distance_mm(1.);
        e.note_on(64, 0.67);
        e.render(&mut adjusted);
        assert!(energy(&adjusted) < 0.4 * energy(&native));
        e.set_pickup_distance_mm(0.);
        for n in [0, 127] {
            e.reset();
            e.note_on(n, 0.65);
            let mut y = vec![
                0.;
                if n == 0 {
                    sr as usize * 2
                } else {
                    sr as usize / 9
                }
            ];
            e.render(&mut y);
            let hz = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
            let spectrum = |f: f64| {
                let wave = &y[(sr / 30) as usize..];
                let mut re = 0.;
                let mut im = 0.;
                for (i, &v) in wave.iter().enumerate() {
                    let theta = TAU * i as f64;
                    let w = (0.5 - 0.5 * (theta / (wave.len() - 1) as f64).cos()) * f64::from(v);
                    let (a, b) = (theta * f / f64::from(sr)).sin_cos();
                    re += b * w;
                    im += a * w;
                }
                re.hypot(im)
            };
            let f = spectrum(hz);
            assert!(
                f > 1e-8 && f > 2. * spectrum(hz * 0.8) && f > 2. * spectrum(hz * 1.2),
                "{sr} {n}: {f}"
            );
            e.set_extended(false);
            assert_eq!(e.pressed(), 0);
            e.set_extended(true);
        }
    }
}
