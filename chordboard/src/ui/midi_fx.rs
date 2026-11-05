//! Global FX page and reusable vertical module stack. Delay controls use local module geometry.
use super::*;
use crate::midi_fx::MidiFxId;

pub(super) const PERFORMANCE_TAB: Rect = (750.0, 24.0, 132.0, 30.0);
pub(super) const FX_TAB: Rect = (890.0, 24.0, 100.0, 30.0);
pub(super) const VIEWPORT: Rect = (16.0, 92.0, W - 32.0, H - 114.0);
const GRIP_W: f32 = 28.0;
const MODULE_H: f32 = 360.0;
const GAP: f32 = 12.0;
/// Canonical content bounds. Drawing and hit-testing apply the same stack offset.
pub(super) const DELAY_SURFACE: Rect = (
    VIEWPORT.0 + GRIP_W,
    VIEWPORT.1,
    VIEWPORT.2 - GRIP_W,
    MODULE_H,
);

#[derive(Default)]
pub(super) struct FxUi {
    pub open: bool,
    scroll: f32,
    pub drag: Option<ModuleDrag>,
}
#[derive(Clone, Copy)]
pub(super) struct ModuleDrag {
    from: usize,
    to: usize,
    origin: (f32, f32),
    pointer: (f32, f32),
    active: bool,
}
fn module_rect(index: usize, scroll: f32) -> Rect {
    (
        VIEWPORT.0,
        VIEWPORT.1 + index as f32 * (MODULE_H + GAP) - scroll,
        VIEWPORT.2,
        MODULE_H,
    )
}
fn grip_rect(index: usize, scroll: f32) -> Rect {
    let r = module_rect(index, scroll);
    (r.0, r.1, GRIP_W, r.3)
}
fn max_scroll(count: usize) -> f32 {
    (count as f32 * (MODULE_H + GAP) - GAP - VIEWPORT.3).max(0.0)
}
impl ModuleDrag {
    fn update(&mut self, x: f32, y: f32, scroll: f32, count: usize) {
        self.pointer = (x, y);
        self.active |= (x - self.origin.0).hypot(y - self.origin.1) >= 4.0;
        if self.active {
            self.to = (((y - VIEWPORT.1 + scroll) / (MODULE_H + GAP))
                .floor()
                .max(0.0) as usize)
                .min(count.saturating_sub(1));
        }
    }
}
impl FxUi {
    fn tick(&mut self, dt: f32, count: usize) -> bool {
        let before = self.scroll;
        self.scroll = before.clamp(0.0, max_scroll(count));
        if let Some(mut drag) = self.drag {
            if drag.from >= count {
                self.drag = None;
                return true;
            }
            if drag.active {
                let y = drag.pointer.1;
                let edge = 36.0;
                let speed = if y < VIEWPORT.1 + edge {
                    -300.0
                } else if y > VIEWPORT.1 + VIEWPORT.3 - edge {
                    300.0
                } else {
                    0.0
                };
                self.scroll = (self.scroll + speed * dt).clamp(0.0, max_scroll(count));
                drag.update(drag.pointer.0, y, self.scroll, count);
                self.drag = Some(drag);
            }
        }
        before != self.scroll
    }
}
impl ChordboardView {
    fn delay_stack_offset(&self) -> f32 {
        let state = self.params.arpeggiator.snapshot();
        let index = state
            .midi_fx
            .order
            .iter()
            .position(|id| *id == MidiFxId::Delay)
            .unwrap_or(0);
        index as f32 * (MODULE_H + GAP) - self.fx_ui.scroll
    }
    pub(super) fn fx_delay_local_y(&self, y: f32) -> f32 {
        y - self.delay_stack_offset()
    }
    pub(super) fn fx_delay_screen_rect(&self, r: Rect) -> Rect {
        (r.0, r.1 + self.delay_stack_offset(), r.2, r.3)
    }
    fn set_fx_page(&mut self, cx: &mut EventContext, open: bool) {
        if self.fx_ui.open == open {
            return;
        }
        self.end_pad_hover(cx);
        self.end_drag(cx);
        self.restore_shift_latch(cx);
        self.cancel_arp_editor();
        self.fx_ui.drag = None;
        self.tracker_drag = None;
        self.edit = None;
        self.menu = None;
        // set_panel() releases keyboard notes; page navigation must retain them.
        self.panel = None;
        self.route_progress = 0.0;
        self.route_start = 0.0;
        self.route_elapsed = pleasant_ui::page_slide::DURATION;
        self.memory_ui.armed = false;
        self.pending_route_remove = None;
        self.selected_modulator = None;
        self.button_press_origin = None;
        self.fx_ui.open = open;
        cx.release();
        cx.needs_redraw();
    }
    pub(super) fn draw_page_navigation(&self, d: &mut Draw) {
        self.button(d, PERFORMANCE_TAB, "Performance", !self.fx_ui.open, TEAL);
        self.button(d, FX_TAB, "MIDI FX", self.fx_ui.open, TEAL);
    }
    pub(super) fn draw_midi_fx(&self, d: &mut Draw) {
        let state = self.params.arpeggiator.snapshot();
        d.scissor(VIEWPORT.0, VIEWPORT.1, VIEWPORT.2, VIEWPORT.3);
        for (index, effect) in state.midi_fx.order.iter().enumerate() {
            let r = module_rect(index, self.fx_ui.scroll);
            if r.1 + r.3 < VIEWPORT.1 || r.1 > VIEWPORT.1 + VIEWPORT.3 {
                continue;
            }
            self.surface(d, r);
            let grip = grip_rect(index, self.fx_ui.scroll);
            let hot = self.pointer.is_some_and(|(x, y)| hit(grip, x, y));
            let moving = self
                .fx_ui
                .drag
                .is_some_and(|drag| drag.active && drag.from == index);
            let color = if hot || moving { TEAL } else { MUTED };
            let mut y = grip.1 + 10.0;
            while y < grip.1 + grip.3 - 8.0 {
                for x in [grip.0 + 10.0, grip.0 + 18.0] {
                    d.circle(x, y, 1.4, color, true);
                }
                y += 8.0;
            }
            d.line(
                grip.0 + GRIP_W,
                r.1 + 8.0,
                grip.0 + GRIP_W,
                r.1 + r.3 - 8.0,
                LINE,
                1.0,
            );
            let oy = d.oy;
            let hover = d.hover;
            let offset = r.1 - DELAY_SURFACE.1;
            d.oy += offset * d.s;
            d.hover = hover.map(|(x, y)| (x, y - offset));
            match effect {
                MidiFxId::Delay => self.draw_delay_strings(d),
            }
            d.oy = oy;
            d.hover = hover;
        }
        if let Some(drag) = self.fx_ui.drag.filter(|drag| drag.active) {
            let r = module_rect(drag.to, self.fx_ui.scroll);
            let y = if drag.to > drag.from {
                r.1 + r.3 + GAP * 0.5
            } else {
                r.1 + 2.0
            };
            d.line(r.0, y, r.0 + r.2, y, TEAL, 2.0);
        }
        d.reset_scissor();
    }
    pub(super) fn tick_midi_fx(&mut self, dt: f32) -> bool {
        if !self.fx_ui.open {
            return false;
        }
        let count = self.params.arpeggiator.snapshot().midi_fx.order.len();
        self.fx_ui.tick(dt, count)
    }
    pub(super) fn midi_fx_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
        if press && (hit(PERFORMANCE_TAB, x, y) || hit(FX_TAB, x, y)) {
            self.set_fx_page(cx, hit(FX_TAB, x, y));
            return true;
        }
        if !self.fx_ui.open || self.tracker_drag.is_some() {
            return false;
        }
        if matches!(
            event,
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseDown(_)
        ) {
            self.pointer = Some((x, y));
            cx.needs_redraw();
        }
        // A held musical key still needs its release while a module or tap is dragged.
        if matches!(event, WindowEvent::KeyUp(_, _)) {
            return false;
        }
        let count = self.params.arpeggiator.snapshot().midi_fx.order.len();
        if let Some(mut drag) = self.fx_ui.drag {
            match event {
                WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::FocusOut => {
                    self.fx_ui.drag = None;
                    cx.release();
                }
                WindowEvent::MouseMove(_, _) => {
                    drag.update(x, y, self.fx_ui.scroll, count);
                    self.fx_ui.drag = Some(drag);
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    drag.update(x, y, self.fx_ui.scroll, count);
                    if drag.active && drag.from != drag.to {
                        self.params
                            .arpeggiator
                            .edit(|s| s.midi_fx.move_effect(drag.from, drag.to));
                    }
                    self.fx_ui.drag = None;
                    cx.release();
                }
                WindowEvent::MouseScroll(_, _) | WindowEvent::MouseDown(_) => {}
                _ => return false,
            }
            cx.needs_redraw();
            return true;
        }
        if press && hit(VIEWPORT, x, y) {
            if let Some(index) = (0..count).find(|&i| hit(grip_rect(i, self.fx_ui.scroll), x, y)) {
                self.cancel_arp_editor();
                self.fx_ui.drag = Some(ModuleDrag {
                    from: index,
                    to: index,
                    origin: (x, y),
                    pointer: (x, y),
                    active: false,
                });
                cx.capture();
                cx.focus();
                self.focused = true;
                return true;
            }
        }
        // A captured value gesture continues beyond the viewport; ordinary clicks do not.
        let local_y = self.fx_delay_local_y(y);
        if (self.arp_editor_dragging()
            || hit(VIEWPORT, x, y)
            || matches!(event, WindowEvent::KeyDown(Code::Escape, _)))
            && self.arp_editor_event(cx, event, x, local_y)
        {
            return true;
        }
        if let WindowEvent::MouseScroll(_, dy) = event {
            if hit(VIEWPORT, x, y) {
                self.fx_ui.scroll = (self.fx_ui.scroll - dy * 24.0).clamp(0.0, max_scroll(count));
                cx.needs_redraw();
                return true;
            }
        }
        // Prevent invisible performance controls receiving pointer events, even in empty space.
        (x >= 0.0 || hit(PIANO_SURFACE, x, y))
            && y >= HEADER_H
            && matches!(
                event,
                WindowEvent::MouseDown(_)
                    | WindowEvent::MouseUp(_)
                    | WindowEvent::MouseMove(_, _)
                    | WindowEvent::MouseDoubleClick(_)
                    | WindowEvent::MouseTripleClick(_)
                    | WindowEvent::MouseScroll(_, _)
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(dead_code)]
    mod support {
        include!("tests/support.rs");
    }
    use support::*;

