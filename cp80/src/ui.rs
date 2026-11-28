pub mod string_action;

use crate::{
    engine::{HIGH, LOW},
    plugin::{Brilliance, Parameters},
};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport, TOOLTIP_HEIGHT},
    instrument_action::StrikePreview,
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

pleasant_ui::appearance_store!("Electric Grand 80");

type Rect = (f32, f32, f32, f32);
const PANEL_Y: f32 = 267.;
const THEME: Rect = (632., 11., 80., 30.);
const TREMOLO: Rect = (534., PANEL_Y + 3., 164., 27.);
const KEYS: Rect = (16., PANEL_Y + 154., 696., 64.);
pub(crate) const SIZE: (f32, f32) = (728., KEYS.1 + KEYS.3 + TOOLTIP_HEIGHT);
const PIANO: Piano = Piano::new(KEYS, LOW, HIGH);
const LABELS: [&str; 6] = ["VOLUME", "BASS", "MIDDLE", "TREBLE", "SPEED", "DEPTH"];
const HELP: [&str; 6] = [
    "Set the output level at the stereo jacks.",
    "Adjust the bass pot in the preamp tone network.",
    "Set the midrange response of the tone circuit.",
    "Adjust the treble pot before the brilliance switch.",
    "Set the speed of the free-running stereo tremolo.",
    "Set how far the tremolo moves between channels.",
];
const BR: [Brilliance; 3] = [Brilliance::Low, Brilliance::Medium, Brilliance::High];
fn knob(i: usize) -> Rect {
    if i < 4 {
        (22. + i as f32 * 82., PANEL_Y + 33., 78., 102.)
    } else {
        (525. + (i - 4) as f32 * 91., PANEL_Y + 33., 91., 102.)
    }
}
fn brilliance_rect(i: usize) -> Rect {
    (357. + i as f32 * 49., PANEL_Y + 72., 46., 26.)
}
struct PanelView {
    params: Arc<Parameters>,
    keyboard: Rc<PlayableKeyboard>,
    strike: Rc<StrikePreview>,
    font: Cell<Option<FontId>>,
    label_font: Cell<Option<FontId>>,
    hover: Option<(f32, f32)>,
    drag: Option<(usize, f32, f32)>,
}
impl PanelView {
    fn pot(&self, i: usize) -> &FloatParam {
        [
            &self.params.volume,
            &self.params.bass,
            &self.params.middle,
            &self.params.treble,
            &self.params.speed,
            &self.params.depth,
        ][i]
    }
    fn end(&mut self, cx: &mut EventContext) {
        if let Some((i, _, _)) = self.drag.take() {
            param::end(cx, self.pot(i).as_ptr());
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
        d.square_rect(0., 0., SIZE.0, 51., PANEL);
        d.square_rect(0., 50., SIZE.0, 1., LINE);
        d.text(22., 34., "ELECTRIC GRAND 80", 24., GOLD);
        d.appearance_button(THEME, appearance.label());
        let action = string_action::StringAction::new(&self.strike);
        let part = self.hover.and_then(|(x, y)| action.hit(x, y));
        action.draw(d, part);
        d.panel((16., PANEL_Y, 496., 143.), PANEL, LINE);
        d.text(
            30.,
            PANEL_Y + 21.,
            "OUTPUT & TONE",
            11.,
            if d.light { TEXT } else { GOLD },
        );
        d.panel((520., PANEL_Y, 192., 143.), PANEL, LINE);
        let tremolo_on = self.params.tremolo.value();
        d.toggle_switch(TREMOLO, "TREMOLO", tremolo_on, TEAL);
        for (i, label) in LABELS.iter().enumerate() {
            let p = self.pot(i);
            let v = p.unmodulated_normalized_value();
            let tremolo_knob = matches!(i, 4 | 5);
            d.knob(
                knob(i),
                label,
                &p.normalized_value_to_string(v, true),
                v,
                if tremolo_knob { TEAL } else { GOLD },
                tremolo_knob && !tremolo_on,
            );
        }
        d.with_label_font(|d| d.text_centered(429., PANEL_Y + 47., "BRILLIANCE", 11., TEXT));
        for (i, label) in ["LOW", "MED", "HIGH"].iter().enumerate() {
            d.button(
                brilliance_rect(i),
                label,
                self.params.brilliance.value() == BR[i],
                GOLD,
            );
        }
        self.keyboard.draw(d, PIANO);
        let mut help = part.map_or(
            "Play a note to see a slowed-down strike in its register.",
            |p| p.help(),
        );
        if let Some((x, y)) = self.hover {
            if point_in_rect(x, y, KEYS) {
                help = playable_keyboard::HELP;
            }
            if point_in_rect(x, y, THEME) {
                help = "Cycle Dark, Light and Auto. Auto follows the system appearance.";
            }
            if (0..3).any(|i| point_in_rect(x, y, brilliance_rect(i))) {
                help = "Switch the high-frequency network between Low, Medium and High.";
            }
            if point_in_rect(x, y, TREMOLO) {
                help = "Switch stereo tremolo on or off. Speed and depth retain their settings.";
            }
            if let Some(i) = (0..6).find(|i| point_in_rect(x, y, knob(*i))) {
                help = HELP[i];
            }
        }
        if let Some((i, _, _)) = self.drag {
            help = HELP[i];
        }
        d.bottom_tooltip(
            (18., SIZE.1 - TOOLTIP_HEIGHT, 692., TOOLTIP_HEIGHT),
            help,
            MUTED,
        );
    }
}
impl View for PanelView {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| {
            let b = cx.bounds();
            let Some(v) = EditorViewport::fit((b.x, b.y, b.w, b.h), SIZE) else {
                return;
            };
            let (x, y) = v.to_local(cx.mouse().cursorx, cx.mouse().cursory);
            self.keyboard.event_for_note(cx, e, || {
                PIANO.note_at(x, y).or_else(|| {
                    (string_action::StringAction::new(&self.strike).hit(x, y)
                        == Some(string_action::Part::Hammer))
                    .then(|| self.strike.note())
                })
            });
            let hit = (0..6).find(|i| point_in_rect(x, y, knob(*i)));
            match e {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    self.end(cx);
                    if point_in_rect(x, y, THEME) {
                        prefs().toggle();
                    } else if let Some(i) = hit {
                        let p = self.pot(i);
                        param::begin(cx, p.as_ptr());
                        self.drag = Some((i, y, p.unmodulated_normalized_value()));
                        cx.capture();
                    } else if let Some(i) =
                        (0..3).find(|i| point_in_rect(x, y, brilliance_rect(*i)))
                    {
                        let p = &self.params.brilliance;
                        param::set_normalized_once(cx, p.as_ptr(), p.preview_normalized(BR[i]));
                    } else if point_in_rect(x, y, TREMOLO) {
                        let p = &self.params.tremolo;
                        param::set_normalized_once(
                            cx,
                            p.as_ptr(),
                            p.preview_normalized(!p.value()),
                        );
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if let Some(i) = hit {
                        self.end(cx);
                        let p = self.pot(i);
                        param::set_normalized_once(cx, p.as_ptr(), p.default_normalized_value());
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((x, y));
                    if let Some((i, start, initial)) = self.drag {
                        param::set_normalized(
                            cx,
                            self.pot(i).as_ptr(),
                            (initial + (start - y) / 140.).clamp(0., 1.),
                        );
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
        });
    }
    fn draw(&self, cx: &mut DrawContext, c: &mut Canvas) {
        let b = cx.bounds();
        load_font_once(c, &self.font, FONT_JETBRAINS_MONO);
        let appearance = prefs().mode();
        if let Some(mut d) = Draw::new_fitted(
            c,
            appearance.resolved() == Appearance::Light,
            (b.x, b.y, b.w, b.h),
            SIZE,
            self.font.get(),
        ) {
            self.paint(&mut d, appearance);
        }
    }
}
pub fn create(params: Arc<Parameters>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor.clone(),
        ViziaTheming::Custom,
        move |cx, gui| {
            let keyboard = Rc::new(PlayableKeyboard::new(params.keyboard.clone(), move || {
                gui.request_process()
            }));
            let poll = params.clone();
            let strike = Rc::new(StrikePreview::default());
            let previous = Cell::new(None);
            PanelView {
                params: params.clone(),
                keyboard: keyboard.clone(),
                strike: strike.clone(),
                font: Cell::new(None),
                label_font: Cell::new(None),
                hover: None,
                drag: None,
            }
            .build(cx, |cx| {
                start_frame_timer(cx, Duration::from_millis(33), move |cx| {
                    let appearance = prefs().mode();
                    let c = (poll.controls(), appearance, appearance.resolved());
                    let notes = strike.poll(&keyboard);
                    if previous.replace(Some(c)) != Some(c) || notes {
                        cx.needs_redraw();
                    }
                })
            })
            .width(Stretch(1.))
            .height(Stretch(1.));
        },
    )
}

#[cfg(all(test, target_os = "macos"))]
#[test]
#[ignore = "Draws the plugin editor into a temporary PPM using macOS OpenGL"]
fn render_panel_preview() {
    let mut gl = pleasant_ui::preview::OffscreenGl::new();
    let params = Arc::new(Parameters::default());
    let view = PanelView {
        keyboard: Rc::new(PlayableKeyboard::new(params.keyboard.clone(), || {})),
        strike: Rc::default(),
        params,
        font: Cell::new(None),
        label_font: Cell::new(None),
        hover: None,
        drag: None,
    };
    load_font_once(gl.canvas_mut(), &view.font, FONT_JETBRAINS_MONO);
    let (w, h) = (SIZE.0 * 2., SIZE.1 * 2.);
    for (appearance, name) in [
        (Appearance::Dark, "cp80-panel.ppm"),
        (Appearance::Light, "cp80-panel-light.ppm"),
    ] {
        gl.render_ppm(w as u32, h as u32, std::env::temp_dir().join(name), |c| {
            let mut d = Draw::new_fitted(
                c,
                appearance == Appearance::Light,
                (0., 0., w, h),
                SIZE,
                view.font.get(),
            )
            .unwrap();
            view.paint(&mut d, appearance);
        });
    }
}
