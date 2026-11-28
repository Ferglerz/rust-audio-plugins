//! Compile a small linear RLC network into fixed-size trapezoidal state updates.
//! Node 0 is AC ground. Right-hand sides store reactive history and external inputs.
#[derive(Clone)]
pub struct Filter<const S: usize, const I: usize = 1, const O: usize = 1> {
    state: [f64; S],
    update: [[f64; S]; S],
    drive: [[f64; I]; S],
    read: [[f64; S]; O],
    direct: [[f64; I]; O],
}
impl<const S: usize, const I: usize, const O: usize> Default for Filter<S, I, O> {
    fn default() -> Self {
        Self {
            state: [0.; S],
            update: [[0.; S]; S],
            drive: [[0.; I]; S],
            read: [[0.; S]; O],
            direct: [[0.; I]; O],
        }
    }
}
impl<const S: usize, const I: usize, const O: usize> Filter<S, I, O> {
    pub fn scale_output(&mut self, port: usize, scale: f64) {
        for gain in self.read[port].iter_mut().chain(&mut self.direct[port]) {
            *gain *= scale;
        }
    }

    pub fn reset(&mut self) {
        self.state.fill(0.);
    }

    /// Absolute bound on the companion history sources, expressed in volts.
    pub fn history_below(&self, volts: f64) -> bool {
        self.state.iter().all(|x| x.abs() < volts)
    }

    /// Direct source-to-terminal coefficients for algebraic loading.
    pub fn feedthrough(&self) -> &[[f64; I]; O] {
        &self.direct
    }

    /// Read all named nodes at the present history without advancing time.
    #[inline]
    pub fn observe(&self, x: [f64; I]) -> [f64; O] {
        std::array::from_fn(|i| {
            self.direct[i]
                .iter()
                .zip(x)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                + self.read[i]
                    .iter()
                    .zip(self.state)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
        })
    }

    /// Apply the source amplitudes, returning all named output nodes.
    #[inline]
    pub fn tick_ports(&mut self, x: [f64; I]) -> [f64; O] {
        let out = self.observe(x);
        let history = |row: &[f64; S]| row.iter().zip(self.state).map(|(a, b)| a * b).sum::<f64>();
        let drive = |row: &[f64; I]| row.iter().zip(x).map(|(a, b)| a * b).sum::<f64>();
        self.state = std::array::from_fn(|i| drive(&self.drive[i]) + history(&self.update[i]));
        out
    }
}

impl<const S: usize> Filter<S> {
    #[inline]
    pub fn tick(&mut self, x: f64) -> f64 {
        self.tick_ports([x])[0]
    }
}

/// Two independent circuit histories during a control transition. The latest
/// prepared setting waits for an in-flight fade, so fast retargets keep both
/// endpoints continuous. Compilation and storage stay off the sample path.
pub struct CrossfadingFilter<const S: usize, const I: usize = 1, const O: usize = 1> {
    active: Filter<S, I, O>,
    previous: Box<Filter<S, I, O>>,
    pending: Box<Filter<S, I, O>>,
    blended_direct: [[f64; I]; O],
    remaining: u32,
    length: u32,
    started: bool,
    queued: bool,
}

impl<const S: usize, const I: usize, const O: usize> CrossfadingFilter<S, I, O> {
    pub fn new(rate: f64) -> Self {
        Self::with_length((rate * 0.005).round() as u32)
    }

    /// A control-rate model can interpolate over its own update interval.
    pub fn with_length(samples: u32) -> Self {
        Self {
            active: Filter::default(),
            previous: Box::default(),
            pending: Box::default(),
            blended_direct: [[0.; I]; O],
            remaining: 0,
            length: samples.max(1),
            started: false,
            queued: false,
        }
    }

    /// Destination for a freshly compiled setting. A newly selected circuit
    /// starts with the same capacitor voltages and inductor currents.
    pub fn prepare(&mut self) -> &mut Filter<S, I, O> {
        if self.remaining != 0 {
            self.queued = true;
            return &mut self.pending;
        }
        if self.started {
            (*self.previous).clone_from(&self.active);
            self.remaining = self.length;
        }
        &mut self.active
    }

