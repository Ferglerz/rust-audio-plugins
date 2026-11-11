//! Native, single-channel port of NARA-WPE's `online_wpe_step`.
//! MIT: Copyright (c) 2018 Communications Engineering Group, Paderborn University.
//! See licenses/NARA-WPE.txt and scripts/check-wpe-reference.py for provenance.
use rustfft::num_complex::Complex64 as C;

pub(crate) const TAPS: usize = 10;
pub(crate) const DELAY: usize = 3;

pub(crate) const ALPHA: f64 = 0.99;

struct Bin {
    covariance: [[C; TAPS]; TAPS],
    filter: [C; TAPS],
    history: Vec<C>,
    room_filter: [C; TAPS],
    window: [C; TAPS],
    numerator: [C; TAPS],
    prediction: C,
    denominator: f64,
    power: f64,
}

impl Bin {
    fn new(history: usize) -> Self {
        Self {
            covariance: std::array::from_fn(|i| {
                std::array::from_fn(|j| C::new(f64::from(i == j), 0.0))
            }),
            filter: [C::ZERO; TAPS],
            history: vec![C::ZERO; history],
            room_filter: [C::ZERO; TAPS],
            window: [C::ZERO; TAPS],
            numerator: [C::ZERO; TAPS],
            prediction: C::ZERO,
            denominator: 0.0,
            power: 0.0,
        }
    }
    fn reset(&mut self) {
        for (i, row) in self.covariance.iter_mut().enumerate() {
            row.fill(C::ZERO);
            row[i] = C::new(1.0, 0.0);
        }
        self.filter.fill(C::ZERO);
        self.room_filter.fill(C::ZERO);
        self.history.fill(C::ZERO);
        self.window.fill(C::ZERO);
        self.numerator.fill(C::ZERO);
        self.prediction = C::ZERO;
        self.denominator = 0.0;
        self.power = 0.0;
    }
}

pub(crate) struct OnlineWpe {
    bins: Vec<Bin>,
    position: usize,
    history: usize,
    lags: [usize; TAPS],
    alpha: f64,
    room_rate: f64,
}

impl OnlineWpe {
    #[cfg(test)]
    pub fn new(bins: usize) -> Self {
        Self::with_hop(bins, 480, 0.0)
    }

    pub fn with_hop(bins: usize, hop: usize, room_seconds: f64) -> Self {
        // Preserve the 40–130 ms predictor and 140 ms power history as the
        // analysis window changes. Alpha likewise retains its real-time decay.
        let lags = std::array::from_fn(|i| ((DELAY + 1 + i) * 480 + hop / 2) / hop);
        let history = ((TAPS + DELAY + 1) * 480 + hop / 2) / hop;
        Self {
            bins: (0..bins).map(|_| Bin::new(history)).collect(),
            position: 0,
            history,
            lags,
            alpha: ALPHA.powf(hop as f64 / 480.0),
            room_rate: if room_seconds > 0.0 {
                1.0 - (-(hop as f64) / (48000.0 * room_seconds)).exp()
            } else {
                1.0
            },
        }
    }

    pub fn reset(&mut self) {
        for bin in &mut self.bins {
            bin.reset();
        }
        self.position = 0;
    }

    /// Input and output are normalized complex spectra. All storage is prepared.
    /// Returns false if numerical recovery required a frame of dry passthrough.
    pub fn process(&mut self, spectrum: &mut [C]) -> bool {
        debug_assert_eq!(spectrum.len(), self.bins.len());
        let mut largest = 0.0_f64;
        for (bin, &current) in self.bins.iter_mut().zip(spectrum.iter()) {
            bin.history[self.position] = current;
            // Upstream online_wpe_step excludes delay+1 frames including current.
            for (i, x) in bin.window.iter_mut().enumerate() {
                *x = bin.history[(self.position + self.history - self.lags[i]) % self.history];
            }
            bin.power = bin.history.iter().map(|v| v.norm_sqr()).sum::<f64>() / self.history as f64;
            bin.prediction = current;
            for (g, x) in bin.filter.iter().zip(&bin.window) {
                bin.prediction -= g.conj() * x;
            }
            for (row, p) in bin.covariance.iter().zip(&mut bin.numerator) {
                *p = row.iter().zip(&bin.window).map(|(a, x)| a * x).sum();
            }
            bin.denominator = self.alpha * bin.power
                + bin
                    .window
                    .iter()
                    .zip(&bin.numerator)
                    .map(|(x, p)| (x.conj() * p).re)
                    .sum::<f64>();
            largest = largest.max(bin.denominator);
        }
        // Same relative spectral floor as NARA-WPE; no absolute input gate.
        let floor = largest * 1e-10;
        let mut valid = true;
        for (bin, output) in self.bins.iter_mut().zip(spectrum.iter_mut()) {
            // Freeze fully empty histories: forgetting through a long silence
            // otherwise grows the inverse covariance without bound.
            if bin.power == 0.0 {
                *output = C::ZERO;
                continue;
            }
            let audible = if self.room_rate < 1.0 {
                *output
                    - bin
                        .room_filter
                        .iter()
                        .zip(&bin.window)
                        .map(|(g, x)| g.conj() * x)
                        .sum::<C>()
            } else {
                bin.prediction
            };
            let inverse = 1.0 / bin.denominator.max(floor);
            for i in 0..TAPS {
                bin.filter[i] += bin.numerator[i] * inverse * bin.prediction.conj();
                // P is Hermitian, hence x^H P = (P x)^H. Preserve that symmetry
                // explicitly to avoid accumulated complex roundoff in long sessions.
                for j in i..TAPS {
                    let p = (bin.covariance[i][j]
                        - bin.numerator[i] * bin.numerator[j].conj() * inverse)
                        / self.alpha;
                    bin.covariance[i][j] = p;
                    bin.covariance[j][i] = p.conj();
                }
                bin.covariance[i][i].im = 0.0;
            }
            // A stationary tone excites only a small subspace. Bound the
            // unconstrained inverse covariance without dropping the learned
            // filter or producing periodic dry frames in the audible bins.
            let largest_diagonal = (0..TAPS)
                .map(|i| bin.covariance[i][i].re)
                .fold(0.0_f64, f64::max);
            if largest_diagonal.is_finite() && largest_diagonal > 1e6 {
                let scale = 1e6 / largest_diagonal;
                for row in &mut bin.covariance {
                    for value in row {
                        *value *= scale;
                    }
                }
            }
            let healthy = bin.prediction.re.is_finite()
                && bin.prediction.im.is_finite()
                && bin.denominator > 0.0
                && bin
                    .filter
                    .iter()
                    .all(|g| g.re.is_finite() && g.im.is_finite())
                && (0..TAPS).all(|i| {
                    let d = bin.covariance[i][i].re;
                    d.is_finite() && d > 0.0
                });
            if healthy {
                *output = audible;
                // Keep a slower frequency/phase/delay estimate of the component
                // being removed. The instantaneous residual still drives RLS;
                // only the audible predictor uses this history, never a notch EQ.
                for (room, current) in bin.room_filter.iter_mut().zip(&bin.filter) {
                    *room += self.room_rate * (*current - *room);
                }
            } else {
                bin.reset();
                valid = false;
            }
        }
        self.position = (self.position + 1) % self.history;
        valid
    }
}
