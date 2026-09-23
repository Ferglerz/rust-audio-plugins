use pleasant_dsp::{
    axis::{flattery_freq_to_pos, flattery_pos_to_freq},
    filters::{BiquadCoefficients, BiquadKind},
    spectrum::spectrum_fall_db,
};

fn main() {
    let position = flattery_freq_to_pos(1_000.0, 20.0, 20_000.0);
    let frequency = flattery_pos_to_freq(position, 20.0, 20_000.0);
    let bell = BiquadCoefficients::design(BiquadKind::Bell, 1_000.0, 6.0, 1.0, 48_000.0);
    println!(
        "frequency={frequency:.3}Hz response={:.3}dB fall={:.3}dB",
        bell.response_db(1_000.0, 48_000.0),
        spectrum_fall_db(256.0)
    );
}
