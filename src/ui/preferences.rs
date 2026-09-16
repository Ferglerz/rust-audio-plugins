//! User appearance preferences deliberately live outside host/preset state.
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

static LIGHT: OnceLock<Mutex<bool>> = OnceLock::new();
fn path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var_os("APPDATA")?);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))?;
    Some(base.join("Damian Channel Strip").join("appearance.txt"))
}
fn preference() -> &'static Mutex<bool> {
    LIGHT.get_or_init(|| {
        Mutex::new(
            path()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .is_some_and(|s| s.trim() == "light"),
        )
    })
}
pub fn light() -> bool {
    *preference().lock().unwrap()
}
pub fn toggle() {
    let mut light = preference().lock().unwrap();
    *light = !*light;
    if let Some(path) = path() {
        let result = std::fs::create_dir_all(path.parent().unwrap())
            .and_then(|_| std::fs::write(path, if *light { "light\n" } else { "dark\n" }));
        if let Err(error) = result {
            eprintln!("Could not save Damian appearance: {error}");
        }
    }
}