    pub fn reset(&mut self) {
        if self.queued {
            std::mem::swap(&mut self.active, &mut *self.pending);
        }
        self.active.reset();
        self.started = false;
        self.queued = false;
        self.remaining = 0;
    }

    pub fn history_below(&self, volts: f64) -> bool {
        self.active.history_below(volts)
            && (self.remaining == 0 || self.previous.history_below(volts))
    }

    fn previous_weight(&self) -> f64 {
        f64::from(self.remaining) / f64::from(self.length)
    }

    // External nonlinear loading needs the same blended terminal voltage
    // and differential admittance as the corresponding tick.
    pub fn feedthrough(&self) -> &[[f64; I]; O] {
        if self.remaining == 0 {
            &self.active.direct
        } else if self.remaining == self.length {
            &self.previous.direct
        } else {
            &self.blended_direct
        }
    }

    pub fn observe(&self, input: [f64; I]) -> [f64; O] {
        let active = self.active.observe(input);
        if self.remaining == 0 {
            return active;
        }
        let previous = self.previous.observe(input);
        let weight = self.previous_weight();
        std::array::from_fn(|i| active[i] + weight * (previous[i] - active[i]))
    }

    #[inline]
    pub fn tick_ports(&mut self, input: [f64; I]) -> [f64; O] {
        self.started = true;
        let active = self.active.tick_ports(input);
        if self.remaining == 0 {
            return active;
        }
        let previous = self.previous.tick_ports(input);
        let weight = self.previous_weight();
        self.remaining -= 1;
        if self.remaining == 0 && self.queued {
            (*self.previous).clone_from(&self.active);
            self.pending.state = self.active.state;
            std::mem::swap(&mut self.active, &mut *self.pending);
            self.remaining = self.length;
            self.queued = false;
        } else if self.remaining != 0 {
            let w = self.previous_weight();
            self.blended_direct = std::array::from_fn(|o| {
                std::array::from_fn(|i| {
                    let now = self.active.direct[o][i];
                    now + w * (self.previous.direct[o][i] - now)
                })
            });
        }
        std::array::from_fn(|i| active[i] + weight * (previous[i] - active[i]))
    }
}

impl<const S: usize> CrossfadingFilter<S> {
    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        self.tick_ports([input])[0]
    }
}

pub type Circuit<const S: usize> = SizedCircuit<S, 20, 16>;

