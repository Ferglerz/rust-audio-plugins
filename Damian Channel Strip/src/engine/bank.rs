use crate::{
    band::Band,
    dsp::BandRuntime,
    processing::{Config, Delay, EqPath},
};

pub struct Bank {
    pub bands: Vec<BandRuntime>,
    pub eq2_bands: Vec<BandRuntime>,
    pub sr: f64,
    pub(super) config: Config,
    path: EqPath,
    dry: Delay,
}

impl Bank {
    pub(super) fn new(snapshot: &[Band], snapshot2: &[Band], sr: f64, config: Config) -> Self {
        let mut combined = Vec::with_capacity(snapshot.len() + snapshot2.len());
        combined.extend_from_slice(snapshot);
        combined.extend_from_slice(snapshot2);
        Self {
            bands: snapshot
                .iter()
                .cloned()
                .map(|b| BandRuntime::new(b, config.mode.rate(sr)))
                .collect(),
            eq2_bands: snapshot2
                .iter()
                .cloned()
                .map(|b| BandRuntime::new(b, config.mode.rate(sr)))
                .collect(),
            sr,
            config,
            path: EqPath::new(&combined, sr, config),
            dry: Delay::new(config.latency(sr)),
        }
    }

    pub(super) fn tick(
        &mut self,
        input: [f64; 2],
        eq1_mix: f64,
        eq2_mix: f64,
    ) -> ([f64; 2], [f64; 2]) {
        let dry = self.dry.tick(input);
        let total_eq_mix = eq1_mix.max(eq2_mix);
        if total_eq_mix == 0.0 {
            (dry, dry)
        } else {
            (
                self.path.tick_dual(
                    input,
                    &mut self.bands,
                    &mut self.eq2_bands,
                    self.sr,
                    eq1_mix,
                    eq2_mix,
                ),
                dry,
            )
        }
    }

    pub(super) fn reset(&mut self) {
        for b in &mut self.bands {
            b.reset();
        }
        for b in &mut self.eq2_bands {
            b.reset();
        }
        self.path.reset();
        self.dry.reset();
    }
}
