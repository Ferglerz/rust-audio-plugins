//! Reduced cross-key mechanical transfer through a single lossy support mode.
use crate::instrument::rotate;
use std::f64::consts::TAU;

/// Free receiver mode at a physical key, with two pickup-bus observation weights.
pub struct ReceiverSpec {
    pub hz: f64,
    pub decay: f64,
    pub damped_decay: f64,
    pub pickup: [f64; 2],
}

#[derive(Clone, Copy, Default)]
struct Receiver {
    position: f64,
    velocity: f64,
    rotation: (f64, f64),
    damp: f64,
    pickup: [f64; 2],
    // Contribution of this key to the common support mode.
    source: (f64, f64),
    drive: f64,
    choke_left: u32,
}

pub struct Sympathetic<const N: usize> {
    low: u8,
    undamped: u128,
    choked: u128,
    choke_samples: u32,
    keys: [Receiver; N],
    support: (f64, f64),
    support_input: f64,
    transfer: f64,
    inputs: u128,
    tails: u128,
    held: u128,
    pedal: bool,
    age: u8,
}

impl<const N: usize> Sympathetic<N> {
    pub fn new(
        rate: f64,
        low: u8,
        undamped: u128,
        frame_hz: f64,
        bandwidth_hz: f64,
        transfer_per_second: f64,
        spec: impl Fn(usize) -> ReceiverSpec,
    ) -> Self {
        assert!(N < 128 && usize::from(low) + N <= 128);
        let radius = (-TAU * bandwidth_hz / rate).exp();
        let (s, c) = (TAU * frame_hz / rate).sin_cos();
        let keys = std::array::from_fn(|i| {
            let spec = spec(i);
            let (s, c) = (TAU * spec.hz / rate).sin_cos();
            let r = (-spec.decay / rate).exp();
            Receiver {
                rotation: (s * r, c * r),
                damp: (-spec.damped_decay / rate).exp(),
                pickup: spec.pickup,
                ..Default::default()
            }
        });
        Self {
            low,
            undamped,
            choked: 0,
            choke_samples: (0.005 * rate).ceil() as u32,
            keys,
            support: (s * radius, c * radius),
            support_input: 2. * (1. - radius),
            transfer: transfer_per_second / rate,
            inputs: 0,
            tails: 0,
            held: 0,
            pedal: false,
            age: 0,
        }
    }

    pub fn set_pickup(&mut self, index: usize, weights: [f64; 2]) {
        self.keys[index].pickup = weights;
    }

    pub fn clear_motion(&mut self) {
        for key in &mut self.keys {
            key.position = 0.;
            key.velocity = 0.;
            key.source = (0., 0.);
            key.drive = 0.;
            key.choke_left = 0;
        }
        self.inputs = 0;
        self.tails = 0;
        self.age = 0;
    }

    pub fn clear(&mut self) {
        self.clear_motion();
        self.held = 0;
        self.choked = 0;
        self.pedal = false;
    }

