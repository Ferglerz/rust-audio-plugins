//! Real-time D6 action. Each key retains one continuous string across its tangent.
use crate::panel::{Board, Panel, ALL_TONES, PICKUP_VOLTS};
use crate::{key, Geometry, Key, Pickups, StringVoice, Switch, Wiring, REST};
use pleasant_dsp::instrument::Decimator;
use std::f64::consts::PI;

pub const LOW: u8 = 29;
pub const HIGH: u8 = 88;
pub const LATENCY: u32 = 8;

fn lerp(a: f64, b: f64, x: f64) -> f64 {
    a + (b - a) * x
}

fn scale(midi: u8) -> Option<Key> {
    if !(LOW..=HIGH).contains(&midi) {
        return None;
    }
    if let Some(k) = key(midi) {
        return Some(k);
    }
    // The three measured-note fits bound the internal scale. Endpoints and the lengths
    // between them are estimates. Pickup endpoint distances come from Remaggi et al.
    let bass = key(46)?;
    let mut low = bass;
    low.length = 0.95;
    low.centre_m = 0.185;
    low.bridge_m = 0.040;
    low.inharmonicity = 0.00022;
    low.loss = 0.28;
    low.cells = 136;
    let middle = key(64)?;
    let high = key(78)?;
    let mut top = high;
    top.length = 0.104;
    top.centre_m = 0.065;
    top.bridge_m = 0.040;
    top.loss = 3.9;
    top.cells = 48;
    let (a, b, n, m) = match midi {
        29..=46 => (low, bass, LOW, 46),
        47..=64 => (bass, middle, 46, 64),
        65..=78 => (middle, high, 64, 78),
        _ => (high, top, 78, HIGH),
    };
    let x = f64::from(midi - n) / f64::from(m - n);
    let mix = |lo: f64, hi: f64| lerp(lo, hi, x);
    // First 23 strings use a steel core and winding. Relative wound area follows
    // the .032/.028/.022-inch service gauges; the core/wrap split remains estimated.
    let (core, winding) = if midi <= 51 {
        let relative_diameter: f64 = match midi {
            29..=38 => 0.032 / 0.022,
            39..=43 => 0.028 / 0.022,
            _ => 1.0,
        };
        let relative_area = relative_diameter.powi(2);
        (
            bass.core_mass_per_m * relative_area,
            bass.winding_mass_per_m * relative_area,
        )
    } else {
        (
            lerp(
                middle.core_mass_per_m,
                high.core_mass_per_m,
                (f64::from(midi) - 64.0).clamp(0.0, 14.0) / 14.0,
            ),
            0.0,
        )
    };
    let hz = 440.0 * 2.0_f64.powf((f64::from(midi) - 69.0) / 12.0);
    Some(Key {
        length: mix(a.length, b.length),
        // Finite clamp and bending raise the pilot's frequency slightly above tension pitch.
        tension_pitch_hz: hz * 0.9985,
        anvil_m: mix(a.anvil_m, b.anvil_m),
        core_mass_per_m: core,
        winding_mass_per_m: winding,
        inharmonicity: mix(a.inharmonicity, b.inharmonicity),
        centre_m: mix(a.centre_m, b.centre_m),
        bridge_m: mix(a.bridge_m, b.bridge_m),
        loss: mix(a.loss, b.loss),
        cells: mix(a.cells as f64, b.cells as f64).round() as usize,
    })
}

fn extended_scale(midi: u8) -> Key {
    let nearest = midi.clamp(LOW, HIGH);
    let mut k = scale(nearest).unwrap();
    if nearest != midi {
        // Retain the endpoint geometry and static tension. A virtual string's
        // linear density changes with the inverse square of its pitch ratio.
        let ratio = 2.0_f64.powf((f64::from(midi) - f64::from(nearest)) / 12.);
        k.tension_pitch_hz *= ratio;
        k.core_mass_per_m /= ratio * ratio;
        k.winding_mass_per_m /= ratio * ratio;
    }
    k
}

fn treble_steps(k: Key, rate: f64) -> usize {
    // A short virtual pitch period still needs enough spatial cells to preserve
    // the clamp and pickup positions. Refine time only for these added notes.
    (60. * k.tension_pitch_hz / rate).ceil().max(1.) as usize
}

