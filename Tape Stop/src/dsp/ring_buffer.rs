use crate::dsp::hermite::hermite_interpolate;

/// 2^25 sample ring buffer (~33.55 million samples, over 12 minutes of audio at 44.1kHz).
pub const BUFFER_SIZE: usize = 1 << 25;
pub const BUFFER_MASK: usize = BUFFER_SIZE - 1;

pub struct StereoRingBuffer {
    left: Vec<f32>,
    right: Vec<f32>,
    write_head: usize,
}

impl Default for StereoRingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoRingBuffer {
    pub fn new() -> Self {
        Self {
            left: vec![0.0; BUFFER_SIZE],
            right: vec![0.0; BUFFER_SIZE],
            write_head: 0,
        }
    }

    /// Reset all samples and pointers.
    pub fn reset(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.write_head = 0;
    }

    /// Write a stereo sample frame at the current write head and advance.
    #[inline(always)]
    pub fn push(&mut self, sample_l: f32, sample_r: f32) {
        let idx = self.write_head & BUFFER_MASK;
        self.left[idx] = sample_l;
        self.right[idx] = sample_r;
        self.write_head = (self.write_head + 1) & BUFFER_MASK;
    }

    /// Current integer write head index.
    #[inline(always)]
    pub fn write_head(&self) -> usize {
        self.write_head
    }

    /// Latest frame that has been written, suitable for a live playback join.
    #[inline(always)]
    pub fn live_head(&self) -> usize {
        (self.write_head + BUFFER_SIZE - 1) & BUFFER_MASK
    }

    #[inline(always)]
    fn interpolation_index(&self, index: isize) -> usize {
        let wrapped = (index as usize) & BUFFER_MASK;
        // Hermite's forward neighbors may extend beyond the recorded live edge.
        if (wrapped.wrapping_sub(self.write_head) & BUFFER_MASK) <= 1 {
            self.live_head()
        } else {
            wrapped
        }
    }

    /// Raw left sample at an integer buffer index (wraps).
    #[inline(always)]
    pub fn left_at(&self, index: isize) -> f32 {
        self.left[(index as usize) & BUFFER_MASK]
    }

    /// Raw right sample at an integer buffer index (wraps).
    #[inline(always)]
    pub fn right_at(&self, index: isize) -> f32 {
        self.right[(index as usize) & BUFFER_MASK]
    }

    /// Read an interpolated sample for the Left channel at a fractional position.
    #[inline(always)]
    pub fn read_left(&self, pos: f64) -> f32 {
        let base = pos.floor();
        let frac = (pos - base) as f32;
        let int_idx = base as isize;

        let y0 = self.left[((int_idx - 1) as usize) & BUFFER_MASK];
        let y1 = self.left[(int_idx as usize) & BUFFER_MASK];
        let y2 = self.left[self.interpolation_index(int_idx + 1)];
        let y3 = self.left[self.interpolation_index(int_idx + 2)];

        hermite_interpolate(y0, y1, y2, y3, frac)
    }

    /// Read an interpolated sample for the Right channel at a fractional position.
    #[inline(always)]
    pub fn read_right(&self, pos: f64) -> f32 {
        let base = pos.floor();
        let frac = (pos - base) as f32;
        let int_idx = base as isize;

        let y0 = self.right[((int_idx - 1) as usize) & BUFFER_MASK];
        let y1 = self.right[(int_idx as usize) & BUFFER_MASK];
        let y2 = self.right[self.interpolation_index(int_idx + 1)];
        let y3 = self.right[self.interpolation_index(int_idx + 2)];

        hermite_interpolate(y0, y1, y2, y3, frac)
    }
}
