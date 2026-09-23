//! Hold logic ported from `Envelope/02_envelope_hold.jsfx-inc`.

pub struct HoldState {
    pub counter_samples: i64,
    pub target_gr_db: f64,
    pub baseline_gr_db: f64,
    pub hold_samples: i64,
}

impl Default for HoldState {
    fn default() -> Self {
        Self {
            counter_samples: 0,
            target_gr_db: 0.0,
            baseline_gr_db: 0.0,
            hold_samples: 0,
        }
    }
}

impl HoldState {
    pub fn update_hold_samples(&mut self, hold_ms: f64, srate: f64) {
        self.hold_samples = (hold_ms * 0.001 * srate).floor() as i64;
    }

    pub fn process_hold(&mut self, target_gr_db: f64, hold_ms: f64) -> f64 {
        if hold_ms <= 0.0 {
            self.counter_samples = 0;
            self.target_gr_db = 0.0;
            self.baseline_gr_db = 0.0;
            return target_gr_db;
        }

        let target_gr_abs = target_gr_db.abs();
        let hold_target_abs = self.target_gr_db.abs();
        let hold_baseline_abs = self.baseline_gr_db.abs();

        if self.counter_samples > 0 {
            if target_gr_abs > hold_target_abs {
                self.target_gr_db = target_gr_db;
                self.counter_samples = self.hold_samples;
            }
            self.counter_samples -= 1;
            self.counter_samples = self.counter_samples.max(0);
            let result = self.target_gr_db;
            if self.counter_samples <= 0 {
                self.baseline_gr_db = result;
            }
            result
        } else if target_gr_abs > hold_baseline_abs {
            self.target_gr_db = target_gr_db;
            self.counter_samples = self.hold_samples;
            self.baseline_gr_db = self.target_gr_db;
            self.target_gr_db
        } else {
            self.baseline_gr_db = target_gr_db;
            self.target_gr_db = target_gr_db;
            target_gr_db
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
