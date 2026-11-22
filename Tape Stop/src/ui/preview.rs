//! Offscreen snapshots of the actual editor, independent of the DAW.
use super::*;
use crate::telemetry::{TransportFrame, TransportStage};
use pleasant_ui::preview::OffscreenGl;
use std::time::{Duration, Instant};

#[test]
#[ignore = "Manual offscreen UI check; writes tape-stop preview images to the temporary directory"]
fn render_reels_preview() {
    let mut gl = OffscreenGl::new();
    let font = gl.canvas_mut().add_font_mem(FONT_JETBRAINS_MONO).unwrap();
    for (light, theme, scale) in [(false, "dark", 1.0), (true, "light", 0.75)] {
        let width = (UI_W * scale).round() as u32;
        let height = (UI_H * scale).round() as u32;
        let view = interaction_tests::view();
        let start = Instant::now();
        {
            let mut d = Draw::new_fitted(
                gl.canvas_mut(),
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
            let path = std::env::temp_dir().join(format!("tape-stop-{name}-{theme}.ppm"));
            gl.render_ppm(width, height, path, |canvas| {
                let mut d = Draw::new_fitted(
                    canvas,
                    light,
                    (0.0, 0.0, width as f32, height as f32),
                    (UI_W, UI_H),
                    Some(font),
                )
                .unwrap();
                view.paint(&mut d, start + Duration::from_millis((i as u64 + 1) * 33));
            });
        }
    }
}
