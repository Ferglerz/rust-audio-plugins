//! Offscreen snapshots of the actual editor, independent of the DAW.
use super::*;
use crate::telemetry::{TransportFrame, TransportStage};
use nih_plug_vizia::vizia::vg::{renderer::OpenGl, ImageFlags, PixelFormat, RenderTarget};
use std::{
    ffi::{c_char, c_void},
    io::Write,
    time::{Duration, Instant},
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

struct Context(*mut c_void);

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: this context was created here and outlives the canvas.
        unsafe {
            CGLSetCurrentContext(std::ptr::null_mut());
            CGLDestroyContext(self.0);
        }
    }
}

#[test]
#[ignore = "Manual offscreen UI check; writes tape-stop preview images to the temporary directory"]
fn render_reels_preview() {
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
    let _context = Context(context);
    // SAFETY: the current OpenGL context outlives the renderer and canvas.
    // Darwin's RTLD_DEFAULT searches the linked images, including OpenGL.
    let renderer = unsafe {
        OpenGl::new_from_function_cstr(|symbol| {
            dlsym(-2_isize as *mut c_void, symbol.as_ptr()).cast()
        })
    }
    .unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();
    let font = canvas.add_font_mem(FONT_JETBRAINS_MONO).unwrap();
    for (light, theme, scale) in [(false, "dark", 1.0), (true, "light", 0.75)] {
        let width = (UI_W * scale).round() as u32;
        let height = (UI_H * scale).round() as u32;
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
        let view = interaction_tests::view();
        let start = Instant::now();
        {
            let mut d = Draw::new_fitted(
                &mut canvas,
                light,
                (0.0, 0.0, width as f32, height as f32),
                (UI_W, UI_H),
                Some(font),
            )
            .unwrap();
            view.paint(&mut d, start);
        }
        for (i, (name, stage, speed)) in [
            ("live", TransportStage::Live, [1.0, 1.0]),
            ("braking", TransportStage::Braking, [0.25, 0.60]),
            ("return", TransportStage::Returning, [2.8, 3.2]),
            ("crossfade", TransportStage::Crossfading, [1.0, 1.0]),
        ]
        .into_iter()
        .enumerate()
        {
            view.telemetry.publish_transport(TransportFrame {
                stage,
                speed,
                brake_progress: [0.7, 0.5],
            });
            let mut d = Draw::new_fitted(
                &mut canvas,
                light,
                (0.0, 0.0, width as f32, height as f32),
                (UI_W, UI_H),
                Some(font),
            )
            .unwrap();
            view.paint(&mut d, start + Duration::from_millis((i as u64 + 1) * 33));
            canvas.flush();
            let pixels = canvas.screenshot().unwrap();
            let path = std::env::temp_dir().join(format!("tape-stop-{name}-{theme}.ppm"));
            let mut file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
            writeln!(file, "P6\n{width} {height}\n255").unwrap();
            let rgb: Vec<_> = pixels.buf().iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            file.write_all(&rgb).unwrap();
            file.flush().unwrap();
        }
        canvas.delete_image(target);
    }
}
