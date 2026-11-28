//! Mechanical event times at the Duo's 44.1 kHz host setting, before the electronics.
use clavinet_d6::StringVoice;
use hohner_duo::{HIGH, LAST_BASS, LOW};
use pianet_n::{Model, Reed};

fn main() {
    const RATE: f64 = 88_200.;
    const RELEASE: usize = 8_820;
    println!("midi,velocity,tangent_contact_ms,anvil_contact_ms,reed_detach_ms,anvil_open_after_keyup_ms,reed_reseat_after_keyup_ms");
    for midi in [LOW, 41, LAST_BASS, LAST_BASS + 1, 64, 78, HIGH] {
        for velocity in [32, 80, 120] {
            let mut string = StringVoice::for_driven_key(midi, RATE).unwrap();
            let mut reed = Reed::at_rate(Model::T, midi, RATE).unwrap();
            reed.step();
            string.press(velocity);
            let anvil = string.geometry().anvil_drop_m;
            let mut events = [None; 5];
            for frame in 0..RELEASE * 2 {
                let down = frame < RELEASE;
                if frame == RELEASE {
                    string.release();
                }
                let (_, travel) = string.step_linked_key();
                let pad = reed.follow_key(travel, down);
                let at_tangent = string.trace().tangent_m;
                let ms = 1000. * ((frame % RELEASE + 1) as f64) / RATE;
                for (event, reached) in events.iter_mut().zip([
                    down && at_tangent > 0.,
                    down && at_tangent == anvil,
                    down && !pad.bonded,
                    !down && at_tangent < anvil,
                    !down && pad.bonded,
                ]) {
                    if reached {
                        event.get_or_insert(ms);
                    }
                }
                if events.iter().all(Option::is_some) {
                    break;
                }
            }
            print!("{midi},{velocity}");
            for event in events {
                match event {
                    Some(ms) => print!(",{ms:.3}"),
                    None => print!(","),
                }
            }
            println!();
        }
    }
}
