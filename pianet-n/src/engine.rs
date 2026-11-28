//! Preallocated keyboard and common pickup bank. All public render data are mono.
use crate::{
    cembalet,
    circuit::{magnetic_jack, NPanel},
    Model, Observation, Reed,
};
use pleasant_dsp::circuit::Filter;
use std::f64::consts::PI;
use std::ops::RangeInclusive;

const OS: usize = 2;
const TAPS: usize = 65;
pub const LATENCY: u32 = (TAPS / 2 / OS) as u32;

enum Key {
    Pad(Reed),
    Plectrum(cembalet::Key),
}
impl Key {
    fn step(&mut self) -> Observation {
        match self {
            Self::Pad(k) => k.step(),
            Self::Plectrum(k) => k.step(),
        }
    }
    fn press(&mut self, v: u8) {
        match self {
            Self::Pad(k) => k.press(v),
            Self::Plectrum(k) => k.press(v),
        }
    }
    fn release(&mut self) {
        match self {
            Self::Pad(k) => k.release(),
            Self::Plectrum(k) => k.release(),
        }
    }
    fn reset(&mut self) {
        match self {
            Self::Pad(k) => k.reset(),
            Self::Plectrum(k) => k.reset(),
        }
    }
}
struct Slot {
    in_audio_band: bool,
    reed: Key,
    rest_c: f64,
    observation: Observation,
    // Receive complete pad return and bond recovery before parking the voice.
    tail: Option<usize>,
    choke: usize,
}

enum Output {
    C(Box<crate::early::CPanel>),
    L(Box<crate::early::LPanel>),
    N(Box<NPanel>),
    T(Filter<3>),
    Rf(Box<cembalet::Panel>),
    CembaletN(Box<NPanel<11>>),
}

