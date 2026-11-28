//! Render one strike per prototype key, with a common time origin and gain for every file.
use clavinet_d6::{
    panel::{Board, Panel, ALL_TONES, PICKUP_VOLTS},
    Pickups, StringVoice, Switch, Wiring, KEYS, OUTPUT_RATE, SUBSTEPS,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{
    error::Error,
    f64::consts::PI,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

const TAPS: usize = 257;
const AUDIO_GAIN: f64 = 0.1;
const POSITIONS: [(&str, Switch); 4] = [
    ("CA", Switch::CA),
    ("CB", Switch::CB),
    ("DA", Switch::DA),
    ("DB", Switch::DB),
];

#[derive(Clone, Copy, Default)]
struct Signals {
    pickups: Pickups,
    // Four reference positions and one continuously running, switched panel.
    jacks: [f64; 5],
}

struct Decimator {
    taps: [f64; TAPS],
    history: [Signals; TAPS],
    cursor: usize,
}

impl Decimator {
    fn new() -> Self {
        // Blackman-windowed sinc, 18.5 kHz cutoff. All files share a 128-tick group delay.
        let fc = 18_500.0 / (f64::from(OUTPUT_RATE) * SUBSTEPS as f64);
        let mut taps = std::array::from_fn(|i| {
            let x = i as f64 - (TAPS / 2) as f64;
            let angle = 2.0 * PI * i as f64 / (TAPS - 1) as f64;
            let window = 0.42 - 0.5 * angle.cos() + 0.08 * (2.0 * angle).cos();
            window
                * if x == 0.0 {
                    2.0 * fc
                } else {
                    (2.0 * PI * fc * x).sin() / (PI * x)
                }
        });
        let sum: f64 = taps.iter().sum();
        taps.iter_mut().for_each(|x| *x /= sum);
        Self {
            taps,
            history: [Signals::default(); TAPS],
            cursor: 0,
        }
    }

    fn frame(&mut self, voice: &mut StringVoice, panels: &mut [Panel; 5]) -> Signals {
        for _ in 0..SUBSTEPS {
            let pickups = voice.step();
            let emf = Pickups {
                centre: pickups.centre * PICKUP_VOLTS,
                bridge: pickups.bridge * PICKUP_VOLTS,
            };
            self.history[self.cursor] = Signals {
                pickups,
                jacks: panels.each_mut().map(|panel| panel.process(emf)),
            };
            self.cursor = (self.cursor + 1) % TAPS;
        }
        let mut p = Signals::default();
        for (x, &w) in self.history[self.cursor..]
            .iter()
            .chain(&self.history[..self.cursor])
            .zip(&self.taps)
        {
            p.pickups.centre += w * x.pickups.centre;
            p.pickups.bridge += w * x.pickups.bridge;
            for (sum, v) in p.jacks.iter_mut().zip(x.jacks) {
                *sum += w * v;
            }
        }
        p
    }
}

#[derive(Clone, Copy)]
struct Registration {
    board: Board,
    wiring: Wiring,
    tones: u8,
}

fn render(
    dir: &Path,
    midi: u8,
    velocity: u8,
    hold: f64,
    panel: Registration,
) -> Result<(), Box<dyn Error>> {
    render_events(
        dir,
        midi,
        panel,
        &[(0.05, velocity), (0.05 + hold, 0)],
        false,
    )
}

/// Times are seconds from the start of the file. A velocity of zero lifts the key.
fn render_events(
    dir: &Path,
    midi: u8,
    panel: Registration,
    events: &[(f64, u8)],
    capture_motion: bool,
) -> Result<(), Box<dyn Error>> {
    let dir = dir.join(midi.to_string());
    fs::create_dir_all(&dir)?;
    let spec = WavSpec {
        channels: 1,
        sample_rate: OUTPUT_RATE,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut stems = WavWriter::create(
        dir.join("pickups.wav"),
        WavSpec {
            channels: 2,
            ..spec
        },
    )?;
    let mut centre_stem = WavWriter::create(dir.join("centre.wav"), spec)?;
    let mut bridge_stem = WavWriter::create(dir.join("bridge.wav"), spec)?;
    let mut switches = Vec::new();
    let mut sweep = WavWriter::create(dir.join("jack-selector-sweep.wav"), spec)?;
    let mut changes = [(0.24, Switch::CB), (0.43, Switch::DA), (0.62, Switch::DB)]
        .map(|(t, mode)| ((t * f64::from(OUTPUT_RATE)).round() as usize, mode))
        .into_iter()
        .peekable();
    let internal_rate = f64::from(OUTPUT_RATE) * SUBSTEPS as f64;
    let mut panels = [Switch::CA, Switch::CB, Switch::DA, Switch::DB, Switch::CA].map(|mode| {
        Panel::for_board(
            panel.board,
            internal_rate,
            mode,
            panel.wiring,
            panel.tones,
            1.0,
        )
    });
    for (name, mode) in POSITIONS {
        switches.push((
            mode,
            WavWriter::create(dir.join(format!("{name}.wav")), spec)?,
            WavWriter::create(dir.join(format!("jack-{name}.wav")), spec)?,
        ));
    }
    let mut trace = BufWriter::new(fs::File::create(dir.join("trace.csv"))?);
    writeln!(trace, "time_s,tangent_m,centre_m,bridge_m,tip_n,anvil_n,net_contact_work_j,viscous_loss_j,stored_j,tip_position_m")?;
    let mut voice = StringVoice::new(midi).unwrap();
    let mut motion = if capture_motion {
        let mut out = BufWriter::new(fs::File::create(dir.join("motion.csv"))?);
        let g = voice.geometry();
        writeln!(
            out,
            "# speaking_m,afterlength_m,centre_m,bridge_m,anvil_drop_m"
        )?;
        writeln!(
            out,
            "# {},{},{},{},{}",
            g.speaking_m, g.afterlength_m, g.centre_m, g.bridge_m, g.anvil_drop_m
        )?;
        writeln!(out, "# time_s,tip_position_m,equally spaced grid displacements from tailpiece to tuning pin (m)")?;
        Some(out)
    } else {
        None
    };
    let mut filter = Decimator::new();
    let mut timeline = events
        .iter()
        .map(|&(t, velocity)| ((t * f64::from(OUTPUT_RATE)).round() as usize, velocity))
        .peekable();
    let last = timeline.clone().last().ok_or("missing key events")?.0;
    for frame in 0..last + OUTPUT_RATE as usize / 2 {
        while let Some((_, velocity)) = timeline.next_if(|&(at, _)| at == frame) {
            voice.press(velocity);
        }
        if let Some((_, mode)) = changes.next_if(|&(at, _)| frame == at) {
            panels[4].set_controls(mode, panel.wiring, panel.tones, 1.);
        }
        let signals = filter.frame(&mut voice, &mut panels);
        sweep.write_sample(signals.jacks[4] as f32)?;
        let p = signals.pickups;
        let centre = (AUDIO_GAIN * p.centre) as f32;
        let bridge = (AUDIO_GAIN * p.bridge) as f32;
        centre_stem.write_sample(centre)?;
        bridge_stem.write_sample(bridge)?;
        stems.write_sample(centre)?;
        stems.write_sample(bridge)?;
        for ((mode, writer, jack), volts) in switches.iter_mut().zip(signals.jacks) {
            writer.write_sample(
                (AUDIO_GAIN * panel.board.selector_output(*mode, panel.wiring, p)) as f32,
            )?;
            jack.write_sample(volts as f32)?;
        }
        if frame % 11 == 0 {
            let t = voice.trace();
            let seconds = (frame + 1) as f64 / f64::from(OUTPUT_RATE);
            if let Some(out) = &mut motion {
                write!(out, "{seconds:.7},{:.7}", t.tip_position_m)?;
                for y in voice.displacements() {
                    write!(out, ",{y:.7}")?;
                }
                writeln!(out)?;
            }
            writeln!(
                trace,
                "{:.7},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e}",
                seconds,
                t.tangent_m,
                t.centre_m,
                t.bridge_m,
                t.tip_n,
                t.anvil_n,
                t.contact_work_j,
                t.viscous_loss_j,
                t.stored_j,
                t.tip_position_m
            )?;
        }
    }
    sweep.finalize()?;
    stems.finalize()?;
    centre_stem.finalize()?;
    bridge_stem.finalize()?;
    for (_, writer, jack) in switches {
        writer.finalize()?;
        jack.finalize()?;
    }
    trace.flush()?;
    if let Some(mut out) = motion {
        out.flush()?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    const USAGE: &str = "clavinet-d6 OUTPUT_DIR [VELOCITY=80] [HOLD_SECONDS=0.75] [da-adds|da-subtracts] [TONE_MASK=15] [d6|e7]\nclavinet-d6 --reference-session OUTPUT_DIR";
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [command, directory] = args.as_slice() {
        if command == "--reference-session" {
            let root = Path::new(directory);
            // Durations inferred from the first isolated Aalto strike and release burst.
            for (midi, long_hold) in KEYS.into_iter().zip([14.607, 5.303, 5.679]) {
                let panel = Registration {
                    board: Board::D6,
                    wiring: Wiring::DaAdds,
                    tones: ALL_TONES,
                };
                render(root, midi, 80, 0.75, panel)?;
                render(&root.join("velocity-32"), midi, 32, 0.75, panel)?;
                render(&root.join("velocity-120"), midi, 120, 0.75, panel)?;
                render(&root.join("short"), midi, 80, 0.19, panel)?;
                render(&root.join("long"), midi, 80, long_hold, panel)?;
                // First strike, early key-up, then restrike the same moving string.
                render_events(
                    &root.join("restrike"),
                    midi,
                    panel,
                    &[(0.05, 80), (0.12, 0), (0.138, 109), (0.328, 0)],
                    true,
                )?;
            }
            return Ok(());
        }
    }
    if matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    if !(1..=6).contains(&args.len()) || args[0].starts_with('-') {
        return Err(USAGE.into());
    }
    let velocity: u8 = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(80);
    let hold: f64 = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(0.75);
    if !(1..=127).contains(&velocity) || !(0.02..=60.0).contains(&hold) {
        return Err("expected velocity 1..127 and hold 0.02..60 seconds".into());
    }
    let wiring = match args.get(3).map(String::as_str).unwrap_or("da-adds") {
        "da-adds" => Wiring::DaAdds,
        "da-subtracts" => Wiring::DaSubtracts,
        _ => return Err("expected da-adds or da-subtracts".into()),
    };
    let tones = args
        .get(4)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(ALL_TONES);
    if tones > ALL_TONES {
        return Err("expected tone mask 0..15".into());
    }
    let board = match args.get(5).map(String::as_str) {
        None | Some("d6") => Board::D6,
        Some("e7") => Board::E7,
        _ => return Err("expected d6 or e7".into()),
    };
    for midi in KEYS {
        render(
            Path::new(&args[0]),
            midi,
            velocity,
            hold,
            Registration {
                board,
                wiring,
                tones,
            },
        )?;
    }
    Ok(())
}
