//! Sub-kick dialog painting.
use super::*;

impl ScdEditorView {
    pub(super) fn draw_sub_kick(&self, draw: &mut Draw<'_>, kit_vu: &[f32; KitPieceId::COUNT]) {
        if !self.sub_kick_open {
            return;
        }
        let (modal_x, modal_y, modal_w, modal_h) = Self::sub_kick_modal();
        paint_glass_modal(
            draw,
            &self.blur_shot,
            &self.blur_src,
            &self.blur_dst,
            &self.blur_dirty,
            modal_x,
            modal_y,
            modal_w,
            modal_h,
        );
        draw_all_chan_meters(draw, kit_vu);

        let knobs = [
            (
                Self::sub_kick_knob_x(modal_x, 0),
                "VOL",
                self.params.sub_kick.vol.unmodulated_normalized_value(),
                false,
            ),
            (
                Self::sub_kick_knob_x(modal_x, 1),
                "LENGTH",
                self.params.sub_kick.length.unmodulated_normalized_value(),
                false,
            ),
            (
                Self::sub_kick_knob_x(modal_x, 2),
                "DIVE",
                self.params.sub_kick.dive.unmodulated_normalized_value(),
                true,
            ),
            (
                Self::sub_kick_knob_x(modal_x, 3),
                "SPEED",
                self.params.sub_kick.speed.unmodulated_normalized_value(),
                false,
            ),
            (
                Self::sub_kick_knob_x(modal_x, 4),
                "OFFSET",
                self.params.sub_kick.offset.unmodulated_normalized_value(),
                false,
            ),
        ];
        for (kx, label, norm, bipolar) in knobs {
            hise_knob(draw, kx, modal_y + 55.0, 60.0, norm, bipolar, Some(label));
        }
        draw_kick_env(
            draw,
            (modal_x, modal_y + 90.0, modal_w, 60.0),
            self.params.sub_kick.vol.unmodulated_plain_value(),
            self.params.sub_kick.length.unmodulated_plain_value(),
            self.params.sub_kick.offset.unmodulated_plain_value(),
        );
    }
}

fn draw_kick_env(
    draw: &mut Draw<'_>,
    rect: (f32, f32, f32, f32),
    vol_gain: f32,
    length: f32,
    offset_ms: f32,
) {
    let (x, y, w, h) = rect;
    let attack_ms = 45.0;
    let hold_ms = (length * 6.0).max(0.0);
    let decay_ms = length_to_decay_ms(length).max(1.0);
    let release_ms = decay_ms;
    let peak_db = util::gain_to_db(vol_gain).clamp(-100.0, 0.0);
    let sustain_db = -100.0;
    let total_ms = attack_ms + hold_ms + decay_ms + release_ms;
    let gx = x + 15.0 + offset_ms;
    let gw = (w - 20.0 - offset_ms).max(8.0);
    let gy = y;
    let gh = h;

    let env_y = |db: f32| gy + gh * (1.0 - ((db + 100.0) / 100.0).clamp(0.0, 1.0));
    let env_x = |ms: f32| gx + gw * (ms / total_ms);

    let mut pts = Vec::with_capacity(20);
    pts.push((env_x(0.0), env_y(sustain_db)));
    const ATTACK_STEPS: usize = 8;
    for i in 1..=ATTACK_STEPS {
        let t = i as f32 / ATTACK_STEPS as f32;
        let shaped = t.powf(0.65);
        let db = sustain_db + (peak_db - sustain_db) * shaped;
        pts.push((env_x(t * attack_ms), env_y(db)));
    }
    let hold_end = attack_ms + hold_ms;
    pts.push((env_x(hold_end), env_y(peak_db)));
    pts.push((env_x(hold_end + decay_ms), env_y(sustain_db)));
    pts.push((env_x(total_ms), env_y(sustain_db)));

    draw.area(&pts, gy + gh, ENV_FILL);
    draw.poly(&pts, ENV_LINE, 1.4);
}
