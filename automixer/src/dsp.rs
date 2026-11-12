use crate::{
    network::{self, Network, Observer, Slot},
    params::SharedParams,
};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy)]
pub struct Settings {
    pub amount: f32,
    pub voice_cap: f32,
    pub other_cap: f32,
    pub voices: usize,
    pub threshold: f32,
    pub hold_ms: f32,
    pub fade_ms: f32,
    pub assist: f32,
}
impl Settings {
    pub fn from_params(p: &SharedParams) -> Self {
        Self {
            amount: p.amount.value() * 0.01,
            voice_cap: p.cap.value(),
            other_cap: p.other_cap.value(),
            voices: p.voices.value() as usize,
            threshold: p.threshold.value(),
            hold_ms: p.hold.value(),
            fade_ms: p.fade.value(),
            assist: p.assist.value() * 0.01,
        }
    }

    pub fn cap(&self, voice: bool) -> f32 {
        if voice {
            self.voice_cap
        } else {
            self.other_cap
        }
    }
}

#[derive(Default)]
pub struct Activity {
    pub active: bool,
    on_ms: f32,
    off_ms: f32,
}
impl Activity {
    pub fn update(&mut self, level: f32, threshold: f32, dt_ms: f32, hold_ms: f32) {
        if self.active {
            self.off_ms = if level < threshold - 3.0 {
                self.off_ms + dt_ms
            } else {
                0.0
            };
            if self.off_ms >= hold_ms {
                self.active = false;
                self.on_ms = 0.0;
            }
        } else {
            self.on_ms = if level >= threshold {
                self.on_ms + dt_ms
            } else {
                0.0
            };
            if self.on_ms >= 20.0 {
                self.active = true;
                self.off_ms = 0.0;
            }
        }
    }
}

#[derive(Default)]
pub struct Chorus {
    pub active: bool,
    quiet_ms: f32,
}
impl Chorus {
    pub fn update(&mut self, count: usize, limit: usize, dt_ms: f32, hold_ms: f32) {
        if limit == 0 {
            self.active = false;
            self.quiet_ms = 0.0;
        } else if count > limit {
            self.active = true;
            self.quiet_ms = 0.0;
        } else if self.active {
            self.quiet_ms += dt_ms;
            if self.quiet_ms >= hold_ms {
                self.active = false;
            }
        }
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.quiet_ms = 0.0;
    }
}

#[derive(Default)]
pub struct Ramp {
    pub value: f32,
}
impl Ramp {
    pub fn tick(&mut self, target: f32, samples: f32) -> f32 {
        let step = 1.0 / samples.max(1.0);
        self.value += (target - self.value).clamp(-step, step);
        self.value
    }
}

pub fn assisted_threshold(threshold: f32, assist: f32, speech: f32) -> f32 {
    threshold
        - pleasant_dynamics::VOICE_THRESH_SHIFT_DB as f32
            * assist.clamp(0.0, 1.0)
            * speech.clamp(0.0, 1.0)
}

pub fn share_reduction(own: f32, total: f32, cap: f32) -> f32 {
    if !total.is_finite() || total <= 1e-15 {
        return 0.0;
    }
    (-20.0 * (own / total).clamp(1e-9, 1.0).log10()).clamp(0.0, cap)
}

pub struct Engine {
    pub activity: Activity,
    pub chorus: Chorus,
    pub chorus_mix: Ramp,
    voice_mix: Ramp,
    amount: Ramp,
    engaged: Ramp,
    observer: Observer,
    pub active_count: usize,
    pub members: usize,
    pub conflict: bool,
    pub reduction: f32,
    sample_rate: f32,
    envelope: f64,
    envelope_coeff: f64,
    opening_coeff: f32,
    closing_coeff: f32,
    control_interval: u32,
    countdown: u32,
    share_target: f32,
    share_current: f32,
    weight: f32,
    stale_after: u64,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(48_000.0, 512)
    }
}

impl Engine {
    pub fn new(sample_rate: f32, max_buffer: u32) -> Self {
        Self {
            activity: Activity::default(),
            chorus: Chorus::default(),
            chorus_mix: Ramp::default(),
            voice_mix: Ramp::default(),
            amount: Ramp::default(),
            engaged: Ramp::default(),
            observer: Observer::default(),
            active_count: 0,
            members: 0,
            conflict: false,
            reduction: 0.0,
            sample_rate,
            envelope: 0.0,
            envelope_coeff: (-1.0 / (sample_rate as f64 * 0.015)).exp(),
            opening_coeff: 1.0 - (-1.0 / (sample_rate * 0.010)).exp(),
            closing_coeff: 1.0 - (-1.0 / (sample_rate * 0.080)).exp(),
            control_interval: (sample_rate * 0.005).round().max(1.0) as u32,
            countdown: 0,
            share_target: 0.0,
            share_current: 0.0,
            weight: 0.0,
            stale_after: (sample_rate * 0.5) as u64 + 2 * max_buffer as u64,
        }
    }

