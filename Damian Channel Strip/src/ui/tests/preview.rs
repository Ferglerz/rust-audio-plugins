//! Offscreen snapshots of the channel-strip editor, independent of the DAW.
use super::super::*;
use super::layout::test_view;
use pleasant_ui::preview::OffscreenGl;

#[test]
#[ignore = "Manual macOS OpenGL check; saves Damian editor frames to the temporary directory"]
fn render_editor_preview() {
    let mut gl = OffscreenGl::new();
    let view = test_view();

    for (light, theme) in [(false, "dark"), (true, "light")] {
        for (scale, size) in [(1.0_f32, 100), (0.75, 75)] {
            let width = (UI_W * scale).round() as u32;
            let height = (UI_H * scale).round() as u32;
            let path = std::env::temp_dir().join(format!("damian-editor-{theme}-{size}.ppm"));
            gl.render_ppm(width, height, path, |canvas| {
                view.paint_editor(
                    canvas,
                    BoundingBox::from_min_max(0.0, 0.0, width as f32, height as f32),
                    light,
                );
            });
        }
    }
}