/// A nodal system with explicit node and right-hand-side capacities.
pub struct SizedCircuit<const S: usize, const N: usize, const R: usize> {
    g: [[f64; N]; N],
    rhs: [[f64; R]; N],
    reactive: [(usize, usize, f64); S],
    count: usize,
    nodes: usize,
    rate: f64,
}
impl<const S: usize, const N: usize, const R: usize> SizedCircuit<S, N, R> {
    pub fn new(nodes: usize, rate: f64) -> Self {
        assert!(nodes < N && S < R && rate.is_finite() && rate > 0.0);
        Self {
            g: [[0.; N]; N],
            rhs: [[0.; R]; N],
            reactive: [(0, 0, 1.0); S],
            count: 0,
            nodes,
            rate,
        }
    }
    pub fn r(&mut self, a: usize, b: usize, ohms: f64) {
        if a == b {
            return;
        }
        let g = 1. / ohms;
        self.g[a][a] += g;
        self.g[b][b] += g;
        self.g[a][b] -= g;
        self.g[b][a] -= g;
    }
    pub fn c(&mut self, a: usize, b: usize, farads: f64) {
        self.reactive(a, b, 2. * self.rate * farads, 1.0);
    }
    pub fn l(&mut self, a: usize, b: usize, henries: f64) {
        self.reactive(a, b, 1. / (2. * self.rate * henries), -1.0);
    }
    // A capacitor's history is equivalent to a voltage source 2v - h at the next
    // sample. For an inductor the next source is h - 2v, from integrating voltage.
    fn reactive(&mut self, a: usize, b: usize, y: f64, sign: f64) {
        if a != b {
            self.r(a, b, 1. / y);
            self.rhs[a][self.count] += y;
            self.rhs[b][self.count] -= y;
        }
        self.reactive[self.count] = (a, b, sign);
        self.count += 1;
    }
    pub fn input(&mut self, node: usize, amps: f64) {
        self.input_port(0, node, amps);
    }
    pub fn input_port(&mut self, port: usize, node: usize, amps: f64) {
        self.rhs[node][S + port] += amps;
    }
    pub fn gm(&mut self, d: usize, s: usize, g: usize, gm: f64) {
        self.g[d][g] += gm;
        self.g[d][s] -= gm;
        self.g[s][g] -= gm;
        self.g[s][s] += gm;
    }
    /// Ideal transformer. A distinct unused matrix index holds secondary
    /// branch current. Voltage is measured at each first pin relative to its second.
    pub fn transformer(
        &mut self,
        primary: [usize; 2],
        secondary: [usize; 2],
        branch: usize,
        ratio: f64,
    ) {
        for (node, coefficient) in [
            (primary[0], -ratio),
            (primary[1], ratio),
            (secondary[0], 1.),
            (secondary[1], -1.),
        ] {
            self.g[branch][node] += coefficient;
            self.g[node][branch] += coefficient;
        }
    }

    pub fn compile(self, out: usize, filter: &mut Filter<S>) {
        self.compile_ports([out], filter);
    }

    pub fn compile_ports<const I: usize, const O: usize>(
        mut self,
        out: [usize; O],
        filter: &mut Filter<S, I, O>,
    ) {
        assert!(I > 0 && S + I <= R);
        assert_eq!(self.count, S);
        self.rhs[0].fill(0.);
        for i in 1..=self.nodes {
            let pivot = (i..=self.nodes)
                .max_by(|a, b| self.g[*a][i].abs().total_cmp(&self.g[*b][i].abs()))
                .unwrap();
            self.g.swap(i, pivot);
            self.rhs.swap(i, pivot);
            let inv = 1. / self.g[i][i];
            for x in &mut self.g[i][i..=self.nodes] {
                *x *= inv;
            }
            for x in &mut self.rhs[i][..S + I] {
                *x *= inv;
            }
            for row in 1..=self.nodes {
                if row == i {
                    continue;
                }
                let k = self.g[row][i];
                for col in i..=self.nodes {
                    self.g[row][col] -= k * self.g[i][col];
                }
                for col in 0..S + I {
                    self.rhs[row][col] -= k * self.rhs[i][col];
                }
            }
        }
        for (i, (a, b, sign)) in self.reactive.into_iter().enumerate() {
            if a == b && sign > 0.0 {
                // A closed ideal shunt discharges a capacitor when the controls change.
                filter.state[i] = 0.0;
                filter.update[i].fill(0.0);
                filter.drive[i].fill(0.0);
                continue;
            }
            filter.update[i] = std::array::from_fn(|j| {
                sign * (2. * (self.rhs[a][j] - self.rhs[b][j]) - f64::from(i == j))
            });
            filter.drive[i] =
                std::array::from_fn(|p| sign * (2. * (self.rhs[a][S + p] - self.rhs[b][S + p])));
        }
        for (i, node) in out.into_iter().enumerate() {
            filter.read[i].copy_from_slice(&self.rhs[node][..S]);
            filter.direct[i].copy_from_slice(&self.rhs[node][S..S + I]);
        }
    }
}

/// Incremental Child–Langmuir triodes around a given quiescent bias, with
/// finite anode collection and implicit feedback from the coupled circuit.
pub struct PushPullTriodes {
    pub gm: f64,
    pub mu: f64,
    pub idle_a: f64,
    pub plate_v: f64,
    pub knee_v: f64,
    pub parallel: f64,
    pub grids: [usize; 2],
    pub plates: [usize; 2],
    pub ports: [usize; 2],
}

