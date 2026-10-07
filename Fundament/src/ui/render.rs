use super::{
    controls::{inside, BYPASS_BUTTON, HEADER_H, IN_METER, OUT_METER, THEME_BUTTON},
    prefs, FundamentView, VIEW_H, VIEW_W,
};
use nih_plug_vizia::vizia::prelude::*;
use pleasant_dsp::units::linear_to_db_with_floor;
use pleasant_ui::theme::{GOLD, MUTED, PANEL, TEAL};
use pleasant_ui::{Draw, FONT_JETBRAINS_MONO};
use std::sync::atomic::Ordering;

fn peak_fill(peak: f32) -> f32 {
    if !peak.is_finite() || peak <= 1.0e-6 {
        return 0.0;
    }
    let db = linear_to_db_with_floor(f64::from(peak), 1.0e-6);
    ((db + 48.0) / 48.0).clamp(0.0, 1.0) as f32
}

impl FundamentView {
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
            (VIEW_W, VIEW_H),
            self.font.get(),
        ) else {
            return;
        };
        d.hover = self.idle_hover();

        d.rounded_rect(0.0, 0.0, VIEW_W, HEADER_H, 0.0, PANEL);
        d.text(20.0, 30.0, "FUNDAMENT", 22.0, GOLD);

        let bypassed = self.params.bypass.value();
        let bypass_hover = self
            .idle_hover()
            .is_some_and(|(x, y)| inside(x, y, BYPASS_BUTTON));
        d.bypass_button(
            BYPASS_BUTTON,
            bypassed,
            TEAL,
            bypass_hover,
            self.bypass_anim.step(),
        );
        d.appearance_button(THEME_BUTTON, prefs().label());
        self.draw_meter(
            &mut d,
            IN_METER,
            "IN",
            self.telemetry.input_peak.load(Ordering::Relaxed),
        );
        self.draw_meter(
            &mut d,
            OUT_METER,
            "OUT",
            self.telemetry.output_peak.load(Ordering::Relaxed),
        );

        self.draw_spectrum(&mut d);
        self.draw_controls(&mut d);
    }

    fn draw_meter(&self, d: &mut Draw, rect: (f32, f32, f32, f32), label: &str, peak: f32) {
        d.text_right(rect.0 - 6.0, rect.1 + 6.0, label, 9.0, MUTED);
        d.rect(rect.0, rect.1, rect.2, rect.3, pleasant_ui::theme::LINE);
        let fill = peak_fill(peak);
        if fill > 0.004 {
            let color = if peak >= 1.0 { GOLD } else { TEAL };
            d.rect(rect.0, rect.1, rect.2 * fill, rect.3, color);
        }
    }
}
