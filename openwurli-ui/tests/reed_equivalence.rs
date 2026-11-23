use openwurli_dsp::tables;
#[path = "support/simd_reed_v091.rs"]
mod candidate;
#[path = "support/scalar_reed_v091.rs"]
mod scalar;

macro_rules! compare_to_scalar {
    ($reed:ty) => {{
        let notes: Vec<u8> = (33..=96).chain([0, 12, 32, 97, 108, 127]).collect();
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let decay = [2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];
        let amplitudes = tables::BASE_MODE_AMPLITUDES;
        for rate in [44_100.0, 48_000.0, 88_200.0, 96_000.0] {
            for &note in &notes {
                for velocity in [0.0, 0.5, 1.0] {
                    for drag in [0.0, 1.6] {
                        for phases in [false, true] {
                            let mut actual = <$reed>::new(
                                tables::midi_to_freq(note),
                                &ratios,
                                &amplitudes,
                                &decay,
                                0.003,
                                velocity,
                                rate,
                            );
                            let mut expected = scalar::ModalReed::new(
                                tables::midi_to_freq(note),
                                &ratios,
                                &amplitudes,
                                &decay,
                                0.003,
                                velocity,
                                rate,
                            );
                            actual.set_air_drag(drag, 1.2, rate);
                            expected.set_air_drag(drag, 1.2, rate);
                            if phases {
                                let phase = [0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7];
                                actual.set_phases(&phase);
                                expected.set_phases(&phase);
                            }
                            for block in 0..48 {
                                if block == 8 || block == 19 {
                                    let scale = if block == 8 { 0.7 } else { 1.4 };
                                    actual.start_damper(note, rate, scale);
                                    expected.start_damper(note, rate, scale);
                                }
                                let len = [0, 1, 64, 257, 1024, 17][block % 6];
                                let mut a = vec![0.125; len];
                                let mut b = a.clone();
                                actual.render(&mut a);
                                expected.render(&mut b);
                                for (index, (a, b)) in a.iter().zip(&b).enumerate() {
                                    assert_eq!(
                                        a.to_bits(),
                                        b.to_bits(),
                                        "rate={rate}, note={note}, velocity={velocity}, drag={drag}, phases={phases}, block={block}, sample={index}"
                                    );
                                }
                                assert_eq!(actual.is_silent(-90.0), expected.is_silent(-90.0));
                                assert_eq!(actual.is_damping(), expected.is_damping());
                                assert_eq!(
                                    actual.release_seconds(rate).to_bits(),
                                    expected.release_seconds(rate).to_bits()
                                );
                            }
                        }
                    }
                }
            }
        }
    }};
}

#[test]
fn original_simd_matches_scalar_v091_bit_for_bit() {
    compare_to_scalar!(candidate::ModalReed);
}

#[test]
fn production_reed_matches_scalar_v091_bit_for_bit() {
    compare_to_scalar!(openwurli_dsp::reed::ModalReed);
}