    pub fn reset(&mut self) {
        let stale_after = self.stale_after;
        *self = Self::new(self.sample_rate, 0);
        self.stale_after = stale_after;
    }

    pub fn input_db(&self) -> f32 {
        (10.0 * self.envelope.max(1e-12).log10()) as f32
    }

    pub fn analyze(&mut self, power: f64) -> bool {
        self.envelope += (power.min(1e12) - self.envelope) * (1.0 - self.envelope_coeff);
        if self.envelope < 1e-24 {
            self.envelope = 0.0;
        }
        if self.countdown == 0 {
            self.countdown = self.control_interval - 1;
            true
        } else {
            self.countdown -= 1;
            false
        }
    }

    pub fn control(
        &mut self,
        network: &Network,
        slot: &Slot,
        mic: usize,
        speech: f32,
        bypass: bool,
        voice: bool,
        settings: Settings,
    ) {
        let dt_ms = self.control_interval as f32 / self.sample_rate * 1000.0;
        let threshold = if voice {
            assisted_threshold(settings.threshold, settings.assist, speech)
        } else {
            settings.threshold
        };
        self.activity
            .update(self.input_db(), threshold, dt_ms, settings.hold_ms);
        let desired_weight = network.params.mics[mic].effective_weight();
        self.weight += (desired_weight - self.weight) * (1.0 - (-dt_ms / 20.0).exp());
        let own = (self.envelope.sqrt() as f32).max(1e-9) * 10.0_f32.powf(self.weight / 20.0);
        if bypass {
            slot.withdraw();
        } else {
            slot.publish(own, self.activity.active, voice);
        }
        let snapshot = self
            .observer
            .read(network, self.control_interval, self.stale_after);
        self.active_count = snapshot.active;
        self.members = snapshot.members;
        self.conflict = snapshot.conflicts & (1 << mic) != 0;
        let total = if voice {
            snapshot.voice_total
        } else {
            snapshot.other_total
        };
        self.share_target = share_reduction(own, total, settings.cap(voice));
        if voice {
            self.chorus.update(
                snapshot.voice_active,
                settings.voices,
                dt_ms,
                settings.hold_ms,
            );
        } else {
            self.chorus.clear();
        }
        slot.input_db.store(self.input_db(), Ordering::Relaxed);
        slot.output_db
            .store(self.input_db() - self.reduction, Ordering::Relaxed);
        slot.reduction_db.store(self.reduction, Ordering::Relaxed);
        slot.speech.store(speech, Ordering::Relaxed);
        slot.active
            .store(self.activity.active && !bypass, Ordering::Relaxed);
        slot.state.store(
            if bypass {
                network::BYPASSED
            } else if self.conflict {
                network::CONFLICT
            } else if !self.activity.active {
                network::QUIET
            } else if self.chorus_mix.value > 0.01 {
                network::CHORUS
            } else {
                network::MIXING
            },
            Ordering::Relaxed,
        );
    }

    pub fn gain(&mut self, settings: Settings, voice_group: bool, bypass: bool) -> f32 {
        let cap = settings.cap(voice_group);
        let fade_samples = settings.fade_ms * 0.001 * self.sample_rate;
        let chorus_target = if voice_group {
            self.chorus.active as u8 as f32
        } else {
            0.0
        };
        let chorus = self.chorus_mix.tick(chorus_target, fade_samples);
        let open = self
            .voice_mix
            .tick(self.activity.active as u8 as f32, fade_samples);
        let amount = self.amount.tick(settings.amount, fade_samples);
        let engaged = self.engaged.tick(
            (!bypass && !self.conflict) as u8 as f32,
            self.sample_rate * 0.020,
        );
        let coefficient = if self.share_target < self.share_current {
            self.opening_coeff
        } else {
            self.closing_coeff
        };
        self.share_current += (self.share_target - self.share_current) * coefficient;
        // Chorus applies only to the Voice group. Active voice mics fade open.
        // Quiet voice mics approach Voice GR. Other mics keep their own share.
        let chorus_reduction = cap * (1.0 - open);
        self.reduction =
            (amount * engaged * ((1.0 - chorus) * self.share_current + chorus * chorus_reduction))
                .clamp(0.0, cap);
        10.0_f32.powf(-self.reduction / 20.0)
    }
}
