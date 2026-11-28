//! Stream two independent unit impulses through one fully loaded registration.
use clavinet_d6::Switch;
use hohner_duo::{circuit::Panel, Controls, Split};
use std::{
    env,
    error::Error,
    io::{self, BufWriter, Write},
};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 8 {
        return Err(
            "expected split pickup tone-mask tab2 tab3 foot clav-volume pianet-volume".into(),
        );
    }
    let split = [
        Split::Preset,
        Split::PianetTreble,
        Split::ClavinetTreble,
        Split::BothTreble,
    ][args[0].parse::<usize>()?];
    let pickup = [Switch::CA, Switch::CB, Switch::DA, Switch::DB][args[1].parse::<usize>()?];
    let controls = Controls {
        split,
        pickup,
        tones: args[2].parse()?,
        tab_two: args[3] == "1",
        tab_three: args[4] == "1",
        foot: args[5] == "1",
        clavinet_volume: args[6].parse()?,
        pianet_volume: args[7].parse()?,
        ..Default::default()
    };
    let mut p = Panel::new(96000., controls);
    let mut w = BufWriter::new(io::stdout().lock());
    for port in 0..2 {
        p.reset();
        for i in 0..131072 {
            let c = f64::from(i == 0 && port == 0);
            let r = f64::from(i == 0 && port == 1);
            for v in p.tick(c, 0.37 * c, r) {
                w.write_all(&v.to_le_bytes())?;
            }
        }
    }
    w.flush()?;
    Ok(())
}
