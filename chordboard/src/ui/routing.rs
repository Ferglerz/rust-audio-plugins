use super::*;
use crate::engine::routing::{SOURCES, TARGETS};

#[derive(Clone, Copy)]
pub(super) struct RouteDrag {
    pub(super) source: usize,
    origin: (f32, f32),
    pub(super) active: bool,
}

impl RouteDrag {
    pub(super) fn new(source: usize, x: f32, y: f32) -> Self {
        Self {
            source,
            origin: (x, y),
            active: false,
        }
    }
    pub(super) fn update(&mut self, x: f32, y: f32) {
        self.active |= (x - self.origin.0).hypot(y - self.origin.1) >= 6.0;
    }
}

impl ChordboardView {
    pub(super) fn toggle_modulator(&mut self, source: usize) {
        if self.panel == Some(Panel::Routes) && self.selected_modulator == Some(source) {
            self.set_panel(None);
            return;
        }
        if let Some(slot) = self
            .params
            .routes
            .iter()
            .rposition(|p| p.source.value() as usize == source + 1)
        {
            self.route_slot = slot;
        } else if let Some(slot) = self
            .params
            .routes
            .iter()
            .position(|p| p.source.value() == 0)
        {
            self.route_slot = slot;
        }
        self.set_panel(Some(Panel::Routes));
        self.selected_modulator = Some(source);
        self.modulator_pulse = 0.0;
    }

