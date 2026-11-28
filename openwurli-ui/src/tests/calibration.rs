//! Run with cargo test -p openwurli-ui --release measure_peak_trims -- --ignored --nocapture
use super::*;
use stage73::era::Era;

#[test]
#[ignore = "offline Wurli peak sweep"]
fn measure_peak_trims_wurli() {
    sweep(PianoType::Wurli, GrandModel::Cp80, false);
}

#[test]
#[ignore = "offline Stage peak sweep"]
fn measure_peak_trims_stage() {
    sweep(PianoType::Stage, GrandModel::Cp80, false);
}

#[test]
#[ignore = "offline Grand peak sweep"]
fn measure_peak_trims_grand() {
    sweep(PianoType::Grand, GrandModel::Cp80, false);
}

#[test]
#[ignore = "offline CP-70B peak sweep"]
fn measure_peak_trims_cp70b() {
    sweep(PianoType::Grand, GrandModel::Cp70B, false);
}

#[test]
#[ignore = "offline original CP-70 peak sweep"]
fn measure_peak_trims_original_cp70() {
    sweep(PianoType::Grand, GrandModel::Cp70B, true);
}

fn sweep(kind: PianoType, grand: GrandModel, original_cp70: bool) {
    let keys = kind.native_range(grand);
    let mut max = 0.0_f32;
    for rate in [44100., 48000., 96000.] {
        let mut p = OpenWurliUi::default();
        let params = Arc::get_mut(&mut p.params).unwrap();
        params.piano = EnumParam::new("Piano", kind);
        params.grand_model = EnumParam::new("Grand Model", grand);
        params.grand_70_original = BoolParam::new("Original CP-70", original_cp70);
        params.extended_notes = BoolParam::new("Extended Notes", true);
        params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
        assert!(p.initialize(
            &OpenWurliUi::AUDIO_IO_LAYOUTS[0],
            &BufferConfig {
                sample_rate: rate,
                min_buffer_size: None,
                max_buffer_size: 256,
                process_mode: ProcessMode::Offline
            },
            &mut TestInitContext
        ));
        let eras: &[Era] = if kind == PianoType::Stage {
            &Era::ALL
        } else {
            &[Era::LateI]
        };
        for (&era, alternate) in eras.iter().flat_map(|era| {
            let paths = if kind == PianoType::Wurli
                || *era == Era::Sparkletop
                || grand == GrandModel::Cp70B
            {
                1
            } else {
                2
            };
            (0..paths).map(move |path| (era, path == 1))
        }) {
            let params = Arc::get_mut(&mut p.params).unwrap();
            params.stage.era = EnumParam::new("Era", era);
            params.stage.peterson = BoolParam::new("Peterson Suitcase", alternate);
            // Both optical gains are at most unity, attained at zero intensity.
            params.stage.intensity =
                FloatParam::new("Intensity", 0., FloatRange::Linear { min: 0., max: 1. });
            params.grand.frame_ring = BoolParam::new("Frame Ring", alternate);
            let mut native = 0.0_f32;
            let mut extended = 0.0_f32;
            let mut chords = 0.0_f32;
            let mut strike = |notes: &[u8], velocity: f32, pedal_change: bool| {
                match kind {
                    PianoType::Wurli => p.engine.clear_keys(),
                    PianoType::Stage => {
                        p.stage.set_controls(p.params.stage_controls());
                        p.stage.reset();
                    }
                    PianoType::Grand => {
                        p.grand.set(p.params.grand_controls());
                        p.grand.reset();
                    }
                }
                let (mut l, mut r) = ([0.; 256], [0.; 256]);
                for _ in 0..4 {
                    p.render_stereo(&mut l, &mut r, || None);
                }
                let note_event = |note, velocity| NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel: 0,
                    note,
                    velocity,
                };
                let mut events: Vec<_> = notes.iter().map(|&n| note_event(n, velocity)).collect();
                let mut sustained = Vec::new();
                if pedal_change {
                    events.insert(0, pedal(1.));
                    for &n in notes {
                        sustained.push(note_off(0, n));
                    }
                    for &n in notes {
                        sustained.push(note_event(n + 5, velocity));
                    }
                }
                let mut events = events.into_iter();
                let mut peak = 0.0_f32;
                for block in 0..(rate * 0.5 / 256.).ceil() as usize {
                    if pedal_change && block == (rate * 0.18 / 256.).round() as usize {
                        events = std::mem::take(&mut sustained).into_iter();
                    }
                    p.render_stereo(&mut l, &mut r, || events.next());
                    for &v in l.iter().chain(&r) {
                        assert!(v.is_finite(), "{kind:?} {rate} {notes:?}");
                        peak = peak.max(v.abs());
                    }
                }
                peak
            };
            for note in 0..=127 {
                let peak = strike(&[note], 1., false);
                if keys.contains(&note) {
                    native = native.max(peak);
                } else {
                    extended = extended.max(peak);
                }
            }
            // Two-hand voicings transposed chromatically across the physical keyboard.
            for root in *keys.start()..=(*keys.end() - 28) {
                let chord = [0, 7, 12, 16, 19, 23, 26, 28].map(|n| n + root);
                for velocity in [0.75, 1.] {
                    chords = chords.max(strike(&chord, velocity, false));
                }
                if root + 33 <= *keys.end() {
                    chords = chords.max(strike(&chord, 1., true));
                }
            }
            // Separately report the non-performance stress case; it does not set the trim.
            let all: Vec<u8> = keys.clone().collect();
            let stress_db = 20. * strike(&all, 1., false).log10();
            max = max.max(native).max(extended).max(chords);
            eprintln!("{kind:?} {era:?} alternate={alternate} {rate}: native {native:.6}, extended {extended:.6}, chord {chords:.6}, full keyboard {stress_db:+.2} dBFS");
        }
    }
    let db = 20. * max.log10();
    let raw_db = db - 20. * output::trim(kind, grand, original_cp70).log10();
    eprintln!("{kind:?} raw {raw_db:.3} dBFS; recommended fixed trim {:+.3} dB; rendered peak {db:.3} dBFS", -1.-raw_db);
    assert!(
        (db + 1.).abs() < 0.05,
        "{kind:?} calibrated peak {db:.3} dBFS"
    );
}

