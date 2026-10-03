//! Control positions for the supported Dark and Light interfaces.
use super::{appearance, theme};

#[derive(Debug, Clone, Copy)]
pub struct ControlLayout {
    pub input_dependence_knob: (f32, f32, f32),
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
    pub offset_knob: (f32, f32),
}

impl ControlLayout {
    pub fn pleasant() -> Self {
        let env_x = appearance::ENV_X;
        let env_controls_y = appearance::ENVELOPE_CONTROLS_Y;
        let prog_knob_y = appearance::PROG_KNOB_Y;
        let harm_cx = appearance::HARM_X + appearance::HARM_W * 0.5;
        let makeup_knob = (
            harm_cx - appearance::GAIN_KNOB_SIZE * 0.5,
            appearance::OUTPUT_Y + appearance::MODULE_HEADER_H + 8.0,
        );
        let quad_left = appearance::HARM_X + 10.0;
        let quad_right =
            appearance::HARM_X + appearance::HARM_W - 10.0 - appearance::HARMONIC_KNOB_W;
        Self {
            input_dependence_knob: (appearance::ENV_X + 14.0, prog_knob_y, theme::KNOB_SIZE),
            detection_btn: (
                appearance::ENV_X + appearance::ENV_W - 10.0 - appearance::DETECTION_BUTTON_W,
                env_controls_y + theme::KNOB_SIZE + appearance::ADAPTIVE_GAP,
            ),
            harmonic_type_switch: (harm_cx - appearance::HARMONIC_BUTTON_W * 0.5, 55.0),
            harmonic_drive: (quad_left, 110.0),
            harmonic_mix: (quad_right, 110.0),
            harmonic_even: (quad_left, 240.0),
            harmonic_odd: (quad_right, 240.0),
            makeup_knob,
            strength_knob: (
                appearance::ENV_X + (appearance::ENV_W - appearance::STRENGTH_KNOB_SIZE) * 0.5,
                env_controls_y,
            ),
            ms_switch: (
                appearance::HARM_X + appearance::HARM_W - 10.0 - appearance::MS_BUTTON_W,
                appearance::OUTPUT_Y + (appearance::MODULE_HEADER_H - 28.0) * 0.5,
            ),
            norm_switch: (
                env_x + (theme::KNOB_SIZE - theme::SWITCH_SLOT_W) * 0.5,
                env_controls_y + theme::KNOB_SIZE + appearance::ADAPTIVE_GAP,
            ),
            rms_knob: (env_x, env_controls_y),
            offset_knob: (
                appearance::ENV_X + appearance::ENV_W - theme::KNOB_SIZE,
                env_controls_y,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_program_and_bottom_controls_do_not_overlap() {
        let l = ControlLayout::pleasant();
        let program_y = l.input_dependence_knob.1;
        let rects = [
            (
                l.offset_knob.0,
                l.offset_knob.1,
                theme::KNOB_SIZE,
                theme::KNOB_SIZE,
            ),
            (
                l.input_dependence_knob.0,
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
            (
                appearance::ENV_X + 258.0,
                program_y,
                theme::KNOB_SIZE,
                theme::KNOB_SIZE,
            ),
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
                appearance::STRENGTH_KNOB_H,
            ),
        ];
        for (i, &(x, y, w, h)) in rects.iter().enumerate() {
            assert!(x >= appearance::ENV_X && x + w <= appearance::ENV_X + appearance::ENV_W);
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
            l.strength_knob.0 + appearance::STRENGTH_KNOB_SIZE * 0.5,
            appearance::ENV_X + appearance::ENV_W * 0.5
        );
        assert!(
            appearance::PLEASANT_GRAPH_Y + appearance::PLEASANT_GRAPH_SIZE
                <= appearance::ENV_Y + appearance::SIDE_H - 12.0
        );
        assert_eq!(
            appearance::PLEASANT_GRAPH_Y + appearance::PLEASANT_METER_HEIGHT,
            appearance::ENV_Y + appearance::SIDE_H - 12.0
        );
        assert_eq!(l.offset_knob.1, l.rms_knob.1);
        assert_eq!(
            l.offset_knob.0 + theme::KNOB_SIZE * 0.5
                - (l.strength_knob.0 + appearance::STRENGTH_KNOB_SIZE * 0.5),
            l.strength_knob.0 + appearance::STRENGTH_KNOB_SIZE * 0.5
                - (l.rms_knob.0 + theme::KNOB_SIZE * 0.5)
        );
        let strength = pleasant_ui::draw::KnobLayout::new((
            l.strength_knob.0,
            l.strength_knob.1,
            appearance::STRENGTH_KNOB_SIZE,
            appearance::STRENGTH_KNOB_H,
        ))
        .with_text_sizes(13.0, 15.0);
        let readout_center = strength.value_y - strength.value_size * 0.35;
        for button_y in [l.norm_switch.1, l.detection_btn.1] {
            assert!((readout_center - button_y - 14.0).abs() < 0.1);
        }
        let standard =
            pleasant_ui::draw::KnobLayout::new((0.0, 0.0, theme::KNOB_SIZE, theme::KNOB_SIZE))
                .with_text_sizes(13.0, 15.0);
        assert!(strength.radius > standard.radius);
        assert!(
            l.strength_knob.1 + appearance::STRENGTH_KNOB_H
                <= appearance::ENV_Y + appearance::ENVELOPE_H
        );
    }
}
