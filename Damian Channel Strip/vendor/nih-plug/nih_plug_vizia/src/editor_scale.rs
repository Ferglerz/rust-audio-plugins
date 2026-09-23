//! Last user scale for new plugin instances.
//!
//! Each plugin instance still stores its own scale in `ViziaState`. This file
//! remembers the most recent resize so the next new instance opens at that size.

use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_SCALE: f64 = 1.0 / 1.2;
const MIN_SCALE: f64 = 0.2;
const MAX_SCALE: f64 = 4.0;

pub(crate) fn default_scale() -> f64 {
    DEFAULT_SCALE
}

pub(crate) fn clamp_scale(scale: f64) -> Option<f64> {
    (scale.is_finite() && (MIN_SCALE..=MAX_SCALE).contains(&scale)).then_some(scale)
}

/// Convert a host-resized unscaled window width back into a user scale.
///
/// `window_w` is Vizia's logical width before the user scale. The host's
/// logical width is `window_w * user_scale`.
pub(crate) fn scale_from_host_resize(
    artwork_w: u32,
    window_w: u32,
    user_scale: f64,
) -> Option<f64> {
    if artwork_w == 0 || window_w == 0 || !user_scale.is_finite() || user_scale <= 0.0 {
        return None;
    }
    if window_w.abs_diff(artwork_w) <= 2 {
        return None;
    }
    clamp_scale(user_scale * f64::from(window_w) / f64::from(artwork_w))
}

pub(crate) fn load_scale(plugin_id: &str) -> Option<f64> {
    let path = scale_path(plugin_id)?;
    let text = fs::read_to_string(path).ok()?;
    clamp_scale(text.trim().parse().ok()?)
}

pub(crate) fn save_scale(plugin_id: &str, scale: f64) {
    let Some(scale) = clamp_scale(scale) else {
        return;
    };
    let Some(path) = scale_path(plugin_id) else {
        return;
    };
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let tmp = path.with_extension("scale.tmp");
    if fs::write(&tmp, format!("{scale:?}\n")).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

fn scale_path(plugin_id: &str) -> Option<PathBuf> {
    if plugin_id.is_empty() {
        return None;
    }
    let root = std::env::var_os("PLEASANT_EDITOR_SCALE_DIR")
        .map(PathBuf::from)
        .or_else(config_dir)?;
    Some(root.join(sanitize(plugin_id)).join("editor-scale"))
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support/Pleasant Audio"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|root| PathBuf::from(root).join("Pleasant Audio"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|root| root.join("Pleasant Audio"))
    }
}

fn sanitize(plugin_id: &str) -> String {
    plugin_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn lock_scale_dir() -> std::sync::MutexGuard<'static, ()> {
    static DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    DIR_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_resize_maps_width_back_to_user_scale() {
        assert_eq!(scale_from_host_resize(1282, 1282, 0.833), None);
        assert_eq!(scale_from_host_resize(1282, 1284, 0.833), None);
        let scale = scale_from_host_resize(1282, 1921, 0.833).unwrap();
        assert!((scale - 1.248).abs() < 0.01);
        assert_eq!(scale_from_host_resize(0, 100, 1.0), None);
    }

    #[test]
    fn saved_scale_round_trips_without_touching_instance_state() {
        let _guard = lock_scale_dir();
        let dir = std::env::temp_dir().join("pleasant-editor-scale-test");
        let _ = fs::remove_dir_all(&dir);
        std::env::set_var("PLEASANT_EDITOR_SCALE_DIR", &dir);

        assert_eq!(load_scale("Damian Channel Strip"), None);
        save_scale("Damian Channel Strip", 1.37);
        assert_eq!(load_scale("Damian Channel Strip"), Some(1.37));
        save_scale("Damian Channel Strip", f64::NAN);
        assert_eq!(load_scale("Damian Channel Strip"), Some(1.37));

        let path = scale_path("Damian Channel Strip").unwrap();
        fs::write(&path, "nope\n").unwrap();
        assert_eq!(load_scale("Damian Channel Strip"), None);
        assert!(path.starts_with(Path::new(&dir)));

        std::env::remove_var("PLEASANT_EDITOR_SCALE_DIR");
        let _ = fs::remove_dir_all(&dir);
    }
}