#[test]
#[ignore = "measure fixed trims for native Hohner instruments"]
fn hohner_performance_headroom() {
    headroom(&Instrument::HOHNER, false);
}

#[test]
#[ignore = "Electra auxiliary jack calibration"]
fn electra_performance_headroom() {
    headroom(&[Instrument::Electra], false);
}

#[test]
#[ignore = "Cembalet detector calibration"]
fn cembalet_performance_headroom() {
    headroom(
        &[
            Instrument::Reed(pianet_n::Model::Cembalet),
            Instrument::Reed(pianet_n::Model::CembaletN),
        ],
        false,
    );
}

#[test]
#[ignore = "early Pianet external jack calibration"]
fn early_pianet_external_headroom() {
    headroom(
        &[
            Instrument::Reed(pianet_n::Model::C),
            Instrument::Reed(pianet_n::Model::L),
        ],
        false,
    );
}
#[test]
#[ignore = "early Pianet headphone and speaker terminal calibration"]
fn early_pianet_local_headroom() {
    headroom(
        &[
            Instrument::Reed(pianet_n::Model::C),
            Instrument::Reed(pianet_n::Model::L),
        ],
        true,
    );
}
#[test]
#[ignore = "Duo mixer and recording jacks"]
fn duo_combined_headroom() {
    headroom(&[Instrument::Duo], false);
}
#[test]
#[ignore = "Duo recording jack calibration"]
fn duo_recording_headroom() {
    headroom(&[Instrument::Duo], true);
}

#[test]
#[ignore = "Claviset 200 and Pianotron output calibration"]
fn claviset_200_headroom() {
    headroom(
        &[Instrument::Claviset(
            pianet_n::claviset::Instrument::Claviset200,
        )],
        false,
    );
}

#[test]
#[ignore = "Selmer Pianotron output calibration"]
fn pianotron_headroom() {
    headroom(
        &[Instrument::Claviset(
            pianet_n::claviset::Instrument::Pianotron,
        )],
        false,
    );
}

#[test]
#[ignore = "Vintage Vibe passive output calibration"]
fn vv64_headroom() {
    headroom(&[Instrument::Vv64], false);
}

#[test]
#[ignore = "Vibanet factory preamp output calibration"]
fn vibanet_headroom() {
    headroom(&[Instrument::Vibanet], false);
}

#[test]
#[ignore = "StereoVibe Rev 4.1 stereo jack output calibration"]
fn stereovibe_headroom() {
    headroom(&[Instrument::StereoVibe], false);
}

#[test]
#[ignore = "Helpinstill Roadmaster jack output calibration"]
fn road_headroom() {
    headroom(&[Instrument::Road], false);
}

#[test]
#[ignore = "Baldwin classroom piano output calibration"]
fn baldwin_headroom() {
    headroom(&[Instrument::Baldwin], false);
}

#[test]
#[ignore = "Rhodes MK8 S active stereo output calibration"]
fn mk8_headroom() {
    headroom(&[Instrument::Mk8], false);
}

#[test]
#[ignore = "Vivi-Tone Clavier terminal output calibration"]
fn vivi_headroom() {
    headroom(&[Instrument::Vivi], false);
}

