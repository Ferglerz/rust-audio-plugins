//! Offscreen render of the actual editor artwork; no DAW or UI automation.
use super::*;
use nih_plug_vizia::vizia::vg::{renderer::OpenGl, ImageFlags, PixelFormat, RenderTarget};
use std::ffi::c_void;

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
}
struct Context(*mut c_void);
impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: owns the context created below, after the canvas has been dropped.
        unsafe {
            CGLSetCurrentContext(std::ptr::null_mut());
            CGLDestroyContext(self.0);
        }
    }
}

#[test]
#[ignore = "Manual offscreen UI check; writes /tmp/automixer-{dark,light}.png"]
fn render_editor_preview() {
    let mut format = std::ptr::null_mut();
    let mut context = std::ptr::null_mut();
    let mut count = 0;
    // SAFETY: terminated attribute list and valid output pointers.
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
    // SAFETY: the current OpenGL context outlives renderer and canvas.
    let renderer = unsafe {
        OpenGl::new_from_function_cstr(|s| libc::dlsym(libc::RTLD_DEFAULT, s.as_ptr()).cast())
    }
    .unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();
    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..8).map(|_| network.register()).collect();
    let params = Arc::new(AutomixerParams::new(
        network.params.clone(),
        registrations[0].slot().cloned(),
        registrations[0].token,
    ));
    for (i, name) in [
        "HOST", "ALICE", "BOB", "CHOIR L", "CHOIR R", "ENSEMBLE", "GUEST", "SPARE",
    ]
    .into_iter()
    .enumerate()
    {
        *network.params.mics[i].name.lock().unwrap() = name.into();
        let slot = &network.slots[i];
        let input = [-21.0, -26.0, -31.0, -48.0, -43.0, -53.0, -57.0, -60.0][i];
        let reduction = [1.5, 5.2, 9.8, 18.0, 16.5, 18.0, 18.0, 18.0][i];
        slot.input_db.store(input, Ordering::Relaxed);
        slot.output_db.store(input - reduction, Ordering::Relaxed);
        slot.reduction_db.store(reduction, Ordering::Relaxed);
        slot.state.store(
            if i < 3 {
                network::MIXING
            } else {
                network::QUIET
            },
            Ordering::Relaxed,
        );
        slot.active.store(i < 3, Ordering::Relaxed);
        slot.publish(0.1, i < 3);
        slot.updated_ms.store(network.now_ms(), Ordering::Relaxed);
    }
    network.params.midi(0, 21, 92.0 / 127.0);
    let font = canvas.add_font_mem(FONT_JETBRAINS_MONO).unwrap();
    let view = AutomixerView {
        params,
        network,
        local_slot: Some(0),
        font: Cell::new(Some(font)),
        hover: None,
        page: 0,
        selected: 1,
        drag: None,
        press: None,
        edit: None,
    };
    for (light, name, scale) in [(false, "dark", 1.0), (true, "light", 1.5)] {
        for slot in &view.network.slots {
            slot.updated_ms
                .store(view.network.now_ms(), Ordering::Relaxed);
        }
        let width = (SIZE.0 * scale) as u32;
        let height = (SIZE.1 * scale) as u32;
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
        canvas.clear_rect(
            0,
            0,
            width,
            height,
            nih_plug_vizia::vizia::vg::Color::black(),
        );
        let mut draw = Draw::new_fitted(
            &mut canvas,
            light,
            (0.0, 0.0, width as f32, height as f32),
            SIZE,
            Some(font),
        )
        .unwrap();
        view.paint(&mut draw);
        canvas.flush();
        let pixels = canvas.screenshot().unwrap();
        let bytes: Vec<u8> = pixels
            .buf()
            .iter()
            .flat_map(|p| [p.r, p.g, p.b, p.a])
            .collect();
        image::save_buffer(
            format!("/tmp/automixer-{name}.png"),
            &bytes,
            width,
            height,
            image::ColorType::Rgba8,
        )
        .unwrap();
        canvas.delete_image(target);
    }
}
