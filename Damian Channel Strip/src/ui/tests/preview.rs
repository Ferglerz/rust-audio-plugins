//! Offscreen snapshots of the channel-strip editor, independent of the DAW.
use super::layout::test_view;
use super::super::*;
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
#[ignore = "Manual macOS OpenGL check; saves Damian editor frames to the temporary directory"]
fn render_editor_preview() {
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
    let view = test_view();

    for (light, theme) in [(false, "dark"), (true, "light")] {
        for (scale, size) in [(1.0_f32, 100), (0.75, 75)] {
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
            canvas.clear_rect(0, 0, width, height, C::black());
            view.paint_editor(
                &mut canvas,
                BoundingBox::from_min_max(0.0, 0.0, width as f32, height as f32),
                light,
            );
            canvas.flush();
            let pixels = canvas.screenshot().unwrap();
            assert_eq!(pixels.buf().len(), (width as usize) * (height as usize));
            let path = std::env::temp_dir().join(format!("damian-editor-{theme}-{size}.ppm"));
            let mut file = BufWriter::new(std::fs::File::create(&path).unwrap());
            writeln!(file, "P6\n{width} {height}\n255").unwrap();
            let rgb: Vec<_> = pixels.buf().iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            file.write_all(&rgb).unwrap();
            file.flush().unwrap();
            canvas.delete_image(target);
        }
    }
}
