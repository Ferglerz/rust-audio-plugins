include!("../../../pleasant-voice/bundle_licenses.rs");

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()?;
    bundle_voice_licenses("Damian Channel Strip")
}