pub struct Engine {
    model: Model,
    keys: Vec<Slot>,
    sounding: Vec<usize>,
    pressed: u128,
    audition: Option<u8>,
    sample_rate: f64,
    panel: Output,
    monitor: f64,
    monitor_target: f64,
    monitor_gains: [f64; 2],
    monitor_step: f64,
    history: [f64; TAPS],
    taps: [f64; TAPS],
    cursor: usize,
    extended: bool,
    gap_mm: f32,
    pad_release: f32,
    sustain_mod: bool,
    sustain_pedal: bool,
}
impl Engine {
    pub fn new(model: Model, sample_rate: f64) -> Self {
        Self::with_load(model, sample_rate, 100e3)
    }
    pub fn with_load(model: Model, sample_rate: f64, load: f64) -> Self {
        assert!(sample_rate.is_finite() && sample_rate >= 8_000.0);
        assert!(load.is_finite() && load > 0.0);
        let solver_rate = sample_rate * OS as f64;
        let keys = (0..=127)
            .map(|m| {
                let mut reed = if model.is_cembalet() {
                    Key::Plectrum(
                        cembalet::Key::extended(model, m, solver_rate).expect("MIDI plectrum"),
                    )
                } else {
                    Key::Pad(Reed::at_rate_extended(model, m, solver_rate).expect("MIDI pad"))
                };
                let observation = reed.step();
                Slot {
                    in_audio_band: crate::beam::midi_hz(m) < 0.5 * sample_rate,
                    reed,
                    rest_c: observation.capacitance,
                    observation,
                    tail: None,
                    choke: 0,
                }
            })
            .collect::<Vec<_>>();
        let mut taps = std::array::from_fn(|i| {
            let x = i as f64 - (TAPS / 2) as f64;
            let a = 2.0 * PI * i as f64 / (TAPS - 1) as f64;
            let fc = 0.45 / OS as f64;
            let sinc = if x == 0.0 {
                2.0 * fc
            } else {
                (2.0 * PI * fc * x).sin() / (PI * x)
            };
            sinc * (0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos())
        });
        let norm = taps.iter().sum::<f64>();
        taps.iter_mut().for_each(|w| *w /= norm);
        Self {
            model,
            monitor: 0.,
            monitor_target: 0.,
            monitor_gains: [1.; 2],
            monitor_step: 1. / (0.004 * solver_rate),
            panel: match model {
                Model::C => Output::C(Box::new(crate::early::CPanel::new(solver_rate, load, 200.))),
                Model::L => Output::L(Box::new(crate::early::LPanel::new(solver_rate, load))),
                Model::N => Output::N(Box::new(NPanel::new(solver_rate))),
                Model::T => Output::T(magnetic_jack(solver_rate, load)),
                Model::Cembalet => Output::Rf(Box::new(cembalet::Panel::new(solver_rate, load))),
                Model::CembaletN => {
                    Output::CembaletN(Box::new(NPanel::with_load(solver_rate, load)))
                }
            },
            sample_rate,
            sounding: Vec::with_capacity(keys.len()),
            keys,
            pressed: 0,
            audition: None,
            history: [0.; TAPS],
            taps,
            cursor: 0,
            extended: false,
            gap_mm: 0.,
            pad_release: 1.,
            sustain_mod: false,
            sustain_pedal: false,
        }
    }
    /// Select the native C headphone or L speaker terminals for audition. Both
    /// electrical paths remain connected and retain their reactive state.
    pub fn set_early_monitor(&mut self, monitor: crate::early::Monitor) {
        use crate::early::Monitor::*;
        self.monitor_target = f64::from(matches!(
            (self.model, monitor),
            (Model::C, Phones) | (Model::L, Speakers)
        ));
    }
    /// Fixed host output alignment, applied to the two electrical jacks before
    /// their monitor crossfade. The default preserves their native voltages.
    pub fn set_early_output_gains(&mut self, external: f64, local: f64) {
        assert!(external.is_finite() && local.is_finite() && external >= 0. && local >= 0.);
        self.monitor_gains = [external, local];
    }
    /// Set the knee/foot swell and the installed modulation switch.
    pub fn set_n_controls(&mut self, swell: f32, vibrato: bool) {
        match &mut self.panel {
            Output::N(p) => p.set(swell, vibrato),
            Output::CembaletN(p) => p.set(swell, vibrato),
            Output::Rf(p) => p.set(swell, vibrato),
            Output::C(p) => p.set(swell, vibrato),
            Output::L(p) => p.set(swell),
            Output::T(_) => {}
        }
    }
    pub fn key_range(&self) -> RangeInclusive<u8> {
        if self.extended {
            0..=127
        } else {
            self.model.key_range()
        }
    }
    pub fn set_extended(&mut self, extended: bool) {
        if self.extended && !extended {
            for n in 0..=127 {
                if !self.model.key_range().contains(&n) {
                    self.choke(n);
                }
            }
        }
        self.extended = extended;
    }
    pub fn set_pickup_distance_mm(&mut self, offset: f32) {
        if self.gap_mm == offset {
            return;
        }
        self.gap_mm = offset;
        for slot in &mut self.keys {
            match &mut slot.reed {
                Key::Pad(k) => {
                    k.set_pickup_distance_mm(offset);
                    slot.observation.capacitance = k.current_capacitance();
                    slot.observation.flux_wb_turns = k.current_flux();
                }
                Key::Plectrum(k) => {
                    k.set_pickup_distance_mm(offset);
                    slot.observation.capacitance = k.capacitance(slot.observation.reed_tip);
                    slot.rest_c = k.resting_capacitance();
                }
            }
        }
    }
    pub fn set_pad_release(&mut self, amount: f32) {
        if self.pad_release == amount {
            return;
        }
        self.pad_release = amount;
        for slot in &mut self.keys {
            if let Key::Pad(k) = &mut slot.reed {
                k.set_pad_release(amount);
                slot.observation.capacitance = k.current_capacitance();
                slot.observation.flux_wb_turns = k.current_flux();
            }
        }
    }
    /// Enable the optional pad-arm catch. Factory keyboards return their pads
    /// on key-up. Both Cembalet actions have plectra and bypass this control.
    pub fn set_sustain_mod(&mut self, enabled: bool) {
        if self.sustain_mod != enabled {
            self.sustain_mod = enabled;
            if !enabled {
                self.drop_catches();
            }
        }
    }
    pub fn set_sustain(&mut self, pedal_down: bool) {
        if self.sustain_pedal != pedal_down {
            self.sustain_pedal = pedal_down;
            if !pedal_down {
                self.drop_catches();
            }
        }
    }
    fn drop_catches(&mut self) {
        for &i in &self.sounding {
            let slot = &mut self.keys[i];
            if let Key::Pad(k) = &mut slot.reed {
                if k.release_sustain() && slot.choke == 0 {
                    slot.tail = Some((self.sample_rate * 0.5) as usize);
                }
            }
        }
    }
    pub fn pressed(&self) -> u128 {
        self.pressed | self.audition.map_or(0, |n| 1u128 << n)
    }
    pub fn caught_pads(&self) -> u128 {
        if !(self.sustain_mod && self.sustain_pedal) {
            return 0;
        }
        self.sounding.iter().fold(0, |mask, &i| {
            if matches!(&self.keys[i].reed, Key::Pad(reed) if reed.catch == crate::PadCatch::Held) {
                mask | (1u128 << i)
            } else {
                mask
            }
        })
    }
    pub fn sounding(&self) -> bool {
        !self.sounding.is_empty()
    }
    /// Last physical observation at an individual key, including a parked reed.
    pub fn observation(&self, midi: u8) -> Option<Observation> {
        self.index(midi).map(|i| self.keys[i].observation)
    }
    fn index(&self, midi: u8) -> Option<usize> {
        self.key_range().contains(&midi).then(|| usize::from(midi))
    }
    pub fn note_on(&mut self, midi: u8, velocity: f32) {
        if velocity <= 0.0 {
            self.note_off(midi);
            return;
        }
        let Some(i) = self.index(midi) else {
            return;
        };
        self.pressed |= 1u128 << midi;
        self.strike(i, velocity);
    }
    fn strike(&mut self, i: usize, velocity: f32) {
        let slot = &mut self.keys[i];
        slot.reed
            .press((velocity.clamp(0., 1.) * 127.).round().max(1.0) as u8);
        slot.tail = None;
        slot.choke = 0;
        if !self.sounding.contains(&i) {
            self.sounding.push(i);
        }
    }
    pub fn note_off(&mut self, midi: u8) {
        let Some(i) = self.index(midi) else {
            return;
        };
        if self.pressed & (1u128 << midi) == 0 {
            return;
        }
        self.pressed &= !(1u128 << midi);
        if self.audition != Some(midi) {
            self.lift(i);
        }
    }
    fn lift(&mut self, i: usize) {
        let slot = &mut self.keys[i];
        if self.sustain_mod && self.sustain_pedal {
            if let Key::Pad(k) = &mut slot.reed {
                if k.hold_for_sustain() {
                    slot.tail = None;
                    return;
                }
            }
        }
        slot.reed.release();
        slot.tail = Some((self.sample_rate * 0.5) as usize);
    }
    pub fn audition(&mut self, note: Option<u8>, velocity: f32) {
        let next = note.filter(|n| self.key_range().contains(n) && velocity > 0.0);
        if self.audition == next {
            return;
        }
        if let Some(old) = self.audition.take() {
            if self.pressed & (1u128 << old) == 0 {
                self.lift(usize::from(old));
            }
        }
        self.audition = next;
        if let Some(n) = next {
            self.strike(usize::from(n), velocity);
        }
    }
    pub fn choke(&mut self, midi: u8) {
        let Some(i) = self.index(midi) else {
            return;
        };
        self.pressed &= !(1u128 << midi);
        if self.audition == Some(midi) {
            self.audition = None;
        }
        if self.sounding.contains(&i) {
            let frames = (self.sample_rate * 0.004).round() as usize;
            let slot = &mut self.keys[i];
            slot.choke = frames;
            slot.tail = Some(frames);
        }
    }
    fn park(&mut self, i: usize) {
        let slot = &mut self.keys[i];
        slot.reed.reset();
        slot.observation = slot.reed.step();
        slot.tail = None;
        slot.choke = 0;
    }
    pub fn reset(&mut self) {
        while let Some(i) = self.sounding.pop() {
            self.park(i);
        }
        self.pressed = 0;
        self.audition = None;
        self.sustain_pedal = false;
        match &mut self.panel {
            Output::N(f) => f.reset(),
            Output::T(f) => f.reset(),
            Output::CembaletN(f) => f.reset(),
            Output::Rf(f) => f.reset(),
            Output::C(f) => f.reset(),
            Output::L(f) => f.reset(),
        };
        self.monitor = self.monitor_target;
        self.history.fill(0.);
        self.cursor = 0;
    }
    pub fn render(&mut self, buffer: &mut [f32]) {
        for sample in buffer {
            for _ in 0..OS {
                let mut source = 0.0;
                for &i in &self.sounding {
                    let slot = &mut self.keys[i];
                    let old = slot.observation;
                    let o = slot.reed.step();
                    let pickup = match self.model {
                        // The N manuals specify 300 V at the electrode. This is
                        // a provisional C/L operating point on their rectified 280 VAC supply.
                        Model::N | Model::CembaletN | Model::C | Model::L => {
                            300.0 * (o.capacitance - old.capacitance)
                        }
                        Model::Cembalet => o.capacitance - slot.rest_c,
                        // Winding reversal is explicit. Magnet/reed polarity per key
                        // is unknown; keep equal positive flux for equal reed motion.
                        Model::T => {
                            if i % 2 == usize::from(*Model::T.key_range().start()) % 2 {
                                o.flux_wb_turns - old.flux_wb_turns
                            } else {
                                old.flux_wb_turns - o.flux_wb_turns
                            }
                        }
                    };
                    let gain = if slot.choke == 0 {
                        1.0
                    } else {
                        slot.choke as f64 / (self.sample_rate * 0.004).round()
                    };
                    if slot.in_audio_band {
                        source += pickup * gain;
                    }
                    slot.observation = o;
                }
                let rate = self.sample_rate * OS as f64;
                self.history[self.cursor] = match &mut self.panel {
                    Output::N(f) => f.tick(source * rate),
                    Output::T(f) => f.tick(source * rate),
                    Output::CembaletN(f) => f.tick(source * rate),
                    Output::Rf(f) => f.tick(source),
                    Output::C(f) => {
                        let t = f.tick(source * rate);
                        mix_monitor(
                            t.line * self.monitor_gains[0],
                            t.headphones * self.monitor_gains[1],
                            &mut self.monitor,
                            self.monitor_target,
                            self.monitor_step,
                        )
                    }
                    Output::L(f) => {
                        let t = f.tick(source * rate);
                        mix_monitor(
                            t.line * self.monitor_gains[0],
                            t.speaker * self.monitor_gains[1],
                            &mut self.monitor,
                            self.monitor_target,
                            self.monitor_step,
                        )
                    }
                };
                self.cursor = (self.cursor + 1) % TAPS;
            }
            *sample = self.history[self.cursor..]
                .iter()
                .chain(&self.history[..self.cursor])
                .zip(&self.taps)
                .map(|(a, b)| a * b)
                .sum::<f64>() as f32;
            let mut pos = 0;
            while pos < self.sounding.len() {
                let i = self.sounding[pos];
                if let Some(t) = &mut self.keys[i].tail {
                    *t -= 1;
                    if *t == 0 {
                        self.sounding.swap_remove(pos);
                        self.park(i);
                        continue;
                    }
                }
                self.keys[i].choke = self.keys[i].choke.saturating_sub(1);
                pos += 1;
            }
        }
    }
}