#[test]
#[ignore = "Valente passive output calibration"]
fn valente_headroom() {
    headroom(&[Instrument::Valente], false);
}

#[test]
#[ignore = "Columbia Elepian passive output calibration"]
fn elepian_headroom() {
    headroom(&[Instrument::Elepian], false);
}

#[test]
#[ignore = "Neo-Bechstein speaker calibration"]
fn neo_grand_headroom() {
    headroom(&[Instrument::Neo], false);
}

#[test]
#[ignore = "Storytone auxiliary jack calibration"]
fn storytone_headroom() {
    headroom(&[Instrument::Storytone], false);
}

fn headroom(models: &'static [Instrument], local: bool) {
    with_circuit_stack(move || {
        let mut peaks = Vec::new();
        for &instrument in models {
            let mut peak = 0.0_f32;
            for rate in [44100., 48000., 96000.] {
                let mut p = OpenWurliUi::default();
                let params = Arc::get_mut(&mut p.params).unwrap();
                params.select_model(instrument);
                params.early_local_output = BoolParam::new("Early Pianet Local Output", local);
                params.duo.recording = BoolParam::new("Duo Separate Recording Outputs", local);
                assert!(p.initialize(
                    &OpenWurliUi::AUDIO_IO_LAYOUTS[0],
                    &BufferConfig {
                        sample_rate: rate,
                        min_buffer_size: None,
                        max_buffer_size: 256,
                        process_mode: ProcessMode::Offline
                    },
                    &mut TestInitContext
                ));
                p.params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
                let range = p.params.key_range();
                use pianet_n::claviset::Instrument::{Claviset200, Pianotron};
                let settings = match instrument {
                    Instrument::Clavinet(_) | Instrument::Duo => crate::models::Pickup::ALL.len(),
                    Instrument::Claviset(Claviset200) => crate::claviset::ToneTab::ALL.len(),
                    Instrument::Claviset(Pianotron) => 3,
                    _ => 1,
                };
                for position in 0..settings {
                    let params = Arc::get_mut(&mut p.params).unwrap();
                    match instrument {
                        Instrument::Clavinet(_) | Instrument::Duo => {
                            let pickup = crate::models::Pickup::ALL[position];
                            params.d6.pickup = EnumParam::new("D6 Pickup", pickup);
                            params.duo.pickup = EnumParam::new("Duo String Pickup", pickup);
                        }
                        Instrument::Claviset(Claviset200) => {
                            params.claviset.register = EnumParam::new(
                                "Claviset Register",
                                crate::claviset::ToneTab::ALL[position],
                            );
                        }
                        Instrument::Claviset(Pianotron) => {
                            params.claviset.tone = FloatParam::new(
                                "Pianotron Tone",
                                position as f32 / 2.,
                                FloatRange::Linear { min: 0., max: 1. },
                            );
                        }
                        _ => {}
                    }
                    let mut strike = |notes: &[u8], velocity: f32| {
                        p.reset();
                        let mut notes = notes.iter().map(|&note| NoteEvent::NoteOn {
                            timing: 0,
                            voice_id: None,
                            channel: 0,
                            note,
                            velocity,
                        });
                        let (mut l, mut r) = ([0.; 256], [0.; 256]);
                        for _ in 0..(rate * 0.18 / 256.).ceil() as usize {
                            p.render_stereo(&mut l, &mut r, || notes.next());
                            for &v in l.iter().chain(&r) {
                                assert!(v.is_finite());
                                peak = peak.max(v.abs());
                            }
                        }
                    };
                    for note in range.clone() {
                        strike(&[note], 1.);
                    }
                    for root in *range.start()..=(*range.end() - 28) {
                        let chord = [0, 7, 12, 16, 19, 23, 26, 28].map(|i| root + i);
                        strike(&chord, 0.75);
                        strike(&chord, 1.);
                    }
                }
            }
            let db = 20. * peak.log10();
            peaks.push((instrument, db));
            eprintln!(
                "{instrument:?} local={local} performance peak {db:.3} dBFS; recommended trim {:+.3} dB",
                -1. - db + 20. * output::trim(instrument, GrandModel::Cp80, false).log10()
            );
        }
        for (instrument, db) in peaks {
            if instrument == Instrument::Duo && local {
                // The two recording jacks keep the same conversion as the combined mixer.
                assert!(
                    db.is_finite() && db <= -1.,
                    "Duo recording peak {db:.3} dBFS"
                );
            } else {
                assert!(
                    (db + 1.).abs() < 0.05,
                    "{instrument:?}: calibrated peak {db:.3} dBFS"
                );
            }
        }
    });
}
