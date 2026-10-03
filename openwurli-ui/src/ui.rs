use crate::params::OpenWurliUiParams;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color, FontId},
    },
    widgets::RawParamEvent,
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport},
    preferences::AppearanceStore,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{Arc, OnceLock},
    time::Duration,
};

const WIDTH: f32 = 720.0;
const HEIGHT: f32 = 550.0;
const KNOB_W: f32 = 150.0;
const KNOB_H: f32 = 140.0;
const CPU_BUTTON: (f32, f32, f32, f32) = (551.0, 328.0, 104.0, 42.0);
const EXTENDED_BUTTON: (f32, f32, f32, f32) = (551.0, 384.0, 104.0, 42.0);
const SAG_BUTTON: (f32, f32, f32, f32) = (551.0, 440.0, 104.0, 42.0);
const THEME_BUTTON: (f32, f32, f32, f32) = (601.0, 21.0, 80.0, 30.0);

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();

fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("OpenWurli UI"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Knob {
    Volume,
    Tremolo,
    Speaker,
    ReedDecay,
    HammerHardness,
    PickupDrive,
    TremoloResponse,
}

impl Knob {
    const ALL: [Self; 7] = [
        Self::Volume,
        Self::Tremolo,
        Self::TremoloResponse,
        Self::Speaker,
        Self::ReedDecay,
        Self::HammerHardness,
        Self::PickupDrive,
    ];

    fn rect(self) -> (f32, f32, f32, f32) {
        let (column, y) = match self {
            Self::Volume => (0, 125.0),
            Self::Tremolo => (1, 125.0),
            Self::TremoloResponse => (2, 125.0),
            Self::Speaker => (3, 125.0),
            Self::ReedDecay => (0, 330.0),
            Self::HammerHardness => (1, 330.0),
            Self::PickupDrive => (2, 330.0),
        };
        (45.0 + column as f32 * 160.0, y, KNOB_W, KNOB_H)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Volume => "VOLUME",
            Self::Tremolo => "TREMOLO DEPTH",
            Self::Speaker => "SPEAKER CHARACTER",
            Self::ReedDecay => "REED DECAY",
            Self::HammerHardness => "HAMMER HARDNESS",
            Self::PickupDrive => "PICKUP DRIVE",
            Self::TremoloResponse => "TREMOLO RESPONSE",
        }
    }
}

struct Drag {
    knob: Knob,
    start_y: f32,
    start_norm: f32,
}

struct OpenWurliView {
    params: Arc<OpenWurliUiParams>,
    font: Cell<Option<FontId>>,
    drag: Option<Drag>,
    hover: Option<(f32, f32)>,
}

impl OpenWurliView {
    fn param(&self, knob: Knob) -> &FloatParam {
        match knob {
            Knob::Volume => &self.params.volume,
            Knob::Tremolo => &self.params.tremolo_depth,
            Knob::Speaker => &self.params.speaker_character,
            Knob::ReedDecay => &self.params.reed_decay,
            Knob::HammerHardness => &self.params.hammer_hardness,
            Knob::PickupDrive => &self.params.pickup_drive,
            Knob::TremoloResponse => &self.params.tremolo_response,
        }
    }

    fn hit(x: f32, y: f32, rect: (f32, f32, f32, f32)) -> bool {
        x >= rect.0 && x <= rect.0 + rect.2 && y >= rect.1 && y <= rect.1 + rect.3
    }

    fn knob_at(&self, x: f32, y: f32) -> Option<Knob> {
        Knob::ALL
            .iter()
            .copied()
            .find(|knob| Self::hit(x, y, knob.rect()))
    }

