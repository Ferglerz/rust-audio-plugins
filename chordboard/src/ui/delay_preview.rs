//! Offscreen visual check for the actual editor at repeatable host bounds.
use super::*;
use crate::midi_fx::{DelayTap, MidiFxId};
use pleasant_ui::preview::OffscreenGl;

#[test]
#[ignore = "Manual macOS OpenGL visual check; writes chordboard-delay preview frames to the temporary directory"]
fn render_delay_preview() {
    let mut gl = OffscreenGl::new();
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
            for (enabled, state) in [(false, "bypassed"), (true, "spotlight")] {
                view.params
                    .arpeggiator
                    .edit(|arp| arp.midi_fx.delay.enabled = enabled);
                let path = std::env::temp_dir().join(format!(
                    "chordboard-delay-{}-{size}-{state}.ppm",
                    label.to_lowercase()
                ));
                gl.render_ppm(width, height, path, |canvas| {
                    view.paint_editor(
                        canvas,
                        BoundingBox::from_min_max(0.0, 0.0, width as f32, height as f32),
                        light,
                        label,
                    );
                });
            }
        }
    }
}
