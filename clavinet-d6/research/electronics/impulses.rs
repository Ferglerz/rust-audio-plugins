//! Write impulse responses for the independent AC sweep in compare.py.
use clavinet_d6::{
    panel::{Board, Panel},
    Pickups, Switch, Wiring,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(std::env::args_os().nth(1).ok_or("expected output folder")?);
    let board = match std::env::args().nth(2).as_deref() {
        Some("e7") => Board::E7,
        None | Some("d6") => Board::D6,
        _ => return Err("expected d6 or e7".into()),
    };
    fs::create_dir_all(&dir)?;
    for (name, mode) in [
        ("CA", Switch::CA),
        ("CB", Switch::CB),
        ("DA", Switch::DA),
        ("DB", Switch::DB),
    ] {
        for mask in 0..=15 {
            let mut panel = Panel::for_board(board, 96_000.0, mode, Wiring::DaAdds, mask, 0.73);
            let file = File::create(dir.join(format!("{name}-{mask}.f64")))?;
            let mut writer = BufWriter::new(file);
            let impulse = Pickups {
                centre: 1.0,
                bridge: 0.37,
            };
            writer.write_all(&panel.process(impulse).to_le_bytes())?;
            for _ in 1..131072 {
                writer.write_all(&panel.process(Pickups::default()).to_le_bytes())?;
            }
            writer.flush()?;
        }
    }
    Ok(())
}
