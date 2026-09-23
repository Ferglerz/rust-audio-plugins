pub struct Delay {
    samples: Vec<[f64; 2]>,
    pos: usize,
}

impl Delay {
    pub fn new(length: usize) -> Self {
        Self {
            samples: vec![[0.0; 2]; length],
            pos: 0,
        }
    }

    pub fn tick(&mut self, x: [f64; 2]) -> [f64; 2] {
        if self.samples.is_empty() {
            return x;
        }
        let out = self.samples[self.pos];
        self.samples[self.pos] = x;
        self.pos = (self.pos + 1) % self.samples.len();
        out
    }

    pub fn reset(&mut self) {
        self.samples.fill([0.0; 2]);
        self.pos = 0;
    }
}
