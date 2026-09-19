use anyhow::bail;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> nih_plug_xtask::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Prior to building, attempt to delete all existing bundles from target and plugin directories
    if is_bundle_command(&args) {
        clean_existing_bundles()?;
    }

    nih_plug_xtask::main()?;

    if is_bundle_command(&args) {
        install_bundles()?;
    }

    Ok(())
}

fn is_bundle_command(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "bundle" || arg == "bundle-universal")
}

fn get_all_bundle_paths() -> Vec<(PathBuf, &'static str)> {
    let mut paths = vec![
        (
            PathBuf::from("target/bundled/tape_stop.clap"),
            "target CLAP bundle",
        ),
        (
            PathBuf::from("target/bundled/tape_stop.vst3"),
            "target VST3 bundle",
        ),
        (
            PathBuf::from("/Library/Audio/Plug-Ins/CLAP/tape_stop.clap"),
            "System CLAP plugin",
        ),
        (
            PathBuf::from("/Library/Audio/Plug-Ins/VST3/tape_stop.vst3"),
            "System VST3 plugin",
        ),
    ];

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        paths.push((
            home.join("Library/Audio/Plug-Ins/CLAP/tape_stop.clap"),
            "User CLAP plugin",
        ));
        paths.push((
            home.join("Library/Audio/Plug-Ins/VST3/tape_stop.vst3"),
            "User VST3 plugin",
        ));
    }

    paths
}

fn clean_existing_bundles() -> nih_plug_xtask::Result<()> {
    for (path, desc) in get_all_bundle_paths() {
        if path.exists() {
            eprintln!(
                "Attempting to delete existing {desc} at '{}'...",
                path.display()
            );
            if let Err(err) = std::fs::remove_dir_all(&path) {
                // Fallback to rm -rf
                let rm_res = Command::new("rm")
                    .args(["-rf", path.to_str().unwrap()])
                    .status();

                if path.exists() || rm_res.is_err() || !rm_res.unwrap().success() {
                    bail!(
                        "ABORT: Cannot delete existing {desc} at '{}' ({err}). The file is locked or in use by a running host (e.g. Reaper). Please close the DAW or remove the plugin instance and try again.",
                        path.display()
                    );
                }
            }
            eprintln!("Successfully deleted {desc}.");
        }
    }
    Ok(())
}

fn copy_bundle(src: &Path, dest_dirs: &[PathBuf], bundle_name: &str) -> nih_plug_xtask::Result<()> {
    if !src.exists() {
        return Ok(());
    }

    for dest_dir in dest_dirs {
        if let Err(err) = std::fs::create_dir_all(dest_dir) {
            eprintln!("Could not create directory '{}': {err}", dest_dir.display());
            continue;
        }

        let target_path = dest_dir.join(bundle_name);
        if target_path.exists() {
            if let Err(err) = std::fs::remove_dir_all(&target_path) {
                let rm_res = Command::new("rm")
                    .args(["-rf", target_path.to_str().unwrap()])
                    .status();
                if target_path.exists() || rm_res.is_err() || !rm_res.unwrap().success() {
                    bail!(
                        "ABORT: Cannot overwrite existing bundle at '{}' ({err}). Please close any DAW currently running the plugin.",
                        target_path.display()
                    );
                }
            }
        }

        let status = Command::new("cp")
            .args(["-R", src.to_str().unwrap(), target_path.to_str().unwrap()])
            .status();

        match status {
            Ok(status) if status.success() => {
                eprintln!("Installed {} to '{}'", bundle_name, target_path.display());
            }
            Ok(status) => {
                bail!(
                    "Install to '{}' failed: cp exited with {status}",
                    target_path.display()
                );
            }
            Err(err) => {
                bail!("Install to '{}' failed: {err}", target_path.display());
            }
        }
    }

    Ok(())
}

fn install_bundles() -> nih_plug_xtask::Result<()> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }

    let mut clap_destinations = vec![PathBuf::from("/Library/Audio/Plug-Ins/CLAP")];
    let mut vst3_destinations = vec![PathBuf::from("/Library/Audio/Plug-Ins/VST3")];

    if let Some(home) = std::env::var_os("HOME") {
        clap_destinations.push(PathBuf::from(&home).join("Library/Audio/Plug-Ins/CLAP"));
        vst3_destinations.push(PathBuf::from(&home).join("Library/Audio/Plug-Ins/VST3"));
    }

    // Install CLAP bundle
    let clap_src = Path::new("target/bundled/tape_stop.clap");
    copy_bundle(clap_src, &clap_destinations, "tape_stop.clap")?;

    // Install VST3 bundle
    let vst3_src = Path::new("target/bundled/tape_stop.vst3");
    copy_bundle(vst3_src, &vst3_destinations, "tape_stop.vst3")?;

    Ok(())
}
