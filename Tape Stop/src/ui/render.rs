use super::{controls::*, *};
use std::time::Instant;

impl TapeStopView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        load_font_once(canvas, &self.font, FONT_JETBRAINS_MONO);

        let Some(mut d) = Draw::new_fitted(
            canvas,
            prefs().light(),
            (bounds.x, bounds.y, bounds.w, bounds.h),
            (UI_W, UI_H),
            self.font.get(),
        ) else {
            return;
        };

        self.paint(&mut d, Instant::now());
    }

    pub(super) fn paint(&self, d: &mut Draw, now: Instant) {
        let mut reels = self.reels.get();
        let enabled = self.params.power.value() && !self.params.bypass.value();
        let transport = reels.advance(now, self.telemetry.transport(), enabled);
        self.reels.set(reels);

        d.rounded_rect(0.0, 0.0, UI_W, HEADER_HEIGHT, 0.0, PANEL);
        d.text(36.0, 44.0, "TAPE STOP", 24.0, GOLD);
        reels.draw(d, transport, enabled, now);
        d.appearance_button(THEME_BUTTON, prefs().label());

        self.draw_graph(d, transport);
        self.draw_performance(d);

        // Circular STOP / REW overlay in the bottom-left of the graph
        let (sx, sy, sw, sh) = STOP_BUTTON;
        let scx = sx + sw * 0.5;
        let scy = sy + sh * 0.5;
        let stop_active = self.telemetry.is_braking() || self.telemetry.get_manual_trigger();
        let stop_color = if stop_active { GOLD } else { MUTED };

        if self.hover_stop.get() {
            d.circle(
                scx,
                scy,
                sw * 0.5 + 3.0,
                pleasant_ui::with_alpha(stop_color, 0.6),
                false,
            );
        }

        let mut stop_fill = rgb(0, 0, 0);
        stop_fill.a = 0.5;
        d.circle(scx, scy, sw * 0.5, stop_fill, true);
        d.circle(scx, scy, sw * 0.5, stop_color, false);
        d.text_centered(
            scx,
            scy + 6.0,
            if stop_active { "REW" } else { "STOP" },
            16.0,
            if stop_active { GOLD } else { TEXT },
        );

        d.rect(20.0, SETTINGS_Y - 14.0, UI_W - 40.0, 1.0, LINE);
        let affect_midi = self.params.affect_midi.value();
        d.button_surface(AFFECT_MIDI, TEAL, affect_midi as u8 as f32, 0.0, 0.0);
        d.text_centered(
            AFFECT_MIDI.0 + AFFECT_MIDI.2 * 0.5,
            AFFECT_MIDI.1 + 20.0,
            if affect_midi {
                "Affect MIDI: On"
            } else {
                "Affect MIDI: Off"
            },
            12.0,
            if affect_midi { TEAL } else { TEXT },
        );
        self.draw_midi_slot(d);
        self.draw_audio_trigger(d);
        for &id in &[KnobId::Xfade, KnobId::StereoDiv] {
            self.draw_axis_slider(d, id);
        }
        d.bottom_tooltip(
            (
                20.0,
                CONTENT_H,
                UI_W - 40.0,
                pleasant_ui::draw::TOOLTIP_HEIGHT,
            ),
            self.tooltip(),
            MUTED,
        );
    }
}
