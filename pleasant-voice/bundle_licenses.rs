// Included by plugin xtasks so both distributions carry the same model notices.
pub fn bundle_voice_licenses(bundle_name: &str) -> nih_plug_xtask::Result<()> {
    use std::path::PathBuf;
    if !std::env::args().any(|a| a == "bundle" || a == "bundle-universal") {
        return Ok(());
    }
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    for format in ["vst3", "clap"] {
        let bundle = target.join(format!("bundled/{bundle_name}.{format}"));
        if !bundle.is_dir() {
            continue;
        }
        let destination = bundle.join("Contents/Resources/licenses");
        std::fs::create_dir_all(&destination)?;
        for entry in std::fs::read_dir(workspace.join("pleasant-voice/licenses"))? {
            let entry = entry?;
            std::fs::copy(entry.path(), destination.join(entry.file_name()))?;
        }
        // Resource changes invalidate the bundler's earlier signature.
        #[cfg(target_os = "macos")]
        for args in [
            &["--force", "--deep", "--sign", "-"][..],
            &["--verify", "--deep"][..],
        ] {
            if !std::process::Command::new("codesign")
                .args(args)
                .arg(&bundle)
                .status()?
                .success()
            {
                return Err(std::io::Error::other("Voice license bundle signing failed").into());
            }
        }
    }
    Ok(())
}
