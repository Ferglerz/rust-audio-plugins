//! Global FX page and reusable vertical module stack. Delay controls use local module geometry.
use super::*;
use crate::midi_fx::{HarmonizerMode, MidiFxId, MidiFxState};

mod eq;

pub(super) const PROGRAMMING_TAB: Rect = (W * 0.5 - 270.0, 0.0, 180.0, HEADER_H);
pub(super) const PROGRAMMING_TRANSPORT: Rect = (PROGRAMMING_TAB.0 + 12.0, (HEADER_H - 28.0) * 0.5, 28.0, 28.0);
pub(super) const PERFORMANCE_TAB: Rect = (W * 0.5 - 90.0, 0.0, 180.0, HEADER_H);
pub(super) const FX_LEARN: Rect = (APPEARANCE.0 - 94.0, APPEARANCE.1, 84.0, APPEARANCE.3);
pub(super) const FX_TAB: Rect = (W * 0.5 + 90.0, 0.0, 180.0, HEADER_H);
pub(super) const VIEWPORT: Rect = (16.0, 92.0, W - 32.0, MODULE_BOTTOM - 92.0);
const GRIP_W: f32 = 28.0;
const GAP: f32 = 12.0;
pub(super) const SOURCE_COLUMN_W: f32 = 174.0;
const SOURCE_NAMES: [&str; 5] = ["Arp", "Strum", "Chords", "Bass", "Melody"];
const RATES: [f32; 9] = [0.0625, 0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0, 2.0, 4.0];
const RATE_NAMES: [&str; 9] = ["1/64", "1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4", "1/2", "1/1"];

pub(super) fn source_rect(index: usize) -> Rect {
    (DELAY_SURFACE.0 + DELAY_SURFACE.2 - SOURCE_COLUMN_W + 12.0,
     DELAY_SURFACE.1 + 58.0 + index as f32 * 36.0, SOURCE_COLUMN_W - 24.0, 32.0)
}
fn fx_screen_rect((x, y, w, h): Rect, offset: f32) -> Rect {
    (x, y + offset, w, h)
}
fn control_rect(column: usize, row: usize) -> Rect {
    (DELAY_SURFACE.0 + 20.0 + column as f32 * 214.0,
     DELAY_SURFACE.1 + 58.0 + row as f32 * 48.0, 200.0, 32.0)
}
fn gate_step_rect(index: usize, count: usize) -> Rect {
    let width = (DELAY_SURFACE.2 - SOURCE_COLUMN_W - 48.0) / count as f32;
    (DELAY_SURFACE.0 + 20.0 + index as f32 * width, DELAY_SURFACE.1 + 154.0,
     width - 4.0, 108.0)
}
fn rate_index(rate: f32) -> usize {
    RATES.iter().enumerate().min_by(|(_, a), (_, b)|
        (**a - rate).abs().total_cmp(&(**b - rate).abs())).map_or(3, |(i, _)| i)
}
#[derive(Clone, Copy)]
enum FxValue {
    HarmonyInterval(usize), HarmonyVelocity(usize), GateSteps, GateRate, GateLength,
    GatePulses, GateRotation, GateStep(usize), SplitChannel(usize), SplitPool(usize),
}
#[derive(Clone, Copy)]
struct ValueDrag {
    value: FxValue,
    origin_y: f32,
    start: f32,
    moved: bool,
}

/// Delay content bounds. The other modules share its local origin and width.
pub(super) const DELAY_SURFACE: Rect = (
    VIEWPORT.0 + GRIP_W,
    VIEWPORT.1,
    VIEWPORT.2 - GRIP_W,
    268.0,
);

#[derive(Default)]
pub(super) struct FxUi {
    pub open: bool,
    collapsed: Vec<MidiFxId>,
    scroll: f32,
    pub drag: Option<ModuleDrag>,
    value_drag: Option<ValueDrag>,
    eq: eq::Editor,
}
#[derive(Clone, Copy)]
pub(super) struct ModuleDrag {
    from: usize,
    to: usize,
    origin: (f32, f32),
    pointer: (f32, f32),
    active: bool,
}
fn collapse_rect() -> Rect {
    (DELAY_SURFACE.0 + DELAY_SURFACE.2 - 40.0, DELAY_SURFACE.1 + 8.0, 28.0, 28.0)
}
impl FxUi {
    fn height(&self, effect: MidiFxId) -> f32 {
        if self.collapsed.contains(&effect) {
            return MODULE_HEADER_H;
        }
        match effect {
            MidiFxId::Harmonizer | MidiFxId::VoiceSplitter => 246.0,
            MidiFxId::RhythmGate => 274.0,
            MidiFxId::Delay => DELAY_SURFACE.3,
            MidiFxId::MidiEq => 312.0,
        }
    }
    fn rect(&self, index: usize, order: &[MidiFxId]) -> Rect {
        let offset: f32 = order.iter().take(index).map(|id| self.height(*id) + GAP).sum();
        (VIEWPORT.0, VIEWPORT.1 + offset - self.scroll, VIEWPORT.2, self.height(order[index]))
    }
    fn grip(&self, index: usize, order: &[MidiFxId]) -> Rect {
        let r = self.rect(index, order);
        (r.0, r.1, GRIP_W, r.3)
    }
    fn limit(&self, order: &[MidiFxId]) -> f32 {
        (order.iter().map(|id| self.height(*id) + GAP).sum::<f32>() - GAP - VIEWPORT.3).max(0.0)
    }
    fn tick(&mut self, dt: f32, order: &[MidiFxId]) -> bool {
        let before = self.scroll;
        self.scroll = before.clamp(0.0, self.limit(order));
        if let Some(mut drag) = self.drag {
            if drag.from >= order.len() { self.drag = None; return true; }
            if drag.active {
                let y = drag.pointer.1;
                let speed = if y < VIEWPORT.1 + 36.0 { -300.0 }
                    else if y > VIEWPORT.1 + VIEWPORT.3 - 36.0 { 300.0 } else { 0.0 };
                self.scroll = (self.scroll + speed * dt).clamp(0.0, self.limit(order));
                drag.update_layout(drag.pointer.0, y, self, order);
                self.drag = Some(drag);
            }
        }
        before != self.scroll
    }
}
impl ModuleDrag {
    fn update_layout(&mut self, x: f32, y: f32, ui: &FxUi, order: &[MidiFxId]) {
        self.pointer = (x, y);
        self.active |= (x - self.origin.0).hypot(y - self.origin.1) >= 4.0;
        if self.active {
            self.to = (0..order.len()).rfind(|&i| y >= ui.rect(i, order).1).unwrap_or(0);
        }
    }
    #[cfg(test)]
    fn update(&mut self, x: f32, y: f32, scroll: f32, count: usize) {
        self.update_layout(x, y, &FxUi { scroll, ..FxUi::default() }, &vec![MidiFxId::Delay; count]);
    }
}
#[cfg(test)]
fn module_rect(index: usize, scroll: f32) -> Rect {
    FxUi { scroll, ..FxUi::default() }.rect(index, &vec![MidiFxId::Delay; index + 1])
}
#[cfg(test)]
fn grip_rect(index: usize, scroll: f32) -> Rect {
    let r = module_rect(index, scroll);
    (r.0, r.1, GRIP_W, r.3)
}
#[cfg(test)]
fn max_scroll(count: usize) -> f32 {
    FxUi::default().limit(&vec![MidiFxId::Delay; count])
}
impl ChordboardView {
    fn effect_stack_offset(&self, effect: MidiFxId) -> f32 {
        let state = self.arp_state();
        let index = state
            .midi_fx
            .order
            .iter()
            .position(|id| *id == effect)
            .expect("MidiFxState::order always contains every effect");
        self.fx_ui.rect(index, &state.midi_fx.order).1 - VIEWPORT.1
    }
    pub(super) fn fx_delay_expanded(&self) -> bool {
        !self.fx_ui.collapsed.contains(&MidiFxId::Delay)
    }
    pub(super) fn fx_delay_local_y(&self, y: f32) -> f32 {
        y - self.effect_stack_offset(MidiFxId::Delay)
    }
    pub(super) fn fx_delay_screen_rect(&self, r: Rect) -> Rect {
        fx_screen_rect(r, self.effect_stack_offset(MidiFxId::Delay))
    }

