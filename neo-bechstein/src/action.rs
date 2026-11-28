//! The normal hammer strikes the light striker's heel, then stops at the rail.
//! String contact after escapement is resolved by the shared stiff-string course.
use pleasant_dsp::struck_string::{CourseSpec, LiveCourse, Readout};

pub const LOW: u8 = 21;
pub const HIGH: u8 = 108;
const GRAVITY: f64 = 9.81;

pub fn strings_at(note: u8) -> usize {
    if (45..=87).contains(&note) {
        2
    } else {
        1
    }
}

/// Estimated scale for the 1.4 m instrument. The surviving Vienna report gives
/// pickup topology, but does not list speaking lengths, tension or hammer weights.
pub fn scale(note: u8) -> Option<CourseSpec> {
    if !(LOW..=HIGH).contains(&note) {
        return None;
    }
    let n = f64::from(note);
    let reg = (n - f64::from(LOW)) / f64::from(HIGH - LOW);
    let bass = ((48. - n) / 27.).max(0.);
    let f = 440. * 2_f64.powf((n - 69.) / 12.);
    let length = if note <= 48 {
        1.16 * (0.76_f64 / 1.16).powf((n - 21.) / 27.)
    } else {
        0.76 * (0.045_f64 / 0.76).powf((n - 48.) / 60.)
    };
    let loss = 0.82 + 8.8 * reg.powf(1.8);
    // Mid-register compliance fitted to the opening dyad of HMV C.2567.
    // In the unmeasured treble, progressively shorten contact with the pitch
    // period. For a quadratic spring, contact duration scales as k^(-1/3).
    let treble = ((n - 69.) / 12.).max(0.);
    let felt_rate = 2_f64.powf(0.70 * treble);
    Some(CourseSpec {
        frequency_hz: f,
        strings: strings_at(note),
        stiffness: 0.00010 + 0.0011 * bass.powi(2) + 0.0010 * reg.powi(6),
        length,
        tension: 265. + 80. * reg,
        hammer_kg: 0.00049 * (0.27_f64).powf(reg),
        contact_k: 55_200. * 3.5_f64.powf(reg) * felt_rate.powi(3),
        contact_linear: 23. * felt_rate.powi(2),
        strike_fraction: 0.123 - 0.025 * reg,
        hammer_width_m: 0.0032 - 0.0017 * reg,
        unison_cents: 1.3,
        read_imbalance: 0.035,
        losses_db_s: [
            loss,
            loss * 1.1,
            loss * 1.3,
            loss * 1.57,
            loss * 1.85,
            loss * 2.15,
        ],
        has_damper: true,
        damper_loss: 65. + 32. * reg,
        readout: Readout::Displacement {
            fraction: (0.052 / length).min(0.36),
            aperture_m: 0.014,
        },
    })
}

#[derive(Clone, Copy)]
struct Launch {
    primary_y: f64,
    primary_v: f64,
    striker_y: f64,
    striker_v: f64,
}

impl Launch {
    fn new(velocity: f32) -> Self {
        Self {
            primary_y: -0.0008,
            primary_v: 0.30 + 2.6 * f64::from(velocity.clamp(0., 1.)).powf(1.55),
            striker_y: 0.,
            striker_v: 0.,
        }
    }

    /// Velocities in m/s, coordinates in metres from the resting microhammer.
    fn tick(&mut self, dt: f64, mass: f64) -> Option<(f64, f64)> {
        const STOP: f64 = 0.00042;
        const STRING: f64 = 0.00215;
        let compression = (self.primary_y - self.striker_y).max(0.);
        let force = if compression > 0. {
            (31_000. * compression + 0.65 * (self.primary_v - self.striker_v)).max(0.)
        } else {
            0.
        };
        if self.primary_y < STOP {
            self.primary_v -= dt * force / (20. * mass);
            self.primary_y += dt * self.primary_v;
            if self.primary_y >= STOP {
                self.primary_y = STOP;
                self.primary_v = 0.;
            }
        }
        self.striker_v += dt * (force / mass - GRAVITY);
        self.striker_y += dt * self.striker_v;
        if self.striker_y <= 0. {
            self.striker_y = 0.;
            self.striker_v = self.striker_v.max(0.);
        }
        (self.striker_y >= STOP && self.striker_v > 0.)
            .then_some((self.striker_v, STRING - self.striker_y))
    }
}

pub struct Action {
    course: LiveCourse,
    launch: Option<Launch>,
    steps: usize,
    dt: f64,
    mass: f64,
    pub strings: usize,
}

impl Action {
    pub fn new(note: u8, host_rate: f64) -> Option<Self> {
        Self::from_spec(scale(note)?, host_rate)
    }
    pub fn extended(note: u8, host_rate: f64) -> Option<Self> {
        if note > 127 {
            return None;
        }
        let mut spec = scale(note.clamp(LOW, HIGH))?;
        spec.frequency_hz = 440. * 2_f64.powf((f64::from(note) - 69.) / 12.);
        Self::from_spec(spec, host_rate)
    }
    /// A measured or estimated physical course for offline regulation studies.
    pub fn from_spec(spec: CourseSpec, host_rate: f64) -> Option<Self> {
        if !host_rate.is_finite() || host_rate < 8000. {
            return None;
        }
        let steps = (176_400. / host_rate).ceil().max(1.) as usize;
        let rate = host_rate * steps as f64;
        let mut course = LiveCourse::at_rates(spec, rate, steps);
        // Light felt near a string end. Loss falls at modal nodes under the felt.
        course.configure_felt_rail(0.13, 6.5);
        Some(Self {
            course,
            launch: None,
            steps,
            dt: 1. / rate,
            mass: spec.hammer_kg,
            strings: spec.strings,
        })
    }
    pub fn strike(&mut self, velocity: f32) {
        self.course.lift_damper();
        self.launch = Some(Launch::new(velocity));
    }
    pub fn release(&mut self) {
        self.launch = None;
        self.course.release();
    }
    pub fn sustain(&mut self, down: bool) {
        self.course.sustain(down);
    }
    pub fn felt(&mut self, down: bool) {
        self.course.set_felt_rail(down);
    }
    pub fn reset(&mut self) {
        self.launch = None;
        self.course.clear();
    }
    pub fn quiet(&self) -> bool {
        self.launch.is_none() && self.course.quiet(1e-11)
    }
    /// Displacement across the sensing aperture, summed over the course's wires.
    pub fn sample(&mut self) -> f64 {
        if self.launch.is_none() {
            return self.course.host_sample();
        }
        let mut x = 0.;
        for _ in 0..self.steps {
            if let Some((speed, gap)) = self
                .launch
                .as_mut()
                .and_then(|l| l.tick(self.dt, self.mass))
            {
                self.course.launch_hammer(speed, gap, GRAVITY);
                self.launch = None;
            }
            x = self.course.step();
        }
        x
    }
}
