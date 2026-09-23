use std::env;
use std::path::Path;
use std::process::Command;

fn main() -> nih_plug_xtask::Result<()> {
    let args: Vec<String> = env::args().collect();
    nih_plug_xtask::main()?;

    if is_bundle_command(&args) {
        install_clap_bundle();
    }

    Ok(())
}

fn is_bundle_command(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "bundle" || arg == "bundle-universal")
}

fn install_clap_bundle() {
    if !cfg!(target_os = "macos") {
        return;
    }

    let clap_src = Path::new("target/bundled/composure.clap");
    if !clap_src.exists() {
        return;
    }

    let Ok(home) = env::var("HOME") else {
        eprintln!("CLAP install skipped: HOME not set");
        return;
    };

    let dest_dir = Path::new(&home).join("Library/Audio/Plug-Ins/CLAP");
    if let Err(err) = std::fs::create_dir_all(&dest_dir) {
        eprintln!("CLAP install failed: could not create '{}': {err}", dest_dir.display());
        return;
    }

    let status = Command::new("cp")
        .args(["-R", clap_src.to_str().unwrap(), dest_dir.to_str().unwrap()])
        .status();

    match status {
        Ok(status) if status.success() => {
            eprintln!(
                "Installed CLAP bundle to '{}'",
                dest_dir.join("composure.clap").display()
            );
        }
        Ok(status) => {
            eprintln!("CLAP install failed: cp exited with {status}");
        }
        Err(err) => {
            eprintln!("CLAP install failed: {err}");
        }
    }
}
