//! Offscreen render of the actual editor artwork; no DAW or UI automation.
use super::*;
use nih_plug_vizia::vizia::vg::{ImageFlags, PixelFormat, RenderTarget};
use pleasant_ui::preview::OffscreenGl;

#[test]
#[ignore = "Manual macOS OpenGL visual check; writes /tmp/dereverb-{dark,light}.png"]
fn render_editor_preview() {
    let mut gl = OffscreenGl::new();
    let canvas = gl.canvas_mut();
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
        label_font: Cell::new(None),
        hover: None,
        drag: None,
        curve_drag: None,
        guard_drag: None,
        press: None,
        edit: None,
    };
    for (light, name, scale, engine, inspecting) in [
        (false, "dark", 1.0, EngineKind::DpdfNet, false),
        (true, "light", 1.5, EngineKind::DpdfNet, false),
        (false, "wpe-dark", 1.0, EngineKind::NaraWpe, false),
        (true, "wpe-light", 1.5, EngineKind::NaraWpe, false),
        (false, "dtln-dark", 1.0, EngineKind::Dtln, false),
        (false, "lens-dark-100", 1.0, EngineKind::NaraWpe, true),
        (true, "lens-light-100", 1.0, EngineKind::NaraWpe, true),
        (false, "lens-dark-75", 0.75, EngineKind::NaraWpe, true),
        (true, "lens-light-75", 0.75, EngineKind::NaraWpe, true),
    ] {
        Arc::get_mut(&mut view.params).unwrap().engine = EnumParam::new("Engine", engine);
        view.shared.engine.store(engine as u32, Ordering::Relaxed);
        view.hover = inspecting.then_some((GRAPH.0 + GRAPH.2 * 0.64, GRAPH.1 + GRAPH.3 * 0.5));
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
            canvas,
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
