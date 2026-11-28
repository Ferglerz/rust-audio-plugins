//! Hold logic ported from `Envelope/02_envelope_hold.jsfx-inc`.

#[derive(Default)]
pub struct HoldState {
    counter_samples: i64,
    target_gr_db: f64,
    hold_samples: i64,
}

impl HoldState {
    pub fn update_hold_samples(&mut self, hold_ms: f64, srate: f64) {
        self.hold_samples = (hold_ms * 0.001 * srate).floor() as i64;
    }

    pub fn reset(&mut self) {
        self.counter_samples = 0;
        self.target_gr_db = 0.0;
    }

    pub fn is_pending(&self) -> bool {
        self.counter_samples > 0
    }

    pub fn process_hold(&mut self, target_gr_db: f64, hold_ms: f64) -> f64 {
        if hold_ms <= 0.0 || self.hold_samples == 0 {
            self.reset();
            return target_gr_db;
        }

        if target_gr_db != 0.0
            && (target_gr_db == self.target_gr_db || target_gr_db.abs() > self.target_gr_db.abs())
        {
            self.target_gr_db = target_gr_db;
            self.counter_samples = self.hold_samples;
        } else if self.is_pending() {
            self.counter_samples -= 1;
        } else {
            self.target_gr_db = target_gr_db;
        }

        self.target_gr_db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_times_the_drop_after_a_sustained_target() {
        let mut h = HoldState::default();
        h.update_hold_samples(4.0, 1000.0);
        for _ in 0..12 {
            assert_eq!(h.process_hold(-6.0, 4.0), -6.0);
            assert!(h.is_pending());
        }
        for _ in 0..4 {
            assert_eq!(h.process_hold(-3.0, 4.0), -6.0);
        }
        assert_eq!(h.process_hold(-3.0, 4.0), -3.0);

        assert_eq!(h.process_hold(-6.0, 4.0), -6.0);
        h.reset();
        assert!(!h.is_pending());
        assert_eq!(h.process_hold(0.0, 4.0), 0.0);
    }

    #[test]
    fn hold_extends_largest_gr() {
        let mut h = HoldState::default();
        h.hold_samples = 100;
        let first = h.process_hold(-6.0, 10.0);
        assert_eq!(first, -6.0);
        let held = h.process_hold(-3.0, 10.0);
        assert_eq!(held, -6.0);
    }
}
