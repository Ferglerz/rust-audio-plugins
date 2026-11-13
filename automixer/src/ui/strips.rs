use super::*;

impl AutomixerView {
    pub(super) fn collect_slots(network: &Network, out: &mut [usize; CAPACITY]) -> usize {
        let mut count = 0;
        for (i, slot) in network.slots.iter().enumerate() {
            if slot.owner.load(Ordering::Acquire) != 0 {
                out[count] = i;
                count += 1;
            }
        }
        out[..count].sort_by_key(|&i| network.slots[i].mic.load(Ordering::Acquire));
        count
    }
    pub(super) fn compute_drawn_strips(
        network: &Network,
        page: usize,
    ) -> [Option<DrawnStrip>; PER_PAGE] {
        let mut slots_buf = [0usize; CAPACITY];
        let count = Self::collect_slots(network, &mut slots_buf);
        let page = page.min(Self::page_count(count) - 1);
        let mut drawn = [None; PER_PAGE];
        for (column, &slot) in slots_buf[..count]
            .iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .enumerate()
        {
            drawn[column] = Some(DrawnStrip {
                mic: network.slots[slot].mic.load(Ordering::Acquire),
                rect: Self::strip_rect(column),
            });
        }
        drawn
    }
    pub(super) fn bypass_at(&self, point: (f32, f32)) -> Option<BypassButton> {
        self.bypass_buttons
            .get()
            .into_iter()
            .flatten()
            .find(|button| point_in_rect(point.0, point.1, button.rect))
    }
    pub(super) fn toggle_bypass_at(&self, point: (f32, f32)) -> bool {
        let Some(button) = self.bypass_at(point) else {
            return false;
        };
        if let Some(params) = self.network.slots[button.slot].editor_params(button.owner) {
            params.toggle_strip_bypass();
        }
        true
    }
    pub(super) fn change_page(&mut self, forward: bool) {
        let mut slots_buf = [0usize; CAPACITY];
        let count = Self::collect_slots(&self.network, &mut slots_buf);
        let last = Self::page_count(count) - 1;
        let current = self.page.min(last);
        let next = if forward {
            (current + 1).min(last)
        } else {
            current.saturating_sub(1)
        };
        if next != current {
            self.bypass_buttons.set([None; PER_PAGE]);
            self.drawn_strips
                .set(Self::compute_drawn_strips(&self.network, next));
        }
        self.page = next;
    }
    pub(super) fn strip_at(&self, point: (f32, f32)) -> Option<DrawnStrip> {
        self.drawn_strips
            .get()
            .into_iter()
            .flatten()
            .find(|strip| point_in_rect(point.0, point.1, strip.rect))
    }
    pub(super) fn control_at(&self, point: (f32, f32)) -> Option<(Control, Rect)> {
        for (i, id) in GLOBAL.into_iter().enumerate() {
            let rect = Self::global_rect(i);
            if point_in_rect(point.0, point.1, rect) {
                return Some((id, rect));
            }
        }
        if point_in_rect(point.0, point.1, LOCAL) {
            return Some((Control::Mic, LOCAL));
        }
        for strip in self.drawn_strips.get().into_iter().flatten() {
            let rect = Self::weight_rect(strip.rect);
            if point_in_rect(point.0, point.1, rect) {
                return Some((Control::Weight(strip.mic), rect));
            }
        }
        None
    }
    pub(super) fn group_at(&self, point: (f32, f32)) -> Option<usize> {
        self.drawn_strips
            .get()
            .into_iter()
            .flatten()
            .find_map(|strip| {
                point_in_rect(point.0, point.1, Self::group_rect(strip.rect)).then_some(strip.mic)
            })
    }
    pub(super) fn finish_group(&mut self, cx: &mut EventContext, point: (f32, f32)) {
        let Some(mic) = self.group_press.take() else {
            return;
        };
        if self.group_at(point) != Some(mic) {
            return;
        }
        let next = if self.params.shared.mics[mic].is_voice() {
            1.0
        } else {
            0.0
        };
        param::set_normalized_once(cx, self.params.shared.mics[mic].group.as_ptr(), next);
    }
    fn display_name(&self, mic: usize, host_name: Option<&str>) -> String {
        let fallback = format!("MIC {:02}", mic + 1);
        let alias = self.params.shared.mics[mic]
            .name
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if alias.is_empty() || *alias == fallback {
            host_name.map(str::to_owned).unwrap_or(fallback)
        } else {
            alias.clone()
        }
    }
    fn paint_bypass(d: &mut Draw, rect: Rect, pressed: bool) {
        let color = if pressed { GOLD } else { TEXT };
        d.button_surface(
            rect,
            GOLD,
            pressed as u8 as f32,
            d.is_hovered(rect) as u8 as f32,
            0.0,
        );
        let x = rect.0 + rect.2 * 0.5;
        let y = rect.1 + rect.3 * 0.5 - 1.0;
        d.power_icon(x - 23.0, y, color);
        d.text_middle(x - 12.0, y, "BYPASS", 11.0, color);
    }
    fn paint_page_button(d: &mut Draw, rect: Rect, forward: bool, enabled: bool) {
        let x = rect.0 + rect.2 * 0.5;
        let y = rect.1 + rect.3 * 0.5 - 1.0;
        let direction = if forward { 1.0 } else { -1.0 };
        if enabled {
            d.button_surface(rect, TEXT, 0.0, d.is_hovered(rect) as u8 as f32, 0.0);
        }
        d.poly(
            &[
                (x - 2.0 * direction, y - 5.0),
                (x + 3.0 * direction, y),
                (x - 2.0 * direction, y + 5.0),
            ],
            if enabled { TEXT } else { MUTED },
            1.5,
        );
    }
    pub(super) fn paint(&self, d: &mut Draw) {
        d.hover = if self.drag.is_none() && self.edit.is_none() {
            self.hover
        } else {
            None
        };
        let p = &self.params.shared;
        d.rect(0.0, 0.0, SIZE.0, 72.0, PANEL);
        d.text(32.0, 46.0, "AUTOMIXER", 25.0, GOLD);
        d.appearance_button(THEME, prefs().label());
        d.text(32.0, 88.0, "ALL MICROPHONES", 10.0, MUTED);
        let labels = [
            "AUTOMIX",
            "VOICE GR",
            "OTHER GR",
            "CHORUS > N",
            "THRESH",
            "HOLD",
            "FADE",
            "VOICE ASSIST",
        ];
        for (index, id) in GLOBAL.into_iter().enumerate() {
            self.paint_control(d, id, Self::global_rect(index), labels[index]);
        }
        self.paint_control(d, Control::Mic, LOCAL, "THIS MIC");
        let monitor = self.network.monitor(self.network.now_ms());
        let active = monitor.active;
        let chorus = monitor.chorus;
        let mut slots_buf = [0usize; CAPACITY];
        let registered = Self::collect_slots(&self.network, &mut slots_buf);
        let slots = &slots_buf[..registered];
        d.text(
            32.0,
            199.0,
            &format!(
                "{registered} LINKED   {} IN MIX   {active} ACTIVE",
                monitor.members
            ),
            14.0,
            TEXT,
        );
        let state: std::borrow::Cow<'static, str> = if p.amount.value() == 0.0 {
            "AUTOMIX OFF".into()
        } else if monitor.members == 0 {
            "NO MICS IN MIX".into()
        } else if chorus > 0.001 {
            format!("CHORUS FADE  {:3.0}%", chorus * 100.0).into()
        } else {
            "GAIN SHARING".into()
        };
        d.text(
            398.0,
            199.0,
            &state,
            13.0,
            if chorus > 0.0 { TEAL } else { GOLD },
        );
        let pages = Self::page_count(registered);
        let page = self.page.min(pages - 1);
        Self::paint_page_button(d, PREVIOUS, false, page > 0);
        Self::paint_page_button(d, NEXT, true, page + 1 < pages);
        d.text_centered(
            1016.0,
            197.0,
            &format!("{} / {pages}", page + 1),
            12.0,
            TEXT,
        );
        let now = self.network.now_ms();
        let conflicts = self.network.conflict_mask();
        let mut bypass_buttons = [None; PER_PAGE];
        let mut drawn_strips = [None; PER_PAGE];
        for (column, &slot_index) in slots
            .iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .enumerate()
        {
            let r = Self::strip_rect(column);
            let slot = &self.network.slots[slot_index];
            let owner = slot.owner.load(Ordering::Acquire);
            let params = slot.editor_params(owner);
            let strip_bypassed = params
                .as_ref()
                .is_some_and(|params| params.strip_bypassed());
            let host_bypassed = params.as_ref().is_some_and(|params| params.bypass.value());
            let mic = slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
            let labels = slot
                .labels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let local = Some(slot_index) == self.local_slot;
            let updated = slot.updated_ms.load(Ordering::Relaxed);
            let fresh = updated != 0 && now.saturating_sub(updated) < 750;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(r, if self.selected == mic { GOLD } else { LINE });
            let name = self.display_name(mic, labels.name.as_deref());
            let center = r.0 + r.2 * 0.5;
            d.text_centered(
                center,
                236.0,
                &format!(
                    "MIC {:02}{}",
                    mic + 1,
                    if local { " · THIS MIC" } else { "" }
                ),
                9.0,
                if local { GOLD } else { MUTED },
            );
            d.text_centered(
                center,
                258.0,
                &name.chars().take(12).collect::<String>(),
                13.0,
                TEXT,
            );
            if let Some(input) = &labels.input {
                d.text_centered(
                    center,
                    276.0,
                    &format!("IN {:02}", input.first_channel + 1),
                    10.0,
                    MUTED,
                );
            }
            let state = slot.state.load(Ordering::Relaxed);
            let has_meter = fresh && state != network::WAITING;
            let (status, color) = if host_bypassed {
                ("HOST BYPASS", GOLD)
            } else if strip_bypassed {
                ("BYPASSED", GOLD)
            } else if conflicts & (1 << mic) != 0 {
                ("ID CONFLICT", GOLD)
            } else if !has_meter {
                ("NO AUDIO", MUTED)
            } else {
                match state {
                    network::BYPASSED => ("BYPASSED", GOLD),
                    network::CHORUS => ("CHORUS", TEAL),
                    network::QUIET => ("QUIET", MUTED),
                    _ => ("ACTIVE", TEAL),
                }
            };
            d.text_centered(center, 297.0, status, 10.0, color);
            let input = if has_meter {
                slot.input_db.load(Ordering::Relaxed)
            } else {
                -120.0
            };
            let output = if has_meter {
                slot.output_db.load(Ordering::Relaxed)
            } else {
                -120.0
            };
            let gr = if has_meter {
                slot.reduction_db.load(Ordering::Relaxed)
            } else {
                0.0
            };
            let cap = if p.mics[mic].is_voice() {
                p.cap.value()
            } else {
                p.other_cap.value()
            };
            let gr_norm = (gr / cap.max(1.0)).clamp(0.0, 1.0);
            for (column, label, value, normalized, color) in [
                (
                    0,
                    "IN",
                    input,
                    ((input + 60.0) / 60.0).clamp(0.0, 1.0),
                    MUTED,
                ),
                (
                    1,
                    "OUT",
                    output,
                    ((output + 60.0) / 60.0).clamp(0.0, 1.0),
                    TEAL,
                ),
                (2, "GR", gr, gr_norm, GOLD),
            ] {
                let x = r.0 + 22.0 + column as f32 * 40.0;
                let height = normalized * 70.0;
                d.text_centered(x, 318.0, label, 9.0, MUTED);
                d.rect(x - 6.0, 328.0, 12.0, 70.0, LINE);
                if height > 0.0 {
                    let top = if column == 2 { 328.0 } else { 398.0 - height };
                    d.rect(x - 6.0, top, 12.0, height, color);
                }
                if has_meter {
                    d.text_centered(x, 412.0, &format!("{:.0}", value.max(-99.0)), 10.0, color);
                } else {
                    d.text_centered(x, 412.0, "–", 10.0, MUTED);
                }
            }
            let voice = p.mics[mic].is_voice();
            let group_label: std::borrow::Cow<'static, str> = if !voice {
                "OTHER".into()
            } else if p.assist.value() > 0.0 && !strip_bypassed && !host_bypassed {
                if !has_meter || !slot.vad_ready.load(Ordering::Relaxed) {
                    "VOICE WAIT".into()
                } else {
                    format!("VOICE {:3.0}%", slot.speech.load(Ordering::Relaxed) * 100.0).into()
                }
            } else {
                "VOICE".into()
            };
            d.button(
                Self::group_rect(r),
                &group_label,
                true,
                if voice { TEAL } else { GOLD },
            );
            let rect = (r.0 + 8.0, 455.0, r.2 - 16.0, 29.0);
            if params.is_some() {
                bypass_buttons[column] = Some(BypassButton {
                    slot: slot_index,
                    owner,
                    rect,
                });
            }
            drawn_strips[column] = Some(DrawnStrip {
                mic,
                rect: r,
            });
            Self::paint_bypass(d, rect, strip_bypassed);
            self.paint_control(d, Control::Weight(mic), Self::weight_rect(r), "WEIGHT");
        }
        self.bypass_buttons.set(bypass_buttons);
        self.drawn_strips.set(drawn_strips);
        if registered == 0 {
            d.text(50.0, 260.0, "NO INSTANCES", 14.0, MUTED);
        }
        let selected = &p.mics[self.selected];
        let labels = self
            .network
            .slots
            .iter()
            .find(|slot| {
                slot.owner.load(Ordering::Acquire) != 0
                    && slot.mic.load(Ordering::Acquire) == self.selected
            })
            .map(|slot| {
                slot.labels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
            })
            .unwrap_or_default();
        let name = self.display_name(self.selected, labels.name.as_deref());
        if let Some(input) = &labels.input {
            d.text(
                32.0,
                675.0,
                &input.label.chars().take(140).collect::<String>(),
                11.0,
                MUTED,
            );
        }
        d.text(
            32.0,
            576.0,
            &format!(
                "SELECTED  {:02}  {}  ·  {}",
                self.selected + 1,
                name.chars().take(48).collect::<String>(),
                if selected.is_voice() { "VOICE" } else { "OTHER" }
            ),
            12.0,
            GOLD,
        );
        let mic = self.selected.min(CAPACITY - 1);
        self.paint_dropdown(
            d,
            Self::channel_rect(),
            "MIDI CHANNEL",
            &channel_label(selected.midi_channel.value()),
            matches!(self.menu, Some(OpenMenu { kind: MenuKind::Channel(open), .. }) if open == mic),
        );
        self.paint_dropdown(
            d,
            Self::cc_rect(),
            "WEIGHT CC",
            &selected.midi_cc.value().to_string(),
            matches!(self.menu, Some(OpenMenu { kind: MenuKind::Cc(open), .. }) if open == mic),
        );
        let warning = if self.local_slot.is_none() {
            Some("CAPACITY REACHED · 64 INSTANCES MAXIMUM")
        } else if conflicts != 0 {
            Some("MIC ID CONFLICT · ASSIGN A UNIQUE THIS MIC NUMBER")
        } else {
            None
        };
        if let Some(warning) = warning {
            d.text(32.0, 706.0, warning, 11.0, GOLD);
        }
        d.line(32.0, 722.0, 1088.0, 722.0, LINE, 1.0);
        d.text(32.0, 742.0, self.tooltip(), 10.0, MUTED);
        self.paint_menu(d);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
    }
    fn paint_control(&self, d: &mut Draw, id: Control, rect: Rect, label: &str) {
        let normalized = self.normalized(id);
        let value = if id == Control::Mic && self.params.mic.value() == 0 {
            format!("Auto {:02}", self.params.mic_index() + 1)
        } else {
            self.parameter(id)
                .text(normalized, !matches!(id, Control::Weight(_)))
        };
        if matches!(id, Control::Weight(_)) {
            d.control_bipolar(rect, label, &value, normalized, GOLD);
        } else {
            d.control(
                rect,
                label,
                &value,
                normalized,
                if id == Control::Amount { GOLD } else { TEAL },
            );
        }
        if self
            .hover
            .is_some_and(|p| point_in_rect(p.0, p.1, Self::value_rect(rect)))
        {
            d.value_underline(Self::value_rect(rect), rect.1 + 18.0, GOLD);
        }
    }
}
