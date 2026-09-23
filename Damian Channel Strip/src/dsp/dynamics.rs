use super::{db_gain, Coeff, Filter};
use crate::band::Shape;
use pleasant_dynamics::{CompSettings, Pse, PseSettings, VocalComp};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pse_advanced_controls_follow_zingzap_transfer_and_timing() {
        let sr = 48000.0;
        let mut settings = PseSettings {
            depth: 18.0,
            hysteresis: 0.0,
            knee: 12.0,
            ..PseSettings::default()
        };
        let mut pse = Pse::default();
        // At threshold, the Hermite knee applies half the configured depth.
        for _ in 0..144000 {
            pse.tick(db_gain(-40.0), -40.0, sr, settings);
        }
        assert!((pse.reduction_db + 9.0).abs() < 0.001);
        settings.hysteresis = 6.0;
        for _ in 0..144000 {
            pse.tick(db_gain(-39.0), -40.0, sr, settings);
        }
        assert!(pse.reduction_db.abs() < 0.001);
        // Peak C closes with a one-second time constant; RMS C uses 200 ms.
        let mut peak = Pse::default();
        settings.peak = true;
        for _ in 0..48000 {
            peak.tick(0.0, -40.0, sr, settings);
        }
        assert!((peak.reduction_db + 18.0 * (1.0 - (-1.0_f64).exp())).abs() < 0.001);
        let mut fast = Pse::default();
        settings.time = 0.0;
        for _ in 0..48000 {
            fast.tick(0.0, -40.0, sr, settings);
        }
        assert!((fast.reduction_db + 18.0).abs() < 0.001);
    }

    #[test]
    fn pse_c_depth_ballistics_and_off() {
        for sr in [44100.0, 48000.0, 96000.0] {
            let mut pse = Pse::default();
            // C is a 200 ms log-domain time constant: one tau toward -10 dB.
            for _ in 0..(sr * 0.2) as usize {
                pse.tick(0.0, -40.0, sr, PseSettings::default());
            }
            assert!((pse.reduction_db + 10.0 * (1.0 - (-1.0_f64).exp())).abs() < 0.002);
            for _ in 0..(sr * 2.0) as usize {
                pse.tick(0.0, -40.0, sr, PseSettings::default());
            }
            assert!((pse.reduction_db + 10.0).abs() < 0.001);
            for _ in 0..(sr * 2.0) as usize {
                pse.tick(0.0, -80.0, sr, PseSettings::default());
            }
            assert!(pse.reduction_db.abs() < 0.001);
            for _ in 0..(sr * 2.0) as usize {
                pse.tick(0.5, -40.0, sr, PseSettings::default());
            }
            assert!(pse.reduction_db.abs() < 0.001);
        }
    }

    #[test]
    fn pse_uses_shared_hpf_and_handles_opposite_phase_stereo() {
        let settings = CompSettings {
            gate: -30.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let mut low_hpf = VocalComp::new();
        let mut high_hpf = VocalComp::new();
        let mut input_energy = 0.0;
        let mut low_energy = 0.0;
        let mut high_energy = 0.0;
        let mut f_low = [Filter::default(); 2];
        let mut f_high = [Filter::default(); 2];
        let c_low = Coeff::make(
            Shape::LowCut,
            20.0,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
            48000.0,
        );
        let c_high = Coeff::make(
            Shape::LowCut,
            500.0,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
            48000.0,
        );
        for i in 0..144000 {
            let x = 0.1 * (std::f64::consts::TAU * 50.0 * i as f64 / 48000.0).sin();
            let sc_low = [f_low[0].tick(x, c_low), f_low[1].tick(-x, c_low)];
            let sc_high = [f_high[0].tick(x, c_high), f_high[1].tick(-x, c_high)];
            let (low, _, _, _) = low_hpf.tick([x, -x], sc_low, settings, 48000.0);
            let (high, _, _, _) = high_hpf.tick([x, -x], sc_high, settings, 48000.0);
            assert!((low[0] + low[1]).abs() < 1e-12);
            if i > 96000 {
                input_energy += x * x;
                low_energy += low[0] * low[0];
                high_energy += high[0] * high[0];
            }
        }
        assert!((10.0 * (low_energy / input_energy).log10()).abs() < 0.01);
        assert!((10.0 * (high_energy / input_energy).log10() + 10.0).abs() < 0.01);
    }

    #[test]
    fn vocal_stereo_link_and_silence() {
        let mut c = VocalComp::new();
        let s = CompSettings {
            threshold: -30.0,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        let mut gr = 0.0;
        for i in 0..48000 {
            let v = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() * 0.5;
            let (out, g, _, _) = c.tick([v, v], [v, v], s, 48000.0);
            assert_eq!(out[0], out[1]);
            gr = g;
        }
        assert!(gr > 10.0);
        for _ in 0..96000 {
            let (out, _, _, _) = c.tick([0.0; 2], [0.0; 2], s, 48000.0);
            assert_eq!(out, [0.0; 2]);
        }
    }

    #[test]
    fn vocal_ratio_and_knee_affect_reduction() {
        let mut c_low = VocalComp::new();
        let mut c_high = VocalComp::new();
        let s_low = CompSettings {
            threshold: -20.0,
            ratio: 2.0,
            knee: 0.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let s_high = CompSettings {
            threshold: -20.0,
            ratio: 10.0,
            knee: 0.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let mut gr_low = 0.0;
        let mut gr_high = 0.0;
        for i in 0..48000 {
            let v = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() * 0.9;
            let (_, g_low, _, _) = c_low.tick([v, v], [v, v], s_low, 48000.0);
            let (_, g_high, _, _) = c_high.tick([v, v], [v, v], s_high, 48000.0);
            gr_low = g_low;
            gr_high = g_high;
        }
        assert!(gr_high > gr_low);
    }

    #[test]
    fn vocal_comp_depth_limits_reduction() {
        let mut c_unlimited = VocalComp::new();
        let mut c_limited = VocalComp::new();
        let s_unlimited = CompSettings {
            threshold: -30.0,
            ratio: 20.0,
            depth: 30.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let s_limited = CompSettings {
            threshold: -30.0,
            ratio: 20.0,
            depth: 5.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let mut gr_unlimited = 0.0;
        let mut gr_limited = 0.0;
        for i in 0..48000 {
            let v = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() * 0.9;
            let (_, gu, _, _) = c_unlimited.tick([v, v], [v, v], s_unlimited, 48000.0);
            let (_, gl, _, _) = c_limited.tick([v, v], [v, v], s_limited, 48000.0);
            gr_unlimited = gu;
            gr_limited = gl;
        }
        assert!(gr_unlimited > 20.0);
        assert!((gr_limited - 5.0).abs() < 0.2);
        assert!(c_limited.uncapped_gr() > 20.0);
        assert!(c_limited.uncapped_gr() > gr_limited + 5.0);
    }

    #[test]
    fn vocal_comp_attack_and_release_affect_timing() {
        let mut fast_comp = VocalComp::new();
        let mut slow_comp = VocalComp::new();
        let s_fast = CompSettings {
            threshold: -20.0,
            ratio: 4.0,
            attack: 0.5,
            release: 30.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let s_slow = CompSettings {
            threshold: -20.0,
            ratio: 4.0,
            attack: 50.0,
            release: 500.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        // After 240 samples (5 ms), fast attack should have responded much more than slow attack
        let mut gr_fast = 0.0;
        let mut gr_slow = 0.0;
        for i in 0..240 {
            let v = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() * 0.8;
            let (_, g_f, _, _) = fast_comp.tick([v, v], [v, v], s_fast, 48000.0);
            let (_, g_s, _, _) = slow_comp.tick([v, v], [v, v], s_slow, 48000.0);
            gr_fast = g_f;
            gr_slow = g_s;
        }
        assert!(gr_fast > gr_slow * 2.0);
    }

    #[test]
    fn pse_voice_detect_lowers_threshold_and_softens_depth() {
        let sr = 48000.0;
        let mut pse_no_vad = Pse::default();
        let mut pse_vad = Pse::default();

        let s_no_vad = PseSettings {
            depth: 10.0,
            hysteresis: 0.0,
            knee: 0.01,
            vad_assist: 0.0,
            speech_env: 0.0,
            ..PseSettings::default()
        };
        let s_vad = PseSettings {
            depth: 10.0,
            hysteresis: 0.0,
            knee: 0.01,
            vad_assist: 1.0,
            speech_env: 1.0,
            ..PseSettings::default()
        };

        // Threshold is -40 dB. Signal is at -42 dB.
        // Without VAD assist: -42 dB is 2 dB below threshold -> closed (depth = 10 dB attenuation).
        // With VAD assist (100% assist + 100% speech): threshold shifts down by 3 dB to -43 dB.
        // -42 dB is now 1 dB above threshold (-43 dB) -> opens up (0 dB attenuation)!
        let settle_samples = (sr * 2.0) as usize;
        for _ in 0..settle_samples {
            pse_no_vad.tick(db_gain(-42.0), -40.0, sr, s_no_vad);
            pse_vad.tick(db_gain(-42.0), -40.0, sr, s_vad);
        }
        assert!(
            pse_no_vad.reduction_db < -9.5,
            "Without VAD should be fully attenuated"
        );
        assert!(
            pse_vad.reduction_db > -0.5,
            "With VAD should be open because threshold was pulled down"
        );

        // When deep below threshold (e.g. -60 dB), check depth softening:
        // Max attenuation without VAD = 10 dB.
        // With VAD = 10 * (1 - 0.15) = 8.5 dB.
        for _ in 0..settle_samples {
            pse_no_vad.tick(db_gain(-60.0), -40.0, sr, s_no_vad);
            pse_vad.tick(db_gain(-60.0), -40.0, sr, s_vad);
        }
        assert!((pse_no_vad.reduction_db - (-10.0)).abs() < 0.1);
        assert!((pse_vad.reduction_db - (-8.5)).abs() < 0.1);
    }

    #[test]
    fn pse_voice_detect_stays_off_at_minus_80_threshold() {
        let sr = 48000.0;
        let mut pse = Pse::default();
        let s = PseSettings {
            depth: 10.0,
            vad_assist: 1.0,
            speech_env: 1.0,
            ..PseSettings::default()
        };
        for _ in 0..1000 {
            pse.tick(db_gain(-50.0), -80.0, sr, s);
        }
        assert_eq!(pse.reduction_db, 0.0);
    }
}