    pub(super) fn delay_footer_hovered(&self) -> bool {
        self.fx_ui.open
            && self.fx_delay_expanded()
            && self.hover_pointer().is_some_and(|(x, y)| {
                hit(VIEWPORT, x, y) && hit(DELAY_SURFACE, x, self.fx_delay_local_y(y))
            })
    }
    pub(super) fn set_fx_page(&mut self, cx: &mut EventContext, open: bool, programming: bool) {
        if self.fx_ui.open == open && self.programming == programming {
            return;
        }
        self.end_pad_hover(cx);
        self.end_drag(cx);
        self.restore_shift_latch(cx);
        self.cancel_arp_editor();
        self.fx_ui.drag = None;
        self.fx_ui.value_drag = None;
        self.fx_ui.eq.drag = None;
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
        self.top_slide.select(self.programming as u64, usize::from(!self.programming));
        self.top_slide.select(programming as u64, usize::from(!programming));
        if self.tracker_symbol_edit {
            self.finish_tracker_symbol();
        }
        self.tracker_skip_line_scroll = false;
        self.programming = programming;
        self.fx_ui.open = open;
        self.custom_drag = None;
        self.refresh_arp_editor_params();
        cx.release();
        cx.needs_redraw();
    }
    pub(super) fn toggle_fx_power(&self, effect: MidiFxId) {
        self.toggle_learnable_power(crate::midi_learn::fx_power_target(effect), |s, enabled| {
            s.midi_fx.set_enabled(effect, enabled);
        });
    }

    pub(super) fn fx_midi_targets(&self) -> Vec<(usize, Rect)> {
        let order = self.arp_state().midi_fx.order.clone();
        order.iter().enumerate().filter_map(|(index, &effect)| {
            let module = self.fx_ui.rect(index, &order);
            let (x, y, w, h) = delay_editor::delay_power_rect();
            let y = y + module.1 - DELAY_SURFACE.1;
            // A clipped switch is unavailable until it can be seen in full.
            (y >= VIEWPORT.1 && y + h <= VIEWPORT.1 + VIEWPORT.3)
                .then(|| (crate::midi_learn::fx_power_target(effect), (x, y, w, h)))
        }).collect()
    }

    pub(super) fn draw_fx_learn_button(&self, d: &mut Draw) {
        if self.quick_midi.show {
            self.icon_button(d, FX_LEARN, "", true, GOLD, style::Icon::Glyph(style::GLYPH_CHECK));
        } else {
            self.icon_button(d, FX_LEARN, "", false, GOLD, style::Icon::MidiDin);
        }
    }