    fn emit_once(cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(
            ptr,
            norm.clamp(0.0, 1.0),
        ));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }

    fn end_drag(&mut self, cx: &mut EventContext) {
        if let Some(drag) = self.drag.take() {
            cx.emit(RawParamEvent::EndSetParameter(
                self.param(drag.knob).as_ptr(),
            ));
        }
    }

    fn draw_faceplate(d: &mut Draw) {
        let rim = [(22.0, 92.0), (698.0, 92.0), (670.0, 516.0), (50.0, 516.0)];
        let face = [(30.0, 100.0), (690.0, 100.0), (662.0, 508.0), (58.0, 508.0)];
        d.fill_rounded_poly(&rim, 2.0, LINE);
        d.fill_poly(&face, PANEL);
        d.stroke_rounded_poly(&rim, 2.0, LINE, 1.0);
        d.stroke_rounded_poly(&face, 0.0, MUTED, 1.5);
        d.line(55.0, 296.0, 665.0, 296.0, MUTED, 2.0);
        d.line(567.0, 277.0, 511.0, 482.0, MUTED, 1.5);
    }

    fn draw_knob(&self, d: &mut Draw, knob: Knob) {
        let param = self.param(knob);
        let value = match knob {
            Knob::Volume | Knob::Tremolo | Knob::Speaker => {
                format!("{:.0}%", param.value() * 100.0)
            }
            _ => format!("{:.2}×", param.value()),
        };
        let color = match knob {
            Knob::Tremolo | Knob::TremoloResponse | Knob::HammerHardness => TEAL,
            _ => GOLD,
        };
        d.knob(
            knob.rect(),
            knob.label(),
            &value,
            param.unmodulated_normalized_value(),
            color,
            false,
        );
    }

    fn draw_switch(
        d: &mut Draw,
        rect: (f32, f32, f32, f32),
        label: &str,
        on: bool,
        state: &str,
        color: Color,
    ) {
        let (x, y, w, _) = rect;
        d.text_centered(x + w * 0.5, y + 10.0, label, 9.0, TEXT);
        d.rounded_rect(
            x + 18.0,
            y + 21.0,
            28.0,
            13.0,
            6.5,
            if on { color } else { LINE },
        );
        d.circle(x + if on { 39.5 } else { 24.5 }, y + 27.5, 4.5, TEXT, true);
        d.text(
            x + 55.0,
            y + 31.0,
            state,
            9.0,
            if on { color } else { MUTED },
        );
        if d.is_hovered(rect) {
            d.line(x + 18.0, y + 39.0, x + w - 13.0, y + 39.0, color, 1.0);
        }
    }
}

impl View for OpenWurliView {
    fn element(&self) -> Option<&'static str> {
        Some("openwurli-ui-view")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| {
            let bounds = cx.bounds();
            let Some(viewport) =
                EditorViewport::fit((bounds.x, bounds.y, bounds.w, bounds.h), (WIDTH, HEIGHT))
            else {
                return;
            };
            let (x, y) = viewport.to_local(cx.mouse().cursorx, cx.mouse().cursory);

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if Self::hit(x, y, THEME_BUTTON) {
                        prefs().toggle();
                    } else if Self::hit(x, y, CPU_BUTTON) {
                        self.end_drag(cx);
                        let next = if self.params.cpu_mode.value() == crate::params::CpuMode::Fast {
                            1.0
                        } else {
                            0.0
                        };
                        Self::emit_once(cx, self.params.cpu_mode.as_ptr(), next);
                    } else if Self::hit(x, y, EXTENDED_BUTTON) {
                        Self::emit_once(
                            cx,
                            self.params.extended_notes.as_ptr(),
                            if self.params.extended_notes.value() {
                                0.0
                            } else {
                                1.0
                            },
                        );
                    } else if Self::hit(x, y, SAG_BUTTON) {
                        let next = if self.params.rail_sag.value() {
                            0.0
                        } else {
                            1.0
                        };
                        Self::emit_once(cx, self.params.rail_sag.as_ptr(), next);
                    } else if let Some(knob) = self.knob_at(x, y) {
                        self.end_drag(cx);
                        let param = self.param(knob);
                        let ptr = param.as_ptr();
                        let start_norm = param.unmodulated_normalized_value();
                        cx.emit(RawParamEvent::BeginSetParameter(ptr));
                        self.drag = Some(Drag {
                            knob,
                            start_y: y,
                            start_norm,
                        });
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if let Some(knob) = self.knob_at(x, y) {
                        self.end_drag(cx);
                        let param = self.param(knob);
                        Self::emit_once(cx, param.as_ptr(), param.default_normalized_value());
                        cx.needs_redraw();
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((x, y));
                    if let Some(drag) = &self.drag {
                        let next = (drag.start_norm + (drag.start_y - y) / 130.0).clamp(0.0, 1.0);
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            self.param(drag.knob).as_ptr(),
                            next,
                        ));
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::FocusOut => {
                    self.end_drag(cx);
                    cx.needs_redraw();
                }
                _ => {}
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
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
            (WIDTH, HEIGHT),
            self.font.get(),
        ) else {
            return;
        };
        d.hover = self.hover;
        d.rounded_rect(0.0, 0.0, WIDTH, 74.0, 0.0, PANEL);
        d.rect(0.0, 73.0, WIDTH, 1.0, LINE);
        d.text(40.0, 45.0, "OPENWURLI", 25.0, GOLD);
        d.appearance_button(THEME_BUTTON, prefs().label());

        Self::draw_faceplate(&mut d);
        for knob in Knob::ALL {
            self.draw_knob(&mut d, knob);
        }
        let heavy = self.params.cpu_mode.value() == crate::params::CpuMode::Heavy;
        Self::draw_switch(
            &mut d,
            CPU_BUTTON,
            "CPU MODE",
            heavy,
            if heavy { "HEAVY" } else { "FAST" },
            GOLD,
        );
        let extended = self.params.extended_notes.value();
        Self::draw_switch(
            &mut d,
            EXTENDED_BUTTON,
            "EXTENDED NOTES",
            extended,
            if extended { "ON" } else { "OFF" },
            TEAL,
        );
        let sag = self.params.rail_sag.value();
        Self::draw_switch(
            &mut d,
            SAG_BUTTON,
            "EXTRA SAG",
            sag,
            if sag { "ON" } else { "OFF" },
            GOLD,
        );
        d.text(
            45.0,
            544.0,
            "DSP: HAL0ZER0 / OPENWURLI  •  GPL-3.0-OR-LATER",
            8.0,
            MUTED,
        );
    }
}

