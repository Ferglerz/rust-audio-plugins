use crate::dsp::ring_buffer::StereoRingBuffer;

/// S-curve deceleration function shared between DSP and UI visualization.
#[inline(always)]
pub fn s_curve(t: f32, exp: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else if t < 0.5 {
        0.5 * (2.0 * t).powf(exp)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - t)).powf(exp)
    }
}

/// Semitone drop constant: 1.0 - 2^(-1/12) ≈ 0.0561256873183065
const SEMI_DROP: f64 = 1.0 - 0.9438743126816935;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TapeState {
    Idle,
    Braking,
    Crossfading,
}

pub struct TapeStopEngine {
    ring_buffer: StereoRingBuffer,
    sample_rate: f32,

    // Playhead tracking (floating-point position in ring buffer)
    play_pos_l: f64,
    play_pos_r: f64,

    // Current instantaneous playback speeds [0.0, 1.0]
    speed_l: f64,
    speed_r: f64,

    // Braking physics state
    is_braking: bool,
    brake_pos_samples: f64,
    stop_samples_l: f64,
    stop_samples_r: f64,
    max_stop_samples: f64,

    // Crossfade release state
    is_crossfading: bool,
    xfade_pos_samples: f64,
    xfade_len_samples: f64,

    // MIDI Note tracking
    active_note_count: usize,
    manual_trigger: bool,

    // Auto-Restart Transient & Envelope Detection
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
    lockout_samples: f32,
    transient_flash: f32,

    // 14-bit MIDI CC override
    cc_override_active: bool,
    target_cc_speed: f64,
    smoothed_cc_speed: f64,
    last_cc_msb: Option<u8>,
    last_cc_lsb: Option<u8>,
}