    pub(super) fn draw_page_navigation(&self, d: &mut Draw) {
        for (r, label, active) in [
            (PROGRAMMING_TAB, "Programming", self.programming && !self.fx_ui.open),
            (PERFORMANCE_TAB, "Performance", !self.programming && !self.fx_ui.open),
            (FX_TAB, "MIDI FX", self.fx_ui.open),
        ] {
            d.rect(r.0, r.1, r.2, r.3, alpha(TEAL, if active { 0.22 } else { 0.04 + self.hover_amount(r) * 0.08 }));
            d.line(r.0, r.1, r.0, r.1 + r.3, LINE, 1.0);
            if active { d.rect(r.0, r.1 + r.3 - 4.0, r.2, 4.0, TEAL); }
            let label_offset = if r == PROGRAMMING_TAB { 18.0 } else { 0.0 };
            d.text_centered(r.0 + r.2 * 0.5 + label_offset, r.1 + r.3 * 0.5 + 5.0, label, 16.0, if active { TEAL } else { TEXT });
        }
        self.draw_tracker_transport(d, PROGRAMMING_TRANSPORT);
    }
    pub(super) fn draw_midi_fx(&self, d: &mut Draw) {
        let mut state = (*self.arp_state()).clone();
        for effect in crate::midi_fx::EFFECTS {
            let target = crate::midi_learn::fx_power_target(effect);
            state.midi_fx.set_enabled(effect, crate::midi_learn::value(&self.params, target) != 0);
        }
        d.scissor(VIEWPORT.0, VIEWPORT.1, VIEWPORT.2, VIEWPORT.3);
        let dimmed = d.dimmed;
        for (index, effect) in state.midi_fx.order.iter().enumerate() {
            let r = self.fx_ui.rect(index, &state.midi_fx.order);
            if r.1 + r.3 < VIEWPORT.1 || r.1 > VIEWPORT.1 + VIEWPORT.3 {
                continue;
            }
            let enabled = state.midi_fx.enabled(*effect);
            d.dimmed = dimmed || (!enabled && *effect != MidiFxId::Delay);
            self.surface(d, r);
            let grip = self.fx_ui.grip(index, &state.midi_fx.order);
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
            d.hover = hover
                .filter(|&(x, y)| hit(VIEWPORT, x, y))
                .map(|(x, y)| (x, y - offset));
            let collapsed = self.fx_ui.collapsed.contains(effect);
            if *effect == MidiFxId::Delay {
                d.with_readable_style(self.delay_label_font.get(), |d| {
                    if collapsed {
                        self.draw_delay_heading(d, enabled);
                    } else {
                        self.draw_delay_strings(d, &state.midi_fx.delay);
                        self.draw_fx_sources(d, *effect, &state.midi_fx, r.3, offset);
                    }
                });
            } else if collapsed {
                let name = match effect {
                    MidiFxId::Delay => "MIDI DELAY", MidiFxId::Harmonizer => "HARMONIZER",
                    MidiFxId::RhythmGate => "RHYTHM GATE", MidiFxId::VoiceSplitter => "VOICE SPLITTER",
                    MidiFxId::MidiEq => "MIDI EQ",
                };
                d.text(DELAY_SURFACE.0 + 48.0, DELAY_SURFACE.1 + 29.0, name, 16.0, TEXT);
                let power = delay_editor::delay_power_rect();
                let hot = d.hover.is_some_and(|(x, y)| hit(power, x, y));
                d.bypass_button(power, !enabled, TEAL, hot, 0.0);
            } else {
                self.draw_fx_module(d, *effect, &state.midi_fx, offset);
                self.draw_fx_sources(d, *effect, &state.midi_fx, r.3, offset);
            }
            let button = collapse_rect();
            self.fx_button(d, button, "", false, offset);
            let x = button.0 + button.2 * 0.5;
            let y = button.1 + button.3 * 0.5;
            let rise = if collapsed { -4.0 } else { 4.0 };
            d.poly(&[(x - 8.0, y + rise), (x, y - rise), (x + 8.0, y + rise)], TEXT, 2.5);
            d.oy = oy;
            d.hover = hover;
        }
        d.dimmed = dimmed;
        if let Some(drag) = self.fx_ui.drag.filter(|drag| drag.active) {
            let r = self.fx_ui.rect(drag.to, &state.midi_fx.order);
            let y = if drag.to > drag.from {
                r.1 + r.3 + GAP * 0.5
            } else {
                r.1 + 2.0
            };
            d.line(r.0, y, r.0 + r.2, y, TEAL, 2.0);
        }
        d.reset_scissor();
        let scroll_limit = self.fx_ui.limit(&state.midi_fx.order);
        if scroll_limit > 0.0 {
            let height = VIEWPORT.3;
            let thumb = height * height / (height + scroll_limit);
            let y = VIEWPORT.1 + (height - thumb) * (self.fx_ui.scroll / scroll_limit).clamp(0.0, 1.0);
            d.rect(W - 11.0, VIEWPORT.1, 3.0, height, LINE);
            d.rect(W - 11.0, y, 3.0, thumb, MUTED);
        }
    }
    pub(super) fn cancel_fx_value(&mut self) {
        self.fx_ui.value_drag = None;
        self.fx_ui.eq.drag = None;
    }
    fn fx_button(&self, d: &mut Draw, r: Rect, label: &str, on: bool, offset: f32) {
        self.button_at(d, r, fx_screen_rect(r, offset), label, on, TEAL);
    }
    fn draw_fx_sources(
        &self,
        d: &mut Draw,
        effect: MidiFxId,
        state: &MidiFxState,
        height: f32,
        offset: f32,
    ) {
        let first = source_rect(0);
        d.line(first.0 - 12.0, DELAY_SURFACE.1 + 8.0, first.0 - 12.0,
            DELAY_SURFACE.1 + height - 12.0, LINE, 1.0);
        if effect == MidiFxId::Delay {
            d.with_label_font(|d| {
                d.text(
                    first.0 + 8.0,
                    DELAY_SURFACE.1 + 29.0,
                    "Sources",
                    pleasant_ui::draw::READABLE_LABEL_SIZE,
                    TEXT,
                );
            });
            for (i, name) in SOURCE_NAMES.iter().enumerate() {
                d.button_tinted(source_rect(i), name, state.sources(effect) & (1 << i) != 0, TEAL);
            }
        } else {
            d.text(first.0 + 8.0, DELAY_SURFACE.1 + 29.0, "SOURCES", 13.0, MUTED);
            for (i, name) in SOURCE_NAMES.iter().enumerate() {
                self.fx_button(
                    d,
                    source_rect(i),
                    name,
                    state.sources(effect) & (1 << i) != 0,
                    offset,
                );
            }
        }
    }
    fn fx_controls(&self, effect: MidiFxId, state: &MidiFxState) -> Vec<(Rect, FxValue, f32, String)> {
        let mut controls = Vec::new();
        match effect {
            MidiFxId::Harmonizer => {
                for (i, voice) in state.harmonizer.voices.iter().enumerate() {
                    let units = if state.harmonizer.mode == HarmonizerMode::Fixed { "st" } else { "steps" };
                    controls.push((control_rect(1, i + 1), FxValue::HarmonyInterval(i), voice.interval as f32,
                        format!("Interval: {:+} {}  ↕", voice.interval, units)));
                    controls.push((control_rect(2, i + 1), FxValue::HarmonyVelocity(i), voice.velocity,
                        format!("Velocity: {:.0}%  ↕", voice.velocity * 100.0)));
                }
            }
            MidiFxId::RhythmGate => {
                let g = &state.rhythm_gate;
                controls.extend([
                    (control_rect(0, 0), FxValue::GateSteps, g.steps as f32, format!("Steps: {}  ↕", g.steps)),
                    (control_rect(1, 0), FxValue::GateRate, rate_index(g.rate) as f32,
                        format!("{}  ↕", RATE_NAMES[rate_index(g.rate)])),
                    (control_rect(2, 0), FxValue::GateLength, g.gate, format!("Gate: {:.0}%  ↕", g.gate * 100.0)),
                    (control_rect(0, 1), FxValue::GatePulses, g.pulses as f32, format!("Fill pulses: {}  ↕", g.pulses)),
                    (control_rect(1, 1), FxValue::GateRotation, g.rotation as f32, format!("Fill rotation: {}  ↕", g.rotation)),
                ]);
            }
            MidiFxId::VoiceSplitter => {
                let s = &state.voice_splitter;
                let mpe = self.params.mpe_enabled();
                for (i, label) in ["Low", "Inner", "High"].iter().enumerate() {
                    let (kind, value, text) = if mpe {
                        (FxValue::SplitPool(i), s.pools[i] as f32, format!("{} pool: {} channels  ↕", label, s.pools[i]))
                    } else {
                        (FxValue::SplitChannel(i), s.channels[i] as f32, format!("{}: channel {}  ↕", label, s.channels[i]))
                    };
                    controls.push((control_rect(i, 0), kind, value, text));
                }
            }
            MidiFxId::Delay | MidiFxId::MidiEq => {}
        }
        controls
    }
    fn draw_fx_module(&self, d: &mut Draw, effect: MidiFxId, state: &MidiFxState, offset: f32) {
        let s = DELAY_SURFACE;
        let name = match effect {
            MidiFxId::Harmonizer => "HARMONIZER",
            MidiFxId::RhythmGate => "RHYTHM GATE",
            MidiFxId::VoiceSplitter => "VOICE SPLITTER",
            MidiFxId::MidiEq => "MIDI EQ",
            MidiFxId::Delay => return,
        };
        let enabled = state.enabled(effect);
        d.text(s.0 + 48.0, s.1 + 29.0, name, 16.0, TEXT);
        let power = delay_editor::delay_power_rect();
        d.bypass_button(power, !enabled, TEAL,
            d.hover.is_some_and(|(x, y)| hit(power, x, y)), 0.0);
        d.line(s.0 + 12.0, s.1 + 43.0, source_rect(0).0 - 24.0, s.1 + 43.0, LINE, 1.0);
        for (r, _, _, label) in self.fx_controls(effect, state) {
            self.fx_button(d, r, &label, false, offset);
        }
        match effect {
            MidiFxId::Harmonizer => {
                let h = &state.harmonizer;
                let mode = match h.mode { HarmonizerMode::Fixed => "Fixed semitones", HarmonizerMode::Scale => "Scale degrees", HarmonizerMode::Chord => "Chord tones" };
                self.fx_button(d, control_rect(0, 0), &format!("Mode: {}  ▸", mode), true, offset);
                self.fx_button(d, control_rect(1, 0), "Original", h.original, offset);
                for (i, voice) in h.voices.iter().enumerate() {
                    self.fx_button(d, control_rect(0, i + 1), &format!("Voice {}", i + 1), voice.enabled, offset);
                }
            }
            MidiFxId::RhythmGate => {
                let g = &state.rhythm_gate;
                self.fx_button(d, control_rect(2, 1), "Euclidean Fill", false, offset);
                for i in 0..g.steps as usize {
                    let r = gate_step_rect(i, g.steps as usize);
                    let velocity = g.pattern[i];
                    self.fx_button(d, r, "", velocity > 0.0, offset);
                    let fill = (r.3 - 40.0) * (velocity / 2.0).clamp(0.0, 1.0);
                    d.rect(r.0 + 5.0, r.1 + r.3 - 8.0 - fill, (r.2 - 10.0).max(1.0), fill, alpha(TEAL, 0.35));
                    d.text_centered(r.0 + r.2 * 0.5, r.1 + 19.0, &format!("{}", i + 1), 12.0, TEXT);
                    d.text_centered(r.0 + r.2 * 0.5, r.1 + 37.0,
                        &if velocity > 0.0 { format!("{:.0}", velocity * 100.0) } else { "—".into() }, 11.0, MUTED);
                }
            }
            MidiFxId::VoiceSplitter => {
                let splitter = &state.voice_splitter;
                if self.params.mpe_enabled() {
                    let members = self.params.members.value() as u8;
                    let upper = self.params.upper.value();
                    if let Some(pools) = splitter.pool_ranges(members, upper) {
                        for (i, mask) in pools.iter().enumerate() {
                            let channels = (0..16).filter(|channel| mask & (1 << channel) != 0)
                                .map(|channel| (channel + 1).to_string()).collect::<Vec<_>>().join(", ");
                            let label = ["Low", "Inner", "High", "Unselected"][i];
                            d.text(s.0 + 20.0, s.1 + 117.0 + i as f32 * 24.0,
                                &format!("{} · channels {}", label, channels), 14.0, if i == 3 { MUTED } else { TEXT });
                        }
                        d.text(s.0 + 20.0, s.1 + 221.0, &format!("Shared MPE master: channel {} · {} members", if upper { 16 } else { 1 }, members), 14.0, TEAL);
                    } else {
                        d.text(s.0 + 20.0, s.1 + 117.0, &format!("Invalid MPE pools · {} member channels", members), 14.0, GOLD);
                    }
                }
            }
            MidiFxId::MidiEq => self.fx_ui.eq.draw(d, &state.midi_eq),
            MidiFxId::Delay => {}
        }
    }
    fn fx_module_press(&mut self, cx: &mut EventContext, effect: MidiFxId, x: f32, y: f32, screen_y: f32) -> bool {
        self.cancel_arp_editor();
        let state = self.arp_state();
        if hit(delay_editor::delay_power_rect(), x, y) {
            self.toggle_fx_power(effect);
        } else if effect == MidiFxId::Harmonizer && hit(control_rect(0, 0), x, y) {
            self.params.arpeggiator.edit(|s| {
                s.midi_fx.harmonizer.mode = match s.midi_fx.harmonizer.mode {
                    HarmonizerMode::Fixed => HarmonizerMode::Scale,
                    HarmonizerMode::Scale => HarmonizerMode::Chord,
                    HarmonizerMode::Chord => HarmonizerMode::Fixed,
                };
            });
        } else if effect == MidiFxId::Harmonizer && hit(control_rect(1, 0), x, y) {
            self.params.arpeggiator.edit(|s| s.midi_fx.harmonizer.original = !s.midi_fx.harmonizer.original);
        } else if let Some(i) = (effect == MidiFxId::Harmonizer)
            .then(|| (0..3).find(|&i| hit(control_rect(0, i + 1), x, y)))
            .flatten()
        {
            self.params.arpeggiator.edit(|s| {
                s.midi_fx.harmonizer.voices[i].enabled = !s.midi_fx.harmonizer.voices[i].enabled
            });
        } else if effect == MidiFxId::RhythmGate && hit(control_rect(2, 1), x, y) {
            self.params.arpeggiator.edit(|s| s.midi_fx.rhythm_gate.fill_euclidean());
        } else {
            let control = self.fx_controls(effect, &state.midi_fx).into_iter().find(|(r, _, _, _)| hit(*r, x, y));
            let control = control.map(|(_, value, start, _)| (value, start)).or_else(|| {
                (effect == MidiFxId::RhythmGate).then(|| {
                    let g = &state.midi_fx.rhythm_gate;
                    (0..g.steps as usize).find(|&i| hit(gate_step_rect(i, g.steps as usize), x, y))
                        .map(|i| (FxValue::GateStep(i), g.pattern[i]))
                }).flatten()
            });
            let Some((value, start)) = control else { return false; };
            self.fx_ui.value_drag = Some(ValueDrag { value, origin_y: screen_y, start, moved: false });
            cx.capture();
            cx.focus();
            self.focused = true;
        }
        cx.needs_redraw();
        true
    }
    fn apply_fx_value(&self, drag: ValueDrag, y: f32) {
        let delta = drag.origin_y - y;
        self.params.arpeggiator.edit(|s| {
            let fx = &mut s.midi_fx;
            match drag.value {
                FxValue::HarmonyInterval(i) => fx.harmonizer.voices[i].interval = (drag.start + delta / 6.0).round().clamp(-24.0, 24.0) as i8,
                FxValue::HarmonyVelocity(i) => fx.harmonizer.voices[i].velocity = (drag.start + delta / 100.0).clamp(0.0, 2.0),
                FxValue::GateSteps => { fx.rhythm_gate.steps = (drag.start + delta / 6.0).round().clamp(1.0, 32.0) as u8; fx.rhythm_gate.pulses = fx.rhythm_gate.pulses.min(fx.rhythm_gate.steps); fx.rhythm_gate.rotation %= fx.rhythm_gate.steps; },
                FxValue::GateRate => fx.rhythm_gate.rate = RATES[(drag.start + delta / 10.0).round().clamp(0.0, 8.0) as usize],
                FxValue::GateLength => fx.rhythm_gate.gate = (drag.start + delta / 100.0).clamp(0.01, 1.0),
                FxValue::GatePulses => fx.rhythm_gate.pulses = (drag.start + delta / 6.0).round().clamp(0.0, fx.rhythm_gate.steps as f32) as u8,
                FxValue::GateRotation => fx.rhythm_gate.rotation = (drag.start + delta / 6.0).round().clamp(0.0, fx.rhythm_gate.steps.saturating_sub(1) as f32) as u8,
                FxValue::GateStep(i) => fx.rhythm_gate.pattern[i] = (if drag.start == 0.0 { 1.0 } else { drag.start } + delta / 70.0).clamp(0.0, 2.0),
                FxValue::SplitChannel(i) => fx.voice_splitter.channels[i] = (drag.start + delta / 8.0).round().clamp(1.0, 16.0) as u8,
                FxValue::SplitPool(i) => fx.voice_splitter.pools[i] = (drag.start + delta / 8.0).round().clamp(1.0, 15.0) as u8,
            }
        });
    }
    pub(super) fn fx_button_rects(&self) -> Vec<(Rect, bool)> {
        let state = self.arp_state();
        let mut buttons = Vec::new();
        for (index, &effect) in state.midi_fx.order.iter().enumerate() {
            let offset = self.fx_ui.rect(index, &state.midi_fx.order).1 - DELAY_SURFACE.1;
            let screen = |r| fx_screen_rect(r, offset);
            buttons.push((screen(collapse_rect()), false));
            if self.fx_ui.collapsed.contains(&effect) {
                buttons.push((screen(delay_editor::delay_power_rect()), state.midi_fx.enabled(effect)));
                continue;
            }
            for i in 0..5 { buttons.push((screen(source_rect(i)), state.midi_fx.sources(effect) & (1 << i) != 0)); }
            if effect != MidiFxId::Delay {
                buttons.push((screen(delay_editor::delay_power_rect()), state.midi_fx.enabled(effect)));
                buttons.extend(self.fx_controls(effect, &state.midi_fx).into_iter().map(|(r, _, _, _)| (screen(r), false)));
                if effect == MidiFxId::Harmonizer {
                    buttons.push((screen(control_rect(0, 0)), true));
                    buttons.push((screen(control_rect(1, 0)), state.midi_fx.harmonizer.original));
                    buttons.extend((0..3).map(|i| (screen(control_rect(0, i + 1)), state.midi_fx.harmonizer.voices[i].enabled)));
                } else if effect == MidiFxId::RhythmGate {
                    buttons.push((screen(control_rect(2, 1)), false));
                    let gate = &state.midi_fx.rhythm_gate;
                    buttons.extend((0..gate.steps as usize).map(|i| (screen(gate_step_rect(i, gate.steps as usize)), gate.pattern[i] > 0.0)));
                }
            }
        }
        buttons
    }
    pub(super) fn fx_hint(&self, x: f32, y: f32) -> Option<String> {
        if !hit(VIEWPORT, x, y) { return None; }
        let state = self.arp_state();
        let fx = &state.midi_fx;
        for (index, &effect) in fx.order.iter().enumerate() {
            let r = self.fx_ui.rect(index, &fx.order);
            if !hit(r, x, y) { continue; }
            let local_y = y - r.1 + DELAY_SURFACE.1;
            if hit(collapse_rect(), x, local_y) { return Some("Collapse / expand this MIDI FX module · Processing stays active".into()); }
            if hit(delay_editor::delay_power_rect(), x, local_y) {
                let screen_y = delay_editor::delay_power_rect().1 + r.1 - DELAY_SURFACE.1;
                let power_h = delay_editor::delay_power_rect().3;
                if screen_y < VIEWPORT.1 || screen_y + power_h > VIEWPORT.1 + VIEWPORT.3 {
                    return Some("Scroll to bring this effect's power switch fully into view".into());
                }
                let action = if fx.enabled(effect) { "Bypass" } else { "Enable" };
                return Some(if effect == MidiFxId::Delay {
                    format!("{action} MIDI Delay · Independent of arpeggiator power")
                } else {
                    format!("{action} this MIDI effect")
                });
            }
            if self.fx_ui.collapsed.contains(&effect) { return None; }
            if let Some(i) = (0..5).find(|&i| hit(source_rect(i), x, local_y)) {
                return Some(format!("{} source · Toggle this effect for these notes independently of the other sources", SOURCE_NAMES[i]));
            }
            return Some(match effect {
                MidiFxId::Harmonizer if hit(control_rect(0, 0), x, local_y) =>
                    "Harmony mode · Click to cycle through fixed semitones, scale degrees, and chord tones. Intervals resolve at note onset.",
                MidiFxId::Harmonizer =>
                    "Harmonizer · Click voices to toggle them. Drag interval and velocity values vertically. Each voice follows the original note’s expression.",
                MidiFxId::RhythmGate if hit(control_rect(2, 1), x, local_y) =>
                    "Euclidean Fill · Click to replace the pattern with the current pulse and rotation settings",
                MidiFxId::RhythmGate if local_y >= gate_step_rect(0, 1).1 =>
                    "Rhythm Gate · Click steps to toggle them or drag vertically for velocity. Held notes join at the next enabled step.",
                MidiFxId::RhythmGate =>
                    "Rhythm Gate · Drag values vertically. Timing follows the global tempo and DAW alignment settings.",
                MidiFxId::VoiceSplitter if self.params.mpe_enabled() => {
                    let members = self.params.members.value() as u8;
                    if fx.voice_splitter.pool_ranges(members, self.params.upper.value()).is_none() {
                        "Invalid MPE pools · Shrink pools or increase MPE members to reserve a channel for each pool, including Unselected."
                    } else if local_y <= control_rect(0, 0).1 + control_rect(0, 0).3 {
                        "Voice Splitter · Drag to resize the MPE pools. Notes retain their channels until release. Full pools drop new notes."
                    } else {
                        "MPE routing · Send each instrument its pool and the shared master channel. Match the MPE zone and pitch bend range in the host."
                    }
                }
                MidiFxId::VoiceSplitter =>
                    "Voice Splitter · Drag channels vertically and route them in the host. Lowest goes to Low, highest or a single note to High, and the rest to Inner. Assignments stay until release.",
                MidiFxId::MidiEq => return self.fx_ui.eq.hint(x, local_y, &fx.midi_eq),
                MidiFxId::Delay => return self.arp_editor_hint(x, y),
            }.into());
        }
        None
    }
    pub(super) fn tick_midi_fx(&mut self, dt: f32) -> bool {
        if !self.fx_ui.open {
            return false;
        }
        let order = self.arp_state().midi_fx.order.clone();
        self.fx_ui.tick(dt, &order)
    }
    fn scroll_fx(&mut self, cx: &mut EventContext, event: &WindowEvent, x: f32, y: f32) -> bool {
        if let WindowEvent::MouseScroll(_, dy) = event {
            if hit(VIEWPORT, x, y) {
                let order = self.arp_state().midi_fx.order.clone();
                self.fx_ui.scroll = (self.fx_ui.scroll - dy * 24.0).clamp(0.0, self.fx_ui.limit(&order));
                cx.needs_redraw();
                return true;
            }
        }
        false
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
        if press && hit(PROGRAMMING_TRANSPORT, x, y) {
            if self.quick_midi.show {
                return false;
            }
            self.toggle_tracker_transport(cx);
            return true;
        }
        if press && (hit(PROGRAMMING_TAB, x, y) || hit(PERFORMANCE_TAB, x, y) || hit(FX_TAB, x, y)) {
            self.set_fx_page(cx, hit(FX_TAB, x, y), hit(PROGRAMMING_TAB, x, y));
            return true;
        }
        if self.fx_ui.open && press && hit(FX_LEARN, x, y) {
            cx.focus();
            self.focused = true;
            self.toggle_quick_midi(cx);
            return true;
        }
        if self.quick_midi.show {
            return self.fx_ui.open && self.scroll_fx(cx, event, x, y);
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
        let order = self.arp_state().midi_fx.order.clone();
        let count = order.len();
        if let Some(mut drag) = self.fx_ui.value_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    drag.moved |= (y - drag.origin_y).abs() >= 3.0;
                    if drag.moved { self.apply_fx_value(drag, y); }
                    self.fx_ui.value_drag = Some(drag);
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    if !drag.moved {
                        if let FxValue::GateStep(index) = drag.value {
                            self.params.arpeggiator.edit(|s| {
                                let v = &mut s.midi_fx.rhythm_gate.pattern[index];
                                *v = if *v > 0.0 { 0.0 } else { 1.0 };
                            });
                        }
                    }
                    self.fx_ui.value_drag = None;
                    cx.release();
                }
                WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::FocusOut => {
                    self.fx_ui.value_drag = None;
                    cx.release();
                }
                _ => {}
            }
            cx.needs_redraw();
            return true;
        }
        if let Some(mut drag) = self.fx_ui.drag {
            match event {
                WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::FocusOut => {
                    self.fx_ui.drag = None;
                    cx.release();
                }
                WindowEvent::MouseMove(_, _) => {
                    drag.update_layout(x, y, &self.fx_ui, &order);
                    self.fx_ui.drag = Some(drag);
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    drag.update_layout(x, y, &self.fx_ui, &order);
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
        if self.midi_eq_event(cx, event, x, y) {
            return true;
        }
        if press && hit(VIEWPORT, x, y) {
            if let Some(index) = (0..count).find(|&i| hit(self.fx_ui.grip(i, &order), x, y)) {
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
        if press && hit(VIEWPORT, x, y) {
            let state = self.arp_state();
            if let Some((index, effect)) = state.midi_fx.order.iter().enumerate()
                .find(|(index, _)| hit(self.fx_ui.rect(*index, &order), x, y)) {
                let local_y = y - (self.fx_ui.rect(index, &state.midi_fx.order).1 - VIEWPORT.1);
                if hit(collapse_rect(), x, local_y) {
                    self.cancel_arp_editor();
                    if self.fx_ui.collapsed.contains(effect) { self.fx_ui.collapsed.retain(|id| id != effect); }
                    else { self.fx_ui.collapsed.push(*effect); }
                    self.fx_ui.scroll = self.fx_ui.scroll.min(self.fx_ui.limit(&order));
                    cx.needs_redraw();
                    return true;
                }
                if self.fx_ui.collapsed.contains(effect) {
                    if hit(delay_editor::delay_power_rect(), x, local_y) {
                        self.toggle_fx_power(*effect);
                        cx.needs_redraw();
                    }
                    return true;
                }
                if let Some(source) = (0..5).find(|&i| hit(source_rect(i), x, local_y)) {
                    self.cancel_arp_editor();
                    self.params.arpeggiator.edit(|s| {
                        let bits = s.midi_fx.sources(*effect) ^ (1 << source);
                        s.midi_fx.set_sources(*effect, bits);
                    });
                    cx.needs_redraw();
                    return true;
                }
                if *effect != MidiFxId::Delay && self.fx_module_press(cx, *effect, x, local_y, y) {
                    return true;
                }
            }
        }
        // A captured value gesture continues beyond the viewport; ordinary clicks do not.
        let local_y = self.fx_delay_local_y(y);
        if !self.fx_ui.collapsed.contains(&MidiFxId::Delay) && (self.arp_editor_dragging()
            || hit(VIEWPORT, x, y)
            || matches!(event, WindowEvent::KeyDown(Code::Escape, _)))
            && self.arp_editor_event(cx, event, x, local_y)
        {
            return true;
        }
        if self.scroll_fx(cx, event, x, y) {
            return true;
        }
        // Prevent invisible performance controls receiving pointer events, even in empty space.
        hit(VIEWPORT, x, y)
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

    fn view(mode: i32, mpe: bool) -> ChordboardView {
        let view = support::view(mode, mpe);
        view.params.arpeggiator.edit(|s| {
            if let Some(index) = s.midi_fx.order.iter().position(|id| *id == MidiFxId::Delay) {
                s.midi_fx.move_effect(index, 0);
            }
        });
        view
    }

    #[test]
    fn midi_fx_delay_spotlight_tracks_the_visible_handle_and_clears_with_state() {
        let has_spotlight = |view: &ChordboardView, x, y| {
            view.fx_hint(x, y)
                .is_some_and(|hint| hint.starts_with("Settings preview"))
        };
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        view.params.arpeggiator.edit(|s| {
            let delay = &mut s.midi_fx.delay;
            delay.enabled = true;
            delay.count = 2;
            delay.taps[0].time = 0.5;
            delay.taps[1].time = 0.5;
        });
        view.fx_ui.scroll = 24.0;
        let state = view.params.arpeggiator.snapshot();
        let delay = &state.midi_fx.delay;
        let domain = delay_editor::time_domain(delay);
        let x = delay_editor::tap_x(&delay.taps[0], domain);
        let y = view.fx_delay_screen_rect(delay_editor::tap_graph()).1 - 9.0;
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseMove(x, y));
        let hint = view.hover_hint(&[]).unwrap();
        assert!(hint.starts_with("Settings preview · Tap 1"));
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.delay, *delay);

        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        assert!(view.arp_editor_dragging());
        assert!(!has_spotlight(&view, x, y));
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseUp(MouseButton::Left));
        view.fx_ui.scroll = 100.0;
        let y = view.fx_delay_screen_rect(delay_editor::tap_graph()).1 - 9.0;
        assert!(!has_spotlight(&view, x, y));
        view.fx_ui.scroll = 0.0;
        let y = delay_editor::tap_graph().1 - 9.0;
        view.fx_ui.collapsed.push(MidiFxId::Delay);
        assert!(!has_spotlight(&view, x, y));
        view.fx_ui.collapsed.clear();
        view.params.arpeggiator.edit(|s| s.midi_fx.delay.enabled = false);
        assert!(!has_spotlight(&view, x, y));
    }

    #[test]
    fn midi_fx_collapse_compacts_stack_without_changing_processing() {
        let mut view = view(2, false);
        view.fx_ui.open = true;
        let (mut cx, target, _) = context();
        let before = view.params.arpeggiator.snapshot().midi_fx;
        let r = view.fx_delay_screen_rect(collapse_rect());
        click(&mut view, &mut cx, target, r);
        assert!(view.fx_ui.collapsed.contains(&MidiFxId::Delay));
        assert_eq!(view.fx_ui.rect(0, &before.order).3, MODULE_HEADER_H);
        assert_eq!(view.fx_ui.rect(1, &before.order).1, VIEWPORT.1 + MODULE_HEADER_H + GAP);
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx, before);
        let power = view.fx_delay_screen_rect(delay_editor::delay_power_rect());
        assert!(view.fx_hint(power.0 + 1.0, power.1 + 1.0).unwrap().contains("MIDI Delay"));
        // Collapse follows the module identity when the processing order changes.
        view.params.arpeggiator.edit(|s| s.midi_fx.move_effect(0, 1));
        let order = view.params.arpeggiator.snapshot().midi_fx.order;
        assert_eq!(view.fx_ui.rect(1, &order).3, MODULE_HEADER_H);
        let r = view.fx_delay_screen_rect(collapse_rect());
        click(&mut view, &mut cx, target, r);
        assert!(!view.fx_ui.collapsed.contains(&MidiFxId::Delay));
        assert_eq!(view.fx_ui.rect(1, &order).3, DELAY_SURFACE.3);
    }

    #[test]
    fn midi_fx_grip_spans_module_height_and_does_not_overlap_controls() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        let grip = grip_rect(0, 0.0);
        let order = view.params.arpeggiator.snapshot().midi_fx.order;
        let last = view.fx_ui.rect(order.len() - 1, &order);
        for (r, _) in view.arp_editor_button_rects() {
            assert!(r.0 > grip.0 + grip.2);
            assert!(r.1 >= DELAY_SURFACE.1);
            assert!(r.1 + r.3 <= last.1 + last.3);
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
        click(&mut view, &mut cx, target, delay_editor::delay_power_rect());
        assert!(view.fx_ui.drag.is_none());
        assert!(view.params.arpeggiator.snapshot().midi_fx.delay.enabled);
    }

    #[test]
    fn midi_fx_drag_commit_reorders_and_focus_loss_cancels() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        let x = VIEWPORT.0 + 14.0;
        let y = VIEWPORT.1 + 20.0;
        for finish in [
            WindowEvent::MouseUp(MouseButton::Left),
            WindowEvent::FocusOut,
        ] {
            let before = view.params.arpeggiator.snapshot().midi_fx;
            let commit = matches!(finish, WindowEvent::MouseUp(_));
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
            let mut expected = before;
            if commit { expected.move_effect(0, 1); }
            assert_eq!(view.params.arpeggiator.snapshot().midi_fx, expected);
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
        drag.update(30.0, module_rect(2, 0.0).1 + 18.0, 0.0, modules.len());
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
            collapsed: Vec::new(),
            open: true,
            scroll: 0.0,
            drag: Some(drag),
            value_drag: None,
            ..FxUi::default()
        };
        assert!(ui.tick(0.1, &[MidiFxId::Delay; 8]));
        assert_eq!(ui.scroll, 30.0);
        ui.tick(100.0, &[MidiFxId::Delay; 8]);
        assert_eq!(ui.scroll, max_scroll(8));
        ui.drag.as_mut().unwrap().pointer.1 = VIEWPORT.1;
        ui.tick(100.0, &[MidiFxId::Delay; 8]);
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
    #[test]
    fn midi_fx_sources_hover_and_select_independently_after_reordering_and_scrolling() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        view.params.arpeggiator.edit(|s| {
            s.midi_fx = MidiFxState::default();
            s.midi_fx.move_effect(0, 2);
            for effect in crate::midi_fx::EFFECTS {
                s.midi_fx.set_sources(effect, 0);
            }
        });
        let order = view.params.arpeggiator.snapshot().midi_fx.order;
        for (index, &effect) in order.iter().enumerate() {
            view.fx_ui.scroll = 0.0;
            let top = view.fx_ui.rect(index, &order).1 - VIEWPORT.1;
            view.fx_ui.scroll = top.min(view.fx_ui.limit(&order));
            let offset = view.effect_stack_offset(effect);
            for (source, name) in SOURCE_NAMES.iter().enumerate() {
                let r = fx_screen_rect(source_rect(source), offset);
                let x = r.0 + r.2 * 0.5;
                let y = r.1 + r.3 * 0.5;
                event(&mut view, &mut cx, target, x, y, WindowEvent::MouseMove(x, y));
                view.tick_motion(0.2);
                assert!(view.fx_hint(x, y).unwrap().starts_with(*name));
                for &other in &order {
                    let other_rect = fx_screen_rect(source_rect(source), view.effect_stack_offset(other));
                    assert_eq!(
                        view.hover_amount(other_rect),
                        (other == effect) as u8 as f32,
                        "{other:?} {name}",
                    );
                }
                click(&mut view, &mut cx, target, r);
                let state = view.params.arpeggiator.snapshot();
                for &other in &order {
                    let expected = if other == effect { (1 << (source + 1)) - 1 } else { 0 };
                    assert_eq!(state.midi_fx.sources(other), expected, "{other:?} {name}");
                }
            }
            assert!(!view.params.arpeggiator.snapshot().midi_fx.enabled(effect));
            for source in 0..SOURCE_NAMES.len() {
                click(&mut view, &mut cx, target, fx_screen_rect(source_rect(source), offset));
            }
            assert_eq!(view.params.arpeggiator.snapshot().midi_fx.sources(effect), 0);
        }
    }

    #[test]
    fn midi_fx_new_controls_edit_harmony_gate_and_mpe_pools() {
        let mut view = view(2, true);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        view.params.arpeggiator.edit(|s| s.midi_fx = MidiFxState::default());
        click(&mut view, &mut cx, target, control_rect(0, 0));
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.harmonizer.mode, HarmonizerMode::Chord);
        click(&mut view, &mut cx, target, control_rect(1, 0));
        assert!(!view.params.arpeggiator.snapshot().midi_fx.harmonizer.original);
        click(&mut view, &mut cx, target, control_rect(0, 3));
        assert!(view.params.arpeggiator.snapshot().midi_fx.harmonizer.voices[2].enabled);
        let gate_offset = view.effect_stack_offset(MidiFxId::RhythmGate);
        let screen = |r: Rect| (r.0, r.1 + gate_offset, r.2, r.3);
        let step = screen(gate_step_rect(1, 16));
        click(&mut view, &mut cx, target, step);
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.rhythm_gate.pattern[1], 1.0);
        let x = step.0 + step.2 * 0.5;
        let y = step.1 + step.3 * 0.5;
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        event(&mut view, &mut cx, target, x, y - 35.0, WindowEvent::MouseMove(x, y - 35.0));
        event(&mut view, &mut cx, target, x, y - 35.0, WindowEvent::MouseUp(MouseButton::Left));
        assert!((view.params.arpeggiator.snapshot().midi_fx.rhythm_gate.pattern[1] - 1.5).abs() < 0.001);
        click(&mut view, &mut cx, target, screen(control_rect(2, 1)));
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.rhythm_gate.pattern[1], 0.0);
        let r = control_rect(0, 0);
        let x = r.0 + r.2 * 0.5;
        let y = r.1 + view.effect_stack_offset(MidiFxId::VoiceSplitter) + r.3 * 0.5;
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        event(&mut view, &mut cx, target, x, y - 8.0, WindowEvent::MouseMove(x, y - 8.0));
        event(&mut view, &mut cx, target, x, y - 8.0, WindowEvent::MouseUp(MouseButton::Left));
        assert_eq!(view.params.arpeggiator.snapshot().midi_fx.voice_splitter.pools[0], 3);
    }

    #[test]
    fn midi_fx_value_capture_clears_on_focus_loss_and_key_releases_pass_through() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        view.params.arpeggiator.edit(|s| s.midi_fx = MidiFxState::default());
        view.focused = true;
        event(&mut view, &mut cx, target, 0.0, 0.0, WindowEvent::KeyDown(Code::Digit1, None));
        let r = control_rect(1, 1);
        let x = r.0 + 20.0;
        let y = r.1 + 10.0;
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        assert!(view.fx_ui.value_drag.is_some());
        event(&mut view, &mut cx, target, x, y, WindowEvent::KeyUp(Code::Digit1, None));
        assert!(!view.pressed[0]);
        event(&mut view, &mut cx, target, x, y, WindowEvent::FocusOut);
        assert!(view.fx_ui.value_drag.is_none());
    }

    #[test]
    fn midi_fx_eq_click_creates_and_drag_scroll_and_right_click_edit_the_band() {
        let mut view = view(2, false);
        let (mut cx, target, _) = context();
        click(&mut view, &mut cx, target, FX_TAB);
        event(&mut view, &mut cx, target, VIEWPORT.0 + 10.0, VIEWPORT.1 + 10.0,
            WindowEvent::MouseScroll(0.0, -100.0));
        let offset = view.effect_stack_offset(MidiFxId::MidiEq);
        let (px, py, w, h) = delay_editor::delay_power_rect();
        click(&mut view, &mut cx, target, (px, py + offset, w, h));

        let graph = eq::graph();
        let (x, y) = graph.from_normalized(60.0 / 127.0, 0.75);
        let y = y + offset;
        assert!(view.fx_hint(x, y).unwrap().starts_with("MIDI EQ"));
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Left));
        let created = view.params.arpeggiator.snapshot().midi_fx.midi_eq;
        assert!(created.enabled);
        assert_eq!(created.bands.iter().flatten().count(), 1);
        assert_eq!(created.bands[0].unwrap().note, 60.0);

        let (x, y) = graph.from_normalized(67.0 / 127.0, 0.25);
        let y = y + offset;
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseMove(x, y));
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseUp(MouseButton::Left));
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseScroll(0.0, 1.0));
        let band = view.params.arpeggiator.snapshot().midi_fx.midi_eq.bands[0].unwrap();
        assert_eq!(band.note, 67.0);
        assert!((band.gain_db + 12.0).abs() < 1.0e-4);
        assert!(band.q > 1.0);
        event(&mut view, &mut cx, target, x, y, WindowEvent::MouseDown(MouseButton::Right));
        assert!(view.params.arpeggiator.snapshot().midi_fx.midi_eq.bands[0].is_none());
    }

    #[test]
    fn midi_fx_eq_meter_captures_incoming_midi_even_after_quick_release() {
        let mut view = view(2, false);
        view.fx_ui.open = true;
        let mut engine = crate::engine::Engine::default();
        let old = engine.snapshot();
        engine.midi_note(true, 1, 73, 0.63, &mut |_| {});
        engine.midi_note(false, 1, 73, 0.0, &mut |_| {});
        engine.midi_note(true, 2, 60, 0.81, &mut |_| {});
        view.snapshot = engine.snapshot();

        assert!(view.tick_midi_eq(&old, 0.01));
        assert_eq!(view.fx_ui.eq.levels[73], 0.63);
        assert_eq!(view.fx_ui.eq.levels[60], 0.81);
        let old = view.snapshot;
        assert!(view.tick_midi_eq(&old, 0.3));
        assert_eq!(view.fx_ui.eq.levels[73], 0.0);
        assert_eq!(view.fx_ui.eq.levels[60], 0.81);
    }

    #[test]
    fn midi_fx_source_column_reserves_space_for_all_module_controls() {
        let view = view(2, false);
        let state = view.params.arpeggiator.snapshot();
        let left = source_rect(0).0 - 12.0;
        for &effect in &state.midi_fx.order {
            let bottom = DELAY_SURFACE.1 + view.fx_ui.height(effect);
            for (r, _, _, _) in view.fx_controls(effect, &state.midi_fx) {
                assert!(r.0 + r.2 < left);
                assert!(r.1 + r.3 < bottom);
            }
            let last_source = source_rect(SOURCE_NAMES.len() - 1);
            assert!(last_source.0 + last_source.2 <= DELAY_SURFACE.0 + DELAY_SURFACE.2);
            assert!(last_source.1 + last_source.3 < bottom);
        }
        for count in 1..=32 {
            let r = gate_step_rect(count - 1, count);
            assert!(r.0 + r.2 < left);
            assert!(r.1 + r.3 < DELAY_SURFACE.1 + view.fx_ui.height(MidiFxId::RhythmGate));
        }
    }

}
