//! Offscreen visual check for the actual editor at repeatable host bounds.
use super::*;
use crate::midi_fx::{DelayTap, MidiFxId};
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
        // SAFETY: the canvas has been dropped and this context is ours.
        unsafe {
            CGLSetCurrentContext(std::ptr::null_mut());
            CGLDestroyContext(self.0);
        }
    }
}

#[test]
#[ignore = "Manual macOS OpenGL visual check; writes chordboard-delay preview frames to the temporary directory"]
fn render_delay_preview() {
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
    let mut view = ChordboardView::new(
        Arc::new(ChordboardParams::default()),
        Arc::new(Bridge::default()),
        Arc::new(PreviewContext),
    );
    view.fx_ui.open = true;
    view.params.arpeggiator.edit(|state| {
        let fx = &mut state.midi_fx;
        let index = fx
            .order
            .iter()
            .position(|effect| *effect == MidiFxId::Delay)
            .unwrap();
        fx.move_effect(index, 0);
        fx.delay.count = 2;
        fx.delay.taps[0] = DelayTap {
            feedback: 0.72,
            ..DelayTap::default()
        };
        fx.delay.taps[1] = DelayTap {
            time: 0.75,
            velocity: 0.50,
            feedback: 0.44,
            ..DelayTap::default()
        };
    });
    let feedback = delay_editor::delay_feedback_rect();
    view.pointer = Some((feedback.0 + feedback.2 * 0.5, feedback.1 + feedback.3 * 0.5));
    for (light, label) in [(false, "Dark"), (true, "Light")] {
        for (scale, size) in [(1.0, 100), (0.75, 75)] {
            let width = (EDITOR_W * scale).round() as u32;
            let height = (H * scale).round() as u32;
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
            for (enabled, state) in [(false, "bypassed"), (true, "spotlight")] {
                view.params
                    .arpeggiator
                    .edit(|arp| arp.midi_fx.delay.enabled = enabled);
                view.paint_editor(
                    &mut canvas,
                    BoundingBox::from_min_max(0.0, 0.0, width as f32, height as f32),
                    light,
                    label,
                );
                canvas.flush();
                let pixels = canvas.screenshot().unwrap();
                let path = std::env::temp_dir().join(format!(
                    "chordboard-delay-{}-{size}-{state}.ppm",
                    label.to_lowercase()
                ));
                let mut file = BufWriter::new(std::fs::File::create(path).unwrap());
                writeln!(file, "P6\n{width} {height}\n255").unwrap();
                let rgb: Vec<_> = pixels.buf().iter().flat_map(|p| [p.r, p.g, p.b]).collect();
                file.write_all(&rgb).unwrap();
                file.flush().unwrap();
            }
            canvas.delete_image(target);
        }
    }
}
