//! Offscreen macOS OpenGL (CGL) canvas harness for headless UI previews and visual tests.

use nih_plug_vizia::vizia::vg::{
    renderer::OpenGl, Canvas, Color, ImageFlags, PixelFormat, RenderTarget,
};
use std::{
    ffi::{c_char, c_void},
    io::{BufWriter, Write},
    path::Path,
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
        // SAFETY: this context was created by `OffscreenGl::new` and `canvas` drops first.
        unsafe {
            CGLSetCurrentContext(std::ptr::null_mut());
            CGLDestroyContext(self.0);
        }
    }
}

/// Owned offscreen OpenGL canvas backed by a macOS CGL context.
pub struct OffscreenGl {
    // Field order matters: `canvas` must drop while `_context` is still current.
    canvas: Canvas<OpenGl>,
    _context: GlContext,
}

impl OffscreenGl {
    pub fn new() -> Self {
        let mut format = std::ptr::null_mut();
        let mut context = std::ptr::null_mut();
        let mut count = 0;
        // SAFETY: the attribute list is 0-terminated and all out-pointers are valid.
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
        let gl_context = GlContext(context);
        // SAFETY: a compatible CGL context is current; Darwin's RTLD_DEFAULT is -2.
        let renderer = unsafe {
            OpenGl::new_from_function_cstr(|name| {
                dlsym(-2_isize as *mut c_void, name.as_ptr()).cast()
            })
        }
        .expect("create OpenGL renderer");
        let canvas = Canvas::new(renderer).expect("create femtovg canvas");
        Self {
            canvas,
            _context: gl_context,
        }
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas<OpenGl> {
        &mut self.canvas
    }

    /// Render `draw` into an offscreen RGBA8 target and save as a binary PPM (`P6`) file.
    pub fn render_ppm(
        &mut self,
        width: u32,
        height: u32,
        path: impl AsRef<Path>,
        draw: impl FnOnce(&mut Canvas<OpenGl>),
    ) {
        self.canvas.set_size(width, height, 1.0);
        let target = self
            .canvas
            .create_image_empty(
                width as usize,
                height as usize,
                PixelFormat::Rgba8,
                ImageFlags::empty(),
            )
            .expect("create offscreen render target");
        self.canvas.set_render_target(RenderTarget::Image(target));
        self.canvas.clear_rect(0, 0, width, height, Color::black());
        draw(&mut self.canvas);
        self.canvas.flush();
        let pixels = self.canvas.screenshot().expect("read offscreen pixels");
        let mut file = BufWriter::new(std::fs::File::create(path).expect("create preview file"));
        writeln!(file, "P6\n{width} {height}\n255").expect("write PPM header");
        let rgb: Vec<_> = pixels.buf().iter().flat_map(|p| [p.r, p.g, p.b]).collect();
        file.write_all(&rgb).expect("write PPM pixels");
        file.flush().expect("flush PPM file");
        self.canvas.delete_image(target);
    }
}

impl Default for OffscreenGl {
    fn default() -> Self {
        Self::new()
    }
}
