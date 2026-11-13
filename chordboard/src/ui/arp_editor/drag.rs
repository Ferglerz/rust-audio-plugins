use super::*;

impl ChordboardView {
    pub(super) fn arp_playback(&self) -> crate::engine::ArpPlayback {
        if !self.programming {
            return self.snapshot.performance_arp;
        }
        crate::engine::ArpPlayback {
            notes: self.snapshot.arp_notes,
            len: self.snapshot.arp_len,
            phase: self.snapshot.arp_phase,
            step: self.snapshot.arp_step,
            running: self.snapshot.arp_running,
            aligned: self.snapshot.arp_aligned,
        }
    }
    pub(in crate::ui) fn editor_arp_enabled(&self) -> bool {
        if self.programming {
            self.arp_state()
                .editor_row(self.arp_ui.edit_page, false)
                .enabled
        } else {
            self.performance_arp_enabled()
        }
    }
    pub(super) fn editor_arp_inheritance(&self) -> Option<bool> {
        if !self.programming {
            return None;
        }
        self.params
            .arpeggiator
            .tracker_arp_inheritance(self.arp_ui.edit_page)
    }
    pub(in crate::ui) fn editor_arp_inherits(&self) -> bool {
        self.editor_arp_inheritance() == Some(true)
    }
    pub(in crate::ui) fn performance_arp_enabled(&self) -> bool {
        crate::midi_learn::value(&self.params, crate::midi_learn::PERFORMANCE_ARP) != 0
    }
    pub(in crate::ui) fn refresh_arp_editor_params(&mut self) {
        let state = self.arp_state();
        // A host preset restores the rows without recreating the open editor.
        // Reconcile both ends of a page animation and any in-flight row edits
        // before the next input event or draw can index the restored rows.
        let count = state.rows.len();
        if self.arp_ui.edit_page >= count
            || self.arp_ui.page_from >= count
            || self.arp_ui.drag.is_some_and(|drag| drag.page >= count)
            || self.tracker_drag.is_some_and(|drag| !drag.valid(&state))
        {
            self.arp_ui.edit_page = self.arp_ui.edit_page.min(count.saturating_sub(1));
            self.arp_ui.page_from = self.arp_ui.edit_page;
            self.arp_ui.page_elapsed = pleasant_ui::page_slide::DURATION;
            self.arp_ui.drag = None;
            self.arp_ui.dropdown = None;
            self.tracker_drag = None;
            self.tracker_selection = false;
            self.tracker_symbol_edit = false;
            self.tracker_symbol.clear();
            self.tracker_proxy_row = usize::MAX;
        }
        let row = state.editor_row(self.arp_ui.edit_page, !self.programming);
        if self.row_slide.select(row.id, self.arp_ui.edit_page) {
            // Slide the arp module once; its pattern does not need a nested slide.
            self.arp_ui.page_from = self.arp_ui.edit_page;
            self.arp_ui.page_elapsed = pleasant_ui::page_slide::DURATION;
            self.cancel_arp_editor();
            self.edit = None;
            self.menu = None;
        }
        if self.panel == Some(Panel::Custom) && self.editor_arp_inherits() {
            self.set_panel(None);
        }
        if self.tracker_proxy_row
            != if self.programming {
                self.arp_ui.edit_page
            } else {
                usize::MAX - 1
            }
            || self.tracker_proxy_settings != row.settings
        {
            *self.arp_editor_params = crate::params::ArpPageParams::from_settings(row.settings);
            self.tracker_proxy_row = if self.programming {
                self.arp_ui.edit_page
            } else {
                usize::MAX - 1
            };
            self.tracker_proxy_settings = row.settings;
        }
    }
    pub(in crate::ui) fn arp_row_param_event(&mut self, event: &RawParamEvent) -> bool {
        let ptr = match event {
            RawParamEvent::BeginSetParameter(ptr)
            | RawParamEvent::SetParameterNormalized(ptr, _)
            | RawParamEvent::EndSetParameter(ptr) => *ptr,
            RawParamEvent::ParametersChanged => return false,
        };
        let p = self.arp_params();
        let Some(control) = p.controls().into_iter().find(|c| c.ptr == ptr) else {
            return false;
        };
        if self.editor_arp_inherits() {
            return true;
        }
        if let RawParamEvent::SetParameterNormalized(_, value) = event {
            let value = value.clamp(0.0, 1.0);
            let mut settings = self
                .arp_state()
                .editor_row(self.arp_ui.edit_page, !self.programming)
                .settings;
            match control.id {
                "arp_mode" => {
                    settings.mode = crate::arpeggiator::ArpMode::from_index(
                        p.arp_mode.preview_plain(value) as u8,
                    )
                }
                "arp_pattern" => settings.pattern = p.arp_pattern.preview_plain(value) as u8,
                "arp_octaves" => settings.octaves = p.arp_octaves.preview_plain(value) as u8,
                "arp_sync" => settings.sync = p.arp_sync.preview_plain(value),
                "arp_free_ms" => settings.free_ms = p.arp_free_ms.preview_plain(value),
                "rate" => settings.rate = p.rate.preview_plain(value),
                "swing" => settings.swing = p.swing.preview_plain(value),
                "humanize" => settings.humanize = p.humanize.preview_plain(value),
                _ => return false,
            }
            self.params.arpeggiator.edit(|s| {
                s.editor_row_mut(self.arp_ui.edit_page, !self.programming)
                    .settings = settings;
            });
            self.refresh_arp_editor_params();
        }
        true
    }
    pub(in crate::ui) fn arp_chooser_open(&self) -> bool {
        self.arp_ui.chooser.is_some()
    }
    pub(super) fn arp_pass_len_for_page(&self, page: usize) -> usize {
        let state = self.arp_state();
        let row = state.editor_row(page, !self.programming);
        // Let users configure a row before assigning its chord. The editor previews
        // the current chordboard voicing; the audio engine still treats the row as
        // silent until the row itself has a chord.
        let recipe = row
            .chord
            .or_else(|| SavedChord::decode(self.snapshot.captured));
        let count = recipe.map_or(self.snapshot.notes.len, |chord| {
            harmony::resolve_chord(chord, self.params.global_harmony()).len
        });
        crate::engine::arpeggiator::settings_pass_length(count, row.settings).max(1)
    }
    pub(super) fn arp_string_layout(&self, page: usize, len: usize) -> StringLayout {
        let state = self.arp_state();
        let steps = state
            .editor_row(page, !self.programming)
            .page
            .flattened_steps(self.arp_pass_len_for_page(page));
        let swing = self.params.swing.value();
        // A fixed cycle origin keeps edit targets still across odd cycles and length modulation.
        StringLayout::new(&steps, len, swing, 0).rotated(
            state
                .editor_row(page, !self.programming)
                .view_step_origin,
        )
    }
    pub(super) fn loop_headers(&self, page: usize, count: usize) -> Vec<(usize, Rect)> {
        if self.programming {
            return Vec::new();
        }
        let state = self.arp_state();
        let (_, len) = self.arp_cycle_for_page(page);
        let origin = state.editor_row(page, !self.programming).view_step_origin % len.max(1);
        let g = graph_rect();
        let shift = origin as f32 / len.max(1) as f32 * g.2;
        (0..count)
            .flat_map(|i| {
                let mut r = loop_rect(i, count);
                r.0 -= shift;
                [r, (r.0 + g.2, r.1, r.2, r.3)]
                    .into_iter()
                    .filter(move |r| r.0 < g.0 + g.2 && r.0 + r.2 > g.0)
                    .map(move |r| (i, r))
            })
            .collect()
    }
    fn loop_header_at(&self, x: f32, y: f32) -> Option<(usize, Rect)> {
        let g = graph_rect();
        if self.programming || x < g.0 || x >= g.0 + g.2 {
            return None;
        }
        self.loop_headers(self.arp_ui.edit_page, self.visible_loops())
            .into_iter()
            .find(|(_, r)| hit(*r, x, y))
    }
    #[cfg(test)]
    pub(in crate::ui) fn loop_button_rect(&self, index: usize, button: usize) -> Rect {
        loop_header_button(loop_rect(index, self.visible_loops()), button)
    }
    fn add_arp_loop(&self, cx: &mut EventContext) {
        let count = self.visible_loops();
        if count >= LOOPS {
            return;
        }
        self.params.arpeggiator.edit(|s| {
            s.editor_row_mut(self.arp_ui.edit_page, !self.programming)
                .page
                .ensure_loops(count + 1);
        });
        self.set_arp_loop_count(cx, count + 1);
    }
    fn set_arp_loop_count(&self, cx: &mut EventContext, count: usize) {
        self.params.arpeggiator.edit(|s| {
            s.editor_row_mut(self.arp_ui.edit_page, !self.programming)
                .settings
                .octaves = count as u8
        });
        let p = &self.arp_params().arp_octaves;
        Self::emit(cx, p.as_ptr(), p.preview_normalized(count as i32));
    }
    pub(super) fn loop_remove_visible(&self, index: usize, count: usize) -> bool {
        let content = content_rect();
        count > 1
            && self.pointer.is_some_and(|(x, y)| {
                hit(content, x, y)
                    && self
                        .loop_headers(self.arp_ui.edit_page, count)
                        .into_iter()
                        .any(|(i, r)| i == index && hit((r.0, content.1, r.2, content.3), x, y))
            })
    }
    fn remove_arp_loop(&self, cx: &mut EventContext, index: usize) {
        let count = self.visible_loops();
        if count <= 1 {
            return;
        }
        self.params.arpeggiator.edit(|s| {
            let page = &mut s
                .editor_row_mut(self.arp_ui.edit_page, !self.programming)
                .page;
            page.loops[index..count].rotate_left(1);
            page.initialized = page.initialized.min((count - 1) as u8);
        });
        self.set_arp_loop_count(cx, count - 1);
    }
    pub(super) fn chooser_count(&self) -> usize {
        8
    }
    pub(super) fn chooser_hit_rect(&self, i: usize) -> Rect {
        chooser_card_for(i, self.chooser_count())
    }
    fn initialize_arp_loops(&self) {
        if self.editor_arp_inherits() {
            return;
        }
        let state = self.arp_state();
        {
            let row = state.editor_row(self.arp_ui.edit_page, !self.programming);
            self.params.arpeggiator.edit(|s| {
                s.editor_row_mut(self.arp_ui.edit_page, !self.programming)
                    .page
                    .ensure_loops(row.settings.octaves as usize);
            });
        }
    }
    pub(super) fn arp_cycle_for_editor(&self) -> ([i16; STEPS], usize) {
        self.arp_cycle_for_page(self.arp_ui.edit_page)
    }
    pub(super) fn arp_cycle_for_page(&self, page: usize) -> ([i16; STEPS], usize) {
        if self.drawing.get() {
            if let Some((_, notes, len)) = self
                .cycle_cache
                .borrow()
                .iter()
                .find(|(cached, _, _)| *cached == page)
            {
                return (*notes, *len);
            }
        }
        let state = self.arp_state();
        let Some(row) = Some(state.editor_row(page, !self.programming)) else {
            return ([0; STEPS], 0);
        };
        let settings = row.settings;
        let recipe = row
            .chord
            .or_else(|| SavedChord::decode(self.snapshot.captured));
        let mut globals = self.params.global_harmony();
        if row.chord.is_none() {
            globals.transpose = 0;
        }
        let chord = recipe
            .map(|c| harmony::resolve_chord(c, globals))
            .unwrap_or(self.snapshot.notes);
        let phase = if (row.chord.is_some() || !self.programming)
            && (!self.programming || self.snapshot.arp_row_index as usize == page)
        {
            self.arp_playback().phase
        } else {
            0
        };
        let mut pattern = row.page;
        if self.programming {
            pattern.offset_voicing(row.arp_octave, row.arp_inversion);
        }
        let (mut notes, len) = crate::engine::arpeggiator::program_cycle(
            chord, recipe, globals, settings, &pattern, 0x4e6f7465, phase,
        );
        if (row.chord.is_some() || !self.programming)
            && (!self.programming || self.snapshot.arp_row_index as usize == page)
        {
            let live = (self.arp_playback().len as usize).min(len);
            notes[..live].copy_from_slice(&self.arp_playback().notes[..live]);
        }
        if self.drawing.get() {
            self.cycle_cache.borrow_mut().push((page, notes, len));
        }
        (notes, len)
    }
    pub(super) fn visible_loops(&self) -> usize {
        self.arp_state()
            .editor_row(self.arp_ui.edit_page, !self.programming)
            .settings
            .octaves as usize
    }
    pub(super) fn edit_tracker_voicing(&self, inversion: bool, delta: i8, reset: bool) {
        self.params.arpeggiator.edit(|state| {
            let row = state.editor_row_mut(self.arp_ui.edit_page, false);
            let value = if inversion {
                &mut row.arp_inversion
            } else {
                &mut row.arp_octave
            };
            *value = if reset { 0 } else { *value + delta };
        });
    }
    fn edit_loop_octave(&self, index: usize, delta: i8, reset: bool) {
        let page = self.arp_ui.edit_page;
        self.params.arpeggiator.edit(|s| {
            let octave = &mut s
                .editor_row_mut(page, !self.programming)
                .page
                .loops[index]
                .octave;
            *octave = if reset {
                0
            } else {
                (*octave + delta).clamp(-4, 4)
            };
        });
    }
    fn edit_loop_inversion(&self, index: usize, delta: i8, reset: bool) {
        let page = self.arp_ui.edit_page;
        self.params.arpeggiator.edit(|s| {
            let inversion = &mut s
                .editor_row_mut(page, !self.programming)
                .page
                .loops[index]
                .inversion;
            *inversion = if reset {
                0
            } else {
                (*inversion + delta).clamp(-12, 12)
            };
        });
    }
    pub(super) fn edit_fixed_steps(&self, delta: i8, reset: bool) {
        let page = self.arp_ui.edit_page;
        self.params.arpeggiator.edit(|s| {
            let steps = &mut s.editor_row_mut(page, !self.programming).settings.steps;
            *steps = if reset {
                4
            } else {
                (*steps as i16 + delta as i16).clamp(1, 24) as u8
            };
        });
    }
    pub(in crate::ui) fn copy_arp_page(&mut self, page: usize) {
        let state = self.arp_state();
        {
            let row = state.editor_row(page, !self.programming);
            self.arp_ui.clipboard = Some((row.page, row.settings));
        }
    }
    pub(in crate::ui) fn paste_arp_page(
        &self,
        _cx: &mut EventContext,
        destination_page: usize,
        pattern_only: bool,
    ) {
        let Some((copied, copied_settings)) = self.arp_ui.clipboard else {
            return;
        };
        self.params.arpeggiator.edit(|s| {
            let row = s.editor_row_mut(destination_page, !self.programming);
            row.page = copied;
            if pattern_only {
                row.settings.pattern = copied_settings.pattern;
                row.settings.octaves = copied_settings.octaves;
                row.settings.lock_length = copied_settings.lock_length;
                row.settings.steps = copied_settings.steps;
                row.settings.fill = copied_settings.fill;
                row.settings.custom = copied_settings.custom;
            } else {
                row.settings = copied_settings;
            }
        });
    }

