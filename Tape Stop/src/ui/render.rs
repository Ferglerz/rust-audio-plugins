use super::{controls::*, *};

impl TapeStopView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }

        let Some(mut d) = Draw::new_fitted(
            canvas,
            prefs().light(),
            (bounds.x, bounds.y, bounds.w, bounds.h),
            (UI_W, UI_H),
            self.font.get(),
        ) else {
            return;
        };

        let braking = self.telemetry.is_braking.load(Ordering::Relaxed);

        d.rounded_rect(0.0, 0.0, UI_W, HEADER_HEIGHT, 0.0, PANEL);
        d.text(36.0, 44.0, "TAPE STOP", 24.0, GOLD);
        d.text(200.0, 44.0, "MIDI TAPE BRAKE", 13.0, TEXT);

        d.appearance_button(THEME_BUTTON, prefs().label());

        self.draw_graph(&mut d);

        // Circular STOP / REW overlay in the bottom-left of the graph
        let (sx, sy, sw, sh) = STOP_BUTTON;
        let scx = sx + sw * 0.5;
        let scy = sy + sh * 0.5;
        let stop_active = braking || self.telemetry.get_manual_trigger();
        let stop_color = if stop_active { GOLD } else { MUTED };

        let click_amt = self.stop_click.get();
        if click_amt > 0.01 {
            self.stop_click.set((click_amt - 0.08).max(0.0));
        }
        let is_hovered = self.hover_stop.get();
        let now_sec = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 100_000) as f32
            * 0.001;
        let pulse = if is_hovered {
            (now_sec * 5.0).sin() * 0.5 + 0.5
        } else {
            0.0
        };

        if is_hovered || click_amt > 0.01 {
            let halo_r = sw * 0.5 + 2.0 + pulse * 3.0 + click_amt * 6.0;
            let mut halo_col = stop_color;
            halo_col.a = (0.25 + 0.25 * pulse + click_amt * 0.4).clamp(0.0, 1.0);
            d.circle(scx, scy, halo_r, halo_col, false);
        }
        if click_amt > 0.05 {
            let rip_r = sw * 0.5 + (1.0 - click_amt) * 14.0;
            let mut rip_col = stop_color;
            rip_col.a = (click_amt * 0.8).clamp(0.0, 1.0);
            d.circle(scx, scy, rip_r, rip_col, false);
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

        self.draw_axis_cog(&mut d);
        if self.show_axis_controls {
            self.draw_midi_slot(&mut d);
            self.draw_audio_trigger(&mut d);
            for &id in &[KnobId::Return, KnobId::Xfade, KnobId::StereoDiv] {
                self.draw_axis_slider(&mut d, id);
            }
        }
    }
}