impl TapeStopEngine {
    pub fn new() -> Self {
        let sample_rate = 44100.0f32;
        let attack_coeff = 1.0 - (-1.0 / (0.001 * sample_rate)).exp();
        let release_coeff = 1.0 - (-1.0 / (0.050 * sample_rate)).exp();

        Self {
            ring_buffer: StereoRingBuffer::new(),
            sample_rate,
            play_pos_l: 0.0,
            play_pos_r: 0.0,
            speed_l: 1.0,
            speed_r: 1.0,
            is_braking: false,
            brake_pos_samples: 0.0,
            stop_samples_l: 44100.0,
            stop_samples_r: 44100.0,
            max_stop_samples: 44100.0,
            is_crossfading: false,
            xfade_pos_samples: 0.0,
            xfade_len_samples: 2205.0,
            active_note_count: 0,
            manual_trigger: false,
            envelope: 0.0,
            attack_coeff,
            release_coeff,
            lockout_samples: 0.0,
            transient_flash: 0.0,
            cc_override_active: false,
            target_cc_speed: 1.0,
            smoothed_cc_speed: 1.0,
            last_cc_msb: None,
            last_cc_lsb: None,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.attack_coeff = 1.0 - (-1.0 / (0.001 * self.sample_rate)).exp();
        self.release_coeff = 1.0 - (-1.0 / (0.050 * self.sample_rate)).exp();
        self.reset();
    }

    pub fn reset(&mut self) {
        self.ring_buffer.reset();
        self.play_pos_l = 0.0;
        self.play_pos_r = 0.0;
        self.speed_l = 1.0;
        self.speed_r = 1.0;
        self.is_braking = false;
        self.brake_pos_samples = 0.0;
        self.is_crossfading = false;
        self.xfade_pos_samples = 0.0;
        self.active_note_count = 0;
        self.manual_trigger = false;
        self.envelope = 0.0;
        self.lockout_samples = 0.0;
        self.transient_flash = 0.0;
        self.cc_override_active = false;
        self.target_cc_speed = 1.0;
        self.smoothed_cc_speed = 1.0;
        self.last_cc_msb = None;
        self.last_cc_lsb = None;
    }

    // --- MIDI Trigger Handling ---

    pub fn note_on(&mut self, _note: u8, _velocity: f32) {
        self.active_note_count += 1;
        self.evaluate_trigger_state();
    }

    pub fn note_off(&mut self, _note: u8) {
        self.active_note_count = self.active_note_count.saturating_sub(1);
        self.evaluate_trigger_state();
    }

    pub fn set_manual_trigger(&mut self, active: bool) {
        self.manual_trigger = active;
        self.evaluate_trigger_state();
    }

    pub fn clear_held_notes(&mut self) {
        self.active_note_count = 0;
        self.evaluate_trigger_state();
    }

    pub fn clear_cc_override(&mut self) {
        self.cc_override_active = false;
        self.target_cc_speed = 1.0;
        self.smoothed_cc_speed = 1.0;
        self.last_cc_msb = None;
        self.last_cc_lsb = None;
    }

    fn evaluate_trigger_state(&mut self) {
        let should_brake = self.active_note_count > 0 || self.manual_trigger;

        if should_brake && !self.is_braking {
            // Engage tape brake
            self.is_braking = true;
            self.brake_pos_samples = 0.0;
            self.is_crossfading = false;
            self.lockout_samples = 0.06 * self.sample_rate;
            // Align playhead to current live write head
            let head = self.ring_buffer.write_head() as f64;
            self.play_pos_l = head;
            self.play_pos_r = head;
        } else if !should_brake && self.is_braking {
            // Release tape brake -> begin smooth crossfade back to live audio
            self.is_braking = false;
            self.is_crossfading = true;
            self.xfade_pos_samples = 0.0;
        }
    }

    // --- 14-bit MIDI CC Continuous Speed Control ---

    pub fn handle_midi_cc(&mut self, cc_num: u8, value: u8, configured_cc: u8) {
        let target_msb = configured_cc;
        let target_lsb = configured_cc.wrapping_add(32);

        if cc_num == target_msb {
            self.last_cc_msb = Some(value);
            self.update_14bit_cc();
        } else if cc_num == target_lsb {
            self.last_cc_lsb = Some(value);
            self.update_14bit_cc();
        }
    }

    fn update_14bit_cc(&mut self) {
        if let (Some(msb), Some(lsb)) = (self.last_cc_msb, self.last_cc_lsb) {
            let combined = ((msb as u32) << 7) | (lsb as u32);
            let normalized = combined as f64 / 16383.0; // 0.0 = stopped, 1.0 = normal
            self.target_cc_speed = (1.0 - normalized).clamp(0.0, 1.0);
            self.cc_override_active = true;
        }
    }

    // --- Core Sample Processing ---

    #[inline(always)]
    pub fn process_sample(
        &mut self,
        in_l: f32,
        in_r: f32,
        drop_time_sec: f32,
        xfade_ms: f32,
        drop_curve: f32,
        stereo_div_pct: f32,
        auto_restart: bool,
        auto_restart_thresh_db: f32,
        power: bool,
    ) -> (f32, f32) {
        // 1. Always record incoming audio into the ring buffer
        self.ring_buffer.push(in_l, in_r);

        // Update audio envelope follower
        let in_peak = in_l.abs().max(in_r.abs());
        if in_peak > self.envelope {
            self.envelope += self.attack_coeff * (in_peak - self.envelope);
        } else {
            self.envelope += self.release_coeff * (in_peak - self.envelope);
        }

        // Decay visual transient flash LED (~100ms decay)
        let flash_decay = 1.0 / (0.10 * self.sample_rate);
        self.transient_flash = (self.transient_flash - flash_decay).max(0.0);

        // If plugin is bypassed/powered off, pass clean input
        if !power {
            self.speed_l = 1.0;
            self.speed_r = 1.0;
            return (in_l, in_r);
        }

        // 2. Compute deceleration parameters
        let stop_samples_base = (drop_time_sec as f64 * self.sample_rate as f64) / SEMI_DROP;
        let div_mult = (stereo_div_pct as f64) / 200.0;
        self.stop_samples_l = (stop_samples_base * (1.0 - div_mult).max(0.1)).max(1.0);
        self.stop_samples_r = (stop_samples_base * (1.0 + div_mult).max(0.1)).max(1.0);
        self.max_stop_samples = self.stop_samples_l.max(self.stop_samples_r);
        self.xfade_len_samples = ((xfade_ms as f64 * self.sample_rate as f64) / 1000.0).max(1.0);

        // 3. Update continuous CC speed smoothing if active
        if self.cc_override_active {
            let coeff = 0.005; // one-pole smoothing
            self.smoothed_cc_speed += coeff * (self.target_cc_speed - self.smoothed_cc_speed);
        }

        // 4. Auto-Restart Transient Detection
        if self.is_braking {
            if self.lockout_samples > 0.0 {
                self.lockout_samples -= 1.0;
            } else if auto_restart {
                let thresh_lin = 10.0f32.powf(auto_restart_thresh_db / 20.0);
                if in_peak >= thresh_lin || self.envelope >= thresh_lin {
                    // Transient triggered auto restart!
                    self.is_braking = false;
                    self.is_crossfading = true;
                    self.xfade_pos_samples = 0.0;
                    self.active_note_count = 0;
                    self.manual_trigger = false;
                    self.transient_flash = 1.0;
                }
            }
        }

        // 5. State evaluation & speed computation
        if self.is_braking {
            self.brake_pos_samples += 1.0;
            if self.brake_pos_samples > self.max_stop_samples {
                self.brake_pos_samples = self.max_stop_samples;
            }

            let prog_l = (self.brake_pos_samples / self.stop_samples_l).min(1.0);
            let prog_r = (self.brake_pos_samples / self.stop_samples_r).min(1.0);

            let curved_l = s_curve(prog_l as f32, drop_curve) as f64;
            let curved_r = s_curve(prog_r as f32, drop_curve) as f64;

            let mut speed_l = (1.0 - curved_l).max(0.0);
            let mut speed_r = (1.0 - curved_r).max(0.0);

            if self.cc_override_active {
                speed_l *= self.smoothed_cc_speed;
                speed_r *= self.smoothed_cc_speed;
            }

            self.speed_l = speed_l;
            self.speed_r = speed_r;

            self.play_pos_l += speed_l;
            self.play_pos_r += speed_r;

            let out_l = self.ring_buffer.read_left(self.play_pos_l);
            let out_r = self.ring_buffer.read_right(self.play_pos_r);
            (out_l, out_r)
        } else if self.is_crossfading {
            self.xfade_pos_samples += 1.0;

            if self.xfade_pos_samples >= self.xfade_len_samples {
                self.is_crossfading = false;
                self.speed_l = 1.0;
                self.speed_r = 1.0;
                (in_l, in_r)
            } else {
                let t = (self.xfade_pos_samples / self.xfade_len_samples).min(1.0);
                // Smoothstep crossfade weight: w = t * t * (3 - 2*t)
                let w = (t * t * (3.0 - 2.0 * t)) as f32;

                self.play_pos_l += self.speed_l;
                self.play_pos_r += self.speed_r;

                let stopped_l = self.ring_buffer.read_left(self.play_pos_l);
                let stopped_r = self.ring_buffer.read_right(self.play_pos_r);

                let out_l = stopped_l * (1.0 - w) + in_l * w;
                let out_r = stopped_r * (1.0 - w) + in_r * w;
                (out_l, out_r)
            }
        } else {
            // Normal live passthrough
            self.speed_l = 1.0;
            self.speed_r = 1.0;
            let head = self.ring_buffer.write_head() as f64;
            self.play_pos_l = head;
            self.play_pos_r = head;
            (in_l, in_r)
        }
    }

    // --- Telemetry Getters ---

    #[inline(always)]
    pub fn speed_left(&self) -> f32 {
        self.speed_l as f32
    }

    #[inline(always)]
    pub fn speed_right(&self) -> f32 {
        self.speed_r as f32
    }

    #[inline(always)]
    pub fn is_braking(&self) -> bool {
        self.is_braking
    }

    #[inline(always)]
    pub fn is_crossfading(&self) -> bool {
        self.is_crossfading
    }

    #[inline(always)]
    pub fn brake_progress(&self) -> f32 {
        if self.max_stop_samples > 0.0 {
            (self.brake_pos_samples / self.max_stop_samples).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }

    #[inline(always)]
    pub fn brake_progress_l(&self) -> f32 {
        if self.stop_samples_l > 0.0 {
            (self.brake_pos_samples / self.stop_samples_l).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }

    #[inline(always)]
    pub fn brake_progress_r(&self) -> f32 {
        if self.stop_samples_r > 0.0 {
            (self.brake_pos_samples / self.stop_samples_r).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }

    #[inline(always)]
    pub fn live_envelope(&self) -> f32 {
        self.envelope
    }

    #[inline(always)]
    pub fn transient_flash(&self) -> f32 {
        self.transient_flash
    }
}
