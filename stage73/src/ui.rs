pub mod voicing;

use crate::{era::Era, params::StageParams};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport, TOOLTIP_HEIGHT},
    load_font_once, param,
    piano::Piano,
    playable_keyboard::{self, PlayableKeyboard},
    point_in_rect,
    preferences::Appearance,
    start_frame_timer,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    FONT_JETBRAINS_MONO,
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};
use voicing::{Part, Voicing};

pleasant_ui::appearance_store!("Stage 73");

const THEME: (f32, f32, f32, f32) = (578., 11., 80., 30.);
const TREMOLO: (f32, f32, f32, f32) = (306., 290., 149., 27.);
const KEYS: (f32, f32, f32, f32) = (22.0, 440.0, 636.0, 60.0);
const PIANO: Piano = Piano::new(KEYS, crate::engine::LOW, crate::engine::HIGH);
pub(super) const SIZE: (f32, f32) = (680.0, KEYS.1 + KEYS.3 + TOOLTIP_HEIGHT);
const LABELS: [&str; 9] = [
    "VOLUME",
    "BASS BOOST",
    "TINE HEIGHT",
    "PICKUP GAP",
    "BASS",
    "TREBLE",
    "RATE",
    "DEPTH",
    "VOLUME",
];
const STAGE_KNOBS: &[usize] = &[0, 1, 2, 3];
const JORDAN_KNOBS: &[usize] = &[8, 4, 5, 6, 7, 2, 3];
const ERA_LABELS: [&str; 6] = [
    "Sparkletop",
    "Early I",
    "Mid I",
    "Late I",
    "Mark II",
    "Mark V",
];

fn era_rect(i: usize) -> (f32, f32, f32, f32) {
    if i == 0 {
        (152., 11., 80., 30.)
    } else {
        (235. + 67. * (i - 1) as f32, 11., 64., 30.)
    }
}
fn era_at(x: f32, y: f32) -> Option<Era> {
    Era::ALL
        .into_iter()
        .find(|era| point_in_rect(x, y, era_rect(era.to_index())))
}
const HELP: [&str; 9] = [
    "Set the volume at the Stage output jack.",
    "Turn clockwise to restore the full bass response.",
    "Offset every tine vertically. Zero restores each key’s calibrated height.",
    "Offset every pickup along the tine axis. Zero restores each key’s calibrated gap.",
    "Set the bass on the Jordan preamp. The centre setting balances the tone network.",
    "Set the treble on the Jordan preamp. The centre setting balances the tone network.",
    "Set the speed of the mono Suitcase tremolo.",
    "Set the intensity of the mono Suitcase tremolo.",
    "Set the loudness pot at the loaded Jordan preamp output.",
];

fn rect(i: usize, era: Era) -> (f32, f32, f32, f32) {
    if era == Era::Sparkletop {
        let slot = JORDAN_KNOBS.iter().position(|&k| k == i).unwrap();
        let (left, width, column) = match slot {
            0..=2 => (24., 86., slot),
            3..=4 => (294., 86., slot - 3),
            _ => (479., 88., slot - 5),
        };
        return (left + width * column as f32, 318., width, 103.);
    }
    (
        32.0 + 152.0 * i as f32 + if i >= 2 { 18.0 } else { 0.0 },
        318.0,
        143.0,
        103.0,
    )
}

#[derive(Clone, Copy)]
enum Target {
    Knob(usize),
    Model(Part),
}

impl Target {
    fn index(self) -> usize {
        match self {
            Self::Knob(i) => i,
            Self::Model(Part::Tine) => 2,
            Self::Model(Part::Pickup) => 3,
        }
    }

    fn help(self, era: Era) -> &'static str {
        match self {
            Self::Knob(1) if era == Era::EarlyI => {
                "Turn down to soften the treble through the early Stage tone circuit."
            }
            Self::Knob(1) if era == Era::MarkV => {
                "Adjust the provisional passive Mark V tone response."
            }
            Self::Knob(i) => HELP[i],
            Self::Model(Part::Tine) => {
                "Drag the tine up or down to change its height. Double-click to reset."
            }
            Self::Model(Part::Pickup) => {
                "Drag the pickup left to close the gap or right to open it. Double-click to reset."
            }
        }
    }
}

struct Drag {
    target: Target,
    start: (f32, f32),
    normalized: f32,
}

impl Drag {
    fn value_at(&self, p: &FloatParam, x: f32, y: f32) -> f32 {
        match self.target {
            Target::Knob(_) => self.normalized + (self.start.1 - y) / 140.0,
            Target::Model(part) => p.preview_normalized(
                p.preview_plain(self.normalized) + part.drag_mm(self.start, (x, y)),
            ),
        }
        .clamp(0.0, 1.0)
    }
}