/// SI geometry for the action diagram; the solver rounds afterlength to a grid cell.
pub fn geometry(midi: u8) -> Option<Geometry> {
    let k = (midi < 128).then(|| extended_scale(midi))?;
    Some(Geometry {
        speaking_m: k.length,
        afterlength_m: k.length * (2.0_f64.powf(0.25) - 1.0),
        centre_m: k.centre_m,
        bridge_m: k.bridge_m,
        anvil_drop_m: k.anvil_m,
    })
}

/// A linked action clocks the reed at the panel rate, including when a virtual
/// treble string needs several mechanical ticks during one reed tick.
pub struct LinkedString {
    pub string: StringVoice,
    steps: usize,
}

impl LinkedString {
    pub fn new(midi: u8, rate: f64) -> Self {
        let k = extended_scale(midi);
        let steps = if midi > HIGH {
            treble_steps(k, rate)
        } else {
            1
        };
        let mechanical = rate * steps as f64;
        Self {
            string: StringVoice::with_grid(k, mechanical, stable_cells(k, mechanical)),
            steps,
        }
    }

    pub fn tick(&mut self) -> (Pickups, f64) {
        let (mut total, mut fraction) = self.string.step_linked_key();
        for _ in 1..self.steps {
            let (p, f) = self.string.step_linked_key();
            total.centre += p.centre;
            total.bridge += p.bridge;
            fraction = f;
        }
        let n = self.steps as f64;
        (
            Pickups {
                centre: total.centre / n,
                bridge: total.bridge / n,
            },
            fraction,
        )
    }
}

fn stable_cells(k: Key, rate: f64) -> usize {
    // The exact stability bound from the stiff-string stencil. Leave a margin
    // before its Nyquist mode and retain reference resolution wherever possible.
    let f = k.tension_pitch_hz / (1.0 + k.inharmonicity).sqrt();
    (4..=k.cells.min(80))
        .rev()
        .find(|&n| {
            let n = n as f64;
            let wave = (2.0 * f * n / rate).powi(2);
            let bend = (2.0 * f * n * n * k.inharmonicity.sqrt() / (rate * PI)).powi(2);
            4.0 * wave / 3.0 + 4.0 * bend + 0.006 * n * n / rate < 0.93
        })
        .expect("live D6 grid requires a positive audio sample rate")
}

impl StringVoice {
    /// Stable live grid for an externally driven key (Duo), at its internal tick rate.
    pub fn for_driven_key(midi: u8, rate: f64) -> Option<Self> {
        if !rate.is_finite() || rate < 44100. {
            return None;
        }
        let key = scale(midi)?;
        Some(Self::with_grid(key, rate, stable_cells(key, rate)))
    }

    pub fn clear(&mut self) {
        let s = self;
        s.current.fill(0.0);
        s.past.fill(0.0);
        s.next.fill(0.0);
        for cell in &mut s.gel {
            cell.clear();
        }
        s.tip = REST;
        s.pressed = false;
        s.centre.previous_flux = 0.0;
        s.bridge.previous_flux = 0.0;
    }

    pub fn mute(&mut self, amount: f64) {
        // A narrow felt strip bears on the speaking section near the tailpiece.
        // Sub-cell overlap preserves the strip width on different host-rate grids.
        let s = self;
        let dx = s.geometry.speaking_m / (s.tangent - 1) as f64;
        let centre = 0.8 * s.geometry.bridge_m;
        let width = 0.016;
        for i in 2..s.tangent {
            let x = (i - 1) as f64 * dx;
            let overlap = ((x + 0.5 * dx).min(centre + width / 2.0)
                - (x - 0.5 * dx).max(centre - width / 2.0))
            .max(0.0)
                / dx;
            s.drag[i] = s.steel_loss + amount * overlap * 2200.0 / s.rate;
            s.inv[i] = 1.0 / (1.0 + s.drag[i]);
        }
    }
}

struct Voice {
    string: StringVoice,
    half_rate: bool,
    steps: usize,
    previous: Pickups,
    latest: Pickups,
    released: usize,
    restrike: u8,
    choke: usize,
}

