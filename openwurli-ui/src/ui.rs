use crate::params::OpenWurliUiParams;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
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
const KNOB_Y: f32 = 95.0;
const KNOB_W: f32 = 198.0;
const KNOB_H: f32 = 168.0;
const KNOB_GAP: f32 = 18.0;
const KNOB_X: f32 = 45.0;
const CPU_BUTTON: (f32, f32, f32, f32) = (45.0, 477.0, 172.0, 36.0);
const THEME_BUTTON: (f32, f32, f32, f32) = (601.0, 21.0, 80.0, 30.0);
const COG_BUTTON: (f32, f32, f32, f32) = (541.0, 21.0, 32.0, 30.0);
const EXTENDED_BUTTON: (f32, f32, f32, f32) = (45.0, 187.0, 238.0, 50.0);
const SAG_BUTTON: (f32, f32, f32, f32) = (531.0, 187.0, 144.0, 50.0);

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
    const MAIN: [Self; 6] = [
        Self::Volume,
        Self::Tremolo,
        Self::Speaker,
        Self::ReedDecay,
        Self::HammerHardness,
        Self::PickupDrive,
    ];
    const ADVANCED: [Self; 1] = [Self::TremoloResponse];

    fn rect(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Volume => (KNOB_X, KNOB_Y, KNOB_W, KNOB_H),
            Self::Tremolo => (KNOB_X + KNOB_W + KNOB_GAP, KNOB_Y, KNOB_W, KNOB_H),
            Self::Speaker => (KNOB_X + 2.0 * (KNOB_W + KNOB_GAP), KNOB_Y, KNOB_W, KNOB_H),
            Self::ReedDecay => (KNOB_X, KNOB_Y + KNOB_H + KNOB_GAP, KNOB_W, KNOB_H),
            Self::HammerHardness => (
                KNOB_X + KNOB_W + KNOB_GAP,
                KNOB_Y + KNOB_H + KNOB_GAP,
                KNOB_W,
                KNOB_H,
            ),
            Self::PickupDrive => (
                KNOB_X + 2.0 * (KNOB_W + KNOB_GAP),
                KNOB_Y + KNOB_H + KNOB_GAP,
                KNOB_W,
                KNOB_H,
            ),
            Self::TremoloResponse => (
                45.0,
                101.0,
                630.0,
                57.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
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
    hover: Option<(f32, f32)>,
    show_advanced: bool,
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
            Knob::TremoloResponse => ("TREMOLO RESPONSE", TEAL),
            _ => return,
        };
        let rect = knob.rect();
        let param = self.param(knob);
        let norm = param.unmodulated_normalized_value();
        let value = format!("{:.2}×", param.value());
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
                    } else if Self::hit(x, y, COG_BUTTON) {
                        self.end_drag(cx);
                        self.show_advanced = !self.show_advanced;
                    } else if !self.show_advanced && Self::hit(x, y, CPU_BUTTON) {
                        self.end_drag(cx);
                        let next = if self.params.cpu_mode.value() == crate::params::CpuMode::Fast {
                            1.0
                        } else {
                            0.0
                        };
                        Self::emit_once(cx, self.params.cpu_mode.as_ptr(), next);
                    } else if self.show_advanced && Self::hit(x, y, EXTENDED_BUTTON) {
                        Self::emit_once(
                            cx,
                            self.params.extended_notes.as_ptr(),
                            if self.params.extended_notes.value() {
                                0.0
                            } else {
                                1.0
                            },
                        );
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
                    self.hover = Some((x, y));
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

        if self.show_advanced {
            for knob in Knob::ADVANCED {
                self.draw_advanced_slider(&mut d, knob);
            }
            d.button(
                EXTENDED_BUTTON,
                "EXTENDED NOTES",
                self.params.extended_notes.value(),
                TEAL,
            );
            d.button(SAG_BUTTON, "EXTRA SAG", self.params.rail_sag.value(), GOLD);
            for (index, line) in [
                "Learned voicing is always active.",
                "Uses recorded piano references to adjust overtone tuning, decay",
                "and pickup response on new notes.",
            ]
            .iter()
            .enumerate()
            {
                d.text(45.0, 278.0 + index as f32 * 22.0, line, 11.0, TEXT);
            }
            d.text(
                45.0,
                354.0,
                "TREMOLO RESPONSE AND EXTRA SAG APPLY LIVE",
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
                    Knob::ReedDecay => "REED DECAY",
                    Knob::HammerHardness => "HAMMER HARDNESS",
                    Knob::PickupDrive => "PICKUP DRIVE",
                    _ => unreachable!(),
                };
                let color = match knob {
                    Knob::Volume | Knob::Speaker | Knob::ReedDecay | Knob::PickupDrive => GOLD,
                    Knob::Tremolo | Knob::HammerHardness => TEAL,
                    _ => unreachable!(),
                };
                let value = match knob {
                    Knob::ReedDecay | Knob::HammerHardness | Knob::PickupDrive => {
                        format!("{:.2}×", param.value())
                    }
                    _ => format!("{:.0}%", param.value() * 100.0),
                };
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
            d.text(
                45.0,
                466.0,
                "VOICING CONTROLS APPLY TO NEW NOTES",
                9.0,
                MUTED,
            );
            let heavy = self.params.cpu_mode.value() == crate::params::CpuMode::Heavy;
            d.button_tinted(
                CPU_BUTTON,
                if heavy { "CPU: HEAVY" } else { "CPU: FAST" },
                heavy,
                GOLD,
            );
            d.text(573.0, 500.0, "64 VOICES", 11.0, TEXT);
        }
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
                show_advanced: false,
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
    fn promoted_knobs_remain_reachable_in_short_hosts_and_settings_hide_them() {
        let mut view = OpenWurliView {
            params: Arc::new(OpenWurliUiParams::default()),
            font: Cell::new(None),
            drag: None,
            hover: None,
            show_advanced: false,
        };
        let viewport = EditorViewport::fit((0.0, 0.0, 720.0, 340.0), (WIDTH, HEIGHT)).unwrap();
        for knob in Knob::MAIN {
            let (x, y, w, h) = knob.rect();
            let screen_x = viewport.x + (x + w * 0.5) * viewport.scale;
            let screen_y = viewport.y + (y + h * 0.5) * viewport.scale;
            assert!(screen_y < 340.0);
            let local = viewport.to_local(screen_x, screen_y);
            assert_eq!(view.knob_at(local.0, local.1), Some(knob));
            assert!(
                !knob.is_advanced(),
                "promoted knobs must use vertical dragging"
            );
            assert!(y + h < CPU_BUTTON.1);
        }
        view.show_advanced = true;
        for knob in [Knob::ReedDecay, Knob::HammerHardness, Knob::PickupDrive] {
            let (x, y, w, h) = knob.rect();
            assert_eq!(view.knob_at(x + w * 0.5, y + h * 0.5), None);
        }
        let (x, y, w, h) = Knob::TremoloResponse.rect();
        assert_eq!(
            view.knob_at(x + w * 0.5, y + h * 0.5),
            Some(Knob::TremoloResponse)
        );
        assert!(y + h < EXTENDED_BUTTON.1);
        assert!(y + h < SAG_BUTTON.1);
    }
}
