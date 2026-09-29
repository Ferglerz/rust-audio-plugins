//! Gain-reduction meters — thresholds and trails.

use std::sync::{atomic::Ordering, Arc};

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::Color as VgColor;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;

use crate::params::ComposureParams;

use super::display::{UiDisplay, METER_TRAIL_FADE_PEAK, TRAIL_MAX_AGE_SECS};
use super::draw_helpers::{self, gr_vg_color};
use super::threshold_lines::{self, ThresholdLine};

pub struct GrMeterView<L, R, P> {
    gr_db: L,
    range_db: R,
    params: P,
    display: Arc<UiDisplay>,
    handles: MeterThresholdHandles,
    dragging_threshold: Option<ThresholdLine>,
    hovered_threshold: Option<ThresholdLine>,
    dragging_knee: bool,
    knee_drag_from_top: bool,
    hovered_knee: bool,
    last_drag_y: f32,
    meter_captured: bool,
}

impl GrMeterView<(), (), ()> {
    pub fn new<L, R, P>(
        cx: &mut Context,
        gr_db: L,
        range_db: R,
        params: P,
        display: Arc<UiDisplay>,
    ) -> Handle<'_, GrMeterView<L, R, P>>
    where
        L: Lens<Target = f32>,
        R: Lens<Target = f32>,
        P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    {
        let handles = MeterThresholdHandles::new(cx, params.clone());

        GrMeterView {
            gr_db,
            range_db,
            params,
            display,
            handles,
            dragging_threshold: None,
            hovered_threshold: None,
            dragging_knee: false,
            knee_drag_from_top: false,
            hovered_knee: false,
            last_drag_y: 0.0,
            meter_captured: false,
        }
        .build(cx, |_| {})
    }
}

impl<L, R, P> GrMeterView<L, R, P>
where
    L: Lens<Target = f32>,
    R: Lens<Target = f32>,
    P: Lens<Target = Arc<ComposureParams>> + Clone,
{
    fn meter_bounds(&self, cx: &EventContext) -> BoundingBox {
        let b = cx.bounds();
        {
            BoundingBox {
                y: b.y + 8.0 * cx.scale_factor(),
                h: b.h - 52.0 * cx.scale_factor(),
                ..b
            }
        }
    }

    fn threshold_hit(
        &self,
        cx: &EventContext,
        params: &ComposureParams,
        x: f32,
        y: f32,
        range: f32,
    ) -> Option<(ThresholdLine, bool)> {
        let bounds = self.meter_bounds(cx);

        let scale = cx.scale_factor();
        for (index, line, db, knee) in meter_thresholds(params) {
            let track = pleasant_track(bounds, scale, index);
            let ty = threshold_y(line, track, db, range);
            if x < track.x - 7.0 * scale || x > track.x + track.w + 7.0 * scale {
                continue;
            }
            if (y - ty).abs() <= 8.0 * scale {
                return Some((line, cx.modifiers().command() && Self::is_gr_line(line)));
            }
            if Self::is_gr_line(line) {
                let (top, height) = knee_zone(track, ty, knee, scale);
                if y >= top && y <= top + height {
                    return Some((line, true));
                }
            }
        }
        None
    }

    fn active_threshold(&self) -> Option<ThresholdLine> {
        self.dragging_threshold.or(self.hovered_threshold)
    }

    fn is_gr_line(line: ThresholdLine) -> bool {
        matches!(
            line,
            ThresholdLine::GrBlendReduction | ThresholdLine::GrBlendAddition
        )
    }

    fn begin_threshold_drag(
        &mut self,
        cx: &mut EventContext,
        line: ThresholdLine,
        knee_mode: bool,
        mouse_y: f32,
    ) {
        self.dragging_threshold = Some(line);
        self.dragging_knee = knee_mode;
        if knee_mode {
            let bounds = self.meter_bounds(cx);
            let scale = cx.scale_factor();
            let params = self.params.get(cx);
            let range = self.range_db.get(cx).max(1.0);
            if let Some((index, _, db, _)) = meter_thresholds(&params)
                .into_iter()
                .find(|(_, candidate, _, _)| *candidate == line)
            {
                let track = { pleasant_track(bounds, scale, index) };
                self.knee_drag_from_top = mouse_y < threshold_y(line, track, db, range);
            }
        }
        self.last_drag_y = mouse_y;
        self.meter_captured = true;
        self.handles.begin_drag(cx, line, knee_mode);
        cx.capture();
    }

    fn end_threshold_drag(&mut self, cx: &mut EventContext) {
        if let Some(line) = self.dragging_threshold.take() {
            self.handles.end_drag(cx, line, self.dragging_knee);
        }
        self.dragging_knee = false;
        if self.meter_captured {
            cx.release();
            self.meter_captured = false;
        }
    }

    fn apply_knee_drag(&self, cx: &mut EventContext, line: ThresholdLine, dy: f32) {
        let bounds = self.meter_bounds(cx);
        let range = self.range_db.get(cx).max(1.0);
        let knee_span = (range * 2.0).max(1.0);
        let delta_db = knee_drag_delta_db(dy, bounds.h, knee_span, self.knee_drag_from_top);
        self.handles.apply_knee_drag(cx, line, delta_db);
    }

    fn apply_threshold_drag(&self, cx: &mut EventContext, line: ThresholdLine, mouse_y: f32) {
        let bounds = self.meter_bounds(cx);
        let range = self.range_db.get(cx).max(1.0);
        self.handles
            .apply_threshold_drag(cx, line, mouse_y, bounds, range);
    }
}

