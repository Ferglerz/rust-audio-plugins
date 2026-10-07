use super::types::MAX_CHANNELS;

#[derive(Default)]
pub struct DelayLine {
    rings: Vec<Vec<f32>>,
    pos: usize,
    delay_samples: usize,
    channels: usize,
}

impl DelayLine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, delay_samples: usize, channels: usize) {
        let channels = channels.clamp(1, MAX_CHANNELS);
        self.rings = vec![vec![0.0; delay_samples]; channels];
        self.pos = 0;
        self.delay_samples = delay_samples;
        self.channels = channels;
    }

    pub fn reset(&mut self) {
        for ring in &mut self.rings {
            ring.fill(0.0);
        }
        self.pos = 0;
    }

    pub fn delay_samples(&self) -> usize {
        self.delay_samples
    }

    pub fn process_frame(&mut self, frame: &mut [f32]) {
        if self.delay_samples == 0 {
            return;
        }
        let n = frame.len().min(self.channels);
        let pos = self.pos;
        for ch in 0..n {
            let ring = &mut self.rings[ch];
            let out = ring[pos];
            ring[pos] = frame[ch];
            frame[ch] = out;
        }
        self.pos = (pos + 1) % self.delay_samples;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_impulse_at(n: usize) {
        let mut delay = DelayLine::new();
        delay.prepare(n, 1);
        assert_eq!(delay.delay_samples(), n);

        let mut frame = [1.0];
        delay.process_frame(&mut frame);
        assert_eq!(frame[0], 0.0);

        for _ in 1..n {
            let mut silent = [0.0];
            delay.process_frame(&mut silent);
            assert_eq!(silent[0], 0.0);
        }

        let mut delayed = [0.0];
        delay.process_frame(&mut delayed);
        assert_eq!(delayed[0], 1.0);
    }

    #[test]
    fn impulse_appears_n_frames_later() {
        assert_impulse_at(5);
        assert_impulse_at(2048);
    }

    #[test]
    fn delay_zero_is_identity() {
        let mut delay = DelayLine::new();
        delay.prepare(0, 2);
        assert_eq!(delay.delay_samples(), 0);
        let mut frame = [0.5, -0.25];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [0.5, -0.25]);
    }

    #[test]
    fn two_channels_independent() {
        let mut delay = DelayLine::new();
        delay.prepare(3, 2);

        let mut frame = [1.0, 0.0];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [0.0, 0.0]);

        let mut frame = [0.0, 2.0];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [0.0, 0.0]);

        let mut frame = [0.0, 0.0];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [0.0, 0.0]);

        let mut frame = [0.0, 0.0];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [1.0, 0.0]);

        let mut frame = [0.0, 0.0];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [0.0, 2.0]);
    }

    #[test]
    fn reset_clears_pending_samples() {
        let mut delay = DelayLine::new();
        delay.prepare(3, 1);

        let mut frame = [1.0];
        delay.process_frame(&mut frame);
        delay.reset();

        for _ in 0..8 {
            let mut silent = [0.0];
            delay.process_frame(&mut silent);
            assert_eq!(silent[0], 0.0);
        }
        assert_eq!(delay.delay_samples(), 3);
    }

    #[test]
    fn unprepared_is_identity() {
        let mut delay = DelayLine::new();
        let mut frame = [0.7, -0.3];
        delay.process_frame(&mut frame);
        assert_eq!(frame, [0.7, -0.3]);
        assert_eq!(delay.delay_samples(), 0);
    }

    #[test]
    fn extra_channels_pass_through() {
        let mut delay = DelayLine::new();
        delay.prepare(2, 1);
        let mut frame = [1.0, 0.9];
        delay.process_frame(&mut frame);
        assert_eq!(frame[0], 0.0);
        assert_eq!(frame[1], 0.9);
    }
}