pub fn create(params: Arc<OpenWurliUiParams>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            OpenWurliView {
                params: params.clone(),
                font: Cell::new(None),
                drag: None,
                hover: None,
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(33), None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.needs_redraw();
                    }
                });
                cx.start_timer(timer);
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_controls_remain_reachable_together_in_short_hosts() {
        let view = OpenWurliView {
            params: Arc::new(OpenWurliUiParams::default()),
            font: Cell::new(None),
            drag: None,
            hover: None,
        };
        let buttons = [CPU_BUTTON, EXTENDED_BUTTON, SAG_BUTTON, THEME_BUTTON];
        for (width, height) in [(WIDTH, HEIGHT), (720.0, 340.0), (360.0, 550.0)] {
            let viewport = EditorViewport::fit((0.0, 0.0, width, height), (WIDTH, HEIGHT)).unwrap();
            for knob in Knob::ALL {
                let (x, y, w, h) = knob.rect();
                let screen_x = viewport.x + (x + w * 0.5) * viewport.scale;
                let screen_y = viewport.y + (y + h * 0.5) * viewport.scale;
                let local = viewport.to_local(screen_x, screen_y);
                assert_eq!(view.knob_at(local.0, local.1), Some(knob));
                assert!(viewport.x + (x + w) * viewport.scale <= width);
                assert!(viewport.y + (y + h) * viewport.scale <= height);
                for button in buttons {
                    assert!(
                        x + w < button.0
                            || x > button.0 + button.2
                            || y + h < button.1
                            || y > button.1 + button.3,
                        "{knob:?} overlaps a switch"
                    );
                }
            }
            for rect in buttons {
                let screen_x = viewport.x + (rect.0 + rect.2 * 0.5) * viewport.scale;
                let screen_y = viewport.y + (rect.1 + rect.3 * 0.5) * viewport.scale;
                let local = viewport.to_local(screen_x, screen_y);
                assert!(OpenWurliView::hit(local.0, local.1, rect));
                assert_eq!(view.knob_at(local.0, local.1), None);
                assert!(viewport.x + (rect.0 + rect.2) * viewport.scale <= width);
                assert!(viewport.y + (rect.1 + rect.3) * viewport.scale <= height);
            }
        }
    }
}