impl Voice {
    fn new(note: u8, internal: f64) -> Self {
        let k = extended_scale(note);
        // The wound register's useful partials occupy a smaller bandwidth. Its
        // time step admits 22.05 kHz at the lowest internal rate. The discrete
        // spatial grid further limits the useful bandwidth of the longest strings.
        let half_rate = k.winding_mass_per_m > 0.0;
        let steps = if note > HIGH {
            treble_steps(k, internal)
        } else if !half_rate && stable_cells(k, internal) < k.cells.min(80) * 9 / 10 {
            2
        } else {
            1
        };
        let rate = if half_rate {
            internal * 0.5
        } else {
            internal * steps as f64
        };
        Self {
            half_rate,
            steps,
            previous: Pickups::default(),
            latest: Pickups::default(),
            string: StringVoice::with_grid(k, rate, stable_cells(k, rate)),
            released: 0,
            restrike: 0,
            choke: 0,
        }
    }

    #[inline]
    fn tick(&mut self, even: bool) -> Pickups {
        if self.steps > 1 {
            let mut sum = self.string.advance::<false>();
            for _ in 1..self.steps {
                let p = self.string.advance::<false>();
                sum.centre += p.centre;
                sum.bridge += p.bridge;
            }
            return Pickups {
                centre: sum.centre / self.steps as f64,
                bridge: sum.bridge / self.steps as f64,
            };
        }
        if !self.half_rate {
            return self.string.advance::<false>();
        }
        if even {
            self.previous = self.latest;
            self.latest = self.string.advance::<false>();
            Pickups {
                centre: 0.5 * (self.latest.centre + self.previous.centre),
                bridge: 0.5 * (self.latest.bridge + self.previous.bridge),
            }
        } else {
            self.latest
        }
    }

    fn clear(&mut self) {
        self.previous = Pickups::default();
        self.latest = Pickups::default();
        self.string.clear();
        self.released = 0;
        self.restrike = 0;
        self.choke = 0;
    }

    fn quiet(&self) -> bool {
        let v = &self.string;
        v.current
            .iter()
            .zip(&v.past)
            .all(|(&a, &b)| a.abs() < 2e-8 && (a - b).abs() < 5e-6 / v.rate)
    }
}

/// The two coil voltages are sampled together before the instrument's preamp.
pub trait PickupBoard {
    fn tick(&mut self, inputs: Pickups) -> f64;
    fn clear(&mut self);
}
impl PickupBoard for Panel {
    fn tick(&mut self, inputs: Pickups) -> f64 {
        self.process(inputs)
    }
    fn clear(&mut self) {
        self.reset();
    }
}
pub type Engine = TangentKeyboard<Panel>;

pub struct TangentKeyboard<P> {
    voices: Vec<Voice>,
    first: u8,
    last: u8,
    distance_mm: [f64; 2],
    rate: f64,
    substeps: usize,
    decimator: Decimator<3>,
    even: bool,
    pub(super) panel: P,
    mute: f64,
    active: u128,
    held: u128,
    audition: Option<u8>,
    panel_tail: usize,
}

impl Engine {
    pub fn new(rate: f64) -> Self {
        Self::with_range(rate, LOW, HIGH)
    }

    pub fn extended(rate: f64) -> Self {
        Self::with_range(rate, 0, 127)
    }

    fn with_range(rate: f64, first: u8, last: u8) -> Self {
        Self::with_panel(rate, first, last, |r| {
            Panel::new(r, Switch::CA, Wiring::DaAdds, ALL_TONES, 1.)
        })
    }

    pub fn set_board(&mut self, board: Board) {
        self.panel.select_board(board);
    }
    pub fn set_power(&mut self, on: bool) {
        self.panel.set_power(on);
    }
    pub fn set_controls(
        &mut self,
        selector: Switch,
        wiring: Wiring,
        tones: u8,
        volume: f64,
        mute: f64,
    ) {
        self.panel
            .set_controls(selector, wiring, tones & ALL_TONES, volume.clamp(0., 1.));
        self.set_mute(mute);
    }
}

impl<P: PickupBoard> TangentKeyboard<P> {
    pub(super) fn with_panel(rate: f64, first: u8, last: u8, panel: impl FnOnce(f64) -> P) -> Self {
        assert!(rate.is_finite() && rate >= 8000.0);
        let substeps = (88_200.0 / rate).ceil().max(1.0) as usize;
        let internal = rate * substeps as f64;
        Self {
            voices: (first..=last).map(|n| Voice::new(n, internal)).collect(),
            first,
            last,
            distance_mm: [0.; 2],
            rate,
            substeps,
            decimator: Decimator::new(substeps, rate, LATENCY as usize),
            even: true,
            panel: panel(internal),
            mute: 0.,
            active: 0,
            held: 0,
            audition: None,
            panel_tail: 0,
        }
    }

