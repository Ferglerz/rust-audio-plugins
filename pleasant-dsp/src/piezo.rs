//! Parallel piezoelectric elements feeding a resistive input via a coupling capacitor.
/// Trapezoidal charge-balance solution. The element generates charge per unit of input
/// force. All electrically connected elements contribute capacitance at every instant.
pub struct PiezoInput {
    feed: f64,
    feedback: f64,
    last_force: f64,
    voltage: f64,
}
impl PiezoInput {
    pub fn new(
        rate: f64,
        charge_per_unit: f64,
        bank_f: f64,
        coupling_f: f64,
        load_ohms: f64,
    ) -> Self {
        let series = bank_f * coupling_f / (bank_f + coupling_f);
        let tau = 2. * rate * series * load_ohms;
        Self {
            feed: charge_per_unit / bank_f * tau / (1. + tau),
            feedback: (tau - 1.) / (tau + 1.),
            last_force: 0.,
            voltage: 0.,
        }
    }
    pub fn reset(&mut self) {
        self.last_force = 0.;
        self.voltage = 0.;
    }
    pub fn tick(&mut self, summed_force: f64) -> f64 {
        self.voltage = self.feedback * self.voltage + self.feed * (summed_force - self.last_force);
        self.last_force = summed_force;
        self.voltage
    }
}
