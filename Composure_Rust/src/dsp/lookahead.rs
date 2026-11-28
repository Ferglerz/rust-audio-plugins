//! Lookahead circular buffer ported from `02_InputProcessing/01_dsp_utils.jsfx-inc`.

/// Maximum lookahead slider value (ms) — matches Composure.jsfx slider10.
const MAX_LOOKAHEAD_MS: f64 = 2000.0;
const DELAY_CROSSFADE_MS: f64 = 20.0;

/// Buffer size for full 2000ms at `srate`, rounded up to next power of 2 (JSFX parity).
pub fn buffer_size_for_srate(srate: f64) -> usize {
    let needed = (MAX_LOOKAHEAD_MS * 0.001 * srate).ceil() as usize;
    (needed + 1).next_power_of_two()
}

#[derive(Debug, Clone)]
pub struct LookaheadBuffer {
    buffer_l: Vec<f64>,
    buffer_r: Vec<f64>,
    mask: usize,
    pos: usize,
    delay_samples: usize,
    tap_delay: usize,
    delay_from: usize,
    crossfade_remaining: usize,
    crossfade_total: usize,
    srate: f64,
    has_history: bool,
}

impl LookaheadBuffer {
    pub fn new(srate: f64) -> Self {
        // Prepare the full slider range before processing starts. A later move
        // from zero to positive lookahead must not allocate in the callback.
        let buffer_size = buffer_size_for_srate(srate);
        Self {
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            mask: buffer_size - 1,
            pos: 0,
            delay_samples: 0,
            tap_delay: 0,
            delay_from: 0,
            crossfade_remaining: 0,
            crossfade_total: (DELAY_CROSSFADE_MS * 0.001 * srate).ceil().max(1.0) as usize,
            srate,
            has_history: false,
        }
    }

    pub fn set_sample_rate(&mut self, srate: f64) {
        if self.srate != srate {
            let delay_ms = self.delay_samples as f64 * 1000.0 / self.srate;
            *self = Self::new(srate);
            self.update_delay_ms(delay_ms);
        }
    }

    pub fn update_delay_ms(&mut self, lookahead_ms: f64) {
        let ms = lookahead_ms.clamp(0.0, MAX_LOOKAHEAD_MS);
        self.delay_samples = ((ms * self.srate / 1000.0).floor() as usize).min(self.mask);
        if !self.has_history {
            self.tap_delay = self.delay_samples;
        }
    }

    pub fn latency_samples(&self) -> u32 {
        self.delay_samples as u32
    }

    fn read_delayed(&self, delay: usize) -> (f64, f64) {
        let delayed_pos = self.pos.wrapping_sub(delay) & self.mask;
        (self.buffer_l[delayed_pos], self.buffer_r[delayed_pos])
    }

    pub fn process(&mut self, input_l: f64, input_r: f64) -> (f64, f64) {
        // Keep L/R history warm at zero latency. Read both stationary taps
        // through a fade before starting another fade to the latest request.
        self.has_history = true;
        if self.crossfade_remaining == 0 && self.tap_delay != self.delay_samples {
            self.delay_from = self.tap_delay;
            self.tap_delay = self.delay_samples;
            self.crossfade_remaining = self.crossfade_total;
        }
        self.buffer_l[self.pos] = input_l;
        self.buffer_r[self.pos] = input_r;

        let (out_l, out_r) = if self.crossfade_remaining > 0 {
            let (new_l, new_r) = self.read_delayed(self.tap_delay);
            let (old_l, old_r) = self.read_delayed(self.delay_from);
            let t = 1.0 - (self.crossfade_remaining as f64 / self.crossfade_total as f64);
            self.crossfade_remaining -= 1;
            (old_l * (1.0 - t) + new_l * t, old_r * (1.0 - t) + new_r * t)
        } else if self.tap_delay > 0 {
            self.read_delayed(self.tap_delay)
        } else {
            (input_l, input_r)
        };

        self.pos = (self.pos + 1) & self.mask;
        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.pos = 0;
        self.crossfade_remaining = 0;
        self.tap_delay = self.delay_samples;
        self.has_history = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_latency_has_prepared_capacity() {
        let la = LookaheadBuffer::new(48000.0);
        assert_eq!(la.buffer_l.len(), 131_072);
        assert_eq!(la.latency_samples(), 0);
    }

    #[test]
    fn first_positive_delay_reuses_prepared_buffers() {
        let mut la = LookaheadBuffer::new(48000.0);
        let left = la.buffer_l.as_ptr();
        let right = la.buffer_r.as_ptr();
        la.update_delay_ms(1.0);
        assert_eq!(la.buffer_l.as_ptr(), left);
        assert_eq!(la.buffer_r.as_ptr(), right);
        assert_eq!(la.latency_samples(), 48);
    }

    #[test]
    fn buffer_size_covers_2000ms_at_48k() {
        assert_eq!(buffer_size_for_srate(48000.0), 131_072);
        let mut la = LookaheadBuffer::new(48000.0);
        la.update_delay_ms(2000.0);
        assert_eq!(la.latency_samples(), 96_000);
        assert_eq!(la.buffer_l.len(), 131_072);
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
    fn live_delay_crossfades_from_and_to_zero_with_continuous_history() {
        let mut delay = LookaheadBuffer::new(1000.0);
        for i in 0..100 {
            delay.process(i as f64, -(i as f64));
        }
        delay.update_delay_ms(40.0);
        for i in 100..120 {
            if i == 105 {
                delay.update_delay_ms(8.0);
                delay.update_delay_ms(0.0);
            }
            let expected = i as f64 - 40.0 * (i - 100) as f64 / 20.0;
            let (l, r) = delay.process(i as f64, -(i as f64));
            assert!((l - expected).abs() < 1e-12 && (r + expected).abs() < 1e-12);
        }
        for i in 120..=140 {
            let expected = i as f64 - 40.0 * (140 - i) as f64 / 20.0;
            let (l, r) = delay.process(i as f64, -(i as f64));
            assert!((l - expected).abs() < 1e-12 && (r + expected).abs() < 1e-12);
        }
        assert_eq!(delay.latency_samples(), 0);
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
        assert!(
            l.abs() > 0.0,
            "buffer should retain audio after delay change"
        );
    }
}
