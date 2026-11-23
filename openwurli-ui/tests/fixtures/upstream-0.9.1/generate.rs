use openwurli_dsp::WurliEngine;
fn main() {
    let mode = if cfg!(feature = "fast") { "fast" } else { "heavy" };
    let destination = std::env::args().nth(1).expect("fixture destination");
    for rate in [44100, 48000, 96000] {
        let mut engine = WurliEngine::new(rate as f64);
        engine.set_mlp_enabled(true);
        engine.set_speaker_character(0.65);
        engine.set_tremolo_depth(0.7);
        engine.warm_up();
        for note in [36, 60, 64, 79, 96] { engine.note_on(note, 0.8); }
        let mut audio = Vec::new();
        for block in 0..16 {
            if block == 4 { engine.set_sustain(true); engine.note_off(60); }
            if block == 8 { engine.set_sustain(false); engine.note_on(72, 0.5); }
            if block == 12 { engine.note_off(36); engine.note_off(96); }
            let mut out = [0.0; 256];
            engine.render(&mut out);
            audio.extend(out.into_iter().flat_map(f32::to_le_bytes));
        }
        std::fs::write(format!("{destination}/{mode}-{rate}.f32le"), audio).unwrap();
    }
}
