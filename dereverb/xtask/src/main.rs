use std::{path::PathBuf, process::Command};

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()?;
    if !std::env::args().any(|a| a == "bundle" || a == "bundle-universal") {
        return Ok(());
    }
    let plugin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| plugin.parent().unwrap().join("target"));
    for format in ["vst3", "clap"] {
        let bundle = target.join(format!("bundled/Dereverb.{format}"));
        if !bundle.is_dir() {
            continue;
        }
        let frameworks = bundle.join("Contents/Frameworks");
        std::fs::create_dir_all(&frameworks)?;
        let runtime = frameworks.join("libonnxruntime.dylib");
        std::fs::copy(plugin.join("runtime/libonnxruntime.dylib"), &runtime)?;
        let licenses = bundle.join("Contents/Resources/licenses");
        std::fs::create_dir_all(&licenses)?;
        for entry in std::fs::read_dir(plugin.join("licenses"))? {
            let entry = entry?;
            std::fs::copy(entry.path(), licenses.join(entry.file_name()))?;
        }
        for path in [&runtime, &bundle] {
            if !Command::new("codesign")
                .args(["--force", "--deep", "--sign", "-"])
                .arg(path)
                .status()?
                .success()
            {
                return Err(
                    std::io::Error::other(format!("Could not sign {}", path.display())).into(),
                );
            }
        }
        if !Command::new("codesign")
            .args(["--verify", "--deep"])
            .arg(&bundle)
            .status()?
            .success()
        {
            return Err(std::io::Error::other("Bundle signature verification failed").into());
        }
        println!("Packaged runtime and notices: {}", bundle.display());
    }
    Ok(())
}
