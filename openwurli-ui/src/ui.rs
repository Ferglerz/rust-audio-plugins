use crate::params::OpenWurliUiParams;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    widgets::RawParamEvent,
    ViziaTheming,
};
use pleasant_ui::{
    draw::Draw,
    preferences::AppearanceStore,
    theme::{BG, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{Arc, OnceLock},
    time::Duration,
};

const WIDTH: f32 = 720.0;
const HEIGHT: f32 = 340.0;
const KNOB_Y: f32 = 95.0;
const KNOB_W: f32 = 198.0;
const KNOB_H: f32 = 168.0;
const KNOB_GAP: f32 = 18.0;
const KNOB_X: f32 = 45.0;
const MLP_BUTTON: (f32, f32, f32, f32) = (45.0, 284.0, 238.0, 36.0);
const THEME_BUTTON: (f32, f32, f32, f32) = (601.0, 21.0, 80.0, 30.0);
const COG_BUTTON: (f32, f32, f32, f32) = (541.0, 21.0, 32.0, 30.0);
const ENGINE_BUTTON: (f32, f32, f32, f32) = (426.0, 21.0, 94.0, 30.0);
const NOISE_BUTTON: (f32, f32, f32, f32) = (375.0, 241.0, 144.0, 50.0);
const SAG_BUTTON: (f32, f32, f32, f32) = (531.0, 241.0, 144.0, 50.0);

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
    NoiseGain,
}

impl Knob {
    const MAIN: [Self; 3] = [Self::Volume, Self::Tremolo, Self::Speaker];
    const ADVANCED: [Self; 5] = [
        Self::ReedDecay,
        Self::HammerHardness,
        Self::PickupDrive,
        Self::TremoloResponse,
        Self::NoiseGain,
    ];

