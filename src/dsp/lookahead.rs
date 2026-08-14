//! Lookahead circular buffer ported from `02_InputProcessing/01_dsp_utils.jsfx-inc`.

/// Maximum lookahead slider value (ms) — matches Composure.jsfx slider10.
const MAX_LOOKAHEAD_MS: f64 = 2000.0;
const DELAY_CROSSFADE_MS: f64 = 20.0;

/// Buffer size for full 2000ms at `srate`, rounded up to next power of 2 (JSFX parity).
pub fn buffer_size_for_srate(srate: f64) -> usize {
    let needed = (MAX_LOOKAHEAD_MS * 0.001 * srate).ceil() as usize;
    needed.max(1).next_power_of_two()
}

#[derive(Debug, Clone)]
pub struct LookaheadBuffer {
    buffer_l: Vec<f64>,
    buffer_r: Vec<f64>,
    buffer_size: usize,
    mask: usize,
    pos: usize,
    delay_samples: usize,
    delay_from: usize,
    crossfade_remaining: usize,
    crossfade_total: usize,
    srate: f64,
    /// Sample-quantized delay last applied — skip redundant updates while smoothing.
    last_applied_delay_samples: usize,
}

impl LookaheadBuffer {
    pub fn new(srate: f64) -> Self {
        Self {
            buffer_l: Vec::new(),
            buffer_r: Vec::new(),
            buffer_size: 0,
            mask: 0,
            pos: 0,
            delay_samples: 0,
            delay_from: 0,
            crossfade_remaining: 0,
            crossfade_total: 0,
            srate,
            last_applied_delay_samples: 0,
        }
    }

    fn ensure_allocated(&mut self) {
        if self.buffer_size > 0 {
            return;
        }
        let buffer_size = buffer_size_for_srate(self.srate);
        self.buffer_l = vec![0.0; buffer_size];
        self.buffer_r = vec![0.0; buffer_size];
        self.buffer_size = buffer_size;
        self.mask = buffer_size - 1;
    }

    pub fn set_sample_rate(&mut self, srate: f64) {
        self.srate = srate;
        if self.buffer_size > 0 {
            let new_size = buffer_size_for_srate(srate);
            if new_size != self.buffer_size {
                self.buffer_l.resize(new_size, 0.0);
                self.buffer_r.resize(new_size, 0.0);
                self.buffer_size = new_size;
                self.mask = new_size - 1;
                self.pos = 0;
                self.crossfade_remaining = 0;
            }
        }
    }

    pub fn update_delay_ms(&mut self, lookahead_ms: f64) {
        if lookahead_ms <= 0.0 && self.crossfade_remaining == 0 && self.delay_samples == 0 {
            self.last_applied_delay_samples = 0;
            return;
        }
        self.ensure_allocated();
        let new_samples = ((lookahead_ms * 0.001 * self.srate).floor() as usize)
            .min(self.buffer_size.saturating_sub(1));
        if new_samples == self.last_applied_delay_samples && self.crossfade_remaining == 0 {
            return;
        }
        self.set_delay_samples(new_samples);
        self.last_applied_delay_samples = new_samples;
    }

    fn set_delay_samples(&mut self, new_samples: usize) {
        if new_samples == self.delay_samples && self.crossfade_remaining == 0 {
            return;
        }
        if self.crossfade_remaining == 0
            && new_samples != self.delay_samples
            && self.delay_samples > 0
            && new_samples > 0
        {
            self.delay_from = self.delay_samples;
            self.crossfade_total =
                (DELAY_CROSSFADE_MS * 0.001 * self.srate).ceil() as usize;
            self.crossfade_remaining = self.crossfade_total.max(1);
        }
        self.delay_samples = new_samples;
    }

    pub fn latency_samples(&self) -> u32 {
        self.delay_samples as u32
    }

    fn read_delayed(&self, delay: usize) -> (f64, f64) {
        if delay == 0 {
            return (0.0, 0.0);
        }
        let delayed_pos = (self.pos + self.buffer_size - delay) & self.mask;
        (
            self.buffer_l[delayed_pos],
            self.buffer_r[delayed_pos],
        )
    }

    pub fn process(&mut self, input_l: f64, input_r: f64) -> (f64, f64) {
        if self.delay_samples == 0 && self.crossfade_remaining == 0 {
            return (input_l, input_r);
        }

        self.buffer_l[self.pos] = input_l;
        self.buffer_r[self.pos] = input_r;

        let (out_l, out_r) = if self.crossfade_remaining > 0 {
            let (new_l, new_r) = self.read_delayed(self.delay_samples);
            let (old_l, old_r) = self.read_delayed(self.delay_from);
            let t = 1.0 - (self.crossfade_remaining as f64 / self.crossfade_total as f64);
            self.crossfade_remaining -= 1;
            (
                old_l * (1.0 - t) + new_l * t,
                old_r * (1.0 - t) + new_r * t,
            )
        } else if self.delay_samples > 0 {
            self.read_delayed(self.delay_samples)
        } else {
            (input_l, input_r)
        };

        self.pos = (self.pos + 1) & self.mask;
        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        if self.buffer_size > 0 {
            self.buffer_l.fill(0.0);
            self.buffer_r.fill(0.0);
        }
        self.pos = 0;
        self.crossfade_remaining = 0;
        self.last_applied_delay_samples = self.delay_samples;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_latency_no_allocation() {
        let la = LookaheadBuffer::new(48000.0);
        assert_eq!(la.buffer_size, 0);
        assert_eq!(la.latency_samples(), 0);
    }

    #[test]
    fn buffer_size_covers_2000ms_at_48k() {
        assert_eq!(buffer_size_for_srate(48000.0), 131_072);
        let mut la = LookaheadBuffer::new(48000.0);
        la.update_delay_ms(2000.0);
        assert_eq!(la.latency_samples(), 96_000);
        assert_eq!(la.buffer_size, 131_072);
    }

    #[test]
    fn zero_latency_passthrough() {
        let mut la = LookaheadBuffer::new(48000.0);
        la.update_delay_ms(0.0);
        let (l, r) = la.process(0.5, -0.3);
        assert!((l - 0.5).abs() < 1e-12);
        assert!((r - (-0.3)).abs() < 1e-12);
    }

    #[test]
    fn delay_verification() {
        let srate = 1000.0;
        let mut la = LookaheadBuffer::new(srate);
        la.update_delay_ms(1.0);
        assert_eq!(la.latency_samples(), 1);
        la.process(1.0, 1.0);
        let (l, _) = la.process(2.0, 2.0);
        assert!((l - 1.0).abs() < 1e-12);
    }

    #[test]
    fn delay_change_does_not_zero_buffer() {
        let srate = 1000.0;
        let mut la = LookaheadBuffer::new(srate);
        la.update_delay_ms(2.0);
        la.process(1.0, 1.0);
        la.process(2.0, 2.0);
        la.update_delay_ms(1.0);
        let (l, _) = la.process(3.0, 3.0);
        assert!(l.abs() > 0.0, "buffer should retain audio after delay change");
    }
}
