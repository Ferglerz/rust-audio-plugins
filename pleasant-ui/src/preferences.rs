use std::{path::PathBuf, sync::Mutex};

pub fn app_appearance_path(app_name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var_os("APPDATA")?);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))?;
    Some(base.join(app_name).join("appearance.txt"))
}

pub fn read_appearance(app_name: &str) -> bool {
    app_appearance_path(app_name)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .is_some_and(|s| s.trim() == "light")
}

pub fn write_appearance(app_name: &str, light: bool) {
    if let Some(path) = app_appearance_path(app_name) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, if light { "light\n" } else { "dark\n" });
    }
}

pub struct AppearanceStore {
    app_name: &'static str,
    light: Mutex<bool>,
}

impl AppearanceStore {
    pub fn new(app_name: &'static str) -> Self {
        let is_light = read_appearance(app_name);
        Self {
            app_name,
            light: Mutex::new(is_light),
        }
    }

    pub fn light(&self) -> bool {
        match self.light.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    pub fn toggle(&self) -> bool {
        let mut light = match self.light.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *light = !*light;
        write_appearance(self.app_name, *light);
        *light
    }
}
