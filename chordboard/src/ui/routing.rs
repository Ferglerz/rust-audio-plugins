use super::*;
use crate::engine::routing::{ROUTE_COUNT, SOURCES, TARGETS};

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
    // Share visible control geometry with painting. Covered controls cannot
    // receive a drop, and routing never changes a parameter on mouse-down.
    pub(super) fn route_targets(&self) -> Vec<(usize, Rect)> {
        if self.expand_t() > 0.0 || self.menu.is_some() || self.edit.is_some() {
            return Vec::new();
        }
        let mut targets: Vec<_> = self
            .base_controls()
            .into_iter()
            .map(|(c, r)| (c.id, r))
            .collect();
        targets.extend([
            ("latch", LATCH),
            ("inversion", (32.0, 610.0, 252.0, 48.0)),
            ("transpose", (326.0, 610.0, 222.0, 48.0)),
            ("mode", (600.0, 122.0, 488.0, 28.0)),
        ]);
        match self.mode() {
            1 => {
                targets.push(("direction", (616.0, 344.0, 222.0, 50.0)));
                targets.push(("strum_sync", STRUM_SYNC));
                if self.params.strum_sync.value() {
                    targets.push(("strum_beats", STRUM_RATE));
                }
            }
            2 => {
                targets.push(("x", self.play_pad()));
            }
            3 => targets.extend([
                ("arp_pattern", (618.0, 194.0, 452.0, 42.0)),
                ("rate", (618.0, 240.0, 452.0, 46.0)),
                ("octaves", (618.0, 302.0, 202.0, 26.0)),
            ]),
            _ => {}
        }
        if self.page_elapsed < pleasant_ui::page_slide::DURATION {
            targets.retain(|(_, r)| r.0 < PAD.0);
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
            .filter_map(|(id, r)| TARGETS.iter().position(|t| t.available() && t.id == id).map(|i| (i, r)))
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
    }

    pub(super) fn route_drag_hint(&self) -> Option<String> {
        let Some(Drag::Route(drag)) = self.drag else {
            return None;
        };
        let source = SOURCES[drag.source + 1];
        let target = self.pointer.and_then(|(x, y)| self.route_target_at(x, y));
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
        let Some((x, y)) = self.pointer else {
            return;
        };
        let drag = if let Some(Drag::Route(drag)) = self.drag {
            Some(drag)
        } else {
            None
        };
        let source = drag.map(|d| d.source).or_else(|| {
            (0..crate::engine::routing::SOURCE_COUNT).find(|&i| hit(meter_rect(i), x, y))
        });
        let Some(source) = source else {
            return;
        };
        let active = drag.is_some_and(|d| d.active);
        for (target, r) in self.route_targets() {
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
            if active && TARGETS[target].id == "x" {
                d.text_centered(
                    r.0 + r.2 / 2.0,
                    r.1 + r.3 / 2.0,
                    "STRUM X / SWEEP",
                    13.0,
                    TEAL,
                );
            }
        }
        if !active {
            return;
        }
        let r = meter_rect(source);
        let start = (r.0 + r.2 / 2.0, r.1 + 2.0);
        let rise = ((y - start.1).abs() * 0.45).clamp(35.0, 150.0);
        let points: Vec<_> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let u = 1.0 - t;
                (
                    u.powi(3) * start.0
                        + 3.0 * u * u * t * start.0
                        + 3.0 * u * t * t * x
                        + t.powi(3) * x,
                    u.powi(3) * start.1
                        + 3.0 * u * u * t * (start.1 - rise)
                        + 3.0 * u * t * t * (y + rise)
                        + t.powi(3) * y,
                )
            })
            .collect();
        d.poly(&points, alpha(TEAL, 0.75), 1.5);
        d.circle(start.0, start.1, 3.0, TEAL, true);
        d.circle(x, y, 4.0, TEAL, false);
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

    pub(super) fn press_routes(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
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
        for i in 0..ROUTE_COUNT {
            if hit(route_slot_rect(i), x, y) {
                self.end_drag(cx);
                self.route_slot = i;
                self.edit = None;
                return true;
            }
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
            692.0,
            244.0,
            &format!("ROUTE {}", self.route_slot + 1),
            13.0,
            TEAL,
        );
        self.button(d, ROUTE_CLEAR, "Clear", false, MUTED);
        for i in 0..ROUTE_COUNT {
            let r = route_slot_rect(i);
            self.button(d, r, &(i + 1).to_string(), i == self.route_slot, TEAL);
            if self.params.routes[i].route().active() {
                d.circle(r.0 + r.2 - 6.0, r.1 + 6.0, 1.6, TEAL, true);
            }
        }
        d.text_centered(875.0, 334.0, "→", 12.0, MUTED);
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
        d.poly(&points, GOLD, 1.5);
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
            let input = self.snapshot.sources[route.source as usize - 1];
            d.circle(
                x + w * input,
                y + h * (1.0 - route.value(input)),
                2.5,
                TEAL,
                true,
            );
            d.text(
                692.0,
                490.0,
                &format!(
                    "In {:.0}% → {}",
                    input * 100.0,
                    target.label(route.value(input))
                ),
                11.0,
                MUTED,
            );
        } else {
            d.text(692.0, 490.0, "Drag endpoints or center node", 11.0, MUTED);
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
            "".into()
        };
        d.text(692.0, 510.0, &help, 11.0, MUTED);
    }

    pub(super) fn routed_plain(&self, id: &str) -> Option<f32> {
        let (i, target) = TARGETS.iter().enumerate().find(|(_, t)| t.id == id)?;
        self.snapshot.routed[i].map(|v| target.plain(v))
    }
    pub(super) fn mode(&self) -> i32 {
        self.routed_plain("mode")
            .map_or(self.params.mode.value(), |v| v as i32)
    }
    pub(super) fn routed_control(&self, c: &Control) -> Option<Control> {
        let (i, target) = TARGETS.iter().enumerate().find(|(_, t)| t.id == c.id)?;
        let norm = self.snapshot.routed[i]?;
        let mut routed = c.clone();
        routed.norm = norm;
        routed.value = target.label(norm);
        Some(routed)
    }
}