struct View73 {
    keyboard: Rc<PlayableKeyboard>,
    params: Arc<StageParams>,
    font: Cell<Option<FontId>>,
    label_font: Cell<Option<FontId>>,
    hover: Option<(f32, f32)>,
    drag: Option<Drag>,
}

impl View73 {
    fn knob(&self, i: usize) -> &FloatParam {
        [
            &self.params.volume,
            &self.params.bass,
            &self.params.height,
            &self.params.gap,
            &self.params.jordan_bass,
            &self.params.treble,
            &self.params.rate,
            &self.params.depth,
            &self.params.jordan_volume,
        ][i]
    }

    fn voicing(&self) -> Voicing {
        Voicing::new(
            self.params.era.value(),
            self.params.height.value(),
            self.params.gap.value(),
        )
    }

    fn hit(&self, x: f32, y: f32) -> Option<Target> {
        let era = self.params.era.value();
        let knobs = if era == Era::Sparkletop {
            JORDAN_KNOBS
        } else {
            STAGE_KNOBS
        };
        knobs
            .iter()
            .copied()
            .find(|&i| point_in_rect(x, y, rect(i, era)))
            .map(Target::Knob)
            .or_else(|| self.voicing().hit(x, y).map(Target::Model))
    }

    fn end(&mut self, cx: &mut EventContext) {
        if let Some(drag) = self.drag.take() {
            param::end(cx, self.knob(drag.target.index()).as_ptr());
            cx.release();
            cx.needs_redraw();
        }
    }

    fn paint(&self, d: &mut Draw, appearance: Appearance) {
        let label_font = load_font_once(
            d.c,
            &self.label_font,
            nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
        );
        d.with_readable_style(label_font, |d| self.paint_content(d, appearance));
    }

    fn paint_content(&self, d: &mut Draw, appearance: Appearance) {
        d.hover = self.hover;
        d.square_rect(0.0, 0.0, SIZE.0, 51.0, PANEL);
        d.square_rect(0., 50., SIZE.0, 1., LINE);
        d.text(22.0, 35.0, "STAGE 73", 25.0, GOLD);
        d.appearance_button(THEME, appearance.label());
        let era = self.params.era.value();
        for (i, label) in ERA_LABELS.iter().enumerate() {
            d.button(era_rect(i), label, Era::ALL[i] == era, GOLD);
        }
        let target = self
            .drag
            .as_ref()
            .map(|drag| drag.target)
            .or_else(|| self.hover.and_then(|(x, y)| self.hit(x, y)));
        let active = target.and_then(|t| match t.index() {
            2 => Some(Part::Tine),
            3 => Some(Part::Pickup),
            _ => None,
        });
        self.voicing().draw(d, active);
        let spark = era == Era::Sparkletop;
        let split = if spark { 477. } else { 344. };
        for (x, w, title, color) in [
            (
                22.,
                if spark { 262. } else { split - 30. },
                if spark { "JORDAN PREAMP" } else { "OUTPUT" },
                GOLD,
            ),
            (split, 658. - split, "VOICING OFFSETS", TEAL),
        ] {
            d.panel((x, 287., w, 143.), PANEL, LINE);
            d.text(
                x + 14.,
                308.,
                title,
                11.,
                if d.light { TEXT } else { color },
            );
        }
        let tremolo_on = self.params.tremolo.value();
        if spark {
            d.panel((292., 287., 177., 143.), PANEL, LINE);
            d.toggle_switch(TREMOLO, "TREMOLO", tremolo_on, TEAL);
        }
        for &i in if spark { JORDAN_KNOBS } else { STAGE_KNOBS } {
            let label = if i == 1 && matches!(era, Era::EarlyI | Era::MarkV) {
                "TONE"
            } else {
                LABELS[i]
            };
            let p = self.knob(i);
            let normalized = p.unmodulated_normalized_value();
            let value = p.normalized_value_to_string(normalized, true);
            let tremolo_knob = matches!(i, 6 | 7);
            d.knob(
                rect(i, era),
                label,
                &value,
                normalized,
                if tremolo_knob || matches!(i, 2 | 3) {
                    TEAL
                } else {
                    GOLD
                },
                tremolo_knob && !tremolo_on,
            );
        }
        self.keyboard.draw(d, PIANO);
        let help = if let Some(target) = target {
            target.help(era)
        } else if d.is_hovered(THEME) {
            "Cycle Dark, Light and Auto. Auto follows the system appearance."
        } else if spark && d.is_hovered(TREMOLO) {
            "Switch mono tremolo on or off. Rate and depth retain their settings."
        } else if let Some(hovered) = self.hover.and_then(|(x, y)| era_at(x, y)) {
            hovered.help()
        } else if self.hover.is_some_and(|(x, y)| point_in_rect(x, y, KEYS)) {
            playable_keyboard::HELP
        } else {
            era.help()
        };
        d.bottom_tooltip(
            (24.0, SIZE.1 - TOOLTIP_HEIGHT, 632.0, TOOLTIP_HEIGHT),
            help,
            MUTED,
        );
    }
}

