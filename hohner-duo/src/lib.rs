//! Hohner Pianet/Clavinet Duo shared-key action and register wiring.
pub mod circuit;
use clavinet_d6::{panel::ALL_TONES, Switch, Wiring};

pub const LOW: u8 = 29;
pub const HIGH: u8 = 88;
/// Hohner's series pickup taps are between TA20 and TA21.
pub const LAST_BASS: u8 = LOW + 19;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Split {
    Preset,
    PianetTreble,
    ClavinetTreble,
    BothTreble,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub split: Split,
    /// Left 1/2 and right 1/3 rockers, true when pressed forward (2 and 3).
    pub tab_two: bool,
    pub tab_three: bool,
    /// Contact A by default, B when latched.
    pub foot: bool,
    pub pickup: Switch,
    pub wiring: Wiring,
    pub tones: u8,
    pub clavinet_volume: f64,
    pub pianet_volume: f64,
    pub mute: bool,
    pub power: bool,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            split: Split::Preset,
            tab_two: true,
            tab_three: true,
            foot: false,
            pickup: Switch::CA,
            wiring: Wiring::DaAdds,
            tones: ALL_TONES,
            clavinet_volume: 1.,
            pianet_volume: 1.,
            mute: false,
            power: true,
        }
    }
}

impl Controls {
    /// A true contact leaves the corresponding volume control connected.
    fn open_outputs(self) -> [bool; 2] {
        [!self.foot || self.tab_three, self.foot || self.tab_two]
    }