    #[test]
    fn midi_fx_grip_spans_module_height_and_does_not_overlap_controls() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        let grip = grip_rect(0, 0.0);
        for (r, _) in view.arp_editor_button_rects() {
            assert!(r.0 > grip.0 + grip.2);
            assert!(r.1 >= DELAY_SURFACE.1 && r.1 + r.3 <= DELAY_SURFACE.1 + DELAY_SURFACE.3);
        }
        for y in [grip.1 + 1.0, grip.1 + grip.3 * 0.5, grip.1 + grip.3 - 1.0] {
            event(
                &mut view,
                &mut cx,
                target,
                grip.0 + 14.0,
                y,
                WindowEvent::MouseDown(MouseButton::Left),
            );
            assert!(view.fx_ui.drag.is_some());
            event(
                &mut view,
                &mut cx,
                target,
                grip.0 + 14.0,
                y,
                WindowEvent::KeyDown(Code::Escape, None),
            );
            assert!(view.fx_ui.drag.is_none());
        }
        click(&mut view, &mut cx, target, arpeggiator::delay_power_rect());
        assert!(view.fx_ui.drag.is_none());
        assert!(view.params.arpeggiator.snapshot().midi_fx.delay.enabled);
    }

    #[test]
    fn midi_fx_single_module_drag_and_focus_loss_preserve_settings() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        let before = view.params.arpeggiator.snapshot().midi_fx;
        let x = VIEWPORT.0 + 14.0;
        let y = VIEWPORT.1 + 20.0;
        for finish in [
            WindowEvent::MouseUp(MouseButton::Left),
            WindowEvent::FocusOut,
        ] {
            event(
                &mut view,
                &mut cx,
                target,
                x,
                y,
                WindowEvent::MouseDown(MouseButton::Left),
            );
            event(
                &mut view,
                &mut cx,
                target,
                x,
                y + 2.0,
                WindowEvent::MouseMove(x, y + 2.0),
            );
            assert!(!view.fx_ui.drag.unwrap().active);
            event(
                &mut view,
                &mut cx,
                target,
                x,
                y + 500.0,
                WindowEvent::MouseMove(x, y + 500.0),
            );
            assert!(view.fx_ui.drag.unwrap().active);
            event(&mut view, &mut cx, target, x, y + 500.0, finish);
            assert!(view.fx_ui.drag.is_none());
            assert_eq!(view.params.arpeggiator.snapshot().midi_fx, before);
        }
    }

    #[test]
    fn midi_fx_synthetic_stack_reorders_payloads_and_clamps_scroll_and_drop() {
        let mut modules = vec![(11, "delay"), (22, "future effect"), (33, "another effect")];
        let mut drag = ModuleDrag {
            from: 0,
            to: 0,
            origin: (30.0, 110.0),
            pointer: (30.0, 110.0),
            active: false,
        };
        drag.update(30.0, 112.0, 0.0, modules.len());
        assert!(!drag.active);
        drag.update(30.0, 110.0 + 2.0 * (MODULE_H + GAP), 0.0, modules.len());
        assert_eq!(drag.to, 2);
        crate::midi_fx::move_item(&mut modules, drag.from, drag.to);
        assert_eq!(
            modules,
            vec![(22, "future effect"), (33, "another effect"), (11, "delay")]
        );
        drag.from = 2;
        drag.update(30.0, -100.0, 0.0, modules.len());
        crate::midi_fx::move_item(&mut modules, drag.from, drag.to);
        assert_eq!(modules[0], (11, "delay"));
        assert_eq!(max_scroll(1), 0.0);
        assert!(max_scroll(8) > 0.0);
        let last = module_rect(7, max_scroll(8));
        assert!((last.1 + last.3 - VIEWPORT.1 - VIEWPORT.3).abs() < 0.01);
        drag.update(30.0, 10000.0, max_scroll(8), 8);
        assert_eq!(drag.to, 7);
        let mut ui = FxUi {
            open: true,
            scroll: 0.0,
            drag: Some(drag),
        };
        assert!(ui.tick(0.1, 8));
        assert_eq!(ui.scroll, 30.0);
        ui.tick(100.0, 8);
        assert_eq!(ui.scroll, max_scroll(8));
        ui.drag.as_mut().unwrap().pointer.1 = VIEWPORT.1;
        ui.tick(100.0, 8);
        assert_eq!(ui.scroll, 0.0);
    }

    #[test]
    fn midi_fx_navigation_preserves_held_keyboard_notes_and_midi_state() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        view.focused = true;
        event(
            &mut view,
            &mut cx,
            target,
            0.0,
            0.0,
            WindowEvent::KeyDown(Code::Digit1, None),
        );
        assert!(view.pressed[0]);
        while view.bridge.commands.pop().is_some() {}
        click(&mut view, &mut cx, target, FX_TAB);
        click(&mut view, &mut cx, target, PERFORMANCE_TAB);
        assert!(view.pressed[0]);
        assert!(view.bridge.commands.is_empty());
        event(
            &mut view,
            &mut cx,
            target,
            0.0,
            0.0,
            WindowEvent::KeyUp(Code::Digit1, None),
        );
        assert!(!view.pressed[0]);
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::KeyUp(0))
        ));
        event(
            &mut view,
            &mut cx,
            target,
            0.0,
            0.0,
            WindowEvent::KeyDown(Code::Digit1, None),
        );
        while view.bridge.commands.pop().is_some() {}
        click(&mut view, &mut cx, target, FX_TAB);
        event(
            &mut view,
            &mut cx,
            target,
            VIEWPORT.0 + 14.0,
            VIEWPORT.1 + 14.0,
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(view.fx_ui.drag.is_some());
        event(
            &mut view,
            &mut cx,
            target,
            VIEWPORT.0 + 14.0,
            VIEWPORT.1 + 14.0,
            WindowEvent::KeyUp(Code::Digit1, None),
        );
        assert!(!view.pressed[0]);
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::KeyUp(0))
        ));
    }
}