impl<L, R, P> View for GrMeterView<L, R, P>
where
    L: Lens<Target = f32>,
    R: Lens<Target = f32>,
    P: Lens<Target = Arc<ComposureParams>> + Clone,
{
    fn element(&self) -> Option<&'static str> {
        Some("composure-gr-meter")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let cursor_y = cx.mouse().cursory;
                let cursor_x = cx.mouse().cursorx;
                let range = self.range_db.get(cx).max(1.0);
                let params = self.params.get(cx);
                if let Some((line, knee)) =
                    self.threshold_hit(cx, &params, cursor_x, cursor_y, range)
                {
                    self.begin_threshold_drag(cx, line, knee, cursor_y);
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging_threshold.is_some() {
                    self.end_threshold_drag(cx);
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, _) => {
                let cursor_y = cx.mouse().cursory;
                let cursor_x = cx.mouse().cursorx;
                let range = self.range_db.get(cx).max(1.0);
                let params = self.params.get(cx);
                if let Some(line) = self.dragging_threshold {
                    let knee_now = { self.dragging_knee };
                    if knee_now != self.dragging_knee && Self::is_gr_line(line) {
                        self.end_threshold_drag(cx);
                        self.begin_threshold_drag(cx, line, knee_now, cursor_y);
                    } else if self.dragging_knee {
                        let dy = cursor_y - self.last_drag_y;
                        if dy.abs() > 0.01 {
                            self.apply_knee_drag(cx, line, dy);
                            self.last_drag_y = cursor_y;
                        }
                    } else {
                        self.apply_threshold_drag(cx, line, cursor_y);
                    }
                    meta.consume();
                } else {
                    let hit = self.threshold_hit(cx, &params, cursor_x, cursor_y, range);
                    self.hovered_threshold = hit.map(|(line, _)| line);
                    self.hovered_knee = hit.is_some_and(|(_, knee)| knee);
                    cx.needs_redraw();
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.display.age_dot_trails_one_frame();

        let bounds = cx.bounds();
        if bounds.w < 1.0 || bounds.h < 1.0 {
            return;
        }

        let params = self.params.get(cx);
        let active = self.active_threshold();
        draw_pleasant_meters(
            cx,
            canvas,
            self.gr_db.get(cx),
            self.range_db.get(cx).max(1.0),
            &params,
            &self.display,
            active.map(|line| {
                (
                    line,
                    if self.dragging_threshold.is_some() {
                        self.dragging_knee
                    } else {
                        self.hovered_knee
                    },
                )
            }),
        );
    }
}

fn pleasant_track(bounds: BoundingBox, scale: f32, index: usize) -> BoundingBox {
    use super::appearance::*;
    BoundingBox {
        x: bounds.x
            + scale
                * (PLEASANT_METER_INSET + index as f32 * (PLEASANT_METER_W + PLEASANT_METER_GAP)),
        w: PLEASANT_METER_W * scale,
        ..bounds
    }
}

// As in Damian, dragging away from the threshold grows the knee on either side.
fn knee_drag_delta_db(dy: f32, height: f32, span: f32, from_top: bool) -> f32 {
    let direction = if from_top { -1.0 } else { 1.0 };
    direction * dy / height.max(1.0) * span
}

// Input release thresholds span -80..0 dB independently of the transfer graph zoom.
const INPUT_RANGE_DB: f32 = 80.0;
fn input_y(bounds: BoundingBox, db: f32) -> f32 {
    bounds.y + (-db / INPUT_RANGE_DB).clamp(0.0, 1.0) * bounds.h
}
fn input_db_from_y(bounds: BoundingBox, y: f32) -> f32 {
    (-INPUT_RANGE_DB * (y - bounds.y) / bounds.h.max(1.0)).clamp(-INPUT_RANGE_DB, 0.0)
}
fn meter_thresholds(params: &ComposureParams) -> Vec<(usize, ThresholdLine, f32, f32)> {
    let mut thresholds = Vec::new();
    if params.program_gr_enabled() {
        thresholds.extend([
            (
                1,
                ThresholdLine::GrBlendReduction,
                params.gr_blend_threshold_reduction_db.value(),
                params.gr_blend_threshold_reduction_knee_db.value(),
            ),
            (
                2,
                ThresholdLine::GrBlendAddition,
                params.gr_blend_threshold_addition_db.value(),
                params.gr_blend_threshold_addition_knee_db.value(),
            ),
        ]);
    }
    if params.program_input_enabled() {
        thresholds.push((
            0,
            ThresholdLine::InputLevel,
            params.input_level_threshold_db.value(),
            0.0,
        ));
    }
    thresholds
}

fn threshold_y(line: ThresholdLine, bounds: BoundingBox, db: f32, range: f32) -> f32 {
    if line == ThresholdLine::InputLevel {
        input_y(bounds, db)
    } else {
        draw_helpers::gr_threshold_y(bounds, db, range, line == ThresholdLine::GrBlendReduction)
    }
}

fn knee_zone(track: BoundingBox, y: f32, knee: f32, scale: f32) -> (f32, f32) {
    let half = (18.0 + (knee / 12.0).clamp(0.0, 1.0) * 45.0) * scale;
    let top = (y - half).max(track.y);
    let bottom = (y + half).min(track.y + track.h);
    (top, bottom - top)
}

// Match Damian's minimum visible fill: don't submit degenerate rectangles to the renderer.
fn visible_meter_height(db: f32, range: f32, height: f32) -> Option<f32> {
    if !db.is_finite() || db <= 0.0 {
        return None;
    }
    let fill = (db / range.max(1.0)).clamp(0.0, 1.0) * height;
    (fill > 0.5).then_some(fill)
}

// Match the orange/blue GR graph palette, softened slightly for solid meter fills.
fn pleasant_meter_accent(index: usize) -> VgColor {
    if index == 0 {
        return pleasant_ui::TEAL;
    }
    let color = gr_vg_color(if index == 1 { -1.0 } else { 1.0 }, 1.0);
    VgColor {
        r: color.r + (1.0 - color.r) * 0.14,
        g: color.g + (1.0 - color.g) * 0.14,
        b: color.b + (1.0 - color.b) * 0.14,
        a: 1.0,
    }
}

fn draw_pleasant_meters(
    cx: &mut DrawContext,
    canvas: &mut Canvas,
    gr: f32,
    range: f32,
    params: &ComposureParams,
    display: &UiDisplay,
    active: Option<(ThresholdLine, bool)>,
) {
    use super::appearance::PLEASANT_METERS_W;
    use pleasant_ui::{LINE, MUTED, TEXT};
    let (active, active_knee) = (
        active.map(|(line, _)| line),
        active.is_some_and(|(_, knee)| knee),
    );
    let height = cx.bounds().h / cx.scale_factor();
    let bounds = BoundingBox {
        x: 0.0,
        y: 8.0,
        w: PLEASANT_METERS_W,
        h: height - 52.0,
    };
    let mut d = super::appearance::painter(cx, canvas, PLEASANT_METERS_W);
    let input = display.input_meter_db.load(Ordering::Relaxed);
    for index in 0..3 {
        let track = pleasant_track(bounds, 1.0, index);
        let accent = pleasant_meter_accent(index);
        d.rect(track.x, track.y, track.w, track.h, LINE);
        if index != 0 {
            display.read_trails(|lines| {
                for line in lines {
                    if (index == 1 && line.gr_db >= 0.0) || (index == 2 && line.gr_db <= 0.0) {
                        continue;
                    }
                    let alpha = super::display::fade_by_age(
                        line.age_secs,
                        TRAIL_MAX_AGE_SECS,
                        METER_TRAIL_FADE_PEAK,
                        true,
                    );
                    if alpha < 0.01 {
                        continue;
                    }
                    let Some(offset) = visible_meter_height(line.gr_db.abs(), range, track.h)
                    else {
                        continue;
                    };
                    let y = if index == 1 {
                        track.y + offset
                    } else {
                        track.y + track.h - offset
                    };
                    d.rect(
                        track.x,
                        y.clamp(track.y, track.y + track.h - 2.0),
                        track.w,
                        2.0,
                        VgColor { a: alpha, ..accent },
                    );
                }
            });
        }
        let value = match index {
            0 => input,
            1 => (-gr).max(0.0),
            _ => gr.max(0.0),
        };
        let (fill_db, fill_range) = if index == 0 {
            (value + INPUT_RANGE_DB, INPUT_RANGE_DB)
        } else {
            (value, range)
        };
        if let Some(fill) = visible_meter_height(fill_db, fill_range, track.h) {
            let y = if index == 1 {
                track.y
            } else {
                track.y + track.h - fill
            };
            d.rect(track.x, y, track.w, fill, accent);
        }
        let knee_active = active_knee
            && match index {
                1 => active == Some(ThresholdLine::GrBlendReduction),
                2 => active == Some(ThresholdLine::GrBlendAddition),
                _ => false,
            };
        d.text_centered(
            track.x + track.w * 0.5,
            height - 24.0,
            match index {
                0 => "INPUT",
                1 | 2 if knee_active => "KNEE",
                1 => "CUT",
                _ => "BOOST",
            },
            13.0,
            if knee_active { accent } else { MUTED },
        );
        let threshold = meter_thresholds(params)
            .into_iter()
            .find(|(i, line, _, _)| *i == index && (index == 0 || active == Some(*line)));
        let readout = if let Some((_, _, db, knee)) = threshold {
            if active_knee {
                format!("{knee:.1}")
            } else {
                format!("{db:.1}")
            }
        } else if index == 0 {
            String::new()
        } else {
            format!("{value:.1}")
        };
        d.text_centered(
            track.x + track.w * 0.5,
            height - 4.0,
            &readout,
            14.0,
            accent,
        );
    }
    for (index, line, db, knee) in meter_thresholds(params) {
        let track = pleasant_track(bounds, 1.0, index);
        let y = threshold_y(line, track, db, range);
        let accent = pleasant_meter_accent(index);
        let hover = active == Some(line);
        if matches!(
            line,
            ThresholdLine::GrBlendReduction | ThresholdLine::GrBlendAddition
        ) {
            let (top, h) = knee_zone(track, y, knee, 1.0);
            let alpha = 0.75 - (knee / 12.0).clamp(0.0, 1.0) * 0.70;
            let zone = (track.x - 7.0, top, track.w + 14.0, h);
            d.rect(
                zone.0,
                zone.1,
                zone.2,
                zone.3,
                VgColor {
                    a: if hover && active_knee {
                        (alpha + 0.18).min(0.95)
                    } else {
                        alpha
                    },
                    ..accent
                },
            );
            if hover && active_knee {
                d.outline(zone, VgColor { a: 0.7, ..accent });
            }
        }
        d.grab_bar(
            (track.x - 5.0, y - 8.0, track.w + 10.0, 16.0),
            if hover && !active_knee { TEXT } else { accent },
        );
    }
}

struct MeterThresholdHandles {
    gr_cut: ParamWidgetBase,
    gr_boost: ParamWidgetBase,
    gr_cut_knee: ParamWidgetBase,
    gr_boost_knee: ParamWidgetBase,
    input_level: ParamWidgetBase,
}

impl MeterThresholdHandles {
    fn new<P>(cx: &mut Context, params: P) -> Self
    where
        P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    {
        Self {
            input_level: ParamWidgetBase::new(cx, params, |p| &p.input_level_threshold_db),
            gr_cut: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.gr_blend_threshold_reduction_db
            }),
            gr_boost: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.gr_blend_threshold_addition_db
            }),
            gr_cut_knee: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.gr_blend_threshold_reduction_knee_db
            }),
            gr_boost_knee: ParamWidgetBase::new(cx, params, |p| {
                &p.gr_blend_threshold_addition_knee_db
            }),
        }
    }

    fn param_for(&self, line: ThresholdLine, knee: bool) -> Option<&ParamWidgetBase> {
        match (line, knee) {
            (ThresholdLine::InputLevel, false) => Some(&self.input_level),
            (ThresholdLine::GrBlendReduction, false) => Some(&self.gr_cut),
            (ThresholdLine::GrBlendAddition, false) => Some(&self.gr_boost),
            (ThresholdLine::GrBlendReduction, true) => Some(&self.gr_cut_knee),
            (ThresholdLine::GrBlendAddition, true) => Some(&self.gr_boost_knee),
            _ => None,
        }
    }

    fn begin_drag(&self, cx: &mut EventContext, line: ThresholdLine, knee: bool) {
        if let Some(base) = self.param_for(line, knee) {
            base.begin_set_parameter(cx);
        }
    }

    fn end_drag(&self, cx: &mut EventContext, line: ThresholdLine, knee: bool) {
        if let Some(base) = self.param_for(line, knee) {
            base.end_set_parameter(cx);
        }
    }

    fn apply_knee_drag(&self, cx: &mut EventContext, line: ThresholdLine, delta_db: f32) {
        let Some(base) = self.param_for(line, true) else {
            return;
        };
        let current = base.unmodulated_plain_value();
        let v = (current + delta_db).clamp(0.0, 12.0);
        base.set_normalized_value(cx, base.preview_normalized(v));
    }

    fn apply_threshold_drag(
        &self,
        cx: &mut EventContext,
        line: ThresholdLine,
        mouse_y: f32,
        bounds: BoundingBox,
        range: f32,
    ) {
        let Some(base) = self.param_for(line, false) else {
            return;
        };
        let value = if line == ThresholdLine::InputLevel {
            Some(input_db_from_y(bounds, mouse_y))
        } else {
            threshold_lines::meter_threshold_value_from_y(line, mouse_y, bounds, range)
        };
        let Some(v) = value else {
            return;
        };
        base.set_normalized_value(cx, base.preview_normalized(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knee_drag_grows_away_from_threshold_and_shrinks_toward_it() {
        for from_top in [true, false] {
            let away = if from_top { -10.0 } else { 10.0 };
            let delta = knee_drag_delta_db(away, 200.0, 24.0, from_top);
            assert!((6.0 + delta - 7.2).abs() < 0.001);
            let delta = knee_drag_delta_db(-away, 200.0, 24.0, from_top);
            assert!((6.0 + delta - 4.8).abs() < 0.001);
        }
    }

    #[test]
    fn input_meter_threshold_round_trips_full_parameter_range() {
        let bounds = BoundingBox {
            x: 0.0,
            y: 8.0,
            w: 38.0,
            h: 422.0,
        };
        for db in [-80.0, -60.0, -24.0, -6.0, 0.0] {
            assert!((input_db_from_y(bounds, input_y(bounds, db)) - db).abs() < 0.001);
        }
        assert_eq!(input_db_from_y(bounds, -100.0), 0.0);
        assert_eq!(input_db_from_y(bounds, 900.0), -80.0);
        let display = UiDisplay::default();
        let silence = display.detector_db.load(Ordering::Relaxed);
        assert_eq!(
            visible_meter_height(silence + INPUT_RANGE_DB, INPUT_RANGE_DB, bounds.h),
            None
        );
    }

    #[test]
    fn idle_and_subpixel_meter_values_have_no_fill_or_trail() {
        for range in [6.0, 24.0, 72.0] {
            for db in [0.0, -0.0, f32::EPSILON, -1.0, f32::NAN] {
                assert_eq!(visible_meter_height(db, range, 422.0), None);
            }
            assert!(visible_meter_height(1.0, range, 422.0).is_some());
            assert_eq!(visible_meter_height(range * 2.0, range, 422.0), Some(422.0));
        }
    }

    #[test]
    fn pleasant_handles_fit_separate_lanes_at_every_scale() {
        for scale in [1.0, 1.5, 2.0] {
            let bounds = BoundingBox {
                x: 0.0,
                y: 8.0 * scale,
                w: super::super::appearance::PLEASANT_METERS_W * scale,
                h: 234.0 * scale,
            };
            let input = pleasant_track(bounds, scale, 0);
            let cut = pleasant_track(bounds, scale, 1);
            let boost = pleasant_track(bounds, scale, 2);
            assert!(input.x - 7.0 * scale >= bounds.x);
            assert!(input.x + input.w + 7.0 * scale < cut.x - 7.0 * scale);
            assert!(cut.x + cut.w + 7.0 * scale < boost.x - 7.0 * scale);
            assert!(boost.x + boost.w + 7.0 * scale <= bounds.x + bounds.w);
            for track in [cut, boost] {
                for y in [track.y, track.y + track.h * 0.5, track.y + track.h] {
                    let (top, height) = knee_zone(track, y, 12.0, scale);
                    assert!(top >= track.y && top + height <= track.y + track.h);
                }
            }
        }
    }
}
