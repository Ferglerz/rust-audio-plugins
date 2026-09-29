//! Control positions for the supported Dark and Light interfaces.
use super::{appearance, theme};

#[derive(Debug, Clone, Copy)]
pub struct ControlLayout {
    pub prog_blend_slider: (f32, f32, f32),
    pub inverse_btn: (f32, f32, f32),
    pub detection_btn: (f32, f32),
    pub harmonic_type_switch: (f32, f32),
    pub harmonic_drive: (f32, f32),
    pub harmonic_mix: (f32, f32),
    pub harmonic_even: (f32, f32),
    pub harmonic_odd: (f32, f32),
    pub makeup_knob: (f32, f32),
    pub strength_knob: (f32, f32),
    pub ms_switch: (f32, f32),
    pub norm_switch: (f32, f32),
    pub rms_knob: (f32, f32),
}

impl ControlLayout {
    pub fn pleasant() -> Self {
        let env_x = appearance::ENV_X + 6.0;
        let bottom_y =
            appearance::ENV_Y + appearance::SIDE_H - 10.0 - appearance::STRENGTH_KNOB_SIZE;
        let harm_cx = appearance::HARM_X + appearance::HARM_W * 0.5;
        let makeup_knob = (
            harm_cx - theme::KNOB_SIZE * 0.5,
            appearance::OUTPUT_Y + appearance::MODULE_HEADER_H + 8.0,
        );
        let quad_left = appearance::HARM_X + 10.0;
        let quad_right =
            appearance::HARM_X + appearance::HARM_W - 10.0 - appearance::HARMONIC_KNOB_W;
        Self {
            prog_blend_slider: (
                appearance::ENV_X + 14.0,
                appearance::ENVELOPE_KNOB_Y,
                theme::KNOB_SIZE,
            ),
            inverse_btn: (appearance::ENV_X + 136.0, bottom_y, 72.0),
            detection_btn: (
                env_x + theme::KNOB_SIZE,
                bottom_y + theme::KNOB_SIZE + appearance::ADAPTIVE_GAP,
            ),
            harmonic_type_switch: (harm_cx - appearance::HARMONIC_BUTTON_W * 0.5, 55.0),
            harmonic_drive: (quad_left, 110.0),
            harmonic_mix: (quad_right, 110.0),
            harmonic_even: (quad_left, 240.0),
            harmonic_odd: (quad_right, 240.0),
            makeup_knob,
            strength_knob: (
                appearance::ENV_X + appearance::ENV_W - 10.0 - appearance::STRENGTH_KNOB_SIZE,
                bottom_y,
            ),
            ms_switch: (
                harm_cx - theme::SWITCH_SLOT_W * 0.5,
                makeup_knob.1 + theme::KNOB_SIZE + 12.0,
            ),
            norm_switch: (
                env_x + (theme::KNOB_SIZE - theme::SWITCH_SLOT_W) * 0.5,
                bottom_y + theme::KNOB_SIZE + appearance::ADAPTIVE_GAP,
            ),
            rms_knob: (env_x, bottom_y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_program_and_bottom_controls_do_not_overlap() {
        let l = ControlLayout::pleasant();
        let program_y = l.prog_blend_slider.1;
        let source_x = appearance::ENV_X + appearance::ENV_W - 130.0;
        let rects = [
            (
                l.prog_blend_slider.0,
                program_y,
                theme::KNOB_SIZE,
                theme::KNOB_SIZE,
            ),
            (
                appearance::ENV_X + 136.0,
                program_y,
                theme::KNOB_SIZE,
                theme::KNOB_SIZE,
            ),
            (source_x, program_y, 120.0, 28.0),
            (source_x, program_y + 36.0, 120.0, 28.0),
            (
                l.rms_knob.0,
                l.rms_knob.1,
                theme::KNOB_SIZE,
                theme::KNOB_SIZE,
            ),
            (
                l.norm_switch.0,
                l.norm_switch.1,
                theme::SWITCH_SLOT_W,
                appearance::ADAPTIVE_BUTTON_H,
            ),
            (l.inverse_btn.0, l.inverse_btn.1, l.inverse_btn.2, 28.0),
            (
                l.detection_btn.0,
                l.detection_btn.1,
                appearance::DETECTION_BUTTON_W,
                28.0,
            ),
            (
                l.strength_knob.0,
                l.strength_knob.1,
                appearance::STRENGTH_KNOB_SIZE,
                appearance::STRENGTH_KNOB_SIZE,
            ),
        ];
        for (i, &(x, y, w, h)) in rects.iter().enumerate() {
            assert!(
                x >= appearance::ENV_X && x + w <= appearance::ENV_X + appearance::ENV_W - 10.0
            );
            assert!(y >= appearance::ENVELOPE_GRAPH_Y + appearance::ENVELOPE_GRAPH_H);
            assert!(y + h <= appearance::ENV_Y + appearance::SIDE_H - 10.0);
            for &(ox, oy, ow, oh) in &rects[i + 1..] {
                assert!(
                    x + w <= ox || ox + ow <= x || y + h <= oy || oy + oh <= y,
                    "controls {i} and ({ox}, {oy}) overlap"
                );
            }
        }
        assert_eq!(l.rms_knob.1, l.strength_knob.1);
        assert_eq!(
            l.norm_switch.1 + appearance::ADAPTIVE_BUTTON_H,
            l.strength_knob.1 + appearance::STRENGTH_KNOB_SIZE
        );
    }
}
