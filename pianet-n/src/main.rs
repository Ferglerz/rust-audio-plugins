use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::{circuit::Panel, Reed, KEYS, RATE, SUBSTEPS, TRIAD};
use std::{error::Error, f64::consts::PI, fs, io::Write, path::Path};

// Sample a 129-tap low-pass once per 48 kHz output frame.
const TAPS: usize = 129;
struct Decimator {
    history: [[f64; 3]; TAPS],
    taps: [f64; TAPS],
    index: usize,
}
impl Decimator {
    fn new() -> Self {
        let mut taps = std::array::from_fn(|i| {
            let x = i as f64 - (TAPS / 2) as f64;
            let a = 2.0 * PI * i as f64 / (TAPS - 1) as f64;
            let fc = 20400.0 / (f64::from(RATE) * SUBSTEPS as f64);
            let sinc = if x == 0.0 {
                2.0 * fc
            } else {
                (2.0 * PI * x * fc).sin() / (PI * x)
            };
            sinc * (0.42 - 0.5 * a.cos() + 0.08 * (2. * a).cos())
        });
        let sum = taps.iter().sum::<f64>();
        taps.iter_mut().for_each(|w| *w /= sum);
        Self {
            history: [[0.; 3]; TAPS],
            taps,
            index: 0,
        }
    }
    fn push(&mut self, x: [f64; 3]) {
        self.history[self.index] = x;
        self.index = (self.index + 1) % TAPS;
    }
    fn frame(&self) -> [f64; 3] {
        let mut out = [0.; 3];
        for (x, &w) in self.history[self.index..]
            .iter()
            .chain(&self.history[..self.index])
            .zip(&self.taps)
        {
            for (y, v) in out.iter_mut().zip(x) {
                *y += v * w;
            }
        }
        out
    }
}

fn render(
    root: &Path,
    name: &str,
    pitches: &[u8],
    events: &[(u32, u8)],
    end_ms: u32,
) -> Result<(), Box<dyn Error>> {
    let mut reeds: Vec<_> = pitches
        .iter()
        .map(|m| Reed::new(*m).expect("supported probe key"))
        .collect();
    let mut panel = Panel::default();
    let mut decimator = Decimator::new();
    // WAV values are physical volts at all three outlets, with a single gain.
    let mut wav = WavWriter::create(
        root.join(format!("{name}.wav")),
        WavSpec {
            channels: 3,
            sample_rate: RATE,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )?;
    let mut csv = std::io::BufWriter::new(fs::File::create(root.join(format!("{name}.csv")))?);
    writeln!(
        csv,
        "time_s,midi,pad_m,reed_at_pad_m,tip_m,force_N,bonded,seating,capacitance_F,bank_drive_A"
    )?;
    let mut events = events.iter().peekable();
    let mut traces = Vec::with_capacity(reeds.len());
    for frame in 0..(RATE / 1000 * end_ms) {
        if let Some(&&(ms, velocity)) = events.peek() {
            if frame == ms * RATE / 1000 {
                for reed in &mut reeds {
                    reed.press(velocity);
                }
                events.next();
            }
        }
        for sub in 0..SUBSTEPS {
            traces.clear();
            traces.extend(reeds.iter_mut().map(Reed::step));
            let (jacks, drive) = panel.tick(traces.iter().map(|o| o.capacitance).sum());
            decimator.push(jacks);
            if frame % 8 == 0 && sub == SUBSTEPS - 1 {
                for (o, m) in traces.iter().zip(pitches) {
                    writeln!(
                        csv,
                        "{:.8},{},{:.8e},{:.8e},{:.8e},{:.8e},{},{:.6},{:.8e},{:.8e}",
                        f64::from(frame + 1) / f64::from(RATE),
                        m,
                        o.pad_height,
                        o.reed_at_pad,
                        o.reed_tip,
                        o.pad_force,
                        u8::from(o.bonded),
                        o.seating,
                        o.capacitance,
                        drive
                    )?;
                }
            }
        }
        for x in decimator.frame() {
            wav.write_sample(x as f32)?;
        }
    }
    csv.flush()?;
    wav.finalize()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: pianet-n OUTPUT_DIR")?;
    let root = Path::new(&root);
    fs::create_dir_all(root)?;
    for midi in KEYS {
        render(
            root,
            &format!("key-{midi}"),
            &[midi],
            &[(50, 80), (800, 0)],
            1100,
        )?;
        render(
            root,
            &format!("repeats-{midi}"),
            &[midi],
            &[
                (50, 45),
                (230, 0),
                (233, 45),
                (380, 0),
                (402, 45),
                (545, 0),
                (780, 110),
                (970, 0),
            ],
            1170,
        )?;
    }
    render(root, "factory-triad", &TRIAD, &[(50, 80), (800, 0)], 1100)?;
    // Keep the circuit transfer separate from the mechanical/optical assumptions.
    let mut impulse = WavWriter::create(
        root.join("panel-impulse.wav"),
        WavSpec {
            channels: 3,
            sample_rate: RATE * SUBSTEPS as u32,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )?;
    let mut panel = Panel::default();
    for i in 0..RATE as usize * SUBSTEPS * 2 {
        for v in panel.drive(if i == 0 { 1e-6 } else { 0.0 }) {
            impulse.write_sample(v as f32)?;
        }
    }
    impulse.finalize()?;
    Ok(())
}