impl PushPullTriodes {
    pub fn correction<const I: usize, const O: usize>(
        &self,
        base: [f64; O],
        direct: &[[f64; I]; O],
        initial: [f64; 2],
    ) -> [f64; 2] {
        let mut correction = initial;
        // Two plate-current residuals close the local plate-resistance
        // feedback through the reflected load and winding parasitics.
        for _ in 0..8 {
            let mut error = [0.; 2];
            let mut jac = [[0.; 2]; 2];
            for i in 0..2 {
                let d = direct;
                let voltage = |node: usize| {
                    base[node]
                        + d[node][self.ports[0]] * correction[0]
                        + d[node][self.ports[1]] * correction[1]
                };
                let vp = voltage(self.plates[i]);
                let drive = voltage(self.grids[i]) + vp / self.mu;
                let charge = (1. + drive * self.gm / (1.5 * self.idle_a)).max(0.);
                let grid_root = charge.sqrt();
                let grid_current = self.idle_a * charge * grid_root;
                // Anode collection tends to zero at zero plate voltage even
                // under positive grid overdrive. This Child-Langmuir ceiling
                // is fitted at the model’s estimated anode knee.
                let anode = ((self.plate_v + vp) / self.knee_v).max(0.);
                let anode_root = anode.sqrt();
                let plate_current = self.idle_a * anode * anode_root;
                let (current, dg, dp) = if grid_current < plate_current {
                    (
                        grid_current,
                        self.gm * grid_root,
                        self.gm * grid_root / self.mu,
                    )
                } else {
                    (
                        plate_current,
                        0.,
                        1.5 * self.idle_a * anode_root / self.knee_v,
                    )
                };
                let residual = self.parallel * (current - self.idle_a - self.gm * drive);
                error[i] = correction[i] - residual;
                for (k, j) in jac[i].iter_mut().enumerate() {
                    *j = f64::from(i == k)
                        - self.parallel
                            * ((dg - self.gm) * d[self.grids[i]][self.ports[k]]
                                + (dp - self.gm / self.mu) * d[self.plates[i]][self.ports[k]]);
                }
            }
            let det = jac[0][0] * jac[1][1] - jac[1][0] * jac[0][1];
            correction[0] -= (error[0] * jac[1][1] - error[1] * jac[0][1]) / det;
            correction[1] -= (jac[0][0] * error[1] - jac[1][0] * error[0]) / det;
            if error[0].abs().max(error[1].abs()) < 1e-11 {
                break;
            }
        }
        correction
    }
}

#[cfg(test)]
mod transition_tests {
    use super::*;

    #[test]
    fn circuit_controls_preserve_continuity_on_retarget_and_reset() {
        let mut pan = CrossfadingFilter::<0, 1, 2>::with_length(4);
        let position = |pan: &mut CrossfadingFilter<0, 1, 2>, value: f64| {
            pan.prepare().direct = [[value], [1. - value]];
        };
        position(&mut pan, 1.);
        assert_eq!(pan.tick_ports([1.]), [1., 0.]);
        position(&mut pan, 0.);
        assert_eq!(pan.tick_ports([1.]), [1., 0.]);
        position(&mut pan, 0.25);
        position(&mut pan, 1.);
        for gain in [0.75, 0.5, 0.25, 0., 0.25, 0.5, 0.75, 1.] {
            assert_eq!(pan.feedthrough()[0], [gain]);
            assert_eq!(pan.observe([1.]), [gain, 1. - gain]);
            assert_eq!(pan.tick_ports([1.]), [gain, 1. - gain]);
        }
        position(&mut pan, 0.5);
        position(&mut pan, 0.);
        pan.reset();
        assert_eq!(pan.tick_ports([1.]), [0., 1.]);
    }
}
