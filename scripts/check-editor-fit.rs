//! Focused host-resize regression check without building/testing vendored crates.
//! Run: rustc --edition 2021 scripts/check-editor-fit.rs -o /tmp/check-editor-fit && /tmp/check-editor-fit
#[allow(dead_code)]
#[path = "../Damian Channel Strip/vendor/nih-plug/nih_plug_vizia/src/editor_scale.rs"]
mod editor_scale;

fn main() {
    for artwork in [(1349, 721), (1040, 860), (1282, 600)] {
        for current_scale in [0.2, 0.833, 1.0, 2.0] {
            for window in [(1100, 180), (300, 900), (40, 20), artwork, (4000, 3000)] {
                let scale = editor_scale::scale_from_host_resize(artwork, window, current_scale)
                    .expect("positive host bounds must keep fitting, even below minimum scale");
                let available = (
                    window.0 as f64 * current_scale,
                    window.1 as f64 * current_scale,
                );
                let fitted = (artwork.0 as f64 * scale, artwork.1 as f64 * scale);
                assert!(
                    fitted.0 <= available.0 + 1e-9 && fitted.1 <= available.1 + 1e-9,
                    "editor must fit both host axes: {artwork:?}, {window:?}, {current_scale}"
                );
                assert!(
                    (fitted.0 - available.0).abs() < 1e-9 || (fitted.1 - available.1).abs() < 1e-9
                );
            }
        }
    }
    // Model successive host rectangles, always deriving the next logical size
    // from the available host bounds, never the previously fitted child.
    let artwork = (1349, 721);
    let mut scale = 1.0;
    for host in [
        (1200.0, 180.0),
        (1200.0, 800.0),
        (1800.0, 800.0),
        (1800.0, 1200.0),
        (300.0, 900.0),
        (1400.0, 900.0),
    ] {
        let logical = (
            (host.0 / scale as f64).round() as u32,
            (host.1 / scale as f64).round() as u32,
        );
        scale = editor_scale::scale_from_host_resize(artwork, logical, scale).unwrap();
        let expected = (host.0 / artwork.0 as f64).min(host.1 / artwork.1 as f64);
        assert!(
            (scale - expected).abs() * (artwork.0 as f64) < 2.0,
            "shrink/grow sequence must keep filling one host axis"
        );
    }
    for window in [(0, 100), (100, 0)] {
        assert!(editor_scale::scale_from_host_resize((100, 100), window, 1.0).is_none());
    }
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(editor_scale::scale_from_host_resize((100, 100), (50, 50), scale).is_none());
    }
    println!("Host fit regression checks passed (60 size/scale combinations and invalid bounds).");
}