impl View for View73 {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| {
            let b = cx.bounds();
            let Some(v) = EditorViewport::fit((b.x, b.y, b.w, b.h), SIZE) else {
                return;
            };
            let (x, y) = v.to_local(cx.mouse().cursorx, cx.mouse().cursory);
            self.keyboard.event(cx, e, PIANO, x, y);
            match e {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    self.end(cx);
                    if point_in_rect(x, y, THEME) {
                        prefs().toggle();
                    } else if self.params.era.value() == Era::Sparkletop
                        && point_in_rect(x, y, TREMOLO)
                    {
                        let p = &self.params.tremolo;
                        param::set_normalized_once(
                            cx,
                            p.as_ptr(),
                            p.preview_normalized(!p.value()),
                        );
                    } else if let Some(era) = era_at(x, y) {
                        let p = &self.params.era;
                        param::set_normalized_once(cx, p.as_ptr(), p.preview_normalized(era));
                    } else if let Some(target) = self.hit(x, y) {
                        let p = self.knob(target.index());
                        param::begin(cx, p.as_ptr());
                        self.drag = Some(Drag {
                            target,
                            start: (x, y),
                            normalized: p.unmodulated_normalized_value(),
                        });
                        cx.capture();
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if let Some(target) = self.hit(x, y) {
                        self.end(cx);
                        let p = self.knob(target.index());
                        param::set_normalized_once(cx, p.as_ptr(), p.default_normalized_value());
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((x, y));
                    if let Some(drag) = &self.drag {
                        let p = self.knob(drag.target.index());
                        param::set_normalized(cx, p.as_ptr(), drag.value_at(p, x, y));
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::FocusOut => self.end(cx),
                _ => {}
            }
        })
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let b = cx.bounds();
        load_font_once(canvas, &self.font, FONT_JETBRAINS_MONO);
        let appearance = prefs().mode();
        if let Some(mut d) = Draw::new_fitted(
            canvas,
            appearance.resolved() == Appearance::Light,
            (b.x, b.y, b.w, b.h),
            SIZE,
            self.font.get(),
        ) {
            self.paint(&mut d, appearance);
        }
    }
}

pub fn create(params: Arc<StageParams>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, gui| {
            let poll = params.clone();
            let previous = Cell::new(None);
            let keyboard = Rc::new(PlayableKeyboard::new(params.keyboard.clone(), move || {
                gui.request_process()
            }));
            View73 {
                keyboard: keyboard.clone(),
                params: params.clone(),
                font: Cell::new(None),
                label_font: Cell::new(None),
                hover: None,
                drag: None,
            }
            .build(cx, |cx| {
                start_frame_timer(cx, Duration::from_millis(33), move |cx| {
                    let appearance = prefs().mode();
                    let c = (
                        poll.controls(),
                        poll.tremolo.value(),
                        poll.depth.value(),
                        appearance,
                        appearance.resolved(),
                    );
                    let notes_changed = keyboard.poll();
                    if previous.replace(Some(c)) != Some(c) || notes_changed {
                        cx.needs_redraw();
                    }
                });
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        },
    )
}

#[cfg(all(test, target_os = "macos"))]
#[test]
#[ignore = "Renders the editor to a PPM in the temporary directory"]
fn render_voicing_preview() {
    let mut gl = pleasant_ui::preview::OffscreenGl::new();
    for (era, appearance, name) in [
        (Era::LateI, Appearance::Dark, "stage73-voicing.ppm"),
        (Era::LateI, Appearance::Light, "stage73-voicing-light.ppm"),
        (Era::Sparkletop, Appearance::Dark, "stage73-sparkletop.ppm"),
        (
            Era::Sparkletop,
            Appearance::Light,
            "stage73-sparkletop-light.ppm",
        ),
    ] {
        let view = View73 {
            keyboard: Rc::new(PlayableKeyboard::new(Arc::default(), || {})),
            params: Arc::new(StageParams {
                tone: crate::params::StageTone {
                    era: EnumParam::new("Era", era),
                    tremolo: BoolParam::new("Tremolo", appearance == Appearance::Light),
                    ..Default::default()
                },
                ..Default::default()
            }),
            font: Cell::new(None),
            label_font: Cell::new(None),
            hover: None,
            drag: None,
        };
        load_font_once(gl.canvas_mut(), &view.font, FONT_JETBRAINS_MONO);
        let (w, h) = (SIZE.0 * 2.0, SIZE.1 * 2.0);
        let path = std::env::temp_dir().join(name);
        gl.render_ppm(w as u32, h as u32, path, |canvas| {
            let mut d = Draw::new_fitted(
                canvas,
                appearance == Appearance::Light,
                (0.0, 0.0, w, h),
                SIZE,
                view.font.get(),
            )
            .unwrap();
            view.paint(&mut d, appearance);
        });
    }
}
