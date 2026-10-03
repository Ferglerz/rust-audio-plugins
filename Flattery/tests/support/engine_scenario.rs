use flattery::{
    dsp::{Engine, Shared},
    params::FlatteryParams,
    strength::StrengthNode,
};
use std::sync::Arc;

// Deliberately uses only pre-optimization public APIs so the same workload can
// capture the baseline and verify current engine output.
pub fn run() -> Vec<[f64; 10]> {
    let shared = Arc::new(Shared::new());
    let mut engine = Engine::new(Arc::clone(&shared), 44100.0);
    let mut settings = FlatteryParams::default().process_settings();
    settings.boost_strength = 77.0;
    settings.cut_strength = 88.0;
    settings.minimum_operating_db = -75.0;
    settings.input_rms_ms = 13.0;
    settings.attack_ms = 3.0;
    settings.release_ms = 27.0;
    let mut result = Vec::new();
    let mut sample_index = 0;
    for (stage, fft_size) in [128, 256, 512, 1024, 2048, 4096, 8192]
        .into_iter()
        .enumerate()
    {
        settings.fft_size = fft_size;
        settings.mid_side = stage % 2 != 0;
        settings.stereo_link = [0.0, 50.0, 100.0][stage % 3];
        settings.amplify = stage % 3 == 0;
        settings.neighbor_radius = stage + 1;
        settings.output_gain_db = stage as f64 * 0.5 - 2.0;
        engine.set_sample_rate(if stage % 2 == 0 { 44100.0 } else { 96000.0 });
        let mut nodes = vec![
            StrengthNode::new(1, 7500.0, 0.2),
            StrengthNode::new(2, 250.0, 2.3),
            StrengthNode::new(3, 250.0, 0.5),
        ];
        for (index, node) in nodes.iter_mut().enumerate() {
            node.radius = (stage + index * 4) % 12 + 1;
        }
        if stage == 0 {
            nodes.clear();
        }
        if stage == 1 {
            nodes.truncate(1);
        }
        shared.publish_nodes(true, Arc::from(nodes.clone()));
        nodes.reverse();
        shared.publish_nodes(false, Arc::from(nodes));
        let mut stats = [0.0; 10];
        for i in 0..fft_size * 3 {
            let phase = sample_index as f64;
            let left = 0.31 * (phase * 0.071).sin() + 0.11 * (phase * 0.139).cos();
            let right = 0.07 * (phase * 0.097).sin() - 0.19 * (phase * 0.053).cos();
            let output = engine.tick(left, right, &settings);
            stats[0] += output.0;
            stats[1] += output.1;
            stats[2] += output.0 * output.0;
            stats[3] += output.1 * output.1;
            for (checkpoint, at) in [fft_size - 1, fft_size * 2 - 1, fft_size * 3 - 1]
                .into_iter()
                .enumerate()
            {
                if i == at {
                    stats[4 + checkpoint * 2] = output.0;
                    stats[5 + checkpoint * 2] = output.1;
                }
            }
            sample_index += 1;
        }
        result.push(stats);
    }
    result
}
