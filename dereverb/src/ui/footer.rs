use super::*;

impl DereverbView {
    pub(super) fn paint_footer(&self, d: &mut Draw, engine: EngineKind, neural: bool) {
        let status = self.shared.status.load(Ordering::Relaxed);
        let rate = self.shared.sample_rate.load(Ordering::Relaxed);
        let latency = self.shared.latency.load(Ordering::Relaxed);
        let active_quality = self.shared.quality.load(Ordering::Relaxed);
        let active_engine = self.shared.engine.load(Ordering::Relaxed);
        let active_slot = EngineKind::MODES.iter().position(|&(kind, quality)| {
            kind as u32 == active_engine
                && (!kind.has_quality() || quality.index() as u32 == active_quality)
        });
        let switching = active_slot != Some(engine.index(self.params.quality.value()));
        let mode = match active_slot.map(|index| EngineKind::MODES[index]) {
            Some((EngineKind::Dtln, _)) => "DTLN DENOISE",
            Some((EngineKind::NaraWpe, Quality::Live)) => "NARA-WPE · SHORT",
            Some((EngineKind::NaraWpe, Quality::HighQuality)) => "NARA-WPE · 20 MS",
            Some((EngineKind::DpdfNet, Quality::HighQuality)) => "DPDFNET HIGH QUALITY",
            _ => "DPDFNET LIVE",
        };
        let state = if switching {
            format!("{mode} · SWITCHING MODE")
        } else {
            match status {
                telemetry::ACTIVE => format!(
                    "{mode}   {:.1} kHz   {:.1} ms",
                    rate / 1000.0,
                    1000.0 * latency as f32 / rate.max(1.0)
                ),
                telemetry::RECOVERING => "RECOVERING · ALIGNED PASSTHROUGH".into(),
                telemetry::FAILED => "BYPASSED · ENGINE UNAVAILABLE".into(),
                _ => "WAITING FOR AUDIO".into(),
            }
        };
        d.line(36.0, 559.0, 764.0, 559.0, LINE, 1.0);
        d.text(
            36.0,
            581.0,
            &state,
            11.0,
            if status == telemetry::ACTIVE {
                TEAL
            } else {
                GOLD
            },
        );
        let tooltip = if let Some(index) = active_slot.filter(|_| status == telemetry::FAILED) {
            self.shared.errors.lock().unwrap_or_else(|e| e.into_inner())[index]
                .chars()
                .take(105)
                .collect::<String>()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, THEME)) {
            "Cycle Dark, Light, and Auto appearance.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, DPDFNET)) {
            "DPDFNet: neural voice restoration with Live and High Quality modes.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, NARA_WPE)) {
            "NARA-WPE: adaptive late reverb reduction. The filter learns from the incoming audio."
                .into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, DTLN)) {
            "DTLN: speech denoising. At full strength, wet audio is limited to about 8 kHz.".into()
        } else if engine == EngineKind::NaraWpe
            && self.hover.is_some_and(|p| point_in_rect(p.0, p.1, LIVE))
        {
            "10 ms analysis window. A half-second average of the learned filter steadies removal."
                .into()
        } else if engine == EngineKind::NaraWpe
            && self.hover.is_some_and(|p| point_in_rect(p.0, p.1, HIGH_QUALITY))
        {
            "20 ms analysis window with finer frequency resolution and faster filter response."
                .into()
        } else if neural && self.hover.is_some_and(|p| point_in_rect(p.0, p.1, LIVE)) {
            "Live uses lighter neural spectral correction.".into()
        } else if neural && self.hover.is_some_and(|p| point_in_rect(p.0, p.1, HIGH_QUALITY)) {
            "High Quality uses the full neural model with deep filtering.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, RELATIVE)) {
            "Relative: level against a 1-second RMS reference; weighting follows whole-recording gain changes.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, ABSOLUTE)) {
            "Absolute: original input RMS in dBFS; gain before Dereverb intentionally changes the weighting.".into()
        } else if self
            .hover
            .is_some_and(|p| point_in_rect(p.0, p.1, TRANSIENT_GUARD))
        {
            "Drag or scroll to set protection for attacks and s, sh, and h. Shift for fine adjustment.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, FLAT)) {
            "Flat: apply the same Strength at every input level.".into()
        } else if self
            .hover
            .is_some_and(|p| point_in_rect(p.0, p.1, LOUDER_BUTTON))
        {
            "Louder: reduce processing on quieter sounds. Drag the curve points to fine-tune."
                .into()
        } else if self.curve_drag.is_some()
            || self
                .hover
                .is_some_and(|p| self.node_at(p).is_some() || point_in_rect(p.0, p.1, CURVE))
        {
            "Drag points vertically; Shift for fine control. Double-click resets. The teal dot follows the loudest channel.".into()
        } else if self.edit.is_some() {
            "Type a percentage. Enter applies; Escape cancels.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, KNOB)) {
            "Drag or scroll Strength; Shift for fine control. Click its value to type; double-click to reset.".into()
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, GRAPH)) {
            "Input and output RMS are time-aligned. Readouts average the latest 200 ms across all channels.".into()
        } else {
            "Input level is normalized internally, then restored. The curve weights the selected Strength.".into()
        };
        d.text(36.0, 605.0, &tooltip, 10.0, MUTED);
    }
}
