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
#[ignore = "Manual macOS OpenGL visual check; writes /tmp/dereverb-{dark,light}.png"]
fn render_editor_preview() {
    let mut format = std::ptr::null_mut();
    let mut context = std::ptr::null_mut();
    let mut count = 0;
    // SAFETY: correctly terminated CGL attribute list and valid output pointers.
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
    // SAFETY: a compatible OpenGL context is current for the canvas's entire lifetime.
    let renderer = unsafe {
        OpenGl::new_from_function_cstr(|s| libc::dlsym(libc::RTLD_DEFAULT, s.as_ptr()).cast())
    }
    .unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();
    let shared = Arc::new(Telemetry::default());
    shared.status.store(telemetry::ACTIVE, Ordering::Relaxed);
    shared.sample_rate.store(44100.0, Ordering::Relaxed);
    shared.latency.store(2772, Ordering::Relaxed);
    for i in 0..POINTS {
        let phase = i as f32 / POINTS as f32;
        let dry = 0.12 * (phase * 19.0).sin().abs() + 0.02;
        shared.push(dry, dry * (0.45 + 0.25 * (phase * 7.0).cos()));
    }
    let font = canvas.add_font_mem(FONT_JETBRAINS_MONO).unwrap();
    let mut params = DereverbParams::default();
    for (parameter, value) in [
        &mut params.weight_0,
        &mut params.weight_1,
        &mut params.weight_2,
        &mut params.weight_3,
        &mut params.weight_4,
    ]
    .into_iter()
    .zip(LOUDER)
    {
        *parameter = FloatParam::new(
            "Preview weight",
            value * 100.0,
            FloatRange::Linear {
                min: 0.0,
                max: 100.0,
            },
        );
    }
    let level = crate::weighting::FrameLevel {
        absolute_db: -24.0,
        relative_db: -4.0,
        active: true,
        ..Default::default()
    };
    let weight = params.weighting().amount(level);
    shared.set_activity(level, weight, weight);
    let mut view = DereverbView {
        params: Arc::new(params),
        shared,
        font: Cell::new(Some(font)),
        hover: None,
        drag: None,
        curve_drag: None,
        guard_drag: None,
        press: None,
        edit: None,
    };
    for (light, name, scale, engine) in [
        (false, "dark", 1.0, EngineKind::DpdfNet),
        (true, "light", 1.5, EngineKind::DpdfNet),
        (false, "wpe-dark", 1.0, EngineKind::NaraWpe),
        (true, "wpe-light", 1.5, EngineKind::NaraWpe),
        (false, "dtln-dark", 1.0, EngineKind::Dtln),
    ] {
        Arc::get_mut(&mut view.params).unwrap().engine = EnumParam::new("Engine", engine);
        view.shared.engine.store(engine as u32, Ordering::Relaxed);
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
        // The offscreen target's stencil storage is undefined until cleared.
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
            format!("/tmp/dereverb-{name}.png"),
            &bytes,
            width,
            height,
            image::ColorType::Rgba8,
        )
        .unwrap();
        canvas.delete_image(target);
    }
}
