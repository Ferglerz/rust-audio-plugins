//! Offscreen visual check of the editor and frequency probe at repeatable sizes.
use super::*;
use crate::dsp::filter_bank::FilterBank;
use pleasant_ui::preview::OffscreenGl;

#[test]
#[ignore = "Manual macOS OpenGL check; saves Flattery probe frames to the temporary directory"]
fn render_frequency_probe_preview() {
    let mut gl = OffscreenGl::new();

    let params = Arc::new(FlatteryParams::default());
    let shared = Arc::new(Shared::new());
    let fft_size = params.fft_size.value().size();
    let sample_rate = f64::from(shared.sample_rate.load(Ordering::Relaxed));
    let bump = |freq: f64, center: f64, width: f64| (-(freq / center).ln().powi(2) / width).exp();
    *shared.spectrum_mags_db.write().unwrap() = (0..fft_size / 2)
        .map(|i| {
            let freq = (i as f64 + 0.5) * sample_rate / fft_size as f64;
            (-71.0
                + 21.0 * bump(freq, 260.0, 0.5)
                + 42.0 * bump(freq, 1075.0, 0.28)
                + 17.0 * bump(freq, 4200.0, 0.8)) as f32
        })
        .collect();
    let mut filters = FilterBank::new();
    filters.init_frequencies(fft_size, sample_rate);
    *shared.filter_display.write().unwrap() = filters
        .filters
        .iter()
        .map(|filter| {
            let freq = filter.center_hz;
            (
                freq as f32,
                (2.7 * bump(freq, 410.0, 0.24) - 5.8 * bump(freq, 1100.0, 0.19)) as f32,
            )
        })
        .collect();
    let mut view = FlatteryView::new(params, shared);
    view.hover = Some((view.layout.freq_to_x(1100.0), GRAPH_Y + 120.0));

    for (light, theme) in [(false, "dark"), (true, "light")] {
        for (scale, size) in [(1.0, 100), (0.75, 75)] {
            let width = (WINDOW_W * scale).round() as u32;
            let height = (WINDOW_H * scale).round() as u32;
            let path = std::env::temp_dir().join(format!("flattery-probe-{theme}-{size}.ppm"));
            gl.render_ppm(width, height, path, |canvas| {
                view.paint_editor(
                    canvas,
                    BoundingBox::from_min_max(0.0, 0.0, width as f32, height as f32),
                    light,
                    if light { "Light" } else { "Dark" },
                );
            });
        }
    }
}
