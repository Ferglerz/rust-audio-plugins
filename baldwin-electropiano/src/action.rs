use pleasant_dsp::{
    struck_keys::StruckAction,
    struck_string::{CourseSpec, LiveCourse, Readout},
};

pub const LOW: u8 = 21;
pub const HIGH: u8 = 108;
pub fn frequency(note: u8) -> f64 {
    440. * 2f64.powf((f64::from(note) - 69.) / 12.)
}

/// Short vertical scale of the classroom instrument. The cabinet is 31¼ inches
/// high. Bearing and core measurements by register have not survived in the spec.
pub fn length_m(note: u8) -> f64 {
    let n = note.clamp(LOW, HIGH);
    let anchors = [
        (LOW, 0.93_f64),
        (40, 0.73),
        (52, 0.55),
        (64, 0.35),
        (84, 0.118),
        (HIGH, 0.037),
    ];
    let &[(a, lo), (b, hi)] = anchors.windows(2).find(|w| n <= w[1].0).unwrap() else {
        unreachable!()
    };
    // Continue shortening the wire above C8 to keep its wave speed and tension.
    lo * (hi / lo).powf(f64::from(n - a) / f64::from(b - a)) / treble_ratio(note)
}
fn treble_ratio(note: u8) -> f64 {
    2f64.powf(f64::from(note.saturating_sub(HIGH)) / 12.)
}

pub fn tension_n(note: u8) -> f64 {
    let n = note.clamp(LOW, HIGH);
    let f = frequency(n);
    let l = length_m(n);
    // Taper the copper winding into the adjacent plain wire. Both ends of
    // the winding scale have the same linear density as their anchor string.
    let steel_density = |diameter: f64| 7850. * std::f64::consts::PI * (diameter / 2.).powi(2);
    if n < 49 {
        let low_density = 345. / (2. * frequency(LOW) * length_m(LOW)).powi(2);
        let progress = f64::from(n - LOW) / f64::from(49 - LOW);
        let density = low_density * (steel_density(0.00090) / low_density).powf(progress);
        density * (2. * f * l).powi(2)
    } else {
        let diameter = 0.00090 * 2f64.powf(-f64::from(n - 49) / 151.);
        steel_density(diameter) * (2. * f * l).powi(2)
    }
}

pub struct Action {
    course: LiveCourse,
    stiffness: f64,
}
impl Action {
    pub fn new(note: u8, rate: f64) -> Self {
        Self::with_regulation(note, rate, 0.035, 0.70, 0.75)
    }

    /// Scale the felt contact, hammer mass and strike line for bench comparison.
    pub fn with_regulation(note: u8, rate: f64, felt: f64, mass: f64, strike: f64) -> Self {
        let n = note.clamp(LOW, HIGH);
        let f = frequency(note);
        let t_ratio = treble_ratio(note);
        let l = length_m(note);
        let t = tension_n(n);
        let x = f64::from(n - LOW) / f64::from(HIGH - LOW);
        let core = if n < 49 {
            0.00072 + (f64::from(n - LOW) / 28.) * 0.00018
        } else {
            0.00090 * 2f64.powf(-f64::from(n - 49) / 151.)
        };
        let area_moment = std::f64::consts::PI * core.powi(4) / 64.;
        let stiffness = std::f64::consts::PI.powi(2) * 2.0e11 * area_moment / (t * l * l);
        // Owner recordings give 3–5 dB/s of early fundamental loss in the bass.
        let base_loss = 1.5 + 2.4 * x.powi(2) + 2.2 * (1. - x).powi(3);
        let spec = CourseSpec {
            frequency_hz: f,
            strings: 1,
            stiffness,
            length: l,
            tension: t,
            hammer_kg: mass * 0.009 * (0.39_f64).powf(x) / t_ratio,
            contact_k: felt * 4.7e8 * (1. + x) * t_ratio.powi(2),
            contact_linear: felt * 11000. * t_ratio,
            strike_fraction: strike * (0.135 - 0.038 * x),
            unison_cents: 0.,
            read_imbalance: 0.,
            losses_db_s: [1., 1.2, 1.7, 2.4, 3.3, 4.4].map(|v| v * base_loss),
            hammer_width_m: 0.010 * (0.24_f64).powf(x) / t_ratio,
            has_damper: n < 90,
            damper_loss: 59.,
            readout: Readout::BearingForce,
        };
        let steps = (176400. / rate).ceil().max(1.) as usize;
        Self {
            course: LiveCourse::at_rates(spec, rate * steps as f64, steps),
            stiffness,
        }
    }
    pub fn inharmonicity(&self) -> f64 {
        self.stiffness
    }
}
impl StruckAction for Action {
    fn strike_key(&mut self, v: f32) {
        self.course
            .strike_speed(0.32 + 3.2 * f64::from(v.clamp(0., 1.)).powf(1.4));
    }
    fn release(&mut self) {
        self.course.release();
    }
    fn sustain(&mut self, down: bool) {
        self.course.sustain(down);
    }
    fn step_output(&mut self, _gap: f64) -> f64 {
        self.course.host_sample()
    }
    fn clear(&mut self) {
        self.course.clear();
    }
    fn quiet(&self) -> bool {
        self.course.quiet(1e-7)
    }
}