    pub(in crate::ui) fn arp_editor_button_rects(&self) -> Vec<(Rect, bool)> {
        if self.fx_ui.open && !self.fx_delay_expanded() {
            return self.fx_button_rects();
        }
        if self.fx_ui.open {
            let state = self.arp_state();
            let delay = &state.midi_fx.delay;
            let mut v = vec![
                (delay_power_rect(), delay.enabled),
                (delay_sync_rect(), delay.sync),
                (delay_time_rect(), false),
                (delay_gain_rect(), false),
                (delay_feedback_rect(), false),
                (
                    tap_lane_rect(),
                    self.arp_ui.dropdown == Some(Dropdown::TapLane),
                ),
            ];
            v.push((ping_rect(), delay.ping_pong));
            v.push((delay_leading_rect(), delay.voice_leading));
            v.push((delay_follow_rect(), delay.follow_chords));
            let domain = time_domain(delay);
            if (delay.count as usize) < TAPS {
                v.push((add_rect(delay, domain), false));
            }
            if delay.count > 1 {
                v.extend(
                    delay.taps[..delay.count as usize]
                        .iter()
                        .enumerate()
                        .map(|(i, tap)| {
                            (
                                tap_remove_rect(tap, domain),
                                self.arp_ui.pending_tap_remove == Some(i),
                            )
                        }),
                );
            }
            return v
                .into_iter()
                .map(|(r, on)| (self.fx_delay_screen_rect(r), on))
                .chain(self.fx_button_rects())
                .collect();
        }
        let mut v = Vec::new();
        if self.programming {
            for inversion in [false, true] {
                for plus in [false, true] {
                    v.push((tracker_voicing_button(inversion, plus), false));
                }
            }
        }
        if let Some(inherited) = self.editor_arp_inheritance() {
            v.push((inherit_rect(), inherited));
            if inherited {
                return v;
            }
        }
        v.extend([
            (power_rect(), self.editor_arp_enabled()),
            (mode_rect(), self.arp_params().arp_mode.value() != 0),
            (order_rect(), self.menu == Some(Menu::ArpPattern)),
            (sync_rect(), self.arp_params().arp_sync.value()),
        ]);
        if !self.programming {
            v.push((loop_add_rect(), false));
        }
        let settings = self
            .arp_state()
            .editor_row(self.arp_ui.edit_page, !self.programming)
            .settings;
        v.extend([
            (lock_length_rect(), settings.lock_length),
            (fixed_steps_rect(), false),
            (fill_rect(), settings.fill == Fill::Evolve),
        ]);
        if settings.pattern == 7 {
            v.push((custom_edit_rect(), self.panel == Some(Panel::Custom)));
        }
        v.extend(
            LANES
                .iter()
                .enumerate()
                .map(|(i, lane)| (lane_rect(i), self.arp_ui.lane == *lane)),
        );
        if matches!(
            self.arp_ui.dropdown,
            Some(Dropdown::Pages | Dropdown::Paste(_))
        ) {
            for page in 0..8 {
                v.extend([
                    (page_rect(page), self.arp_ui.edit_page == page),
                    (page_copy_rect(page), false),
                    (page_paste_rect(page), self.arp_ui.clipboard.is_some()),
                ]);
            }
        }
        if let Some(Dropdown::Paste(page)) = self.arp_ui.dropdown {
            v.extend((0..2).map(|i| (page_paste_option_rect(page, i), false)));
        }
        if matches!(
            self.arp_ui.dropdown,
            Some(Dropdown::Pages | Dropdown::Paste(_))
        ) {
            return v;
        }
        for (i, r) in self.loop_headers(self.arp_ui.edit_page, self.visible_loops()) {
            for button in 0..5 {
                if button != 2 || self.loop_remove_visible(i, self.visible_loops()) {
                    v.push((loop_header_button(r, button), false));
                }
            }
        }
        if self.arp_ui.chooser.is_some() {
            let selected = match self.arp_ui.chooser_draw {
                Chooser::Order => self.arp_params().arp_pattern.value() as usize,
            };
            v.extend((0..self.chooser_count()).map(|i| (self.chooser_hit_rect(i), selected == i)));
            return v;
        }
        let state = self.arp_state();
        let (_, len) = self.arp_cycle_for_editor();
        let layout = self.arp_string_layout(self.arp_ui.edit_page, len);
        v.extend((0..len).map(|i| {
            (
                layout.status(i),
                state
                    .editor_row(self.arp_ui.edit_page, !self.programming)
                    .page
                    .step(i, self.arp_pass_len_for_page(self.arp_ui.edit_page))
                    .state
                    != StepState::Play,
            )
        }));
        v
    }
    pub(in crate::ui) fn arp_editor_route_targets(&self) -> Vec<(usize, Rect)> {
        Vec::new()
    }
    pub(in crate::ui) fn arp_editor_hint(&self, x: f32, y: f32) -> Option<String> {
        if self.fx_ui.open {
            let y = self.fx_delay_local_y(y);
            if hit(delay_sync_rect(), x, y) {
                return Some(
                    "Tempo sync locks delay time to the host clock · Off uses milliseconds".into(),
                );
            }
            if hit(ping_rect(), x, y) {
                return Some("Ping pong · Alternate delay repeats left and right".into());
            }
            if hit(delay_leading_rect(), x, y) {
                return Some("Voice leading · Include sounding echoes in chord voice leading and apply its octave placement to repeats · Uses the keyboard's Nearest / Furthest mode; Off leaves pitches unchanged".into());
            }
            if hit(delay_follow_rect(), x, y) {
                return Some("Follow chords · Move each echo to the nearest current chord tone · Equal distance resolves downward".into());
            }
            if hit(delay_time_rect(), x, y) {
                return Some(
                    "Selected tap interval · Drag vertically or scroll · Cmd/Ctrl-click resets"
                        .into(),
                );
            }
            if hit(delay_gain_rect(), x, y) || hit(delay_feedback_rect(), x, y) {
                return Some(
                    "Selected tap · Drag vertically or scroll · Cmd/Ctrl-click resets".into(),
                );
            }
            if let Some(index) = self.arp_ui.pending_tap_remove {
                let state = self.arp_state();
                let delay = &state.midi_fx.delay;
                if index < delay.count as usize
                    && hit(tap_remove_rect(&delay.taps[index], time_domain(delay)), x, y)
                {
                    return Some(format!(
                        "Click again to remove tap {} · Esc cancels",
                        index + 1
                    ));
                }
            }
            if hit(tap_graph(), x, y) {
                return Some("Drag a numbered top handle sideways to move its tap · Drag a string vertically, then across to paint · Shift makes a straight ramp · Cmd/Ctrl resets".into());
            }
            return Some("Each tap has an independent echo train · + adds a tap · × below a tap asks for confirmation before deleting it".into());
        }
        if !hit(ARP_SURFACE, x, y) {
            return None;
        }
        if self.programming && hit(inherit_rect(), x, y) {
            return Some(match self.editor_arp_inheritance() {
                None => "The first tracker entry has no previous arp to inherit",
                Some(false) => {
                    "Use the previous tracker entry’s arp settings and pattern with this chord"
                }
                Some(true) => "Use this entry’s own arp settings and pattern again",
            }
            .into());
        }
        if self.programming && hit(tracker_voicing_rect(false), x, y) {
            return Some("Octave · Move this tracker entry's arp up or down · Scroll to adjust · Cmd/Ctrl-click resets".into());
        }
        if self.programming && hit(tracker_voicing_rect(true), x, y) {
            return Some("Inversion · Rotate this tracker entry's arp voices above or below the saved voicing · Scroll to adjust · Cmd/Ctrl-click resets".into());
        }
        if self.editor_arp_inherits() {
            return None;
        }
        if self.arp_ui.chooser.is_some() && hit(chooser_area(), x, y) {
            return Some(
                "Choose ordering · Left/Right to browse · Enter to choose · Escape to return"
                    .into(),
            );
        }
        if hit(power_rect(), x, y) {
            return Some(if self.programming {
                "Tracker Arp · Enable or bypass arpeggiator playback · Off uses the Performance Arp settings for this entry".into()
            } else {
                "Performance Arp · Enable or bypass arpeggiator playback · Arpeggiate played-in chords independently of the tracker".into()
            });
        }
        if hit(mode_rect(), x, y) {
            return Some("Repeat this pattern · On: repeat continuously · Off: play once · DAW alignment is set in the tracker tempo area".into());
        }
        if hit(sync_rect(), x, y) {
            return Some("Sync the arp to the host clock · Off uses a free millisecond rate".into());
        }
        if !self.programming && hit(loop_add_rect(), x, y) {
            return Some(
                "Loops · + Loop adds a pass · Hover a loop and click × to remove that pass".into(),
            );
        }
        if (0..8).any(|p| hit(page_copy_rect(p), x, y)) {
            return Some("Copy this complete row, including settings and adjustments".into());
        }
        if (0..8).any(|p| hit(page_paste_rect(p), x, y)) {
            return Some("Paste into this row · Choose Complete row or Pattern only".into());
        }
        if hit(gate_rect(), x, y) {
            return Some("Timing · Drag the arrow handle sideways ±49% of the step interval · Available on every adjustment layer · Cmd/Ctrl-click resets".into());
        }
        if hit(lock_length_rect(), x, y) {
            return Some("Lock Length · Keep a fixed number of clock steps despite chord complexity · Mute consumes time; Skip always removes time".into());
        }
        if !self
            .arp_state()
            .editor_row(self.arp_ui.edit_page, !self.programming)
            .settings
            .lock_length
            && (hit(fixed_steps_rect(), x, y) || hit(fill_rect(), x, y))
        {
            return Some("Enable Lock Length to set Steps and Repeat/Evolve".into());
        }
        if hit(fixed_steps_rect(), x, y) {
            return Some("Steps per loop · Click left/right or scroll: 1–24 · Cmd/Ctrl-click restores 4 · Applies at the next loop boundary".into());
        }
        if hit(fill_rect(), x, y) {
            return Some("Repeat restarts the note order each loop · Evolve carries it across loops and row repetitions".into());
        }
        if hit(custom_edit_rect(), x, y) && self.arp_params().arp_pattern.value() == 7 {
            return Some(
                "Edit the row's seven Custom degree steps in place of the chordboard".into(),
            );
        }
        if let Some((index, rect)) = self.loop_header_at(x, y) {
            let state = self.arp_state();
            let row = state.editor_row(self.arp_ui.edit_page, !self.programming);
            let inversion = row.page.loops[index].inversion;
            let octave = row.page.loops[index].octave;
            if x < loop_header_button(rect, 0).0 {
                return Some(format!("Loop {}", index + 1));
            }
            let octave_side = hit(loop_header_button(rect, 3), x, y)
                || hit(loop_header_button(rect, 4), x, y)
                || x >= rect.0 + 60.0 + 124.0;
            if octave_side && !hit(loop_header_button(rect, 2), x, y) {
                return Some(format!(
                    "Octave −/+ · currently {octave:+} · shifts this loop · Scroll · Cmd/Ctrl-click resets"
                ));
            }
            if hit(loop_header_button(rect, 0), x, y)
                || hit(loop_header_button(rect, 1), x, y)
                || x < rect.0 + 60.0 + 124.0
            {
                return Some(format!(
                    "Inversion −/+ · currently {inversion:+} · rotates voices down or up, atop the global inversion · Scroll · Cmd/Ctrl-click resets"
                ));
            }
            return Some(format!("Remove loop {}", index + 1));
        }
        if let Some(i) = (0..LANES.len()).find(|&i| hit(lane_rect(i), x, y)) {
            let lane = LANES[i];
            let label = match lane {
                Lane::Mod1 if !self.programming => "Perf Mod 1",
                Lane::Mod2 if !self.programming => "Perf Mod 2",
                _ => lane.label(),
            };
            return Some(format!(
                "{} · {} · Select to edit directly",
                label,
                lane.description()
            ));
        }
        let (_, len) = self.arp_cycle_for_editor();
        let layout = self.arp_string_layout(self.arp_ui.edit_page, len);
        if (0..len).any(|i| hit(layout.status(i), x, y)) {
            return Some("Right-click: make this step the visual one · Click: Play → Mute → Skip · Mute keeps its clock slot; Skip always removes time, including with Lock Length · Cmd/Ctrl-click restores Play".into());
        }
        if hit(graph_rect(), x, y) {
            return Some("Click cycles Play, Mute, and Skip · Right-click sets the visual step · Shift-drag ramps · Drag paints · Cmd/Ctrl-click resets".into());
        }
        None
    }
    pub(in crate::ui) fn arp_editor_open(&self) -> bool {
        true
    }
    pub(in crate::ui) fn arp_controls_covered(&self, x: f32, y: f32) -> bool {
        self.arp_ui.dropdown.is_some() && hit(ARP_SURFACE, x, y)
            || self.fx_ui.open && hit(DELAY_SURFACE, x, y)
    }
    pub(in crate::ui) fn arp_editor_value_at(&self, x: f32, y: f32) -> bool {
        if self.fx_ui.open {
            self.fx_delay_expanded() && hit(tap_graph(), x, self.fx_delay_local_y(y))
        } else {
            hit(graph_rect(), x, y) && !self.editor_arp_inherits()
        }
    }
    pub(in crate::ui) fn arp_editor_dragging(&self) -> bool {
        self.arp_ui.drag.is_some()
    }
    pub(in crate::ui) fn arp_editor_editing(&self) -> bool {
        false
    }
    pub(in crate::ui) fn tick_arp_editor(&mut self, dt: f32) -> bool {
        self.initialize_arp_loops();
        let u = &mut self.arp_ui;
        let mut changed = false;
        if u.chooser_elapsed < pleasant_ui::page_slide::DURATION {
            u.chooser_elapsed = (u.chooser_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
            u.chooser_position = pleasant_ui::page_slide::position(
                u.chooser_start,
                if u.chooser.is_some() { 1.0 } else { 0.0 },
                u.chooser_elapsed,
            );
            changed = true;
        }
        if u.page_elapsed < pleasant_ui::page_slide::DURATION {
            u.page_elapsed = (u.page_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
            changed = true;
        }
        changed
    }
    pub(super) fn set_arp_editor_page(&mut self, page: usize) {
        if page != self.arp_ui.edit_page {
            self.arp_ui.page_from = self.arp_ui.edit_page;
            self.arp_ui.edit_page = page;
            self.arp_ui.page_elapsed = 0.0;
            self.arp_ui.drag = None;
        }
    }
    pub(super) fn set_arp_chooser(&mut self, chooser: Option<Chooser>) {
        if chooser.is_some() {
            self.release_keys();
            self.memory_held.fill(false);
        }
        let pattern = self.arp_params().arp_pattern.value() as usize;
        let u = &mut self.arp_ui;
        if let Some(which) = chooser {
            if u.chooser_draw != which {
                u.chooser_position = 0.0;
            }
            u.chooser_draw = which;
            u.chooser_cursor = match which {
                Chooser::Order => pattern,
            };
        }
        u.chooser_start = u.chooser_position;
        u.chooser_elapsed = 0.0;
        u.chooser = chooser;
        u.dropdown = None;
        u.drag = None;
    }
    pub(super) fn choose_arp_option(&mut self, cx: &mut EventContext, index: usize) {
        match self.arp_ui.chooser {
            Some(Chooser::Order) => {
                let p = &self.arp_params().arp_pattern;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(index as i32));
            }
            None => return,
        }
        self.set_arp_chooser(None);

        cx.needs_redraw();
    }
    pub(in crate::ui) fn cancel_arp_editor(&mut self) {
        self.cancel_fx_value();
        self.arp_ui.drag = None;
        self.arp_ui.dropdown = None;
        self.arp_ui.pending_tap_remove = None;
        self.arp_ui.chooser = None;
        self.arp_ui.chooser_position = 0.0;
        self.arp_ui.chooser_elapsed = pleasant_ui::page_slide::DURATION;
    }
    pub(super) fn start_arp_drag(
        &mut self,
        cx: &mut EventContext,
        kind: DragKind,
        index: usize,
        value: f32,
        (x, y): (f32, f32),
        reset: bool,
    ) {
        self.restore_shift_latch(cx);
        self.arp_ui.pending_tap_remove = None;
        self.arp_ui.drag = Some(Drag {
            kind,
            page: self.arp_ui.edit_page,
            origin: index,
            last: index,
            x,
            y,
            value,
            last_value: value,
            reset,
            pass_len: self.arp_pass_len_for_page(self.arp_ui.edit_page),
        });
        cx.capture();
    }
    pub(super) fn apply_arp_drag(&mut self, cx: &mut EventContext, x: f32, y: f32) {
        if !self.fx_ui.open && self.editor_arp_inherits() {
            self.arp_ui.drag = None;
            cx.release();
            return;
        }
        let Some(mut drag) = self.arp_ui.drag else {
            return;
        };
        if matches!(drag.kind, DragKind::Step(_) | DragKind::Timing)
            && drag.pass_len != self.arp_pass_len_for_page(drag.page)
        {
            self.arp_ui.drag = None;
            cx.release();
            return;
        }
        let ramp = cx.modifiers().shift();
        let reset = drag.reset;
        match drag.kind {
            DragKind::Step(lane) => {
                let (_, len) = self.arp_cycle_for_editor();
                if len == 0 {
                    return;
                }
                let layout = self.arp_string_layout(self.arp_ui.edit_page, len);
                let next = layout.at(x);
                let (lo, hi) = lane.range();
                let v = if reset {
                    lane.get(&Step::default())
                } else {
                    (drag.value + (drag.y - y) * (hi - lo) / graph_rect().3).clamp(lo, hi)
                };
                let (from, start) = if ramp {
                    (drag.origin, drag.value)
                } else {
                    (drag.last, drag.last_value)
                };
                self.params.arpeggiator.edit(|s| {
                    paint_between(
                        layout.visual(from),
                        layout.visual(next),
                        if reset { v } else { start },
                        v,
                        |i, value| {
                            lane.set(
                                s.editor_row_mut(drag.page, !self.programming)
                                    .page
                                    .step_mut(layout.source(i), drag.pass_len),
                                value,
                            )
                        },
                    )
                });
                drag.last = next;
                drag.last_value = v;
            }
            DragKind::Timing => {
                let (_, len) = self.arp_cycle_for_editor();
                let v = if reset {
                    0.0
                } else {
                    drag.value
                        + (x - drag.x)
                            / self.arp_string_layout(drag.page, len).interval[drag.origin]
                            * 100.0
                };
                self.params.arpeggiator.edit(|s| {
                    s.editor_row_mut(drag.page, !self.programming)
                        .page
                        .step_mut(drag.origin, drag.pass_len)
                        .micro = v.clamp(-49.0, 49.0).round() as i16
                });
            }
            DragKind::Tap(lane) => {
                let state = self.arp_state();
                let delay = &state.midi_fx.delay;
                let domain = time_domain(delay);
                let g = tap_graph();
                let next = (0..delay.count as usize)
                    .min_by(|a, b| {
                        (tap_x(&delay.taps[*a], domain) - x)
                            .abs()
                            .total_cmp(&(tap_x(&delay.taps[*b], domain) - x).abs())
                    })
                    .unwrap_or(0);
                let (lo, hi) = lane.range();
                let v = if reset {
                    lane.get(&DelayTap::default())
                } else {
                    (drag.value + (drag.y - y) * (hi - lo) / g.3).clamp(lo, hi)
                };
                // Paint by spatial position; tap order remains stable even after taps cross.
                let from = if ramp { drag.origin } else { drag.last };
                let start = if reset {
                    v
                } else if ramp {
                    drag.value
                } else {
                    drag.last_value
                };
                let a = tap_x(&delay.taps[from], domain);
                let b = tap_x(&delay.taps[next], domain);
                self.params.arpeggiator.edit(|s| {
                    for i in 0..delay.count as usize {
                        let xx = tap_x(&delay.taps[i], domain);
                        if xx >= a.min(b) - 0.01 && xx <= a.max(b) + 0.01 {
                            let f = if (b - a).abs() < 0.01 {
                                1.0
                            } else {
                                (xx - a) / (b - a)
                            };
                            lane.set(&mut s.midi_fx.delay.taps[i], start + (v - start) * f);
                        }
                    }
                });
                drag.last = next;
                drag.last_value = v;
                self.arp_ui.selected_tap = next;
            }
            DragKind::TapTime => {
                let sync = self.arp_state().midi_fx.delay.sync;
                let (lo, hi) = time_range(sync);
                let value = if reset {
                    if sync {
                        0.5
                    } else {
                        250.0
                    }
                } else {
                    drag.value + (x - drag.x) / tap_graph().2 * drag.last_value
                };
                self.params
                    .arpeggiator
                    .edit(|s| s.midi_fx.delay.taps[drag.origin].time = value.clamp(lo, hi));
            }
            DragKind::BasicTime | DragKind::BasicGain | DragKind::BasicFeedback => {
                let sync = self.arp_state().midi_fx.delay.sync;
                self.params.arpeggiator.edit(|s| {
                    let t = &mut s.midi_fx.delay.taps[drag.origin];
                    match drag.kind {
                        DragKind::BasicTime => {
                            let (lo, hi) = time_range(sync);
                            let value = if reset {
                                if sync {
                                    0.5
                                } else {
                                    250.0
                                }
                            } else {
                                drag.value * 2.0_f32.powf((drag.y - y) / 70.0)
                            };
                            t.time = value.clamp(lo, hi)
                        }
                        DragKind::BasicGain => {
                            t.velocity = if reset {
                                0.75
                            } else {
                                (drag.value + (drag.y - y) / 100.0).clamp(0.0, 2.0)
                            }
                        }
                        DragKind::BasicFeedback => {
                            t.feedback = if reset {
                                0.35
                            } else {
                                (drag.value + (drag.y - y) / 160.0).clamp(0.0, 0.95)
                            }
                        }
                        _ => {}
                    }
                });
            }
        }
        self.arp_ui.drag = Some(drag);
        cx.needs_redraw();
    }

    pub(in crate::ui) fn arp_editor_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if matches!(event, WindowEvent::FocusOut) {
            self.cancel_arp_editor();
            cx.release();
            return false;
        }
        // Route chips are drawn above the arp, including its compact header.
        // Let their existing overlay handler receive presses before controls below.
        if self.arp_ui.drag.is_none()
            && self.route_chip_at(x, y).is_some()
            && matches!(
                event,
                WindowEvent::MouseDown(_)
                    | WindowEvent::MouseDoubleClick(_)
                    | WindowEvent::MouseTripleClick(_)
                    | WindowEvent::MouseScroll(_, _)
            )
        {
            return false;
        }
        if self.arp_ui.chooser.is_some() {
            match event {
                WindowEvent::KeyDown(code, _) => {
                    match code {
                        Code::ArrowLeft | Code::ArrowUp => {
                            self.arp_ui.chooser_cursor =
                                (self.arp_ui.chooser_cursor + self.chooser_count() - 1)
                                    % self.chooser_count()
                        }
                        Code::ArrowRight | Code::ArrowDown => {
                            self.arp_ui.chooser_cursor =
                                (self.arp_ui.chooser_cursor + 1) % self.chooser_count()
                        }
                        Code::Enter | Code::NumpadEnter => {
                            self.choose_arp_option(cx, self.arp_ui.chooser_cursor)
                        }
                        Code::Escape => self.set_arp_chooser(None),
                        _ => {}
                    }
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::KeyUp(_, _) | WindowEvent::CharInput(_) => return true,
                _ => {}
            }
        }
        if self.arp_ui.drag.is_some() {
            match event {
                WindowEvent::MouseMove(_, _) => self.apply_arp_drag(cx, x, y),
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::KeyDown(Code::Escape, _) => {
                    self.arp_ui.drag = None;
                    cx.release();
                    cx.needs_redraw();
                }
                _ => {}
            }
            return true;
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _))
            && self.arp_ui.pending_tap_remove.take().is_some()
        {
            cx.needs_redraw();
            return true;
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _)) && self.arp_ui.dropdown.is_some()
        {
            self.arp_ui.dropdown = None;
            cx.needs_redraw();
            return true;
        }
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
        if press {
            if let Some(index) = self.arp_ui.pending_tap_remove {
                let state = self.arp_state();
                let delay = &state.midi_fx.delay;
                if index >= delay.count as usize
                    || !hit(
                        tap_remove_rect(&delay.taps[index], time_domain(delay)),
                        x,
                        y,
                    )
                {
                    self.arp_ui.pending_tap_remove = None;
                }
            }
        }
        if let Some(which) = self.arp_ui.dropdown {
            if press {
                if matches!(which, Dropdown::Pages | Dropdown::Paste(_)) {
                    for page in 0..8 {
                        if hit(page_copy_rect(page), x, y) {
                            self.copy_arp_page(page);
                            self.arp_ui.dropdown = Some(Dropdown::Pages);
                            cx.needs_redraw();
                            return true;
                        }
                        if hit(page_paste_rect(page), x, y) {
                            if self.arp_ui.clipboard.is_some() {
                                self.arp_ui.dropdown = Some(Dropdown::Paste(page));
                            }
                            cx.needs_redraw();
                            return true;
                        }
                        if hit(page_rect(page), x, y) {
                            self.set_arp_editor_page(page);

                            self.arp_ui.dropdown = None;
                            cx.needs_redraw();
                            return true;
                        }
                    }
                }
                let count = which.len();
                if let Some(i) = (0..count).find(|&i| hit(dropdown_rect(which, i), x, y)) {
                    match which {
                        Dropdown::TapLane => self.arp_ui.tap_lane = TAP_LANES[i],
                        Dropdown::Paste(page) => self.paste_arp_page(cx, page, i == 1),
                        Dropdown::Pages => {}
                    }
                }
                self.arp_ui.dropdown = None;
                cx.needs_redraw();
                return true;
            }
            return hit(
                if which == Dropdown::TapLane {
                    DELAY_SURFACE
                } else {
                    ARP_SURFACE
                },
                x,
                y,
            ) && matches!(
                event,
                WindowEvent::MouseScroll(_, _)
                    | WindowEvent::MouseDown(_)
                    | WindowEvent::MouseUp(_)
            );
        }
        if self.fx_ui.open {
            if press && hit(delay_power_rect(), x, y) {
                self.toggle_fx_power(crate::midi_fx::MidiFxId::Delay);
                cx.needs_redraw();
                return true;
            }
            return hit(DELAY_SURFACE, x, y) && self.delay_editor_event(cx, event, x, y, press);
        }
        if !hit(ARP_SURFACE, x, y) {
            return false;
        }
        if self.programming {
            if let Some(inversion) = [false, true]
                .into_iter()
                .find(|&inversion| hit(tracker_voicing_rect(inversion), x, y))
            {
                let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
                let delta = if let WindowEvent::MouseScroll(_, dy) = event {
                    dy.signum() as i8
                } else if press && hit(tracker_voicing_button(inversion, false), x, y) {
                    -1
                } else if press && hit(tracker_voicing_button(inversion, true), x, y) {
                    1
                } else {
                    0
                };
                if delta != 0 || (press && reset) {
                    self.edit_tracker_voicing(inversion, delta, reset);
                    cx.needs_redraw();
                    return true;
                }
            }
        }
        let inheritance = self.editor_arp_inheritance();
        if self.programming && press && hit(inherit_rect(), x, y) {
            if let Some(inherited) = inheritance {
                let index = self.arp_ui.edit_page;
                self.params.arpeggiator.edit(|state| {
                    if let Some(row) = state.rows.get_mut(index) {
                        row.inherit_arp = !inherited;
                    }
                });
                self.cancel_arp_editor();
                self.refresh_arp_editor_params();
                cx.needs_redraw();
            }
            return true;
        }
        if inheritance == Some(true) {
            return matches!(
                event,
                WindowEvent::MouseDown(_)
                    | WindowEvent::MouseDoubleClick(_)
                    | WindowEvent::MouseTripleClick(_)
                    | WindowEvent::MouseScroll(_, _)
            );
        }
        if (self.arp_ui.chooser_position > 0.0 || self.arp_ui.chooser.is_some())
            && hit(chooser_area(), x, y)
        {
            if press
                && self.arp_ui.chooser.is_some()
                && self.arp_ui.chooser_elapsed >= pleasant_ui::page_slide::DURATION
            {
                if let Some(index) =
                    (0..self.chooser_count()).find(|&i| hit(self.chooser_hit_rect(i), x, y))
                {
                    self.choose_arp_option(cx, index);
                }
            }
            return true;
        }
        if self.arp_ui.page_elapsed < pleasant_ui::page_slide::DURATION && hit(content_rect(), x, y)
        {
            return true;
        }
        if matches!(event, WindowEvent::MouseDown(MouseButton::Right)) {
            let (_, len) = self.arp_cycle_for_editor();
            if len > 0
                && (hit(graph_rect(), x, y)
                    || (0..len).any(|i| {
                        hit(
                            self.arp_string_layout(self.arp_ui.edit_page, len).status(i),
                            x,
                            y,
                        )
                    }))
            {
                let origin = self.arp_string_layout(self.arp_ui.edit_page, len).at(x);
                self.params.arpeggiator.set_view_step_origin(
                    self.arp_ui.edit_page,
                    !self.programming,
                    origin,
                );
                cx.needs_redraw();
                return true;
            }
        }
        // Consume disabled controls before click, reset, or wheel handling.
        if !self
            .arp_state()
            .editor_row(self.arp_ui.edit_page, !self.programming)
            .settings
            .lock_length
            && (hit(fixed_steps_rect(), x, y) || hit(fill_rect(), x, y))
        {
            return true;
        }
        self.initialize_arp_loops();
        let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
        if let WindowEvent::MouseScroll(_, dy) = event {
            if *dy == 0.0 {
                return true;
            }
            if hit(fixed_steps_rect(), x, y) {
                self.edit_fixed_steps(dy.signum() as i8, reset);
                cx.needs_redraw();
                return true;
            }
            if !self.programming && hit(loop_add_rect(), x, y) {
                let p = &self.arp_params().arp_octaves;
                Self::emit(
                    cx,
                    p.as_ptr(),
                    p.preview_normalized((p.value() + dy.signum() as i32).clamp(1, LOOPS as i32)),
                );
                cx.needs_redraw();
                return true;
            }
            if let Some((i, r)) = self.loop_header_at(x, y) {
                if self.visible_loops() > 1 && hit(loop_header_button(r, 2), x, y) {
                    return true;
                }
                if hit(loop_header_button(r, 0), x, y) || hit(loop_header_button(r, 1), x, y) {
                    self.edit_loop_inversion(i, dy.signum() as i8, reset);
                } else {
                    self.edit_loop_octave(i, dy.signum() as i8, reset);
                }
                cx.needs_redraw();
                return true;
            }
            if hit(graph_rect(), x, y) {
                let (_, len) = self.arp_cycle_for_editor();
                if len > 0 {
                    let i = self.arp_string_layout(self.arp_ui.edit_page, len).at(x);
                    let page = self.arp_ui.edit_page;
                    let lane = self.arp_ui.lane;
                    let pass_len = self.arp_pass_len_for_page(page);
                    self.params.arpeggiator.edit(|s| {
                        let t = s
                            .editor_row_mut(page, !self.programming)
                            .page
                            .step_mut(i, pass_len);
                        lane.set(
                            t,
                            if reset {
                                lane.get(&Step::default())
                            } else {
                                lane.get(t) + dy.signum()
                            },
                        )
                    });
                    cx.needs_redraw()
                }
                return true;
            }
            return false;
        }
        if !press {
            return false;
        }
        cx.focus();
        self.focused = true;
        if hit(lock_length_rect(), x, y) {
            let page = self.arp_ui.edit_page;
            self.params.arpeggiator.edit(|s| {
                let settings = &mut s.editor_row_mut(page, !self.programming).settings;
                settings.lock_length = !reset && !settings.lock_length;
            });
        } else if hit(fixed_steps_rect(), x, y) {
            let r = fixed_steps_rect();
            self.edit_fixed_steps(if x < r.0 + r.2 * 0.5 { -1 } else { 1 }, reset);
        } else if hit(fill_rect(), x, y) {
            let page = self.arp_ui.edit_page;
            self.params.arpeggiator.edit(|s| {
                let fill = &mut s.editor_row_mut(page, !self.programming).settings.fill;
                *fill = if !reset && *fill == Fill::Evolve {
                    Fill::Repeat
                } else {
                    Fill::Evolve
                };
            });
        } else if hit(custom_edit_rect(), x, y) && self.arp_params().arp_pattern.value() == 7 {
            self.set_panel(Some(Panel::Custom));
        } else if hit(power_rect(), x, y) {
            if self.programming {
                self.params.arpeggiator.edit(|s| {
                    let row = s.editor_row_mut(self.arp_ui.edit_page, false);
                    row.enabled = !row.enabled;
                });
            } else {
                self.toggle_learnable_power(crate::midi_learn::PERFORMANCE_ARP, |s, enabled| {
                    s.performance.enabled = enabled;
                });
            }
        } else if hit(mode_rect(), x, y) {
            let p = &self.arp_params().arp_mode;
            Self::emit(
                cx,
                p.as_ptr(),
                p.preview_normalized(if reset || p.value() == 0 { 2 } else { 0 }),
            );
        } else if hit(order_rect(), x, y) {
            if reset {
                let p = &self.arp_params().arp_pattern;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(0));
            } else {
                if self.explain_modulation("arp_pattern") {
                    return true;
                }
                self.open_menu(Menu::ArpPattern);
            }
        } else if !self.programming && hit(loop_add_rect(), x, y) {
            self.add_arp_loop(cx);
        } else if hit(sync_rect(), x, y) {
            let p = &self.arp_params().arp_sync;
            Self::emit(
                cx,
                p.as_ptr(),
                p.preview_normalized(if reset { true } else { !p.value() }),
            );
        } else if hit(rate_rect(), x, y) && self.arp_params().arp_sync.value() {
            if self.explain_modulation("rate") {
                return true;
            }
            if reset {
                let p = &self.arp_params().rate;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(0.25));
            } else {
                self.open_menu(Menu::ArpRate)
            }
        } else if let Some((i, r)) = self.loop_header_at(x, y) {
            if hit(loop_header_button(r, 0), x, y) {
                self.edit_loop_inversion(i, -1, reset);
            } else if hit(loop_header_button(r, 1), x, y) {
                self.edit_loop_inversion(i, 1, reset);
            } else if self.visible_loops() > 1 && hit(loop_header_button(r, 2), x, y) {
                self.remove_arp_loop(cx, i);
            } else if hit(loop_header_button(r, 3), x, y) {
                self.edit_loop_octave(i, -1, reset);
            } else if hit(loop_header_button(r, 4), x, y) {
                self.edit_loop_octave(i, 1, reset);
            }
        } else if let Some(i) = (0..LANES.len()).find(|&i| hit(lane_rect(i), x, y)) {
            self.arp_ui.lane = LANES[i];
        } else {
            let (_, len) = self.arp_cycle_for_editor();
            if len == 0 {
                return false;
            }
            let page = self.arp_ui.edit_page;
            let layout = self.arp_string_layout(page, len);
            if !hit(content_rect(), x, y) {
                return false;
            }
            if hit(gate_rect(), x, y) {
                let state = self.arp_state();
                let i = (0..len)
                    .min_by(|&a, &b| {
                        let position = |i| {
                            layout.x[i]
                                + state
                                    .editor_row(page, !self.programming)
                                    .page
                                    .step(i, self.arp_pass_len_for_page(page))
                                    .micro as f32
                                    * layout.interval[i]
                                    / 100.0
                        };
                        (position(a) - x).abs().total_cmp(&(position(b) - x).abs())
                    })
                    .unwrap_or(0);
                let value = state
                    .editor_row(page, !self.programming)
                    .page
                    .step(i, self.arp_pass_len_for_page(page))
                    .micro as f32;
                self.start_arp_drag(cx, DragKind::Timing, i, value, (x, y), reset);
                if reset {
                    self.apply_arp_drag(cx, x, y);
                }
            } else if let Some(i) = (0..len).find(|&i| hit(layout.status(i), x, y)) {
                let pass_len = self.arp_pass_len_for_page(page);
                self.params.arpeggiator.edit(|s| {
                    let t = s
                        .editor_row_mut(page, !self.programming)
                        .page
                        .step_mut(i, pass_len);
                    t.state = if reset {
                        StepState::Play
                    } else {
                        t.state.next()
                    };
                });
            } else if hit(graph_rect(), x, y) {
                let state = self.arp_state();
                let lane = self.arp_ui.lane;
                let i = layout.at(x);
                let value = lane.get(
                    state
                        .editor_row(page, !self.programming)
                        .page
                        .step(i, self.arp_pass_len_for_page(page)),
                );
                self.start_arp_drag(cx, DragKind::Step(lane), i, value, (x, y), reset);
                if reset {
                    self.apply_arp_drag(cx, x, y)
                }
            } else {
                return false;
            }
        }
        cx.needs_redraw();
        true
    }
}