    fn rect(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Volume => (KNOB_X, KNOB_Y, KNOB_W, KNOB_H),
            Self::Tremolo => (KNOB_X + KNOB_W + KNOB_GAP, KNOB_Y, KNOB_W, KNOB_H),
            Self::Speaker => (KNOB_X + 2.0 * (KNOB_W + KNOB_GAP), KNOB_Y, KNOB_W, KNOB_H),
            Self::ReedDecay => (
                45.0,
                101.0,
                300.0,
                57.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
            Self::HammerHardness => (
                375.0,
                101.0,
                300.0,
                57.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
            Self::PickupDrive => (
                45.0,
                171.0,
                300.0,
                57.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
            Self::TremoloResponse => (
                375.0,
                171.0,
                300.0,
                57.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
            Self::NoiseGain => (
                45.0,
                241.0,
                300.0,
                50.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        }
    }

    fn is_advanced(self) -> bool {
        !Self::MAIN.contains(&self)
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
    show_advanced: bool,
    show_engine: bool,
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
            Knob::NoiseGain => &self.params.noise_gain,
        }
    }

    fn hit(x: f32, y: f32, rect: (f32, f32, f32, f32)) -> bool {
        x >= rect.0 && x <= rect.0 + rect.2 && y >= rect.1 && y <= rect.1 + rect.3
    }

    fn knob_at(&self, x: f32, y: f32) -> Option<Knob> {
        if self.show_engine {
            return None;
        }
        let choices: &[Knob] = if self.show_advanced {
            &Knob::ADVANCED
        } else {
            &Knob::MAIN
        };
        choices
            .iter()
            .copied()
            .find(|knob| Self::hit(x, y, knob.rect()))
    }

    fn slider_norm(knob: Knob, x: f32) -> f32 {
        let rect = knob.rect();
        ((x - (rect.0 + 12.0)) / (rect.2 - 24.0)).clamp(0.0, 1.0)
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

    fn draw_advanced_slider(&self, d: &mut Draw, knob: Knob) {
        let (label, color) = match knob {
            Knob::ReedDecay => ("REED DECAY", GOLD),
            Knob::HammerHardness => ("HAMMER HARDNESS", TEAL),
            Knob::PickupDrive => ("PICKUP DRIVE", GOLD),
            Knob::TremoloResponse => ("TREMOLO RESPONSE", TEAL),
            Knob::NoiseGain => ("NOISE LEVEL", MUTED),
            _ => return,
        };
        let rect = knob.rect();
        let param = self.param(knob);
        let norm = param.unmodulated_normalized_value();
        let value = if knob == Knob::NoiseGain {
            format!("{:.1}×", param.value())
        } else {
            format!("{:.2}×", param.value())
        };
        d.rect(rect.0, rect.1, rect.2, rect.3, PANEL);
        d.outline(rect, LINE);
        d.text(rect.0 + 12.0, rect.1 + 20.0, label, 11.0, TEXT);
        d.text_right(rect.0 + rect.2 - 12.0, rect.1 + 20.0, &value, 12.0, color);
        let bar_x = rect.0 + 12.0;
        let bar_y = rect.1 + rect.3 - 13.0;
        let bar_w = rect.2 - 24.0;
        d.rect(bar_x, bar_y, bar_w, 5.0, LINE);
        d.rect(bar_x, bar_y, bar_w * norm, 5.0, color);
        d.circle(bar_x + bar_w * norm, bar_y + 2.5, 5.0, color, true);
    }
}

impl View for OpenWurliView {
    fn element(&self) -> Option<&'static str> {
        Some("openwurli-ui-view")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| {
            let bounds = cx.bounds();
            if bounds.w <= 0.0 {
                return;
            }
            let scale = bounds.w / WIDTH;
            let x = (cx.mouse().cursorx - bounds.x) / scale;
            let y = (cx.mouse().cursory - bounds.y) / scale;

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if Self::hit(x, y, THEME_BUTTON) {
                        prefs().toggle();
                    } else if Self::hit(x, y, ENGINE_BUTTON) {
                        self.end_drag(cx);
                        self.show_engine = !self.show_engine;
                    } else if Self::hit(x, y, COG_BUTTON) {
                        self.end_drag(cx);
                        self.show_advanced = self.show_engine || !self.show_advanced;
                        self.show_engine = false;
                    } else if self.show_engine {
                        // The information page has no sound controls.
                    } else if !self.show_advanced && Self::hit(x, y, MLP_BUTTON) {
                        let next = if self.params.mlp_enabled.value() {
                            0.0
                        } else {
                            1.0
                        };
                        Self::emit_once(cx, self.params.mlp_enabled.as_ptr(), next);
                    } else if self.show_advanced && Self::hit(x, y, NOISE_BUTTON) {
                        let next = if self.params.noise_enabled.value() {
                            0.0
                        } else {
                            1.0
                        };
                        Self::emit_once(cx, self.params.noise_enabled.as_ptr(), next);
                    } else if self.show_advanced && Self::hit(x, y, SAG_BUTTON) {
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
                        if knob.is_advanced() {
                            cx.emit(RawParamEvent::SetParameterNormalized(
                                ptr,
                                Self::slider_norm(knob, x),
                            ));
                        }
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
                    if let Some(drag) = &self.drag {
                        let next = if drag.knob.is_advanced() {
                            Self::slider_norm(drag.knob, x)
                        } else {
                            (drag.start_norm + (drag.start_y - y) / 130.0).clamp(0.0, 1.0)
                        };
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            self.param(drag.knob).as_ptr(),
                            next,
                        ));
                        cx.needs_redraw();
                    }
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
        let mut d = Draw::new(
            canvas,
            prefs().light(),
            bounds.w / WIDTH,
            bounds.x,
            bounds.y,
            self.font.get(),
        );
        d.rect(0.0, 0.0, WIDTH, HEIGHT, BG);
        d.rect(0.0, 0.0, WIDTH, 74.0, PANEL);
        d.rect(0.0, 73.0, WIDTH, 1.0, LINE);
        d.text(40.0, 45.0, "OPENWURLI", 25.0, GOLD);
        d.text(230.0, 45.0, "PLEASANT UI", 12.0, MUTED);
        d.rect(
            COG_BUTTON.0,
            COG_BUTTON.1,
            COG_BUTTON.2,
            COG_BUTTON.3,
            PANEL,
        );
        d.outline(COG_BUTTON, if self.show_advanced { GOLD } else { LINE });
        d.cog_icon(
            COG_BUTTON.0 + COG_BUTTON.2 * 0.5,
            COG_BUTTON.1 + COG_BUTTON.3 * 0.5,
            if self.show_advanced { GOLD } else { MUTED },
        );
        d.appearance_button(THEME_BUTTON, prefs().label());
        d.button(ENGINE_BUTTON, "ENGINE", self.show_engine, TEAL);

        if self.show_engine {
            use crate::engine_info;
            d.text(45.0, 103.0, engine_info::TITLE, 15.0, GOLD);
            d.text(45.0, 125.0, engine_info::MODEL, 11.0, MUTED);
            for (index, line) in engine_info::CHANGES.iter().enumerate() {
                d.text(45.0, 157.0 + index as f32 * 21.0, line, 11.0, TEXT);
            }
            d.rect(45.0, 218.0, 630.0, 1.0, LINE);
            d.text(45.0, 244.0, engine_info::CPU_RESULT, 12.0, TEAL);
            d.text(45.0, 268.0, engine_info::AUDIO_RESULT, 12.0, GOLD);
            d.text(45.0, 307.0, engine_info::CONDITIONS, 10.0, MUTED);
        } else if self.show_advanced {
            for knob in Knob::ADVANCED {
                self.draw_advanced_slider(&mut d, knob);
            }
            d.button(
                NOISE_BUTTON,
                "HISS",
                self.params.noise_enabled.value(),
                TEAL,
            );
            d.button(SAG_BUTTON, "SAG", self.params.rail_sag.value(), GOLD);
            d.text(
                45.0,
                318.0,
                "VOICING: NEW NOTES  /  EFFECTS: LIVE",
                10.0,
                MUTED,
            );
        } else {
            for knob in Knob::MAIN {
                let param = self.param(knob);
                let label = match knob {
                    Knob::Volume => "VOLUME",
                    Knob::Tremolo => "TREMOLO DEPTH",
                    Knob::Speaker => "SPEAKER CHARACTER",
                    _ => unreachable!(),
                };
                let color = match knob {
                    Knob::Volume => GOLD,
                    Knob::Tremolo => TEAL,
                    Knob::Speaker => GOLD,
                    _ => unreachable!(),
                };
                let value = format!("{:.0}%", param.value() * 100.0);
                let r = knob.rect();
                d.rect(r.0, r.1, r.2, r.3, PANEL);
                d.outline(r, LINE);
                d.knob(
                    r,
                    label,
                    &value,
                    param.unmodulated_normalized_value(),
                    color,
                    false,
                );
            }
            d.button(
                MLP_BUTTON,
                "MLP CORRECTIONS",
                self.params.mlp_enabled.value(),
                TEAL,
            );
            d.text(307.0, 307.0, "PHYSICAL MODEL  /  64 VOICES", 11.0, TEXT);
        }
        d.text(
            45.0,
            334.0,
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
                show_advanced: false,
                show_engine: false,
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
