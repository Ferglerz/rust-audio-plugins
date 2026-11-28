//! A soundboardless spinet string course projects its motion over the coil head.
use crate::amp::{Amplifier, Controls, Output};
use pleasant_dsp::struck_string::{CourseSpec, LiveCourse, Readout};

pub const LOW: u8 = 21;
pub const HIGH: u8 = 108;
/// One admissible plate ordering with all three documented plate sizes and
/// equal 44-key runs on T1's two 9-ohm inputs. Order requires inspection.
pub const PLATES: [u8; 8] = [8, 10, 10, 16, 8, 10, 10, 16];
pub fn plate(note: u8) -> Option<usize> {
    if !(LOW..=HIGH).contains(&note) {
        return None;
    }
    let mut n = note - LOW;
    PLATES.iter().position(|&width| {
        if n < width {
            true
        } else {
            n -= width;
            false
        }
    })
}

/// Estimated register scale of a short upright. The reference drawing depicts
/// a single string at the sensor and does not list a string scale or unisons.
pub fn scale(note: u8) -> Option<CourseSpec> {
    let _ = plate(note)?;
    let f = 440. * 2f64.powf((f64::from(note) - 69.) / 12.);
    let n = f64::from(note);
    let t = (n - 21.) / 87.;
    // Below the wound/plain transition, mass loading keeps bass lengths inside the case.
    let length = if note <= 49 {
        1.09 * ((0.69_f64 / 1.09).powf((n - 21.) / 28.))
    } else {
        0.69 * (0.048_f64 / 0.69).powf((n - 49.) / 59.)
    };
    let bass = ((49. - n) / 28.).max(0.);
    let stiffness = 0.00011 + 0.0013 * bass * bass + 0.0014 * t.powi(5);
    let loss = 2.0 + 6.0 * t * t;
    // The owner-recording fit uses one smooth register transition for the
    // effective hammer mass and felt stiffness, without per-note corrections.
    let treble = ((n - 51.) / 27.).clamp(0., 1.);
    Some(CourseSpec {
        frequency_hz: f,
        strings: 1,
        stiffness,
        length,
        tension: 560.,
        hammer_kg: 0.010 * (0.0025_f64 / 0.010).powf(t) * 0.45_f64.powf(treble),
        contact_k: 8e6 * (9.0_f64).powf(t) * 2.0_f64.powf(treble),
        contact_linear: 600.,
        strike_fraction: 0.128 - 0.027 * t,
        unison_cents: 0.,
        read_imbalance: 0.,
        losses_db_s: [
            loss,
            loss * 1.16,
            loss * 1.45,
            loss * 1.8,
            loss * 2.2,
            loss * 2.6,
        ],
        hammer_width_m: 0.009 - 0.005 * t,
        has_damper: note <= 88,
        damper_loss: 46. + 24. * t,
        readout: Readout::Displacement {
            fraction: (0.060 / length).min(0.40),
            aperture_m: 0.030,
        },
    })
}

struct Voice {
    course: LiveCourse,
    plate: usize,
    voicing: f64,
    flux: f64,
    last_x: f64,
    choke: u32,
}
impl Voice {
    fn tick(&mut self, rate: f64, field_width: f64) -> f64 {
        let x = self.course.host_sample();
        // Difference flux linkage around rest; positive coefficient corresponds
        // to magnetizing the string along the already established pole field.
        // A fixed, assumed 3 mm effective field width bounds large excursions.
        let linked = 0.000002 * (x / field_width).atan();
        self.last_x = x;
        let voltage = -(linked - self.flux) * rate;
        self.flux = linked;
        voltage
    }
    fn clear(&mut self) {
        self.course.clear();
        self.flux = 0.;
        self.last_x = 0.;
        self.choke = 0;
    }
}

#[derive(Default, Clone, Copy)]
pub struct Taps {
    pub pickups_v: [f64; 2],
    pub plate_emf_v: [f64; 8],
    pub amplifier: Output,
}

