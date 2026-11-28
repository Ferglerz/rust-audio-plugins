//! Linear trapezoidal circuit solver. Eliminate the internal nodes when a pot
//! moves. The audio loop then advances only reactive-element histories.
#[derive(Clone, Copy, Default)]
struct Branch {
    a: usize,
    b: usize,
    conductance: f64,
}

pub(super) struct Network<const N: usize, const S: usize> {
    // Index zero is ground. Other numbers match the construction below.
    g: [[f64; N]; N],
    capacitors: [Branch; S],
    count: usize,
    coil_node: usize,
    coil_admittance: f64,
    coil_z: f64,
    rate: f64,
}

#[derive(Clone)]
pub(super) struct Filter<const S: usize> {
    history: [f64; S],
    response: [[f64; S]; S],
    drive: [f64; S],
    output: [f64; S],
    direct: f64,
}

impl<const S: usize> Filter<S> {
    pub fn empty() -> Self {
        Self {
            history: [0.; S],
            response: [[0.; S]; S],
            drive: [0.; S],
            output: [0.; S],
            direct: 0.,
        }
    }
    pub fn clear(&mut self) {
        self.history.fill(0.0);
    }

    pub fn tick(&mut self, input: f64) -> f64 {
        let dot = |weights: &[f64; S]| {
            weights
                .iter()
                .zip(self.history)
                .map(|(a, b)| a * b)
                .sum::<f64>()
        };
        let out = self.direct * input + dot(&self.output);
        self.history = std::array::from_fn(|i| self.drive[i] * input + dot(&self.response[i]));
        out
    }
}

impl<const N: usize, const S: usize> Network<N, S> {
    pub fn new(rate: f64, node: usize, coil_factor: f64, extra_ohms: f64) -> Self {
        let coil_z = 2.0 * rate * 0.075 * coil_factor;
        let series = coil_z + 180.0 * coil_factor + extra_ohms;
        let mut circuit = Self {
            g: [[0.; N]; N],
            capacitors: [Branch::default(); S],
            count: 0,
            rate,
            coil_node: node,
            coil_admittance: 1. / series,
            coil_z,
        };
        circuit.resistor(node, 0, series);
        circuit
    }
    pub fn resistor(&mut self, a: usize, b: usize, r: f64) {
        let g = 1. / r.max(0.01);
        self.g[a][a] += g;
        self.g[b][b] += g;
        self.g[a][b] -= g;
        self.g[b][a] -= g;
    }
    pub fn capacitor(&mut self, a: usize, b: usize, c: f64) {
        let conductance = 2. * self.rate * c;
        self.resistor(a, b, 1. / conductance);
        self.capacitors[self.count] = Branch { a, b, conductance };
        self.count += 1;
    }
    // Small-signal collector current = gm * (Vbase - Vemitter).
    pub fn transconductance(&mut self, drain: usize, source: usize, gate: usize, gm: f64) {
        self.g[drain][gate] += gm;
        self.g[drain][source] -= gm;
        self.g[source][gate] -= gm;
        self.g[source][source] += gm;
    }
    pub fn compile(mut self, out: usize, filter: &mut Filter<S>) {
        assert_eq!(self.count + 1, S);
        // One RHS per history. The final RHS also represents the input EMF,
        // which is in series with the inductor's stored voltage.
        let mut rhs = [[0.; S]; N];
        for (i, c) in self.capacitors[..self.count].iter().enumerate() {
            rhs[c.a][i] += c.conductance;
            rhs[c.b][i] -= c.conductance;
        }
        let coil = S - 1;
        rhs[self.coil_node][coil] = self.coil_admittance;
        // Ground voltage is identically zero.
        rhs[0] = [0.; S];
        for i in 1..N {
            let pivot = (i..N)
                .max_by(|&a, &b| self.g[a][i].abs().total_cmp(&self.g[b][i].abs()))
                .unwrap();
            self.g.swap(i, pivot);
            rhs.swap(i, pivot);
            let inv = self.g[i][i].recip();
            for a in &mut self.g[i][i..] {
                *a *= inv;
            }
            for a in &mut rhs[i] {
                *a *= inv;
            }
            for row in 1..N {
                if row == i {
                    continue;
                }
                let k = self.g[row][i];
                if k == 0.0 {
                    continue;
                }
                for col in i..N {
                    self.g[row][col] -= k * self.g[i][col];
                }
                let pivot_rhs = rhs[i];
                for (entry, pivot) in rhs[row].iter_mut().zip(pivot_rhs) {
                    *entry -= k * pivot;
                }
            }
        }
        for (i, c) in self.capacitors[..self.count].iter().enumerate() {
            filter.response[i] = std::array::from_fn(|j| 2. * (rhs[c.a][j] - rhs[c.b][j]));
        }
        filter.response[coil] = std::array::from_fn(|j| {
            2. * self.coil_z
                * self.coil_admittance
                * (f64::from(j == coil) - rhs[self.coil_node][j])
        });
        for (i, row) in filter.response.iter_mut().enumerate() {
            filter.drive[i] = row[coil];
            row[i] -= 1.;
        }
        filter.output = rhs[out];
        filter.direct = rhs[out][coil];
    }
}