fn mix_monitor(external: f64, secondary: f64, mix: &mut f64, target: f64, step: f64) -> f64 {
    *mix += (target - *mix).clamp(-step, step);
    external + *mix * (secondary - external)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sustain_mod_only_catches_released_pad_actions() {
        for model in Model::ALL {
            let mut native = Engine::new(model, 48000.);
            let mut pedal = Engine::new(model, 48000.);
            pedal.set_sustain(true);
            for e in [&mut native, &mut pedal] {
                e.note_on(64, 0.7);
                e.render(&mut [0.; 2048]);
                e.note_off(64);
            }
            let mut a = [0.; 3072];
            let mut b = a;
            native.render(&mut a);
            pedal.render(&mut b);
            assert_eq!(a, b, "factory pedal bypass {model:?}");
            for e in [&mut native, &mut pedal] {
                e.reset();
                e.note_on(64, 0.7);
                e.audition(Some(64), 0.7);
                e.render(&mut a);
                e.note_off(64);
                assert_eq!(e.pressed(), 1 << 64);
            }
            pedal.set_sustain(true);
            pedal.set_sustain_mod(true);
            for e in [&mut native, &mut pedal] {
                e.audition(None, 0.);
                assert_eq!(e.pressed(), 0);
            }
            native.render(&mut a);
            pedal.render(&mut b);
            if model.is_cembalet() {
                assert_eq!(a, b, "plectrum {model:?}");
                assert_eq!(pedal.caught_pads(), 0);
                continue;
            }
            assert_eq!(pedal.caught_pads(), 1u128 << 64);
            assert!(!pedal.observation(64).unwrap().bonded);
            assert!(native.observation(64).unwrap().bonded);
            for _ in 0..9 {
                pedal.render(&mut b);
            }
            assert!(pedal.sounding(), "released pad was parked {model:?}");
            pedal.note_on(67, 0.8);
            pedal.render(&mut b);
            pedal.set_sustain_mod(false);
            assert_eq!(pedal.caught_pads(), 0);
            pedal.render(&mut b);
            assert!(pedal.observation(64).unwrap().bonded);
            assert!(!pedal.observation(67).unwrap().bonded);
            pedal.set_sustain_mod(true);
            pedal.note_off(67);
            pedal.render(&mut b);
            assert!(!pedal.observation(67).unwrap().bonded);
            pedal.choke(67);
            pedal.render(&mut b);
            assert!(pedal.observation(67).unwrap().bonded);
            pedal.reset();
            pedal.note_on(64, 0.7);
            pedal.render(&mut b);
            pedal.note_off(64);
            pedal.render(&mut b);
            assert!(pedal.observation(64).unwrap().bonded, "reset retained CC64");
        }
    }

    #[test]
    fn sustain_restrike_reseats_before_plucking_the_ringing_reed() {
        for model in [Model::C, Model::L, Model::N, Model::T] {
            let mut k = Reed::at_rate(model, 64, 96000.).unwrap();
            assert!(!k.hold_for_sustain(), "pedal plucked a resting reed");
            k.press(95);
            for _ in 0..5000 {
                k.step();
            }
            assert!(k.hold_for_sustain());
            let mut quiet = k.clone();
            quiet.set_pad_release(0.);
            for _ in 0..5000 {
                let a = k.step();
                let b = quiet.step();
                assert!(!a.bonded);
                assert_eq!(a.capacitance, b.capacitance);
                assert_eq!(a.flux_wb_turns, b.flux_wb_turns);
            }
            for mut reed in [k.clone(), quiet] {
                reed.press(95);
                assert!(!reed.release_sustain(), "pedal-up aborted a new stroke");
                let mut touched = false;
                let mut detached = false;
                for _ in 0..12000 {
                    let o = reed.step();
                    touched |= o.bonded;
                    detached |= touched && !o.bonded;
                }
                assert!(detached, "{model:?} restrike failed to reseat and pluck");
            }
            let mut quiet = k.clone();
            quiet.set_pad_release(0.);
            assert!(k.release_sustain() && quiet.release_sustain());
            let mut collision = false;
            let mut noise_differs = false;
            for _ in 0..10000 {
                let a = k.step();
                let b = quiet.step();
                collision |= a.bonded;
                let delta = a.capacitance != b.capacitance || a.flux_wb_turns != b.flux_wb_turns;
                assert!(!delta || collision, "return noise before contact");
                assert_eq!(a.reed_tip, b.reed_tip);
                noise_differs |= delta;
            }
            assert!(collision && noise_differs, "pedal-up failed to reseat {model:?}");
        }
    }

    #[test]
    fn keyboard_ownership_survives_pointer_and_midi_release() {
        for model in Model::ALL {
            let mut engine = Engine::new(model, 48_000.);
            engine.note_on(60, 0.75);
            engine.audition(Some(60), 0.8);
            engine.note_off(60);
            assert_eq!(engine.pressed(), 1u128 << 60);
            engine.note_on(62, 0.8);
            engine.audition(Some(62), 0.9);
            assert_eq!(engine.pressed(), 1u128 << 62);
            engine.audition(None, 0.1);
            assert_eq!(engine.pressed(), 1u128 << 62);
            engine.choke(62);
            engine.reset();
            let mut silence = [1.; 128];
            engine.render(&mut silence);
            assert!(!engine.sounding() && silence == [0.; 128]);
        }
    }

    #[test]
    fn all_native_keys_sound_and_reject_outside_the_native_range() {
        for model in Model::ALL {
            let mut engine = Engine::new(model, 44_100.);
            let last = *model.key_range().end();
            for midi in model.key_range() {
                engine.note_on(midi, 0.7);
                let mut audio = [0.; 2048];
                engine.render(&mut audio);
                assert!(
                    audio.iter().all(|s| s.is_finite()) && audio.iter().any(|s| s.abs() > 1e-4),
                    "{model:?} MIDI {midi}"
                );
                engine.reset();
            }
            engine.note_on(28, 1.);
            engine.audition(Some(last + 1), 1.);
            assert_eq!(engine.pressed(), 0);
        }
    }

    #[test]
    fn extended_endpoints_keep_their_own_pitches_and_native_gate_releases_them() {
        use std::f64::consts::TAU;
        for model in Model::ALL {
            let mut e = Engine::new(model, 48000.);
            e.set_extended(true);
            for n in [0, 127] {
                e.reset();
                e.note_on(n, 0.68);
                assert_eq!(e.pressed(), 1u128 << n);
                let mut y = vec![0.; if n == 0 { 96000 } else { 3000 }];
                e.render(&mut y);
                let start = if n == 0 { 7000 } else { 900 };
                let wave = &y[start..];
                let project = |hz: f64| {
                    let mut a = 0.;
                    let mut b = 0.;
                    for (i, &x) in wave.iter().enumerate() {
                        let w = (0.5 - 0.5 * (TAU * i as f64 / (wave.len() - 1) as f64).cos())
                            * f64::from(x);
                        let (s, c) = (TAU * hz * (i as f64) / 48000.).sin_cos();
                        a += w * s;
                        b += w * c;
                    }
                    a.hypot(b)
                };
                let hz = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
                let peak = project(hz);
                assert!(
                    peak.is_finite() && peak > 1e-7,
                    "{model:?} {n} has no fundamental {peak}"
                );
                assert!(
                    peak > project(hz * 0.8) * 2. && peak > project(hz * 1.2) * 2.,
                    "{model:?} {n} off pitch {peak}"
                );
                e.set_extended(false);
                assert_eq!(e.pressed(), 0);
                e.render(&mut [0.; 1024]);
                assert!(!e.sounding());
                e.set_extended(true);
            }
            e.reset();
            e.set_extended(false);
            e.note_on(64, 0.7);
            let mut native = [0.; 4096];
            e.render(&mut native);
            e.reset();
            e.set_extended(true);
            e.note_on(64, 0.7);
            let mut extended = native;
            e.render(&mut extended);
            assert_eq!(native.as_slice(), extended.as_slice());
            e.reset();
            e.set_pickup_distance_mm(0.8);
            e.note_on(64, 0.7);
            e.render(&mut extended);
            let energy = |a: &[f32]| a.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
            assert!(
                energy(&extended) < energy(&native) * 0.75,
                "pickup distance {model:?}"
            );
        }
    }

    #[test]
    fn return_noise_changes_only_the_observed_contact_transient() {
        for model in [Model::N, Model::C, Model::L, Model::T] {
            let mut native = Reed::at_rate(model, 64, 96000.).unwrap();
            let mut quiet = native.clone();
            quiet.set_pad_release(0.);
            let mut half = native.clone();
            half.set_pad_release(0.5);
            for reed in [&mut native, &mut quiet, &mut half] {
                reed.press(80);
            }
            let signal = |o: Observation| {
                if model == Model::T {
                    o.flux_wb_turns
                } else {
                    o.capacitance
                }
            };
            let mut difference = 0.;
            for i in 0..22000 {
                if i == 12000 {
                    for reed in [&mut native, &mut quiet, &mut half] {
                        reed.release();
                    }
                }
                let a = native.step();
                let b = quiet.step();
                let c = half.step();
                assert_eq!(a.reed_tip, b.reed_tip);
                assert_eq!(a.pad_force, b.pad_force);
                assert_eq!(a.bonded, b.bonded);
                assert_eq!(a.seating, b.seating);
                let (a, b, c) = (signal(a), signal(b), signal(c));
                if i <= 12000 || i > 17000 {
                    assert_eq!(a, b);
                }
                difference += (a - b).abs();
                assert!((c - (a + b) * 0.5).abs() <= 4. * f64::EPSILON * (a.abs() + b.abs()));
            }
            assert!(difference > 0., "no separate return component at {model:?}");
        }
    }

    #[test]
    fn seating_bounce_is_quieter_at_each_lower_return_setting() {
        // A leaking cup can reseal twice in one return. Restarting the transient
        // window at the second collision introduced a click on the middle reed.
        for model in [Model::N, Model::T] {
            for midi in [45, 64, 81] {
                let mut reeds = [0., 0.5, 1.].map(|mix| {
                    let mut k = Reed::at_rate(model, midi, 96000.).unwrap();
                    k.set_pad_release(mix);
                    k
                });
                let mut energy = [0.; 3];
                let mut previous = [0.; 3];
                let mut previous_delta = [0.; 3];
                for t in 0..50600 {
                    for (i, k) in reeds.iter_mut().enumerate() {
                        if t == 4800 {
                            k.press(80);
                        }
                        if t == 48000 {
                            k.release();
                        }
                        let o = k.step();
                        let raw = if model == Model::T {
                            o.flux_wb_turns
                        } else {
                            o.capacitance
                        };
                        let delta = raw - previous[i];
                        if t >= 48672 {
                            energy[i] += (delta - previous_delta[i]).powi(2);
                        }
                        previous[i] = raw;
                        previous_delta[i] = delta;
                    }
                }
                assert!(
                    energy[0] < energy[1] && energy[1] < energy[2],
                    "{model:?} MIDI {midi}: {energy:?}"
                );
            }
        }
    }

    #[test]
    fn sub_nyquist_host_tracks_high_keys_without_a_folded_tone() {
        for model in Model::ALL {
            let mut e = Engine::new(model, 8000.);
            e.set_extended(true);
            e.note_on(127, 0.8);
            assert_eq!(e.pressed(), 1u128 << 127);
            let mut out = [0.; 800];
            e.render(&mut out);
            assert!(out.iter().all(|x| x.abs() < 1e-11));
            e.set_extended(false);
            assert_eq!(e.pressed(), 0);
        }
    }

    #[test]
    fn native_monitor_keeps_companion_history_during_switch() {
        for (model, monitor) in [
            (Model::C, crate::early::Monitor::Phones),
            (Model::L, crate::early::Monitor::Speakers),
        ] {
            let sr = 48000.;
            let mut switching = Engine::new(model, sr);
            let mut native = Engine::new(model, sr);
            native.set_early_monitor(monitor);
            native.reset();
            for e in [&mut switching, &mut native] {
                e.note_on(64, 0.72);
            }
            let mut a = [0.; 3000];
            let mut b = a;
            switching.render(&mut a);
            native.render(&mut b);
            assert!(a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-4));
            switching.set_early_monitor(monitor);
            switching.render(&mut a);
            native.render(&mut b);
            assert_eq!(&a[250..], &b[250..]);
        }
    }

    #[test]
    fn host_rate_preserves_middle_note_level_and_pitch() {
        for model in Model::ALL {
            let mut levels = vec![];
            for rate in [44_100, 48_000, 96_000] {
                let mut engine = Engine::new(model, f64::from(rate));
                engine.note_on(64, 0.65);
                let mut y = vec![0.; rate as usize / 3];
                engine.render(&mut y);
                let held = &y[rate as usize / 10..];
                let rms = (held.iter().map(|s| s * s).sum::<f32>() / held.len() as f32).sqrt();
                let crossings = held.windows(2).filter(|s| s[0] <= 0. && s[1] > 0.).count();
                let f = crossings as f64 * f64::from(rate) / held.len() as f64;
                assert!((f - 329.63).abs() < 5.0, "{model:?} {rate} Hz pitch {f}");
                levels.push(rms);
            }
            for l in &levels[1..] {
                assert!(
                    (20.0 * (l / levels[0]).log10()).abs() < 1.0,
                    "{model:?} {levels:?}"
                );
            }
        }
    }
}