    pub(super) fn editor_route_chips(&self) -> Vec<(usize, Rect)> {
        self.params
            .routes
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.source.value() != 0
                    && self
                        .selected_modulator
                        .is_none_or(|s| p.source.value() as usize == s + 1)
            })
            .enumerate()
            .map(|(position, (slot, _))| (slot, route_slot_rect(position)))
            .collect()
    }
    pub(super) fn has_selected_route(&self) -> bool {
        let source = self.params.routes[self.route_slot].source.value();
        source != 0
            && self
                .selected_modulator
                .is_none_or(|s| source as usize == s + 1)
    }
    pub(super) fn add_route_rect(&self) -> Option<Rect> {
        self.params
            .routes
            .iter()
            .any(|p| p.source.value() == 0)
            .then(|| route_slot_rect(self.editor_route_chips().len()))
    }
    pub(super) fn modulation_route(&self, id: &str) -> Option<usize> {
        self.params.routes.iter().rposition(|p| {
            let r = p.route();
            r.active() && TARGETS[r.target as usize].id == id
        })
    }
    pub(super) fn modulation_hint(&self, id: &str) -> Option<String> {
        let slot = self.modulation_route(id)?;
        let r = self.params.routes[slot].route();
        Some(format!(
            "{} is controlled by {} · Edit or remove its route to change it manually",
            TARGETS[r.target as usize].name, SOURCES[r.source as usize]
        ))
    }
    pub(super) fn explain_modulation(&mut self, id: &str) -> bool {
        let Some(slot) = self.modulation_route(id) else {
            return false;
        };
        self.status = self.modulation_hint(id).unwrap_or_default();
        self.route_hover = Some(self.params.routes[slot].source.value() as usize - 1);
        self.action_flash = Some((
            meter_rect(self.params.routes[slot].source.value() as usize - 1),
            1.0,
        ));
        true
    }
    pub(super) fn route_remove_rect(r: Rect) -> Rect {
        (r.0 + r.2 - 32.0, r.1 + 3.0, 28.0, r.3 - 6.0)
    }
    pub(super) fn press_route_overlay(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        for (slot, r) in self.route_chips() {
            if !hit(r, x, y) {
                continue;
            }
            if self.pending_route_remove == Some(slot) {
                if hit(Self::route_remove_rect(r), x, y) {
                    Self::emit(cx, self.params.routes[slot].source.as_ptr(), 0.0);
                    self.pending_route_remove = None;
                    self.status = "Route removed".into();
                }
                return true;
            }
            if hit(Self::route_remove_rect(r), x, y) {
                self.pending_route_remove = Some(slot);
                return true;
            }
        }
        false
    }
    pub(super) fn route_chips(&self) -> Vec<(usize, Rect)> {
        let Some(source) = self
            .selected_modulator
            .or(self.route_hover)
            .filter(|_| self.drag.is_none() && self.menu.is_none())
        else {
            return Vec::new();
        };
        let slots: Vec<_> = self
            .params
            .routes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.source.value() as usize == source + 1)
            .map(|(i, _)| i)
            .collect();
        let columns = slots.len().clamp(1, 3);
        let width = columns as f32 * 158.0 - 6.0;
        let meter = meter_rect(source);
        let x = (meter.0 + meter.2 / 2.0 - width / 2.0).clamp(16.0, W - width - 16.0);
        slots
            .into_iter()
            .enumerate()
            .map(|(i, slot)| {
                (
                    slot,
                    (
                        x + (i % columns) as f32 * 158.0,
                        meter.1 + meter.3 + 6.0 + (i / columns) as f32 * 30.0,
                        152.0,
                        26.0,
                    ),
                )
            })
            .collect()
    }
    pub(super) fn update_route_hover(&mut self, x: f32, y: f32) {
        if let Some(source) =
            (0..crate::engine::routing::SOURCE_COUNT).find(|&i| hit(meter_rect(i), x, y))
        {
            self.route_hover = Some(source);
            return;
        }
        if let (Some(source), Some((_, last))) = (self.route_hover, self.route_chips().last()) {
            let chips = self.route_chips();
            let meter = meter_rect(source);
            let left = chips.iter().map(|(_, r)| r.0).fold(meter.0, f32::min);
            let right = chips
                .iter()
                .map(|(_, r)| r.0 + r.2)
                .fold(meter.0 + meter.2, f32::max);
            if hit(
                (left, meter.1, right - left, last.1 + last.3 - meter.1),
                x,
                y,
            ) {
                return;
            }
        }
        self.route_hover = None;
    }
    pub(super) fn route_chip_at(&self, x: f32, y: f32) -> Option<usize> {
        self.route_chips()
            .into_iter()
            .find(|(_, r)| hit(*r, x, y))
            .map(|(slot, _)| slot)
    }
    pub(super) fn draw_route_chips(&self, d: &mut Draw) {
        for (slot, r) in self.route_chips() {
            let route = self.params.routes[slot].route();
            let color = if route.enabled { TEAL } else { MUTED };
            d.rounded_rect(r.0, r.1, r.2, r.3, 5.0, BG);
            d.outline_rounded(r.0, r.1, r.2, r.3, 5.0, alpha(color, 0.65), 1.0);
            let label = self.fit_text(
                d,
                if self.pending_route_remove == Some(slot) {
                    "Are you sure?"
                } else {
                    TARGETS[route.target as usize].name
                },
                r.2 - 36.0,
                11.0,
            );
            if self.pointer.is_some_and(|(x, y)| hit(r, x, y)) {
                d.rounded_rect(r.0, r.1, r.2, r.3, 5.0, alpha(color, 0.12));
            }
            d.text_centered(r.0 + (r.2 - 28.0) / 2.0, r.1 + 17.0, &label, 11.0, color);
            self.close_icon(
                d,
                Self::route_remove_rect(r),
                color,
                self.pending_route_remove == Some(slot),
            );
        }
    }

    // Share visible control geometry with painting. Covered controls cannot
    // receive a drop, and routing never changes a parameter on mouse-down.
    pub(super) fn route_targets(&self) -> Vec<(usize, Rect)> {
        if self.menu.is_some() || self.edit.is_some() {
            return Vec::new();
        }
        self.visible_route_targets()
    }

    fn visible_route_targets(&self) -> Vec<(usize, Rect)> {
        if self.expand_t() > 0.0 {
            return Vec::new();
        }
        let mut targets: Vec<_> = self
            .base_controls()
            .into_iter()
            .map(|(c, r)| (c.id, r))
            .collect();
        targets.extend([
            ("latch", LATCH),
            ("voice_leading", PIANO_LEADING),
            ("spread", PIANO_KEYS),
            (
                "inversion",
                (
                    inversion_control_rect(self.params.mpe_enabled(), 0).0,
                    ALWAYS_BASS.1,
                    64.0,
                    28.0,
                ),
            ),
            ("transpose", TRANSPOSE),
            ("mode", (PAD.0, 132.0, PAD.2, 24.0)),
        ]);
        match self.mode() {
            1 | 3 => {
                targets.push(("octaves", (octave_rect(0).0, octave_rect(0).1, 124.0, 28.0)));
                targets.push((
                    "arp_pattern",
                    (
                        pattern_rect(0).0,
                        pattern_rect(0).1,
                        PAD.2 - 16.0,
                        pattern_rect(0).3,
                    ),
                ));
                targets.push(("strum_sync", RATE_SYNC));
                if self.mode() == 1 {
                    targets.push(("strum_hold", STRUM_HOLD));
                }
                if self.sweep_synced() {
                    targets.push((
                        "rate",
                        (
                            rate_rect(0).0,
                            RATE_HEADER.1,
                            PAD.2 - 16.0,
                            rate_rect(0).1 + rate_rect(0).3 - RATE_HEADER.1,
                        ),
                    ));
                }
            }
            2 => targets.push(("x", self.play_pad())),
            _ => {}
        }
        if self.page_elapsed < pleasant_ui::page_slide::DURATION {
            targets.retain(|(_, r)| r.0 < PAD.0 || r.1 >= PAD.1 + PAD.3);
        }
        if self.route_progress > 0.0 && self.panel != Some(Panel::Routes) {
            targets.retain(|(_, r)| !hit(CHORDS_SURFACE, r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
        }
        if let Some(panel) = self.panel {
            let p = self.panel_rect(panel);
            targets.retain(|(_, r)| {
                r.0 + r.2 <= p.0 || r.0 >= p.0 + p.2 || r.1 + r.3 <= p.1 || r.1 >= p.1 + p.3
            });
            targets.extend(self.panel_controls().into_iter().map(|(c, r)| (c.id, r)));
            if panel == Panel::Mapping && self.mapping_axis == 1 {
                targets.push(("y_target", self.menu_trigger_rect(Menu::YTarget)));
            }
        }
        targets
            .into_iter()
            .filter_map(|(id, r)| {
                TARGETS
                    .iter()
                    .position(|t| t.available() && t.id == id)
                    .map(|i| (i, r))
            })
            .collect()
    }

    pub(super) fn route_target_at(&self, x: f32, y: f32) -> Option<usize> {
        self.route_targets()
            .into_iter()
            .find(|(_, r)| hit(*r, x, y))
            .map(|(i, _)| i)
    }

    fn route_for_target(&self, target: usize) -> Option<usize> {
        // Match the engine's last-active-route priority; reuse that link instead
        // of stacking new, competing assignments onto the same destination.
        self.params
            .routes
            .iter()
            .rposition(|p| p.route().active() && p.target.value() as usize == target)
            .or_else(|| {
                self.params
                    .routes
                    .iter()
                    .rposition(|p| p.source.value() > 0 && p.target.value() as usize == target)
            })
    }

    pub(super) fn finish_route_drag(
        &mut self,
        cx: &mut EventContext,
        source: usize,
        x: f32,
        y: f32,
    ) {
        let Some(target) = self.route_target_at(x, y) else {
            return;
        };
        if !TARGETS[target].accepts_source(source as u8 + 1) {
            self.status = "A controller cannot modulate its own setup".into();
            return;
        }
        let existing = self.route_for_target(target);
        let slot = existing.or_else(|| {
            self.params
                .routes
                .iter()
                .position(|p| p.source.value() == 0)
        });
        let Some(slot) = slot else {
            self.status = "All 16 routes are assigned · Clear a route to add another".into();
            return;
        };
        self.route_slot = slot;
        let route = &self.params.routes[slot];
        if existing.is_none() {
            Self::emit(
                cx,
                route.target.as_ptr(),
                route.target.preview_normalized(target as i32),
            );
            Self::emit(cx, route.min.as_ptr(), 0.0);
            Self::emit(cx, route.max.as_ptr(), 1.0);
            Self::emit(cx, route.curve.as_ptr(), 0.5);
        }
        if !route.enabled.value() {
            Self::emit(cx, route.enabled.as_ptr(), 1.0);
        }
        if route.source.value() as usize != source + 1 {
            Self::emit(
                cx,
                route.source.as_ptr(),
                route.source.preview_normalized(source as i32 + 1),
            );
        }
        self.status.clear();
        self.set_panel(Some(Panel::Routes));
        self.selected_modulator = Some(source);
    }

    pub(super) fn route_drag_hint(&self) -> Option<String> {
        let Some(Drag::Route(drag)) = self.drag else {
            return None;
        };
        let source = SOURCES[drag.source + 1];
        let target = self
            .pointer
            .and_then(|(x, y)| self.route_target_at(x, y))
            .filter(|&t| TARGETS[t].accepts_source(drag.source as u8 + 1));
        Some(if let Some(target) = target.filter(|_| drag.active) {
            let action = self.route_for_target(target).map_or("Link", |slot| {
                if self.params.routes[slot].source.value() as usize == drag.source + 1 {
                    "Edit"
                } else {
                    "Replace link with"
                }
            });
            format!("{action} {source} → {}", TARGETS[target].name)
        } else {
            format!("Drag {source} to a highlighted control · Drop elsewhere to cancel")
        })
    }

    pub(super) fn draw_route_drag(&self, d: &mut Draw) {
        if self.selected_modulator.is_none()
            && self.drag.is_some()
            && !matches!(self.drag, Some(Drag::Route(_)))
        {
            return;
        }
        let (x, y) = self.pointer.unwrap_or((-1.0, -1.0));
        let drag = if let Some(Drag::Route(drag)) = self.drag {
            Some(drag)
        } else {
            None
        };
        let source = drag
            .map(|d| d.source)
            .or(self.selected_modulator)
            .or(self.route_hover);
        let Some(source) = source else {
            return;
        };
        let active = drag.is_some_and(|d| d.active);
        for (target, r) in self.visible_route_targets() {
            if !TARGETS[target].accepts_source(source as u8 + 1) {
                continue;
            }
            let linked = self.params.routes.iter().any(|p| {
                p.route().active()
                    && p.source.value() as usize == source + 1
                    && p.target.value() as usize == target
            });
            if !active && !linked {
                continue;
            }
            let hovered = active && hit(r, x, y);
            d.rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(TEAL, if hovered { 0.16 } else { 0.035 }),
            );
            d.outline_rounded(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(TEAL, if hovered || linked { 0.9 } else { 0.35 }),
                if hovered { 2.0 } else { 1.0 },
            );
            if linked && !active {
                let origin = meter_rect(source);
                self.draw_route_connection(
                    d,
                    route_anchor(origin, r.1 + r.3 * 0.5),
                    route_anchor(r, origin.1 + origin.3 * 0.5),
                );
            }
            if active && TARGETS[target].id == "x" {
                d.text_centered(r.0 + r.2 / 2.0, r.1 + r.3 / 2.0, "STRUMFIELD", 13.0, TEAL);
            }
        }
        if !active {
            return;
        }
        let r = meter_rect(source);
        let start = route_anchor(r, y);
        self.draw_route_connection(d, start, (x, y));
        if let Some(label) = self.route_drag_hint() {
            let label = if self.route_target_at(x, y).is_some() {
                label
            } else {
                SOURCES[source + 1].into()
            };
            let w = (self.text_width(d, &label, 11.0) + 24.0).min(W - 32.0);
            let bx = (x - w / 2.0).clamp(16.0, W - w - 16.0);
            let by = (y - 38.0).max(8.0);
            d.rounded_rect(bx, by, w, 26.0, 4.0, BG);
            d.outline_rounded(bx, by, w, 26.0, 4.0, alpha(TEAL, 0.8), 1.0);
            d.text_centered(bx + w / 2.0, by + 17.0, &label, 11.0, TEAL);
        }
    }

    fn draw_route_connection(&self, d: &mut Draw, start: (f32, f32), end: (f32, f32)) {
        let bend = ((end.1 - start.1).abs() * 0.4).clamp(24.0, 180.0)
            * if end.1 >= start.1 { 1.0 } else { -1.0 };
        let points: Vec<_> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let u = 1.0 - t;
                (
                    u.powi(3) * start.0
                        + 3.0 * u * u * t * start.0
                        + 3.0 * u * t * t * end.0
                        + t.powi(3) * end.0,
                    u.powi(3) * start.1
                        + 3.0 * u * u * t * (start.1 + bend)
                        + 3.0 * u * t * t * (end.1 - bend)
                        + t.powi(3) * end.1,
                )
            })
            .collect();
        let pulse = if self.selected_modulator.is_some() {
            0.6 + 0.4 * self.modulator_pulse.sin().abs()
        } else {
            0.75
        };
        d.poly(&points, alpha(TEAL, pulse), 1.5 + pulse);
        d.circle(start.0, start.1, 3.0, TEAL, true);
        d.circle(end.0, end.1, 4.0, TEAL, false);
    }

    pub(super) fn route_menu_allowed(&self, menu: Menu, index: usize) -> bool {
        let route = self.params.routes[self.route_slot].route();
        match menu {
            Menu::RouteSource => {
                index == 0 || TARGETS[route.target as usize].accepts_source(index as u8)
            }
            Menu::RouteTarget => crate::engine::routing::available_targets()
                .nth(index)
                .is_some_and(|(_, t)| t.accepts_source(route.source)),
            _ => true,
        }
    }

    pub(super) fn route_nodes(&self) -> [(f32, f32); 3] {
        let route = self.params.routes[self.route_slot].route();
        let (x, y, w, h) = ROUTE_GRAPH;
        [
            (x, y + h * (1.0 - route.min)),
            (x + w / 2.0, y + h * (1.0 - route.value(0.5))),
            (x + w, y + h * (1.0 - route.max)),
        ]
    }
    pub(super) fn route_node_value(&self, node: u8, y: f32) -> Option<f32> {
        let value = (1.0 - (y - ROUTE_GRAPH.1) / ROUTE_GRAPH.3).clamp(0.0, 1.0);
        if node != 1 {
            return Some(value);
        }
        let route = self.params.routes[self.route_slot].route();
        let span = route.max - route.min;
        if span.abs() < 0.0001 {
            return None;
        }
        let fraction = ((value - route.min) / span).clamp(0.00001, 0.99999);
        let curve = (fraction.ln() / 0.5_f32.ln()).log2() / 3.0;
        Some(
            self.params.routes[self.route_slot]
                .curve
                .preview_normalized(curve.clamp(-1.0, 1.0)),
        )
    }

    pub(super) fn route_contour_range(
        origin: (f32, f32),
        point: (f32, f32),
        min: f32,
        max: f32,
        fine: bool,
        axis: &mut pleasant_ui::pointer::AxisLock,
    ) -> (f32, f32) {
        let up_sign = if origin.0 < ROUTE_GRAPH.0 + ROUTE_GRAPH.2 * 0.5 {
            -1.0
        } else {
            1.0
        };
        let delta = axis.delta(point.0 - origin.0, point.1 - origin.1, up_sign) / ROUTE_GRAPH.2
            * 2.0
            * if fine { 0.1 } else { 1.0 };
        let center = (min + max) * 0.5;
        let limit = center.min(1.0 - center) * 2.0;
        let span = (max - min + delta).clamp(-limit, limit);
        (center - span * 0.5, center + span * 0.5)
    }

    pub(super) fn press_routes(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if !self.has_selected_route() && (hit(ROUTE_GRAPH, x, y) || hit(ROUTE_LINEAR, x, y)) {
            return true;
        }
        if hit(ROUTE_LINEAR, x, y) {
            Self::emit(cx, self.params.routes[self.route_slot].curve.as_ptr(), 0.5);
            return true;
        }
        for (node, (nx, ny)) in self.route_nodes().into_iter().enumerate() {
            if (x - nx).hypot(y - ny) <= 12.0 {
                self.end_drag(cx);
                let route = &self.params.routes[self.route_slot];
                let ptr = match node {
                    0 => route.min.as_ptr(),
                    2 => route.max.as_ptr(),
                    _ => route.curve.as_ptr(),
                };
                cx.emit(RawParamEvent::BeginSetParameter(ptr));
                self.drag = Some(Drag::RouteNode(node as u8, ptr));
                cx.capture();
                return true;
            }
        }
        if hit(ROUTE_GRAPH, x, y) {
            self.end_drag(cx);
            let route = &self.params.routes[self.route_slot];
            cx.emit(RawParamEvent::BeginSetParameter(route.min.as_ptr()));
            cx.emit(RawParamEvent::BeginSetParameter(route.max.as_ptr()));
            self.drag = Some(Drag::RouteContour {
                origin: (x, y),
                min: route.min.value(),
                max: route.max.value(),
                slot: self.route_slot,
                axis: pleasant_ui::pointer::AxisLock::default(),
            });
            cx.capture();
            return true;
        }
        for (slot, r) in self.editor_route_chips() {
            if hit(r, x, y) {
                self.end_drag(cx);
                self.route_slot = slot;
                self.edit = None;
                return true;
            }
        }
        if self.add_route_rect().is_some_and(|r| hit(r, x, y)) {
            if let Some(slot) = self
                .params
                .routes
                .iter()
                .position(|p| p.source.value() == 0)
            {
                self.route_slot = slot;
                let route = &self.params.routes[slot];
                if let Some(source) = self.selected_modulator {
                    Self::emit(
                        cx,
                        route.source.as_ptr(),
                        route.source.preview_normalized(source as i32 + 1),
                    );
                }
                Self::emit(cx, route.enabled.as_ptr(), 1.0);
                self.open_menu(if self.selected_modulator.is_some() {
                    Menu::RouteTarget
                } else {
                    Menu::RouteSource
                });
            }
            return true;
        }
        if !self.has_selected_route() {
            return true;
        }
        for menu in [Menu::RouteSource, Menu::RouteTarget] {
            if hit(menu.trigger_rect(), x, y) {
                self.open_menu(menu);
                return true;
            }
        }
        if hit(ROUTE_CLEAR, x, y) {
            let route = &self.params.routes[self.route_slot];
            Self::emit(cx, route.source.as_ptr(), 0.0);
            Self::emit(cx, route.enabled.as_ptr(), 1.0);
            Self::emit(cx, route.min.as_ptr(), 0.0);
            Self::emit(cx, route.max.as_ptr(), 1.0);
            Self::emit(cx, route.curve.as_ptr(), 0.5);
            return true;
        }
        false
    }

    pub(super) fn draw_routes(&self, d: &mut Draw) {
        let route = self.params.routes[self.route_slot].route();
        let target = &TARGETS[route.target as usize];
        d.font = self.font.get();
        d.text(
            Panel::Routes.rect().0 + 16.0,
            module_title_y(Panel::Routes.rect().1, MODULE_TITLE_SIZE),
            "MODULATION",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        if let Some(source) = self.selected_modulator {
            d.text(
                Panel::Routes.rect().0 + 120.0,
                module_title_y(Panel::Routes.rect().1, MODULE_TITLE_SIZE),
                &format!("· {} ROUTES", SOURCES[source + 1].to_uppercase()),
                MODULE_TITLE_SIZE,
                TEAL,
            );
        }
        d.font = self.ui_font.get();
        self.button(d, ROUTE_CLEAR, "Clear", false, MUTED);
        for (slot, r) in self.editor_route_chips() {
            let assigned = self.params.routes[slot].route();
            self.button(
                d,
                r,
                TARGETS[assigned.target as usize].name,
                slot == self.route_slot,
                if assigned.enabled { TEAL } else { MUTED },
            );
        }
        if let Some(r) = self.add_route_rect() {
            self.button(d, r, "+ Add route", false, TEAL);
        }
        if !self.has_selected_route() {
            d.text(
                44.0,
                304.0,
                "Add a route to connect this modulator to a destination",
                12.0,
                MUTED,
            );
            return;
        }
        d.text_centered(470.0, ROUTE_SOURCE.1 + 18.0, "→", 12.0, MUTED);
        self.button(
            d,
            ROUTE_SOURCE,
            &format!("{} ▾", SOURCES[route.source as usize]),
            false,
            TEAL,
        );
        self.button(d, ROUTE_TARGET, &format!("{} ▾", target.name), false, GOLD);
        let (x, y, w, h) = ROUTE_GRAPH;
        d.rect(x, y, w, h, BG);
        d.outline(ROUTE_GRAPH, LINE);
        for fraction in [0.25, 0.5, 0.75] {
            d.line(
                x + w * fraction,
                y,
                x + w * fraction,
                y + h,
                alpha(LINE, 0.5),
                1.0,
            );
            d.line(
                x,
                y + h * fraction,
                x + w,
                y + h * fraction,
                alpha(LINE, 0.5),
                1.0,
            );
        }
        let points: Vec<_> = (0..=48)
            .map(|i| {
                let input = i as f32 / 48.0;
                (x + w * input, y + h * (1.0 - route.value(input)))
            })
            .collect();
        d.poly(&points, GOLD, 2.0);
        d.text_centered(x + w * 0.5, y - 10.0, "CONTOUR", TEXT_SMALL, MUTED);
        for (i, (nx, ny)) in self.route_nodes().into_iter().enumerate() {
            d.circle(nx, ny, 6.0, BG, true);
            d.circle(nx, ny, 5.0, if i == 1 { GOLD } else { TEAL }, false);
        }
        d.text(
            x,
            y - 10.0,
            &format!("Min {}", target.label(route.min)),
            11.0,
            TEAL,
        );
        d.text_right(
            x + w,
            y - 10.0,
            &format!("Max {}", target.label(route.max)),
            11.0,
            TEAL,
        );
        self.button(d, ROUTE_LINEAR, "Linear ↺", route.curve.abs() < 0.001, GOLD);
        if route.active() {
            let input = if route.source == 7 && target.uses_input_velocity() {
                self.snapshot.input_velocity
            } else {
                self.snapshot.sources[route.source as usize - 1]
            };
            d.circle(
                x + w * input,
                y + h * (1.0 - route.value(input)),
                2.5,
                TEAL,
                true,
            );
            d.text(
                44.0,
                484.0,
                &format!(
                    "In {:.0}% → {}",
                    input * 100.0,
                    target.label(route.value(input))
                ),
                11.0,
                MUTED,
            );
        } else {
            d.text(
                44.0,
                484.0,
                "Drag contour to tilt · Endpoints set range · Center bends curve",
                11.0,
                MUTED,
            );
        }
        let overridden = self
            .params
            .routes
            .iter()
            .enumerate()
            .skip(self.route_slot + 1)
            .find(|(_, p)| {
                let other = p.route();
                other.active() && other.target == route.target
            });
        let help = if let Some((i, _)) = overridden.filter(|_| route.active()) {
            format!("Route {} has priority for this destination", i + 1)
        } else if route.min > route.max {
            "Reversed range".into()
        } else {
            "Drag contour to tilt · Endpoints set range · Center bends curve".into()
        };
        d.text(44.0, 501.0, &help, 11.0, MUTED);
    }

    /// A translucent upper half denotes the live choice; the normal underline
    /// continues to denote the saved choice, including when both coincide.
    pub(super) fn live_highlight(&self, d: &mut Draw, r: Rect) {
        d.rounded_rect(r.0, r.1, r.2, r.3 * 0.5, 3.0, alpha(TEAL, 0.22));
        d.line(r.0 + 3.0, r.1 + 1.0, r.0 + r.2 - 3.0, r.1 + 1.0, TEAL, 2.0);
    }

    pub(super) fn live_choice(&self, d: &mut Draw, id: &str, value: f32, r: Rect) {
        if self
            .routed_plain(id)
            .is_some_and(|live| (live - value).abs() < 0.0001)
        {
            self.live_highlight(d, r);
        }
    }

    pub(super) fn routed_plain(&self, id: &str) -> Option<f32> {
        let (i, target) = TARGETS.iter().enumerate().find(|(_, t)| t.id == id)?;
        self.snapshot.routed[i].map(|v| target.plain(v))
    }
    pub(super) fn mode(&self) -> i32 {
        self.playback_mode()
    }
    pub(super) fn playback_mode(&self) -> i32 {
        self.routed_plain("mode")
            .map_or(self.params.mode.value(), |v| v as i32)
    }
    pub(super) fn routed_control(&self, c: &Control) -> Option<Control> {
        let (i, target) = TARGETS.iter().enumerate().find(|(_, t)| t.id == c.id)?;
        let norm = self.snapshot.routed[i]?;
        let mut routed = c.clone();
        // Route space is linear in destination units, while the control may
        // use a skewed parameter range. Keep both markers on the same scale.
        // The view owns the parameters referenced by this pointer.
        let plain = target.plain(norm)
            + if matches!(c.id, "bass_channel" | "upper_channel" | "output_channel") {
                1.0
            } else {
                0.0
            };
        routed.norm = unsafe { c.ptr.preview_normalized(plain) };
        routed.value = target.label(norm);
        Some(routed)
    }
}

// Wires always leave and enter the edge facing the other endpoint.
pub(super) fn route_anchor(r: Rect, toward_y: f32) -> (f32, f32) {
    (
        r.0 + r.2 / 2.0,
        if toward_y < r.1 + r.3 / 2.0 {
            r.1
        } else {
            r.1 + r.3
        },
    )
}
