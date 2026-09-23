/// Convert decibels to a linear amplitude ratio.
#[inline]
pub fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

/// Convert a linear amplitude ratio to decibels using pleasant-ui's legacy floor.
#[inline]
pub fn linear_to_db(lin: f64) -> f64 {
    linear_to_db_with_floor(lin, 1.0e-9)
}

/// Convert a linear amplitude ratio to decibels with an explicit floor.
#[inline]
pub fn linear_to_db_with_floor(lin: f64, floor: f64) -> f64 {
    20.0 * lin.max(floor).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_preserve_legacy_values() {
        for db in [-60.0, -18.0, 0.0, 6.0, 24.0] {
            assert!((linear_to_db(db_to_linear(db)) - db).abs() < 1.0e-10);
        }
        assert_eq!(linear_to_db(0.0), -180.0);
        assert_eq!(linear_to_db_with_floor(0.0, 1.0e-12), -240.0);
    }
}
