//! Offscreen visual check of the editor and frequency probe at repeatable sizes.
use super::*;
use crate::dsp::filter_bank::FilterBank;
use nih_plug_vizia::vizia::vg::{renderer::OpenGl, ImageFlags, PixelFormat, RenderTarget};
use std::{
    ffi::{c_char, c_void},
    io::{BufWriter, Write},
};

#[link(name = "OpenGL", kind = "framework")]
extern "C" {
    fn CGLChoosePixelFormat(
        attributes: *const u32,
        format: *mut *mut c_void,
        count: *mut i32,
    ) -> i32;
    fn CGLCreateContext(format: *mut c_void, share: *mut c_void, context: *mut *mut c_void) -> i32;
    fn CGLSetCurrentContext(context: *mut c_void) -> i32;
    fn CGLDestroyContext(context: *mut c_void) -> i32;
    fn CGLDestroyPixelFormat(format: *mut c_void) -> i32;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

struct GlContext(*mut c_void);

impl Drop for GlContext {
    fn drop(&mut self) {
        // SAFETY: this context is ours and the canvas has been dropped.
        unsafe {
            CGLSetCurrentContext(std::ptr::null_mut());
            CGLDestroyContext(self.0);
        }
    }
}

#[test]
#[ignore = "Manual macOS OpenGL check; saves Flattery probe frames to the temporary directory"]
fn render_frequency_probe_preview() {
    let mut format = std::ptr::null_mut();
    let mut context = std::ptr::null_mut();
    let mut count = 0;
    // SAFETY: the attribute list is terminated and the output pointers are valid.
    unsafe {
        assert_eq!(
            CGLChoosePixelFormat([99, 0x3200, 0].as_ptr(), &mut format, &mut count),
            0
        );
        assert_eq!(
            CGLCreateContext(format, std::ptr::null_mut(), &mut context),
            0
        );
        CGLDestroyPixelFormat(format);
        assert_eq!(CGLSetCurrentContext(context), 0);
    }
    let _context = GlContext(context);
    // SAFETY: a compatible context is current. Darwin's RTLD_DEFAULT is -2.
    let renderer = unsafe {
        OpenGl::new_from_function_cstr(|name| dlsym(-2_isize as *mut c_void, name.as_ptr()).cast())
    }
    .unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();

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
            canvas.set_size(width, height, 1.0);
            let target = canvas
                .create_image_empty(
                    width as usize,
                    height as usize,
                    PixelFormat::Rgba8,
                    ImageFlags::empty(),
                )
                .unwrap();
            canvas.set_render_target(RenderTarget::Image(target));
            canvas.clear_rect(0, 0, width, height, Color::black());
            view.paint_editor(
                &mut canvas,
                BoundingBox::from_min_max(0.0, 0.0, width as f32, height as f32),
                light,
                if light { "Light" } else { "Dark" },
            );
            canvas.flush();
            let pixels = canvas.screenshot().unwrap();
            let path = std::env::temp_dir().join(format!("flattery-probe-{theme}-{size}.ppm"));
            let mut file = BufWriter::new(std::fs::File::create(path).unwrap());
            writeln!(file, "P6\n{width} {height}\n255").unwrap();
            let rgb: Vec<_> = pixels.buf().iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            file.write_all(&rgb).unwrap();
            file.flush().unwrap();
            canvas.delete_image(target);
        }
    }
}