    /// Direct signal paths from this key to the Clavinet and Pianet sliders.
    pub fn selected_branches(self, midi: u8) -> [bool; 2] {
        if !self.power || midi > 127 {
            return [false; 2];
        }
        let (clav, reed) = self.split.banks();
        let [c, p] = self.open_outputs();
        [c && clav.contains(midi), p && reed.contains(midi)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bank {
    Both,
    Bass,
    Treble,
}

impl Bank {
    fn contains(self, midi: u8) -> bool {
        match self {
            Self::Both => true,
            Self::Bass => midi <= LAST_BASS,
            Self::Treble => midi > LAST_BASS,
        }
    }

    fn keys(self) -> u8 {
        match self {
            Self::Both => 60,
            Self::Bass => 20,
            Self::Treble => 40,
        }
    }
}

impl Split {
    fn banks(self) -> (Bank, Bank) {
        use Bank::*;
        match self {
            Self::Preset => (Both, Both),
            Self::PianetTreble => (Bass, Treble),
            Self::ClavinetTreble => (Treble, Bass),
            Self::BothTreble => (Treble, Both),
        }
    }
}

mod engine;
pub use engine::{Engine, LATENCY};

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(e: &mut Engine, count: usize) -> [f64; 3] {
        let mut sum = [0.; 3];
        for _ in 0..count {
            for (v, x) in sum.iter_mut().zip(e.jacks()) {
                assert!(x.is_finite());
                *v += x * x;
            }
        }
        sum.map(|v| (v / count as f64).sqrt())
    }

    #[test]
    fn adjacent_taps_route_each_still_ringing_action_to_its_recording_socket() {
        for (selector, low, high) in [
            (Split::Preset, [true, true], [true, true]),
            (Split::PianetTreble, [true, false], [false, true]),
            (Split::ClavinetTreble, [false, true], [true, false]),
            (Split::BothTreble, [false, true], [true, true]),
        ] {
            let mut e = Engine::new(48000.);
            for (n, expected) in [(48, low), (49, high)] {
                e.reset();
                e.set_controls(Controls::default());
                e.note_on(n, 0.7);
                rms(&mut e, 2200);
                let before = rms(&mut e, 1800);
                e.set_controls(Controls {
                    split: selector,
                    ..Default::default()
                });
                rms(&mut e, 4500);
                let after = rms(&mut e, 1200);
                for port in 0..2 {
                    let ratio = after[port] / before[port];
                    assert!(
                        if expected[port] {
                            ratio > 0.08
                        } else {
                            // The pots stay connected through the analog mixer. Bound
                            // leakage relative to the branch driving it in this position.
                            after[port] < 0.015 * after[1 - port]
                        },
                        "{selector:?} MIDI {n}, port {port}, ratio {ratio}"
                    );
                }
                e.set_controls(Controls::default());
                rms(&mut e, 3200);
                let revealed = rms(&mut e, 1000);
                assert!(revealed[0] > 0.001 && revealed[1] > 0.005);
            }
        }
    }

    #[test]
    fn footswitch_grounds_only_the_selected_recording_output() {
        let mut e = Engine::new(44100.);
        e.note_on(62, 0.8);
        for two in [false, true] {
            for three in [false, true] {
                for foot in [false, true] {
                    e.set_controls(Controls {
                        tab_two: two,
                        tab_three: three,
                        foot,
                        ..Default::default()
                    });
                    rms(&mut e, 128);
                    let y = rms(&mut e, 512);
                    assert_eq!(y[0] == 0., foot && !three, "string {two}/{three}/{foot}");
                    assert_eq!(y[1] == 0., !foot && !two, "reed {two}/{three}/{foot}");
                }
            }
        }
    }

    #[test]
    fn extended_notes_follow_split_and_noise_only_changes_the_return() {
        let mut quiet = Engine::extended(48000.);
        let mut stock = Engine::extended(48000.);
        quiet.set_pad_release_noise(0.);
        for e in [&mut stock, &mut quiet] {
            e.note_on(0, 0.75);
            e.note_on(127, 0.75);
            assert_eq!(e.pressed(), 1 | (1u128 << 127));
            e.reset();
            e.note_on(64, 0.6);
        }
        for _ in 0..9600 {
            assert_eq!(stock.jacks(), quiet.jacks());
        }
        quiet.note_off(64);
        stock.note_off(64);
        let mut residual = 0.;
        for _ in 0..6000 {
            residual += (stock.jacks()[1] - quiet.jacks()[1]).powi(2);
        }
        assert!(
            residual > 1e-9,
            "reenacting the return should change the reed socket"
        );
        for (n, selected) in [(0, 0), (127, 1)] {
            quiet.reset();
            quiet.set_controls(Controls {
                split: Split::PianetTreble,
                ..Default::default()
            });
            quiet.note_on(n, 0.8);
            let ports = rms(&mut quiet, 4800);
            assert!(
                ports[selected] > 1e-7 && ports[selected] > ports[1 - selected] * 10.,
                "MIDI {n} on recording socket {selected}: {ports:?}"
            );
        }
    }

    #[test]
    fn electrical_power_and_mechanical_mute_keep_distinct_states() {
        let mut a = Engine::new(48000.);
        let mut b = Engine::new(48000.);
        b.set_controls(Controls {
            mute: true,
            ..Default::default()
        });
        a.note_on(64, 0.7);
        b.note_on(64, 0.7);
        rms(&mut a, 12000);
        rms(&mut b, 12000);
        let dry = rms(&mut a, 4800);
        let muted = rms(&mut b, 4800);
        assert!(muted[0] < dry[0] * 0.25 && (muted[1] / dry[1] - 1.).abs() < 0.002);
        b.set_controls(Controls {
            power: false,
            ..Default::default()
        });
        rms(&mut b, 64);
        assert_eq!(rms(&mut b, 1024), [0.; 3]);
        b.set_controls(Controls::default());
        assert!(rms(&mut b, 1024)[1] > 0.05);
        assert_eq!(b.pressed(), 1u128 << 64);
    }

    #[test]
    fn native_range_audition_repeats_and_reset_work_at_all_host_rates() {
        for rate in [8000, 44100, 48000, 96000] {
            let mut e = Engine::new(rate.into());
            e.note_on(LOW - 1, 1.);
            e.note_on(HIGH + 1, 1.);
            assert_eq!(e.pressed(), 0);
            for n in LOW..=HIGH {
                e.reset();
                e.note_on(n, 0.7);
                let y = rms(&mut e, rate as usize / 20);
                assert!(y.into_iter().all(|v| v > 0.0001), "{n} {rate} {y:?}");
            }
            e.note_on(60, 0.6);
            e.audition(Some(60), 0.6);
            e.note_off(60);
            assert_eq!(e.pressed() & (1 << 60), 1 << 60);
            e.audition(None, 0.);
            e.choke(HIGH);
            assert_eq!(e.pressed(), 0);
            rms(&mut e, rate as usize / 50);
            e.reset();
            assert!(!e.sounding());
            assert_eq!(e.jacks(), [0.; 3]);
        }
    }
}
