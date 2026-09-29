use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

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

/// UI-only appearance preference, separate from host/preset state.
pub struct AppearanceStore {
    app_name: &'static str,
    appearance: Mutex<Appearance>,
}

impl AppearanceStore {
    pub fn new(app_name: &'static str) -> Self {
        Self {
            app_name,
            appearance: Mutex::new(Appearance::read(app_name)),
        }
    }

    pub fn mode(&self) -> Appearance {
        *self.appearance.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn label(&self) -> &'static str {
        self.mode().label()
    }

    /// Call from the editor thread, never from audio processing.
    pub fn light(&self) -> bool {
        self.mode().resolved() == Appearance::Light
    }

    pub fn toggle(&self) -> bool {
        let mut mode = self.appearance.lock().unwrap_or_else(|e| e.into_inner());
        *mode = match *mode {
            Appearance::Dark => Appearance::Light,
            Appearance::Light => Appearance::Auto,
            _ => Appearance::Dark,
        };
        mode.write(self.app_name);
        mode.resolved() == Appearance::Light
    }
}

/// Cache system reads across editors; no worker survives a plugin unload.
/// Editors repaint regularly, so Auto follows changes within about a second.
fn system_appearance() -> Appearance {
    static CACHE: Mutex<Option<(Instant, Appearance)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((checked, mode)) = *cache {
        if checked.elapsed() < Duration::from_secs(1) {
            return mode;
        }
    }
    let mode = match dark_light::detect() {
        Ok(dark_light::Mode::Light) => Appearance::Light,
        _ => Appearance::Dark,
    };
    *cache = Some((Instant::now(), mode));
    mode
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    Analog,
    Auto,
}

impl Appearance {
    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Analog,
            Self::Analog => Self::Auto,
            Self::Auto => Self::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "DARK",
            Self::Light => "LIGHT",
            Self::Analog => "ANALOG",
            Self::Auto => "AUTO",
        }
    }

    /// Resolve Auto on the editor thread while keeping the saved choice intact.
    pub fn resolved(self) -> Self {
        if self == Self::Auto {
            system_appearance()
        } else {
            self
        }
    }

    pub fn read(app_name: &str) -> Self {
        let value = app_appearance_path(app_name).and_then(|p| std::fs::read_to_string(p).ok());
        Self::parse(value.as_deref().unwrap_or_default())
    }

    fn parse(value: &str) -> Self {
        match value.trim() {
            "light" => Self::Light,
            "analog" => Self::Analog,
            "auto" => Self::Auto,
            _ => Self::Dark,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Analog => "analog",
            Self::Auto => "auto",
        }
    }

    pub fn write(self, app_name: &str) {
        if let Some(path) = app_appearance_path(app_name) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, format!("{}\n", self.as_str()));
        }
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::Appearance;

    #[test]
    fn appearance_cycle_and_persistence_compatibility() {
        let mut mode = Appearance::Dark;
        for expected in [
            Appearance::Light,
            Appearance::Analog,
            Appearance::Auto,
            Appearance::Dark,
        ] {
            mode = mode.next();
            assert_eq!(mode, expected);
            assert_eq!(Appearance::parse(&format!("{}\n", mode.as_str())), mode);
        }
        assert_eq!(Appearance::parse(""), Appearance::Dark);
        assert_eq!(Appearance::parse("unknown"), Appearance::Dark);
    }
}