    pub fn set_pickup_distance_mm(&mut self, centre: f64, bridge: f64) {
        if [centre, bridge] != self.distance_mm && centre.is_finite() && bridge.is_finite() {
            let mut keys = self.active;
            while keys != 0 {
                let v = &mut self.voices[keys.trailing_zeros() as usize];
                keys &= keys - 1;
                v.string.set_pickup_distance_mm(centre, bridge);
            }
            self.distance_mm = [centre, bridge];
        }
    }

    pub fn set_mute(&mut self, mute: f64) {
        let mute = mute.clamp(0.0, 1.0);
        if mute != self.mute {
            for v in &mut self.voices {
                v.string.mute(mute);
            }
        }
        self.mute = mute;
    }
    pub(super) fn fit_gel(&mut self, relaxed: f64, transient: f64, relaxation_s: f64) {
        for v in &mut self.voices {
            v.string.fit_gel(relaxed, transient, relaxation_s);
        }
    }

    pub fn reset(&mut self) {
        let mut active = self.active;
        while active != 0 {
            self.voices[active.trailing_zeros() as usize].clear();
            active &= active - 1;
        }
        self.active = 0;
        self.held = 0;
        self.audition = None;
        self.panel_tail = 0;
        self.even = true;
        self.decimator.reset();
        self.panel.clear();
    }

