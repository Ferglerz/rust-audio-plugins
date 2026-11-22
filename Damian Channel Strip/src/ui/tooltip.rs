use super::*;

impl StripView {
    pub(super) fn tooltip(&self) -> &'static str {
        if self.edit.is_some() {
            return "Type a value in the control's units · Enter applies · Escape cancels";
        }
        if self.menu.is_some() || self.processing_menu.is_some() || self.scale_menu {
            return "Choose an option · Click outside the menu to close";
        }
        let Some((x, y)) = self.hover else {
            return "Hover a control for help · Drag to adjust · Click a value to type · Shift for fine adjustment";
        };
        for (rect, text) in [
            (THEME_BUTTON, "Cycle Dark, Light, and Auto appearance."),
            (
                PROCESS_BUTTON,
                "Choose the EQ processing mode; linear phase adds latency.",
            ),
            (
                resolution_button_rect(),
                "Choose linear-phase resolution; higher resolution adds latency.",
            ),
            (
                self.footer_comp_routing_rect(),
                "Place compression before or after the EQ.",
            ),
            (
                self.footer_auto_rect(),
                "Automatically compensate for gain lost through compression.",
            ),
            (
                self.footer_link_rect(),
                "Link left and right compression for a stable stereo image.",
            ),
            (
                output_gain_rect(),
                "Output gain: drag to adjust; click the value to type; double-click to reset.",
            ),
            (
                self.dyn_power_button_rect(),
                "Bypass or enable the compressor module.",
            ),
            (
                self.pse_power_button_rect(),
                "Bypass or enable the primary source expander.",
            ),
            (
                self.wall_power_button_rect(),
                "Bypass or enable the peak limiter.",
            ),
        ] {
            if inside(x, y, rect) {
                return text;
            }
        }
        if self.value_at(x, y).is_some() {
            return "Click the value to type · Drag to adjust · Shift for fine adjustment · Double-click to reset";
        }
        if inside(x, y, self.eq_bounds()) {
            return "Drag EQ nodes to adjust frequency and gain; scroll for Q. Select a band for its controls.";
        }
        if inside(x, y, self.dyn_bounds()) {
            return "Compression: drag the threshold, depth, or knee handles. Open settings for attack, release, and ratio.";
        }
        if inside(x, y, self.pse_bounds()) {
            return "Primary source expander: set the threshold and depth; open settings for timing and voice detection.";
        }
        if inside(x, y, self.wall_bounds()) {
            return "Peak limiter: adjust the threshold; open settings for harmonic character and lookahead.";
        }
        "Hover a control for help · Drag to adjust · Click a value to type · Shift for fine adjustment"
    }
}
