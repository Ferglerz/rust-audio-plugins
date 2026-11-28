//! Native Roadmaster 64 and the independent line and soundboard observations.
use super::{Probe, Taps, Three, HIGH, LOW};
pub struct Engine {
    mechanics: Probe,
    held: u128,
    mouse: Option<u8>,
    extended: bool,
    quiet: u32,
    tail: u32,
    aux: Panel,
    last: Output,
    muted_tail: Output,
    fade_left: u32,
    fade_length: u32,
}
#[derive(Clone, Copy, Default)]
pub struct Output {
    pub sensor_v: Three,
    pub jack_v: f64,
    pub board_m_s: f64,
}
struct Panel {
    network: pleasant_dsp::circuit::Filter<7, 3>,
}
impl Panel {
    fn new(rate: f64) -> Self {
        use pleasant_dsp::circuit::{Circuit, Filter};
        let mut n = Circuit::<7>::new(7, rate);
        // Three passive pickup Thevenin sources, estimated strip lengths and
        // windings driving the original mono jack into a 1 Mohm instrument input.
        for (i, span) in [0.28, 0.40, 0.32].into_iter().enumerate() {
            let r = 2. * (span + 0.0127) * 300. * 1.36;
            let p = 2 * i + 1;
            n.input_port(i, p, 1. / r);
            n.r(p, 0, r);
            n.l(p, p + 1, 0.055 * span / 0.32);
            n.c(p + 1, 0, 120e-12);
            n.r(p + 1, 7, 10_000.);
        }
        n.r(7, 0, 1e6);
        n.c(7, 0, 470e-12);
        let mut network = Filter::default();
        n.compile_ports([7], &mut network);
        Self { network }
    }
}
impl Engine {
    pub fn new(rate: f64) -> Self {
        let rate = rate.max(22050.);
        let steps = (96000. / rate).ceil().max(1.) as usize;
        Self {
            mechanics: Probe::from_scale(rate, steps, true, &(0..=127).collect::<Vec<_>>(), true),
            held: 0,
            mouse: None,
            extended: false,
            quiet: 0,
            tail: (rate * 0.6) as u32,
            aux: Panel::new(rate),
            last: Output::default(),
            muted_tail: Output::default(),
            fade_left: 0,
            fade_length: (rate * 0.004).round() as u32,
        }
    }
    pub fn set_extended(&mut self, yes: bool) {
        self.extended = yes;
    }
    pub fn set_pickup_distance_mm(&mut self, offsets: Three) {
        for (dest, mm) in self.mechanics.gap_gain.iter_mut().zip(offsets) {
            if mm.is_finite() {
                *dest = (-mm.clamp(-1., 2.) / 3.5).exp();
            }
        }
    }
    pub fn set_sustain(&mut self, yes: bool) {
        self.mechanics.sustain(yes);
    }
    fn accepts(&self, n: u8) -> bool {
        n < 128 && (self.extended || (LOW..=HIGH).contains(&n))
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        if v <= 0. {
            self.note_off(n);
            return;
        }
        if !v.is_finite() || !self.accepts(n) {
            return;
        }
        self.held |= 1u128 << n;
        self.quiet = self.tail;
        self.mechanics.strike(usize::from(n), v.into());
    }
    pub fn note_off(&mut self, n: u8) {
        if n < 128 {
            self.held &= !(1u128 << n);
            if self.mouse != Some(n) {
                self.mechanics.release(n.into());
            }
        }
    }
    pub fn audition(&mut self, n: Option<u8>, v: f32) {
        if let Some(old) = self.mouse.take() {
            if self.held & (1u128 << old) == 0 {
                self.mechanics.release(old.into());
            }
        }
        if let Some(n) = n.filter(|&n| self.accepts(n) && v > 0. && v.is_finite()) {
            self.mouse = Some(n);
            self.mechanics.strike(n.into(), v.into());
            self.quiet = self.tail;
        }
    }
    pub fn choke(&mut self, n: u8) {
        if n >= 128 {
            return;
        }
        self.held &= !(1u128 << n);
        if self.mouse == Some(n) {
            self.mouse = None;
        }
        if self.quiet == 0 {
            return;
        }
        let k = &mut self.mechanics.keys[usize::from(n)];
        if !k.native && !k.sounding {
            return;
        }
        k.held = false;
        k.striking = false;
        k.choked = true;
        k.returning = 0;
    }
    /// CC120 clears board motion as well as every course. A short output
    /// taper removes the old voltage while a new strike may start immediately.
    pub fn all_sound_off(&mut self) {
        let previous = self.last;
        let pedal = self.mechanics.sustain;
        self.reset();
        self.set_sustain(pedal);
        self.last = previous;
        self.muted_tail = previous;
        self.fade_left = self.fade_length;
    }
    pub fn pressed(&self) -> u128 {
        self.held | self.mouse.map_or(0, |n| 1u128 << n)
    }
    pub fn sounding(&self) -> bool {
        self.quiet > 0 || self.pressed() != 0 || self.fade_left > 0
    }
    pub fn reset(&mut self) {
        self.held = 0;
        self.mouse = None;
        self.quiet = 0;
        self.fade_left = 0;
        self.muted_tail = Output::default();
        self.last = Output::default();
        let p = &mut self.mechanics;
        p.prev_flux = [0.; 3];
        p.board_x = [0.; 3];
        p.board_v = [0.; 3];
        p.sustain = false;
        p.decimator.reset();
        for k in &mut p.keys {
            k.held = false;
            k.sounding = false;
            k.choked = false;
            k.striking = false;
            k.returning = 0;
            for m in &mut k.modes {
                m.x = 0.;
                m.v = 0.;
            }
        }
        p.inverse_mass = Probe::schur(&p.keys, p.board_mass);
        self.aux.network.reset();
    }
    fn physical_outputs(&mut self) -> Output {
        if self.quiet == 0 && self.pressed() == 0 {
            return Output::default();
        }
        let Taps {
            sensor_v,
            surface_m_s,
        } = self.mechanics.sample();
        let jack_v = self.aux.network.tick_ports(sensor_v)[0];
        if self.pressed() == 0 {
            if jack_v.abs().max(surface_m_s.abs()) > 1e-8 {
                self.quiet = self.tail;
            } else {
                self.quiet = self.quiet.saturating_sub(1);
            }
        }
        Output {
            jack_v,
            sensor_v,
            board_m_s: surface_m_s,
        }
    }
    pub fn next_outputs(&mut self) -> Output {
        let mut out = self.physical_outputs();
        if self.fade_left > 0 {
            self.fade_left -= 1;
            let t = f64::from(self.fade_left) / f64::from(self.fade_length);
            let gain = t * t * (3. - 2. * t);
            out.jack_v += self.muted_tail.jack_v * gain;
            out.board_m_s += self.muted_tail.board_m_s * gain;
            for (tap, tail) in out.sensor_v.iter_mut().zip(self.muted_tail.sensor_v) {
                *tap += tail * gain;
            }
        }
        self.last = out;
        out
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for v in out {
            *v = self.next_outputs().jack_v as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_host_rates_preserve_the_native_timebase() {
        for rate in [22050., 32000., 44100., 48000., 96000., 192000., 384000.] {
            let mut engine = Engine::new(rate);
            assert!(
                (engine.mechanics.dt * engine.mechanics.steps as f64 * rate - 1.).abs() < 1e-12
            );
            engine.note_on(60, 0.75);
            let mut changes = 0;
            let mut previous = 0.;
            for i in 0..(rate * 0.1) as usize {
                let output = engine.next_outputs();
                assert!(output.jack_v.is_finite());
                let x = engine.mechanics.keys[60].modes[0].x;
                if i > (rate * 0.01) as usize && previous < 0. && x >= 0. {
                    changes += 1;
                }
                previous = x;
            }
            assert!(
                (22..=25).contains(&changes),
                "rate {rate}: {changes} C4 cycles in 90 ms"
            );
        }
    }

    #[test]
    fn extended_courses_preserve_native_gates_pitch_and_key_ownership() {
        let rate = 48_000.;
        let mut e = Engine::new(rate);
        for note in [LOW - 1, HIGH + 1] {
            e.note_on(note, 1.);
        }
        assert_eq!(e.pressed(), 0);
        assert_eq!(e.next_outputs().jack_v, 0.);
        e.set_extended(true);
        for note in [0, 127] {
            e.reset();
            e.note_on(note, 0.8);
            e.audition(Some(note), 0.8);
            e.set_extended(false);
            e.note_off(note);
            assert_eq!(e.pressed(), 1u128 << note);
            // The struck fundamental on a movable bridge can beat slightly
            // against its partner. Fit the oscillation in the individual string mode.
            let period = rate / (440. * 2f64.powf((f64::from(note) - 69.) / 12.));
            let mut crossings = Vec::new();
            let length = (period * 13.).max(1200.) as usize;
            let mut prior = 0.;
            for i in 0..length {
                assert!(e.next_outputs().jack_v.is_finite());
                let voice = e.mechanics.keys[note as usize].modes[0].x;
                if prior < 0. && voice >= 0. && i as f64 > 2. * period {
                    crossings.push(i as f64 - voice / (voice - prior));
                }
                prior = voice;
            }
            assert!(crossings.len() >= 6, "MIDI {note}: {crossings:?}");
            let mean = (crossings.last().unwrap() - crossings[0]) / (crossings.len() - 1) as f64;
            assert!(
                (mean / period - 1.).abs() < 0.015,
                "MIDI {note}: {mean} vs {period} samples"
            );
            e.audition(None, 0.);
            assert_eq!(e.pressed(), 0);
            assert!(!e.mechanics.keys[note as usize].held);
            e.set_extended(true);
        }
    }

    #[test]
    fn idle_chokes_and_cc120_restore_all_sympathetic_courses_for_the_next_chord() {
        let mut observed = Engine::new(48_000.);
        let mut isolated = Engine::new(48_000.);
        observed.set_sustain(true);
        isolated.set_sustain(true);
        for n in 0..=127 {
            observed.choke(n);
        }
        assert!(observed.mechanics.keys.iter().all(|k| !k.choked));
        let chord = [44, 51, 56, 60, 63, 67];
        let compare = |observed: &mut Engine, isolated: &mut Engine| {
            for note in chord {
                observed.note_on(note, 0.78);
                isolated.note_on(note, 0.78);
            }
            for _ in 0..7200 {
                let a = observed.next_outputs();
                let b = isolated.next_outputs();
                assert_eq!(a.jack_v, b.jack_v);
                assert_eq!(a.board_m_s, b.board_m_s);
                assert_eq!(a.sensor_v, b.sensor_v);
            }
        };
        compare(&mut observed, &mut isolated);
        // A sounding damper rail and a moving board both contribute to the
        // observed voltage. All Sound Off must remove both before the next take.
        let last = observed.last;
        assert!(last.jack_v.abs() > 1e-9);
        observed.all_sound_off();
        let first = observed.next_outputs();
        assert!((first.jack_v - last.jack_v).abs() < last.jack_v.abs() * 0.001);
        assert!(observed.mechanics.sustain);
        for _ in 0..40_000 {
            observed.next_outputs();
        }
        assert_eq!(observed.last.jack_v, 0.);
        assert!(!observed.sounding());
        isolated.reset();
        isolated.set_sustain(true);
        compare(&mut observed, &mut isolated);
        // A pitch-specific choke also expires on a passively moving neighbor.
        observed.choke(45);
        assert!(observed.mechanics.keys[45].choked);
        for _ in 0..24_000 {
            observed.next_outputs();
        }
        assert!(!observed.mechanics.keys[45].choked);
    }

    #[test]
    fn strip_setback_changes_only_the_selected_emf_and_choke_releases_with_pedal_down() {
        let mut a = Engine::new(48000.);
        let mut b = Engine::new(48000.);
        a.set_sustain(true);
        b.set_sustain(true);
        a.note_on(48, 0.8);
        b.note_on(48, 0.8);
        b.set_pickup_distance_mm([1., 0., 0.]);
        let mut peak = 0f64;
        for _ in 0..5000 {
            let (x, y) = (a.next_outputs(), b.next_outputs());
            assert_eq!(x.board_m_s, y.board_m_s);
            assert_eq!(x.sensor_v[1..], y.sensor_v[1..]);
            assert!((x.sensor_v[0] * (-1f64 / 3.5).exp() - y.sensor_v[0]).abs() < 1e-12);
            peak = peak.max((x.jack_v - y.jack_v).abs());
        }
        assert!(peak > 1e-4);
        a.choke(48);
        b.note_off(48);
        let mut e = [0.; 2];
        for i in 0..12000 {
            for (sum, model) in e.iter_mut().zip([&mut a, &mut b]) {
                let v = model.next_outputs().jack_v;
                if i > 4000 {
                    *sum += v * v;
                }
            }
        }
        assert!(e[0] < e[1] * 0.01, "choked / sustained {e:?}");
    }
}
