include!("../../../scripts/xtask_post_bundle.rs");

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()?;
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    bundle_voice_licenses(&workspace, "Damian Channel Strip")
}
