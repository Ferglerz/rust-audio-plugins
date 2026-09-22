mod generative;
mod pack;
mod xml_import;

use generative::pack_from_kit;
use pack::{finish_pack, AudioPacker};
use scd_core::kit::{count_kit_strikes, load_stonehouse};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    if args[1] == "--from-xml" {
        run_xml_mode(&args)?;
    } else {
        run_generative_mode(&args)?;
    }

    Ok(())
}

fn run_generative_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // scd-packer [--strict] [--kit stonehouse] <samples_dir> <output.scdpack>
    let mut strict = false;
    let mut kit_name = "stonehouse";
    let mut positional = Vec::new();

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--strict" => strict = true,
            "--kit" => {
                i += 1;
                if i >= args.len() {
                    return Err("--kit requires a value".into());
                }
                kit_name = &args[i];
            }
            s if s.starts_with("--kit=") => kit_name = s.trim_start_matches("--kit="),
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}").into()),
            _ => positional.push(&args[i]),
        }
        i += 1;
    }

    if positional.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let samples_dir = Path::new(positional[0]);
    let output_path = Path::new(positional[1]);

    let kit = match kit_name {
        "stonehouse" => load_stonehouse(),
        other => return Err(format!("unknown kit: {other} (available: stonehouse)").into()),
    };

    println!("Packer: generative mode (kit={})", kit_name);
    println!("Packer: reading audio from {:?}", samples_dir);
    println!("Packer: output will be {:?}", output_path);
    println!(
        "Packer: expected {} strikes across {} kit pieces",
        count_kit_strikes(&kit),
        kit.order.len()
    );

    let temp_audio_path = output_path.with_extension("audio.tmp");
    let mut packer = AudioPacker::new(BufWriter::new(File::create(&temp_audio_path)?));

    let strikes = pack_from_kit(&kit, samples_dir, &mut packer, strict)?;

    println!("Packer: strikes indexed: {}", strikes.len());
    let audio_bytes = finish_pack(packer, &temp_audio_path, output_path, &strikes)?;
    println!("Packer: unique WAV files packed: (see audio bytes below)");
    println!("Packer: total audio bytes: {}", audio_bytes);
    println!("Packer complete! Output written to {:?}", output_path);
    Ok(())
}

fn run_xml_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // scd-packer --from-xml <samplemaps_dir> <samples_dir> <output.scdpack>
    if args.len() < 5 {
        eprintln!("Usage: scd-packer --from-xml <samplemaps_dir> <samples_dir> <output.scdpack>");
        std::process::exit(1);
    }

    let samplemaps_dir = Path::new(&args[2]);
    let samples_dir = Path::new(&args[3]);
    let output_path = Path::new(&args[4]);

    println!("Packer: legacy XML import from {:?}", samplemaps_dir);
    println!("Packer: reading audio from {:?}", samples_dir);
    println!("Packer: output will be {:?}", output_path);

    let temp_audio_path = output_path.with_extension("audio.tmp");
    let mut packer = AudioPacker::new(BufWriter::new(File::create(&temp_audio_path)?));

    let strikes = xml_import::pack_from_xml(samplemaps_dir, samples_dir, &mut packer)?;

    println!("Packer: strikes indexed: {}", strikes.len());
    let audio_bytes = finish_pack(packer, &temp_audio_path, output_path, &strikes)?;
    println!("Packer: total audio bytes: {}", audio_bytes);
    println!("Packer complete! Output written to {:?}", output_path);
    Ok(())
}

fn print_usage() {
    eprintln!(
        "Usage:\n\
         \n\
         Generative (default):\n\
           scd-packer [--strict] [--kit stonehouse] <samples_dir> <output.scdpack>\n\
         \n\
         Legacy HISE XML import:\n\
           scd-packer --from-xml <samplemaps_dir> <samples_dir> <output.scdpack>\n\
         \n\
         The generative packer builds strike maps from kit definitions and the\n\
         naming convention: {{kitpiece}}-{{articulation}}-{{mic}}-{{n}}.wav"
    );
}
