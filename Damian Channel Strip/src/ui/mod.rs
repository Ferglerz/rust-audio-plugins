mod bands;
mod dynamics;
mod edit;
mod eq;
mod events;
mod geometry;
mod hud;
mod hud_draw;
mod layout;
mod lift;
mod preferences;
mod render;
#[cfg(test)]
mod tests;
mod value_input;

use dynamics::*;
use eq::*;
use hud::*;
use hud_draw::*;
use layout::*;
use lift::*;
use value_input::{parse_value, typed_char, ValueTarget};

use crate::{
    band::{infer_shape, Band, Shape, MAX_RANGE_DB},
    dsp::BandCoeffs,
    engine::Shared,
    lift::{filter_influence, LiftBand, LIFT_ID_BASE},
    params::StripParams,
    processing::{Config, ProcessingMode, MODES, RESOLUTIONS},
};
use nih_plug::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color as C, FontId, Paint, Path},
    },
    widgets::RawParamEvent,
    ViziaTheming,
};
use pleasant_ui::{
    spectrum::smooth_bins,
    theme::{rgb, BG, EQ_COLORS as BAND_COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    ButtonAnim, Draw, ValueEdit, FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

pub const LIFT_COLOR: C = layout::LIFT_COLOR;
pub const EQ2_ID_BASE: u64 = layout::EQ2_ID_BASE;
pub const SC_EQ_ID_BASE: u64 = layout::SC_EQ_ID_BASE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DynPage {
    Main,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PsePage {
    Main,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WallPage {
    Main,
    Controls,
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    Value(ValueTarget),
    Global(usize),
    Band(usize),
    LiftBand(usize),
    Node(u64),
    Range(u64),
    Threshold(u64),
    CompKnee {
        from_top: bool,
    },
    PseKnee {
        from_top: bool,
    },
    SoloAudition {
        id: u64,
        restore: u64,
        grab_dx: f32,
        grab_dy: f32,
    },
}
#[derive(Clone, Copy, PartialEq)]
enum PendingCreate {
    Curve,
    Background,
}
const DRAG_START_THRESHOLD: f32 = 5.0;
#[derive(Clone, Copy)]
enum BandMenu {
    Shape,
    Order,
}
impl BandMenu {
    fn count(self) -> usize {
        match self {
            Self::Shape => Shape::ALL.len(),
            Self::Order => 8,
        }
    }
    fn text_size(self) -> f32 {
        match self {
            Self::Shape => HUD_SHAPE_TEXT,
            Self::Order => HUD_LABEL_SIZE,
        }
    }
    fn row_h(self) -> f32 {
        dropdown_row_h(self.text_size())
    }
    fn rect_at(self, b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
        let h = self.count() as f32 * self.row_h();
        let (bx, by, bw, _) = hud_rect_for_at(b, range, gx, gw);
        let anchor = match self {
            Self::Shape => hud_shape_rect(bx, by, bw),
            Self::Order => hud_order_rect(bx, by),
        };
        hud_menu_rect(anchor, h)
    }
    fn rect_lift_at(self, b: &LiftBand, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
        let h = self.count() as f32 * self.row_h();
        let (bx, by, bw, _) = hud_rect_for_lift_at(b, range, gx, gw);
        let anchor = match self {
            Self::Shape => hud_shape_rect(bx, by, bw),
            Self::Order => hud_order_rect(bx, by),
        };
        hud_menu_rect(anchor, h)
    }
}
pub fn create(params: Arc<StripParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            StripView {
                params: params.clone(),
                shared: shared.clone(),
                selected: None,
                dyn_page: Cell::new(DynPage::Main),
                pse_page: Cell::new(PsePage::Main),
                wall_page: Cell::new(WallPage::Main),
                band_dyn_page: Cell::new(false),
                drag: None,
                hover: None,
                shift_down: false,
                font: Cell::new(None),
                signature: Cell::new(None),
                graph_db: display_range(*params.graph_range.lock().unwrap()),
                scale_menu: false,
                processing_menu: None,
                edit: None,
                value_press: None,
                down: (0.0, 0.0),
                last_drag: (0.0, 0.0),
                pending_create: None,
                menu: None,
                active_eq: Cell::new(0),
                anim_progress: Cell::new(0.0),
                anim_target: Cell::new(0.0),
                anim_start: Cell::new(0.0),
                anim_time: Cell::new(1.0),
                last_tick: Cell::new(None),
                knee_bulge: Cell::new(0.0),
                pse_knee_bulge: Cell::new(0.0),
                dyn_anim_progress: Cell::new(0.0),
                dyn_anim_target: Cell::new(0.0),
                pse_anim_progress: Cell::new(0.0),
                pse_anim_target: Cell::new(0.0),
                wall_anim_progress: Cell::new(0.0),
                wall_anim_target: Cell::new(0.0),
                band_dyn_anim_progress: Cell::new(0.0),
                band_dyn_anim_target: Cell::new(0.0),
                dyn_away_since: Cell::new(None),
                pse_away_since: Cell::new(None),
                wall_away_since: Cell::new(None),
                pending_solo: None,
                alt_solo_restore: None,
                eq_bypass_anim: ButtonAnim::new(),
                pse_bypass_anim: ButtonAnim::new(),
                dyn_bypass_anim: ButtonAnim::new(),
                wall_bypass_anim: ButtonAnim::new(),
                dyn_band_anim: ButtonAnim::new(),
                band_bypass_anim: ButtonAnim::new(),
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.needs_redraw();
                    }
                });
                cx.start_timer(timer);
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .focusable(true);
        },
    )
}
struct StripView {
    dyn_page: Cell<DynPage>,
    pse_page: Cell<PsePage>,
    wall_page: Cell<WallPage>,
    band_dyn_page: Cell<bool>,
    params: Arc<StripParams>,
    shared: Arc<Shared>,
    selected: Option<u64>,
    drag: Option<Target>,
    hover: Option<(f32, f32)>,
    shift_down: bool,
    font: Cell<Option<FontId>>,
    signature: Cell<Option<FontId>>,
    graph_db: f64,
    scale_menu: bool,
    processing_menu: Option<bool>,
    edit: Option<ValueEdit<ValueTarget>>,
    value_press: Option<pleasant_ui::pointer::ValuePress<ValueTarget>>,
    down: (f32, f32),
    last_drag: (f32, f32),
    pending_create: Option<PendingCreate>,
    menu: Option<BandMenu>,
    active_eq: Cell<usize>,
    anim_progress: Cell<f32>,
    anim_target: Cell<f32>,
    anim_start: Cell<f32>,
    anim_time: Cell<f32>,
    last_tick: Cell<Option<Instant>>,
    knee_bulge: Cell<f32>,
    pse_knee_bulge: Cell<f32>,
    dyn_anim_progress: Cell<f32>,
    dyn_anim_target: Cell<f32>,
    pse_anim_progress: Cell<f32>,
    pse_anim_target: Cell<f32>,
    wall_anim_progress: Cell<f32>,
    wall_anim_target: Cell<f32>,
    band_dyn_anim_progress: Cell<f32>,
    band_dyn_anim_target: Cell<f32>,
    dyn_away_since: Cell<Option<Instant>>,
    pse_away_since: Cell<Option<Instant>>,
    wall_away_since: Cell<Option<Instant>>,
    pending_solo: Option<u64>,
    alt_solo_restore: Option<u64>,
    eq_bypass_anim: ButtonAnim,
    pse_bypass_anim: ButtonAnim,
    dyn_bypass_anim: ButtonAnim,
    wall_bypass_anim: ButtonAnim,
    dyn_band_anim: ButtonAnim,
    band_bypass_anim: ButtonAnim,
}
fn display_range(range: f64) -> f64 {
    if !range.is_finite() {
        return 24.0;
    }
    let mut closest = SCALES[0];
    let mut min_diff = (range - closest).abs();
    for &s in &SCALES[1..] {
        let diff = (range - s).abs();
        if diff < min_diff {
            min_diff = diff;
            closest = s;
        }
    }
    closest
}
fn format_pse_time(time_val: f64, peak: bool) -> String {
    let (attack, release) = crate::dsp::pse_time_to_times(time_val, peak);
    if peak {
        if release >= 10.0 {
            format!("{:.0} ms / {:.0} s", attack * 1000.0, release)
        } else if release >= 1.0 {
            if (release * 10.0).fract().abs() < 1e-3 {
                format!("{:.0} ms / {:.1} s", attack * 1000.0, release)
            } else {
                format!("{:.0} ms / {:.2} s", attack * 1000.0, release)
            }
        } else {
            format!("{:.0} ms / {:.0} ms", attack * 1000.0, release * 1000.0)
        }
    } else if release >= 10.0 {
        format!("{:.1} s", release)
    } else if release >= 1.0 {
        if (release * 10.0).fract().abs() < 1e-3 {
            format!("{:.1} s", release)
        } else {
            format!("{:.2} s", release)
        }
    } else {
        format!("{:.0} ms", release * 1000.0)
    }
}

impl View for StripView {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let was_editing = self.edit.is_some();
        self.handle_event(cx, event);
        pleasant_ui::value_edit::sync_text_input(cx, was_editing, self.edit.is_some());
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.render(cx, canvas);
    }
}