    fn strike(&mut self, note: u8, velocity: f32) {
        let i = usize::from(note - self.first);
        let v = &mut self.voices[i];
        v.string
            .set_pickup_distance_mm(self.distance_mm[0], self.distance_mm[1]);
        let vel = (velocity.clamp(0.0, 1.0) * 127.0).round().max(1.0) as u8;
        self.active |= 1 << i;
        v.choke = 0;
        v.released = 0;
        if v.string.pressed {
            // A second MIDI press on a held key must lift its tip before reclamping.
            v.string.release();
            v.restrike = vel;
        } else {
            v.string.press(vel);
            v.restrike = 0;
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        if !(self.first..=self.last).contains(&note) {
            return;
        }
        if velocity <= 0.0 {
            self.note_off(note);
            return;
        }
        self.held |= 1 << (note - self.first);
        self.strike(note, velocity);
    }

    pub fn note_off(&mut self, note: u8) {
        if !(self.first..=self.last).contains(&note) {
            return;
        }
        self.held &= !(1 << (note - self.first));
        if self.audition != Some(note) {
            self.lift(note);
        }
    }

    fn lift(&mut self, note: u8) {
        let v = &mut self.voices[usize::from(note - self.first)];
        v.string.release();
        v.restrike = 0;
    }

    pub fn audition(&mut self, note: u8, velocity: f32) {
        if !(self.first..=self.last).contains(&note) {
            return;
        }
        self.audition_off();
        self.audition = Some(note);
        self.strike(note, velocity);
    }

    pub fn audition_off(&mut self) {
        if let Some(n) = self.audition.take() {
            if self.held & (1 << (n - self.first)) == 0 {
                self.lift(n);
            }
        }
    }

    pub fn held_notes(&self) -> u128 {
        let ui = self.audition.map(|n| 1u128 << n).unwrap_or(0);
        (self.held << self.first) | ui
    }

    pub fn choke(&mut self, note: u8) {
        if !(self.first..=self.last).contains(&note) {
            return;
        }
        if self.audition == Some(note) {
            self.audition = None;
        }
        self.note_off(note);
        self.voices[usize::from(note - self.first)].choke = (self.rate * 0.004).ceil() as usize;
    }

    pub fn sounding(&self) -> bool {
        self.active != 0 || self.panel_tail != 0
    }

    fn frame<const STEMS: bool>(&mut self) -> [f64; 3] {
        if !self.sounding() {
            return [0.0; 3];
        }
        if self.active != 0 {
            self.panel_tail = (1.5 * self.rate) as usize;
        } else {
            self.panel_tail -= 1;
        }
        let mut result = [0.0; 3];
        let choke_frames = (self.rate * 0.004).ceil();
        for substep in 0..self.substeps {
            let mut p = Pickups::default();
            let mut mask = self.active;
            while mask != 0 {
                let i = mask.trailing_zeros() as usize;
                mask &= mask - 1;
                let bit = 1u128 << i;
                let v = &mut self.voices[i];
                if v.restrike != 0 && v.string.tip == REST {
                    v.string.press(v.restrike);
                    v.restrike = 0;
                }
                let x = v.tick(self.even);
                let gain = if v.choke != 0 {
                    v.choke as f64 / choke_frames
                } else {
                    1.0
                };
                p.centre += gain * x.centre;
                p.bridge += gain * x.bridge;
                if substep + 1 == self.substeps {
                    if v.choke != 0 {
                        v.choke -= 1;
                        if v.choke == 0 {
                            v.clear();
                            self.active &= !bit;
                        }
                    } else if !v.string.pressed && v.restrike == 0 {
                        v.released = v.released.wrapping_add(1);
                        if v.released & 511 == 0 && v.quiet() {
                            v.clear();
                            self.active &= !bit;
                        }
                    }
                }
            }
            self.even = !self.even;
            let jack = self.panel.tick(Pickups {
                centre: p.centre * PICKUP_VOLTS,
                bridge: p.bridge * PICKUP_VOLTS,
            });
            self.decimator.push([jack, p.centre, p.bridge]);
            // Read phase zero. Its input time is exactly 8 host frames behind this frame.
            if substep == 0 {
                result = if STEMS {
                    self.decimator.read()
                } else {
                    [self.decimator.read::<1>()[0], 0., 0.]
                };
            }
        }
        result
    }

    pub fn sample(&mut self) -> f32 {
        self.frame::<false>()[0] as f32
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out {
            *sample = self.sample();
        }
    }
    /// Same sample origin and decimator for the two raw stems and the loaded jack.
    pub fn sample_with_pickups(&mut self) -> (Pickups, f32) {
        let [jack, centre, bridge] = self.frame::<true>();
        (Pickups { centre, bridge }, jack as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(values: &[f32]) -> f64 {
        (values.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
    }

    #[test]
    fn e7_power_switch_preserves_both_pickups_on_a_held_string() {
        let mut e7 = Engine::new(48_000.);
        let mut d6 = Engine::new(48_000.);
        e7.set_board(Board::E7);
        e7.set_power(false);
        for engine in [&mut d6, &mut e7] {
            engine.note_on(64, 0.7);
        }
        let mut d6_energy = 0.;
        let mut e7_energy = 0.;
        let mut difference = 0.;
        for i in 0..4096 {
            if i == 2048 {
                e7.set_power(true);
            }
            let (a, d6_jack) = d6.sample_with_pickups();
            let (b, e7_jack) = e7.sample_with_pickups();
            assert_eq!((a.centre, a.bridge), (b.centre, b.bridge));
            if i < 2048 {
                assert_eq!(e7_jack, 0.0);
            } else {
                e7_energy += e7_jack * e7_jack;
                difference += (d6_jack - e7_jack).powi(2);
                d6_energy += d6_jack * d6_jack;
            }
        }
        assert!(e7_energy > 0.01 && difference > d6_energy * 0.01);
    }

    #[test]
    fn full_compass_uses_winding_only_on_its_first_23_keys() {
        assert!(scale(LOW - 1).is_none() && scale(HIGH + 1).is_none());
        for n in LOW..=HIGH {
            let k = scale(n).unwrap();
            assert_eq!(k.winding_mass_per_m > 0.0, n <= 51);
            assert!(0.0 < k.bridge_m && k.bridge_m < k.centre_m && k.centre_m < k.length);
        }
        for n in [46, 64, 78] {
            let k = scale(n).unwrap();
            assert_eq!(k.length, key(n).unwrap().length);
        }
    }

    #[test]
    fn extended_ends_keep_independent_notes_and_physical_pitch() {
        let rate = 48_000.;
        let mut e = Engine::extended(rate);
        for note in [0, 28, 89, 127] {
            e.reset();
            e.note_on(note, 0.75);
            assert_eq!(e.held_notes(), 1u128 << note);
            let hz = 440. * 2_f64.powf((f64::from(note) - 69.) / 12.);
            let size = ((12. * rate / hz).ceil() as usize).max(4096);
            let mut p = Vec::with_capacity(size);
            for _ in 0..2400 {
                e.sample();
            }
            for _ in 0..size {
                p.push(e.sample_with_pickups().0.centre);
            }
            // Reject an octave fold or a clamp to the nearest stock pitch.
            let projection = |frequency: f64| {
                let mut iq = [0.; 2];
                for (i, &sample) in p.iter().enumerate() {
                    let phase = 2. * PI * i as f64 / rate * frequency;
                    let win = 0.5 - 0.5 * (2. * PI * i as f64 / size as f64).cos();
                    iq[0] += sample * win * phase.sin();
                    iq[1] += sample * win * phase.cos();
                }
                iq[0].hypot(iq[1])
            };
            assert!(projection(hz) > 20. * projection(hz * 0.5), "MIDI {note}");
            e.note_off(note);
            assert_eq!(e.held_notes(), 0);
        }
        e.note_on(0, 0.5);
        e.note_on(127, 0.5);
        e.choke(127);
        assert_eq!(e.held_notes(), 1);
    }

    #[test]
    fn centre_height_moves_one_sensor_without_moving_the_string() {
        let mut stock = Engine::new(44100.);
        let mut moved = Engine::extended(44100.);
        for e in [&mut stock, &mut moved] {
            e.note_on(64, 0.75);
        }
        for _ in 0..2000 {
            assert_eq!(stock.sample(), moved.sample());
        }
        moved.set_pickup_distance_mm(1., 0.);
        let mut energy = [0.; 2];
        for _ in 0..4000 {
            let a = stock.sample_with_pickups().0;
            let b = moved.sample_with_pickups().0;
            assert_eq!(a.bridge, b.bridge);
            energy[0] += a.centre.powi(2);
            energy[1] += b.centre.powi(2);
        }
        assert!(energy[1] < 0.7 * energy[0]);
    }

    #[test]
    fn physical_mute_attenuates_the_held_string_and_both_raw_pickups() {
        let mut dry = Engine::new(48_000.0);
        let mut muted = Engine::new(48_000.0);
        muted.set_controls(Switch::CA, Wiring::DaAdds, ALL_TONES, 1.0, 1.0);
        dry.note_on(64, 0.7);
        muted.note_on(64, 0.7);
        let mut energy = [[0.0; 2]; 2];
        for frame in 0..14_400 {
            let (p, _) = dry.sample_with_pickups();
            if frame >= 9600 {
                energy[0][0] += p.centre.powi(2);
                energy[0][1] += p.bridge.powi(2);
            }
            let (p, _) = muted.sample_with_pickups();
            if frame >= 9600 {
                energy[1][0] += p.centre.powi(2);
                energy[1][1] += p.bridge.powi(2);
            }
        }
        let [dry, muted] = energy;
        for (i, (dry, muted)) in dry.into_iter().zip(muted).enumerate() {
            assert!(muted < dry * 0.25, "pickup {i}");
        }
    }

    #[test]
    fn midi_and_audition_owners_release_independently_and_restrike_in_place() {
        let mut e = Engine::new(44100.);
        let mut block = [0.; 2048];
        e.note_on(46, 0.7);
        e.render(&mut block);
        assert!(rms(&block) > 0.001);
        e.audition(46, 0.8);
        e.note_off(46);
        assert_eq!(e.held_notes(), 1u128 << 46);
        e.render(&mut block);
        assert!(e.voices[usize::from(46 - LOW)].string.pressed);
        e.audition_off();
        assert_eq!(e.held_notes(), 0);
        e.render(&mut block[..200]);
        e.note_on(46, 0.55);
        e.audition(64, 0.6);
        e.audition_off();
        assert_eq!(e.held_notes(), 1u128 << 46);
        e.render(&mut block);
        assert!(rms(&block) > 0.001);
        e.choke(46);
        e.reset();
        assert!(!e.sounding() && e.held_notes() == 0);
        e.render(&mut block);
        assert!(block.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn hard_full_compass_holds_are_finite_and_release_retires_every_key() {
        for rate in [44_100., 48_000., 96_000.] {
            let mut e = Engine::new(rate);
            for n in LOW..=HIGH {
                e.note_on(n, 1.0);
            }
            let mut block = [0.0; 512];
            for _ in 0..6 {
                e.render(&mut block);
                assert!(block.iter().all(|x| x.is_finite() && x.abs() < 100.0));
            }
            for n in LOW..=HIGH {
                e.note_off(n);
            }
            for _ in 0..(rate * 5.0 / 512.0) as usize {
                e.render(&mut block);
                assert!(block.iter().all(|x| x.is_finite()));
                if !e.sounding() {
                    break;
                }
            }
            assert!(!e.sounding(), "{rate} Hz left mask {:x}", e.active);
        }
    }
}
