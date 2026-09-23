use pleasant_eq::{BandSettings, StaticEqProcessor};

fn main() {
    let sample_rate = 48_000.0;
    let mut equalizer = StaticEqProcessor::prepare(
        &[BandSettings {
            gain_db: 6.0,
            ..BandSettings::default()
        }],
        sample_rate,
    );
    let impulse = equalizer.process_sample([1.0, 1.0]);
    println!(
        "impulse=({:.6},{:.6}) response@1k={:.3}dB latency={}samples",
        impulse[0],
        impulse[1],
        equalizer.response_db(1_000.0),
        equalizer.latency_samples()
    );
}
