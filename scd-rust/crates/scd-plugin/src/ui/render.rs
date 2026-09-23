//! Composition of the SCD editor at its current window scale.
use super::*;

impl ScdEditorView {
    pub(super) fn draw_content(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        // Backdrop images must be recaptured after a host resize.
        if self.last_draw_size.replace((bounds.w, bounds.h)) != (bounds.w, bounds.h) {
            self.blur_dirty.set(true);
        }
        if self.font.get().is_none() {
            self.font.set(
                canvas
                    .add_font_mem(nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR)
                    .ok(),
            );
        }
        if self.icon_font.get().is_none() {
            self.icon_font.set(
                canvas
                    .add_font_mem(nih_plug_vizia::vizia_assets::fonts::TABLER_ICONS)
                    .ok(),
            );
        }
        ensure_img(&self.bg_img, canvas, BG_PNG);
        ensure_img(&self.logo_img, canvas, LOGO_PNG);
        ensure_img(&self.stone_img, canvas, STONE_PNG);

        let s = artwork_scale(bounds.w);
        canvas.save();
        canvas.reset_transform();
        let font = self.font.get();
        let mut draw = Draw::new(canvas, false, s, bounds.x, bounds.y, font);

        let new_peaks = self.params.vu.read_and_clear();
        let mut vu = self.vu_display.get();
        for (display, peak) in vu.iter_mut().zip(new_peaks) {
            *display = display.max(peak_to_meter(peak));
        }
        let new_kit = self.params.vu.read_and_clear_kit();
        let mut kit_vu = self.kit_vu.get();
        for (display, peak) in kit_vu.iter_mut().zip(new_kit) {
            *display = display.max(peak_to_meter(peak));
        }

        if let Some(id) = self.bg_img.get() {
            blit(&mut draw, id, 0.0, 0.0, WINDOW_W, WINDOW_H);
        } else {
            draw.rect(0.0, 0.0, WINDOW_W, WINDOW_H, HEADER);
        }

        draw.rect(0.0, 0.0, WINDOW_W, HEADER_H, HEADER);

        self.draw_preset_header(&mut draw);
        self.draw_mapping_header(&mut draw);

        let (vu_x, vu_y, vu_w, vu_h) = MASTER_VU;
        let master_l = vu[12].clamp(0.0, 1.0);
        let master_r = vu[13].clamp(0.0, 1.0);
        if master_l > 0.002 {
            draw.rect(vu_x, vu_y, vu_w * master_l, vu_h * 0.5 - 1.0, VU_FILL);
        }
        if master_r > 0.002 {
            draw.rect(
                vu_x,
                vu_y + vu_h * 0.5 + 1.0,
                vu_w * master_r,
                vu_h * 0.5 - 1.0,
                VU_FILL,
            );
        }

        self.draw_cc_header(&mut draw);

        self.draw_mixer(&mut draw, &kit_vu);

        self.draw_sub_kick(&mut draw, &kit_vu);

        self.draw_velocity_map(&mut draw, &kit_vu);

        self.draw_preset_menu(&mut draw);

        self.draw_mapping_menu(&mut draw);

        self.draw_cc_menu(&mut draw);

        self.draw_add_preset(&mut draw, &kit_vu);

        for v in vu.iter_mut() {
            *v *= VU_DECAY;
        }
        self.vu_display.set(vu);
        for v in kit_vu.iter_mut() {
            *v *= VU_DECAY;
        }
        self.kit_vu.set(kit_vu);
        draw.c.restore();
    }
}