pub struct Engine {
    voices: Vec<Voice>,
    amp: Amplifier,
    rate: f64,
    held: u128,
    extended: bool,
    field_width: f64,
    active: u128,
    audition: Option<u8>,
    sustain: bool,
    soft: bool,
    tail: bool,
    quiet_samples: u32,
    tail_hold: u32,
    phase: u8,
    fade: u32,
}
impl Engine {
    pub fn new(rate: f64) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let steps = (176400. / rate).ceil().max(1.) as usize;
        let voices = (0_u8..=127)
            .map(|n| {
                let nearest = n.clamp(LOW, HIGH);
                let mut spec = scale(nearest).unwrap();
                if n != nearest {
                    spec.frequency_hz = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
                }
                Voice {
                    course: LiveCourse::at_rates(spec, rate * steps as f64, steps),
                    plate: plate(nearest).unwrap(),
                    voicing: 1.,
                    flux: 0.,
                    last_x: 0.,
                    choke: 0,
                }
            })
            .collect();
        Self {
            voices,
            amp: Amplifier::new(rate, Controls::default()),
            rate,
            held: 0,
            extended: false,
            field_width: 0.003,
            active: 0,
            audition: None,
            sustain: false,
            soft: false,
            tail: false,
            quiet_samples: 0,
            tail_hold: (rate * 0.08).ceil() as u32,
            phase: 0,
            fade: (rate * 0.004).ceil() as u32,
        }
    }
    fn accepts(&self, note: u8) -> bool {
        note <= 127 && (self.extended || (LOW..=HIGH).contains(&note))
    }

    /// The factory compass is active by default. Retire only newly disabled keys.
    pub fn set_extended(&mut self, enabled: bool) {
        if self.extended && !enabled {
            for n in (0..LOW).chain(HIGH + 1..=127) {
                if (self.active | self.held_notes()) & (1u128 << n) != 0 {
                    self.choke(n);
                }
            }
        }
        self.extended = enabled;
    }

    /// Physical pickup setback in millimeters. Zero preserves native regulation.
    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        if !offset.is_finite() {
            return;
        }
        let width = 0.003 + 0.001 * offset.clamp(-0.25, 2.);
        if self.field_width != width {
            self.field_width = width;
            let mut keys = self.active;
            while keys != 0 {
                let v = &mut self.voices[keys.trailing_zeros() as usize];
                keys &= keys - 1;
                v.flux = 0.000002 * (v.last_x / width).atan();
            }
        }
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.clear();
            v.course.sustain(false);
        }
        self.held = 0;
        self.active = 0;
        self.audition = None;
        self.sustain = false;
        self.soft = false;
        self.tail = false;
        self.quiet_samples = 0;
        self.phase = 0;
        self.amp.reset();
    }
    pub fn set_controls(&mut self, controls: Controls) {
        self.amp.set_controls(controls);
    }
    /// Signed relative coil/longitudinal-string voicing, unity by default.
    pub fn set_voicing(&mut self, n: u8, factor: f64) {
        if (LOW..=HIGH).contains(&n) && factor.is_finite() {
            self.voices[usize::from(n)].voicing = factor.clamp(-2., 2.);
        }
    }
    pub fn set_sustain(&mut self, down: bool) {
        if down != self.sustain {
            self.sustain = down;
            for v in &mut self.voices {
                v.course.sustain(down);
            }
        }
    }
    pub fn set_soft(&mut self, down: bool) {
        self.soft = down;
    }
    fn strike(&mut self, n: u8, v: f32) {
        self.active |= 1u128 << n;
        let voice = &mut self.voices[usize::from(n)];
        let speed = (0.16 + 3.4 * f64::from(v.clamp(0., 1.)).powf(1.5))
            * if self.soft { 0.62 } else { 1.0 };
        voice.choke = 0;
        voice.course.strike_speed(speed);
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        if !self.accepts(n) || !v.is_finite() {
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
        if n <= 127 {
            self.held &= !(1u128 << n);
            if self.audition != Some(n) {
                self.voices[usize::from(n)].course.release();
            }
        }
    }
    pub fn audition(&mut self, n: u8, v: f32) {
        if self.accepts(n) && v.is_finite() && v > 0. {
            self.audition_off();
            self.audition = Some(n);
            self.strike(n, v);
        }
    }
    pub fn audition_off(&mut self) {
        if let Some(n) = self.audition.take() {
            if self.held & (1u128 << n) == 0 {
                self.voices[usize::from(n)].course.release();
            }
        }
    }
    pub fn held_notes(&self) -> u128 {
        self.held | self.audition.map_or(0, |n| 1u128 << n)
    }
    pub fn choke(&mut self, n: u8) {
        if n <= 127 {
            if self.audition == Some(n) {
                self.audition = None;
            }
            self.note_off(n);
            self.voices[usize::from(n)].choke = self.fade;
        }
    }
    pub fn sounding(&self) -> bool {
        self.active != 0 || self.tail
    }
    pub fn next_outputs(&mut self) -> Taps {
        if !self.sounding() {
            return Taps::default();
        }
        self.phase = self.phase.wrapping_add(1);
        let mut plate_emf_v = [0.; 8];
        let mut mask = self.active;
        while mask != 0 {
            let n = mask.trailing_zeros();
            mask &= mask - 1;
            let v = &mut self.voices[n as usize];
            let emf = v.tick(self.rate, self.field_width) * v.voicing;
            if v.choke != 0 {
                plate_emf_v[v.plate] += emf * f64::from(v.choke) / f64::from(self.fade);
                v.choke -= 1;
                if v.choke != 0 {
                    continue;
                }
            } else {
                plate_emf_v[v.plate] += emf;
                if self.phase != 0 || !v.course.quiet(1e-13) {
                    continue;
                }
            }
            v.clear();
            self.active &= !(1u128 << n);
        }
        let pickups_v = [plate_emf_v[..4].iter().sum(), plate_emf_v[4..].iter().sum()];
        let amplifier = self.amp.tick(pickups_v);
        // A bias-bypass capacitor can retain an inaudible charge for seconds.
        // Preserve the complete electrical tail through 80 ms under -150 dBV.
        self.quiet_samples = if self.active == 0 && amplifier.auxiliary_v.abs() < 3e-8 {
            self.quiet_samples + 1
        } else {
            0
        };
        self.tail = self.quiet_samples < self.tail_hold
            && (self.active != 0 || self.phase != 0 || !self.amp.at_rest());
        if !self.tail {
            self.amp.reset();
        }
        Taps {
            pickups_v,
            plate_emf_v,
            amplifier,
        }
    }
    pub fn render(&mut self, samples: &mut [f32]) {
        for s in samples {
            *s = self.next_outputs().amplifier.auxiliary_v as f32;
        }
    }
}