    pub fn held(&mut self, note: u8, held: bool) {
        if (self.low..self.low + N as u8).contains(&note) {
            let bit = 1 << (note - self.low);
            if held {
                self.held |= bit;
                // A choke silences the previous excitation. The new hammer
                // can again drive any physically undamped receiver.
                self.choked = 0;
                self.keys[usize::from(note - self.low)].choke_left = 0;
            } else {
                self.held &= !bit;
            }
        }
    }
    pub fn pedal(&mut self, on: bool) {
        if on != self.pedal {
            self.choked = 0;
        }
        self.pedal = on;
    }
    pub fn choke(&mut self, note: u8) {
        if (self.low..self.low + N as u8).contains(&note) {
            let bit = 1 << (note - self.low);
            if self.choked & bit == 0 {
                self.keys[usize::from(note - self.low)].choke_left = self.choke_samples;
            }
            self.choked |= bit;
            self.held &= !bit;
        }
    }
    pub fn ringing(&self) -> bool {
        self.tails != 0
    }
    pub fn drive(&mut self, note: u8, velocity: f64) {
        if (self.low..self.low + N as u8).contains(&note) {
            let i = note - self.low;
            self.keys[usize::from(i)].drive = velocity;
            self.inputs |= 1 << i;
        }
    }
    pub fn tick(&mut self) -> [f64; 2] {
        let mut sources = self.inputs;
        let mut frame = 0.;
        while sources != 0 {
            let i = sources.trailing_zeros() as usize;
            sources &= sources - 1;
            let k = &mut self.keys[i];
            k.source = rotate(
                k.source.0,
                k.source.1 + self.support_input * k.drive,
                self.support.0,
                self.support.1,
            );
            if k.drive == 0. && k.source.0.abs() + k.source.1.abs() < 1e-10 {
                k.source = (0., 0.);
                self.inputs &= !(1 << i);
            }
            k.drive = 0.;
            // Superposition gives one shared support response. Retaining the
            // terms lets us subtract each key's already-modeled self response.
            frame += k.source.1;
        }
        let open = (if self.pedal {
            (1 << N) - 1
        } else {
            self.held | self.undamped
        }) & !self.choked;
        let mut receivers = self.tails;
        if self.inputs != 0 {
            // A solitary source cannot drive itself through this cross-key path.
            receivers |= if self.inputs.is_power_of_two() {
                open & !self.inputs
            } else {
                open
            };
        }
        self.tails = receivers;
        self.age = self.age.wrapping_add(1);
        let mut output = [0.; 2];
        while receivers != 0 {
            let i = receivers.trailing_zeros() as usize;
            receivers &= receivers - 1;
            let bit = 1 << i;
            let k = &mut self.keys[i];
            if open & bit != 0 {
                k.velocity += self.transfer * (frame - k.source.1);
            } else {
                k.position *= k.damp;
                k.velocity *= k.damp;
            }
            if k.choke_left > 0 {
                let left = k.choke_left;
                k.choke_left -= 1;
                let gain = f64::from(k.choke_left) / f64::from(left);
                k.position *= gain;
                k.velocity *= gain;
            }
            (k.position, k.velocity) = rotate(k.position, k.velocity, k.rotation.0, k.rotation.1);
            output[0] += k.velocity * k.pickup[0];
            output[1] += k.velocity * k.pickup[1];
            if self.age == 0 && k.velocity.abs() + k.position.abs() < 1e-7 {
                k.position = 0.;
                k.velocity = 0.;
                self.tails &= !bit;
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unplayed_receiver_rings_at_its_own_pitch_and_damps_on_pedal_up() {
        let rate = 48_000.;
        let mut modes = Sympathetic::<2>::new(rate, 40, 0, 220., 140., 2., |i| ReceiverSpec {
            hz: [82.41, 440.][i],
            decay: 1.,
            damped_decay: 85.,
            pickup: [0., i as f64],
        });
        // A partial of the source at the receiver's pitch, observed only
        // through the second pickup. No key-on is ever sent to the receiver.
        modes.held(40, true);
        for i in 0..4800 {
            modes.drive(40, (TAU * 440. * i as f64 / rate).sin());
            assert_eq!(modes.tick(), [0.; 2]);
        }
        modes.pedal(true);
        for i in 4800..24000 {
            modes.drive(40, (TAU * 440. * i as f64 / rate).sin());
            modes.tick();
        }
        let tail: Vec<_> = (0..4800).map(|_| modes.tick()[1]).collect();
        let crossings = tail.windows(2).filter(|w| w[0] < 0. && w[1] >= 0.).count();
        assert!((43..=45).contains(&crossings));
        assert!(tail.iter().map(|x| x.abs()).fold(0., f64::max) > 0.05);
        modes.pedal(false);
        for _ in 0..16_000 {
            modes.tick();
        }
        assert!(!modes.ringing());
        assert_eq!(modes.tick(), [0.; 2]);
    }
}
