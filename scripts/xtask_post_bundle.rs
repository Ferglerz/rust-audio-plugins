// Included by plugin xtasks to package extra runtime files and notices, then re-sign on macOS.
pub struct BundleCopy<'a> {
    pub source: &'a std::path::Path,
    pub dest_dir: &'a str,
    pub sign_separately: bool,
}

pub fn post_bundle(bundle_name: &str, copies: &[BundleCopy<'_>]) -> nih_plug_xtask::Result<()> {
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
        #[cfg(target_os = "macos")]
        let mut sign_targets: Vec<PathBuf> = Vec::new();
        for copy in copies {
            let dest_dir = bundle.join(copy.dest_dir);
            std::fs::create_dir_all(&dest_dir)?;
            if copy.source.is_dir() {
                for entry in std::fs::read_dir(copy.source)? {
                    let entry = entry?;
                    std::fs::copy(entry.path(), dest_dir.join(entry.file_name()))?;
                }
            } else {
                let file_name = copy.source.file_name().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Invalid source file: {}", copy.source.display()),
                    )
                })?;
                let dest = dest_dir.join(file_name);
                std::fs::copy(copy.source, &dest)?;
                #[cfg(target_os = "macos")]
                if copy.sign_separately {
                    sign_targets.push(dest);
                }
                #[cfg(not(target_os = "macos"))]
                let _ = (dest, copy.sign_separately);
            }
        }
        // Resource changes invalidate the bundler's earlier signature.
        #[cfg(target_os = "macos")]
        {
            for path in sign_targets.iter().chain(std::iter::once(&bundle)) {
                if !std::process::Command::new("codesign")
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
            if !std::process::Command::new("codesign")
                .args(["--verify", "--deep"])
                .arg(&bundle)
                .status()?
                .success()
            {
                return Err(std::io::Error::other("Bundle signature verification failed").into());
            }
        }
        println!("Packaged bundle resources: {}", bundle.display());
    }
    Ok(())
}

pub fn bundle_voice_licenses(bundle_name: &str) -> nih_plug_xtask::Result<()> {
    let licenses = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("pleasant-voice/licenses");
    post_bundle(
        bundle_name,
        &[BundleCopy {
            source: &licenses,
            dest_dir: "Contents/Resources/licenses",
            sign_separately: false,
        }],
    )
}
