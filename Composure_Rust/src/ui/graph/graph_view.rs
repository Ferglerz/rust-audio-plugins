//! Compression graph — curve, histograms, point editing.

use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;

use super::graph_display::DISPLAY_PAD_DB;
use crate::dsp::graph::{apply_extrapolation_caps, sample_curve_with_segments, CurveSegment};
use crate::graph_store::GraphStore;
use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::graph_display;
use super::graph_hit;
use super::graph_view_draw::*;
use super::theme;
use super::threshold_lines::{self, ThresholdLine};

const DOUBLE_CLICK_MS: u128 = 300;
const GRAPH_POINT_RADIUS: f32 = 8.0;
const GRAPH_POINT_RADIUS_SELECTED: f32 = 10.0;

#[derive(Default)]
struct GraphDrawCache {
    store_generation: u32,
    graph_points_version: u32,
    bounds_w_bits: u32,
    bounds_h_bits: u32,
    min_db: f64,
    max_db: f64,
    range_db: f64,
    corner_x: f64,
    corner_y: f64,
    point_coords: Vec<(f32, f32)>,
    segments: Vec<CurveSegment>,
    ext_in: f64,
    ext_out: f64,
}

impl GraphDrawCache {
    fn matches(&self, store_generation: u32, graph_points_version: u32, w: f32, h: f32) -> bool {
        self.store_generation == store_generation
            && self.graph_points_version == graph_points_version
            && self.bounds_w_bits == w.to_bits()
            && self.bounds_h_bits == h.to_bits()
    }
}

pub struct GraphView<L1, L2, P> {
    graph: GraphStore,
    display: Arc<UiDisplay>,
    params: P,
    detector_db: L1,
    gr_db: L2,
    input_thresh_base: ParamWidgetBase,
    dragging_point: i32,
    mouse_down: bool,
    dragging_threshold: bool,
    hovered_threshold: Option<ThresholdLine>,
    hovered_point: i32,
    curve_drag: bool,
    curve_drag_start_x: f32,
    curve_drag_start_amount: f64,
    last_click_point: i32,
    last_click_time: Option<Instant>,
    draw_cache: RefCell<GraphDrawCache>,
}

impl GraphView<(), (), ()> {
    pub fn new<L1, L2, P>(
        cx: &mut Context,
        graph: GraphStore,
        display: Arc<UiDisplay>,
        detector_db: L1,
        gr_db: L2,
        params: P,
    ) -> Handle<'_, GraphView<L1, L2, P>>
    where
        L1: Lens<Target = f32>,
        L2: Lens<Target = f32>,
        P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    {
        let input_thresh_base = ParamWidgetBase::new(cx, params, |p| &p.input_level_threshold_db);
        GraphView {
            graph,
            display,
            params,
            detector_db,
            gr_db,
            input_thresh_base,
            dragging_point: -1,
            mouse_down: false,
            dragging_threshold: false,
            hovered_threshold: None,
            hovered_point: -1,
            curve_drag: false,
            curve_drag_start_x: 0.0,
            curve_drag_start_amount: 0.0,
            last_click_point: -1,
            last_click_time: None,
            draw_cache: RefCell::new(GraphDrawCache::default()),
        }
        .build(cx, |_| {})
    }
}

impl<L1, L2, P> GraphView<L1, L2, P>
where
    L1: Lens<Target = f32>,
    L2: Lens<Target = f32>,
    P: Lens<Target = Arc<ComposureParams>> + Clone,
{
    fn local_mouse(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        (cx.mouse().cursorx - b.x, cx.mouse().cursory - b.y)
    }

    fn mutate_graph<F>(&self, cx: &mut EventContext, f: F)
    where
        F: FnOnce(&mut crate::dsp::graph::CompressionGraph),
    {
        self.graph.mutate_and_publish(f);
        let num_points = self.graph.load().graph.num_points;
        self.display.sync_graph_points(num_points);
        cx.needs_redraw();
    }

    fn active_threshold(&self) -> Option<ThresholdLine> {
        if self.dragging_threshold {
            Some(ThresholdLine::InputLevel)
        } else {
            self.hovered_threshold
        }
    }

    fn update_graph_hint(&self) {
        let show = self.dragging_threshold
            || self.dragging_point >= 0
            || self.hovered_threshold.is_some()
            || self.hovered_point >= 0;
        self.display.set_graph_hint(show);
    }

    fn update_point_readout(&self, idx: usize) {
        let g = &self.graph.load().graph;
        let in_db = g.get_point_x(idx);
        let out_db = g.get_point_y(idx);
        let curve = g.get_curve_amount(idx);
        let text = if self.curve_drag {
            format!("Curve: {curve:.0}%")
        } else {
            format!("{:.1} dB | {:.1} dB", out_db - in_db, out_db)
        };
        self.display.set_graph_readout(text);
    }

    fn reset_point_unity(&self, cx: &mut EventContext, idx: usize) {
        self.mutate_graph(cx, |g| g.reset_point_to_unity(idx));
    }
}

impl<L1, L2, P> View for GraphView<L1, L2, P>
where
    L1: Lens<Target = f32>,
    L2: Lens<Target = f32>,
    P: Lens<Target = Arc<ComposureParams>> + Clone,
{
    fn element(&self) -> Option<&'static str> {
        Some("composure-graph")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if !super::graph_pages::transfer_active(cx) {
            return;
        }
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let (mx, my) = Self::local_mouse(cx);
                let b = cx.bounds();

                if cx.modifiers().alt() {
                    self.mutate_graph(cx, |g| {
                        if let Some(idx) =
                            graph_hit::find_interior_point(g, mx, my, b.w, b.h, DISPLAY_PAD_DB)
                        {
                            if !g.delete_point(idx) {
                                g.reset_point_to_unity(1);
                                g.reset_point_to_unity(2);
                            }
                        }
                    });
                    meta.consume();
                    return;
                }

                let plugin_params = self.params.get(cx);
                let snapshot = self.graph.load();
                let graph = &snapshot.graph;
                let (min_db, _max_db, range_db) = (graph.min_db, graph.max_db, graph.range_db);
                let line_y = graph_display::axis_label_y(
                    plugin_params.input_level_threshold_db.value() as f64,
                    min_db,
                    range_db,
                    b.h,
                );

                if threshold_lines::find_graph_threshold(&plugin_params, my, line_y).is_some() {
                    self.dragging_threshold = true;
                    self.mouse_down = true;
                    self.input_thresh_base.begin_set_parameter(cx);
                    cx.capture();
                    meta.consume();
                    return;
                }

                self.mouse_down = true;
                cx.capture();

                if let Some(idx) =
                    graph_hit::find_interior_point(graph, mx, my, b.w, b.h, DISPLAY_PAD_DB)
                {
                    let now = Instant::now();
                    if self.last_click_point == idx as i32 {
                        if let Some(t) = self.last_click_time {
                            if now.duration_since(t).as_millis() < DOUBLE_CLICK_MS {
                                self.reset_point_unity(cx, idx);
                                self.dragging_point = -1;
                                self.mouse_down = false;
                                cx.release();
                                meta.consume();
                                return;
                            }
                        }
                    }
                    self.last_click_point = idx as i32;
                    self.last_click_time = Some(now);

                    if cx.modifiers().command() {
                        self.curve_drag = true;
                        self.curve_drag_start_x = mx;
                        self.curve_drag_start_amount = graph.get_curve_amount(idx);
                        self.dragging_point = idx as i32;
                    } else {
                        self.dragging_point = idx as i32;
                    }
                } else if mx >= 0.0
                    && my >= 0.0
                    && mx <= b.w
                    && my <= b.h
                    && !graph_hit::too_close_to_points(graph, mx, my, b.w, b.h, DISPLAY_PAD_DB)
                {
                    let new_idx = {
                        let mut captured = None;
                        self.graph.mutate_and_publish(|g| {
                            let (in_db, out_db) = graph_display::pixel_to_db_with_pad(
                                mx,
                                my,
                                b.w,
                                b.h,
                                g.min_db,
                                g.max_db,
                                g.range_db,
                                DISPLAY_PAD_DB,
                            );
                            captured = g.add_point(in_db, out_db);
                        });
                        captured
                    };
                    if let Some(idx) = new_idx {
                        self.display
                            .sync_graph_points(self.graph.load().graph.num_points);
                        self.dragging_point = idx as i32;
                        cx.needs_redraw();
                    }
                }

                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                let (mx, my) = Self::local_mouse(cx);
                let b = cx.bounds();
                self.mutate_graph(cx, |g| {
                    if let Some(idx) =
                        graph_hit::find_interior_point(g, mx, my, b.w, b.h, DISPLAY_PAD_DB)
                    {
                        if !g.delete_point(idx) {
                            g.reset_point_to_unity(1);
                            g.reset_point_to_unity(2);
                        }
                    }
                });
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging_threshold {
                    self.input_thresh_base.end_set_parameter(cx);
                    self.dragging_threshold = false;
                    self.update_graph_hint();
                }
                if self.mouse_down {
                    self.mouse_down = false;
                    self.dragging_point = -1;
                    self.curve_drag = false;
                    self.display.clear_graph_readout();
                    cx.release();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, _) => {
                if self.dragging_threshold && self.mouse_down {
                    let (_mx, my) = Self::local_mouse(cx);
                    let b = cx.bounds();
                    let snapshot = self.graph.load();
                    let graph = &snapshot.graph;
                    let (min_db, max_db, range_db) = (graph.min_db, graph.max_db, graph.range_db);
                    let new_db = threshold_lines::clamp_input_level(
                        graph_display::graph_y_to_db(my, b.h, min_db, range_db),
                        min_db,
                        max_db,
                    );
                    self.input_thresh_base.set_normalized_value(
                        cx,
                        self.input_thresh_base.preview_normalized(new_db as f32),
                    );
                    meta.consume();
                } else if self.dragging_point >= 0 && self.mouse_down {
                    let (mx, my) = Self::local_mouse(cx);
                    let b = cx.bounds();
                    let idx = self.dragging_point as usize;

                    if self.curve_drag {
                        let dx = mx - self.curve_drag_start_x;
                        let new_amount = (self.curve_drag_start_amount
                            + (dx as f64 / 100.0) * 100.0)
                            .clamp(0.0, 100.0);
                        self.mutate_graph(cx, |g| g.set_curve_amount(idx, new_amount));
                        self.update_point_readout(idx);
                    } else {
                        self.mutate_graph(cx, |g| {
                            let (in_db, out_db) = graph_display::pixel_to_db_with_pad(
                                mx,
                                my,
                                b.w,
                                b.h,
                                g.min_db,
                                g.max_db,
                                g.range_db,
                                DISPLAY_PAD_DB,
                            );
                            g.move_interior_point(idx, in_db, out_db);
                        });
                        self.update_point_readout(idx);
                    }
                    cx.needs_redraw();
                    meta.consume();
                } else {
                    let (mx, my) = Self::local_mouse(cx);
                    let b = cx.bounds();
                    let plugin_params = self.params.get(cx);
                    let snapshot = self.graph.load();
                    let graph = &snapshot.graph;
                    let hovered_point =
                        graph_hit::find_interior_point(graph, mx, my, b.w, b.h, DISPLAY_PAD_DB)
                            .map(|i| i as i32)
                            .unwrap_or(-1);
                    let (min_db, range_db) = (graph.min_db, graph.range_db);
                    let line_y = graph_display::axis_label_y(
                        plugin_params.input_level_threshold_db.value() as f64,
                        min_db,
                        range_db,
                        b.h,
                    );
                    self.hovered_threshold =
                        threshold_lines::find_graph_threshold(&plugin_params, my, line_y);
                    self.hovered_point = hovered_point;
                    self.update_graph_hint();
                    let _ = mx;
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.graph.collect_retired();
        let bounds = cx.bounds();
        if bounds.w < 2.0 || bounds.h < 2.0 {
            return;
        }

        self.display.age_dot_trails_one_frame();

        let opacity = cx.opacity();
        let light = super::EditorData::appearance.get(cx) == 1;
        let vg_color =
            |color, alpha| pleasant_ui::theme::transform_color(vg_color(color, alpha), light);
        let detector = self.detector_db.get(cx);
        let plugin_params = self.params.get(cx);
        self.graph
            .sync_range_if_needed(plugin_params.graph_range_mode.value().range_db());
        let store_generation = self.graph.generation();
        let graph_points_version = self.display.graph_points_version.load(Ordering::Relaxed);

        if !self.draw_cache.borrow().matches(
            store_generation,
            graph_points_version,
            bounds.w,
            bounds.h,
        ) {
            let (
                min_db,
                max_db,
                range_db,
                corner_x,
                corner_y,
                ext_in,
                ext_out,
                point_coords_local,
                segments_local,
            ) = {
                let snapshot = self.graph.load();
                let graph = &snapshot.graph;
                let min_db = graph.min_db;
                let max_db = graph.max_db;
                let range_db = graph.range_db.max(1.0);
                let corner_x = graph.get_point_x(0);
                let corner_y = graph.get_point_y(0);
                let num_points = graph.num_points;
                let point_coords_local: Vec<(f32, f32)> = (1..num_points - 1)
                    .map(|i| {
                        graph_display::db_to_pixel_with_pad(
                            graph.get_point_x(i),
                            graph.get_point_y(i),
                            bounds.w,
                            bounds.h,
                            min_db,
                            range_db,
                            DISPLAY_PAD_DB,
                        )
                    })
                    .collect();
                let segments_local = graph.segments_ref().to_vec();
                let ext_in = graph_display::display_max_db(max_db);
                let ext_out = apply_extrapolation_caps(
                    ext_in,
                    sample_curve_with_segments(ext_in, &segments_local, min_db),
                    min_db,
                    max_db,
                    range_db,
                );
                (
                    min_db,
                    max_db,
                    range_db,
                    corner_x,
                    corner_y,
                    ext_in,
                    ext_out,
                    point_coords_local,
                    segments_local,
                )
            };
            *self.draw_cache.borrow_mut() = GraphDrawCache {
                store_generation,
                graph_points_version,
                bounds_w_bits: bounds.w.to_bits(),
                bounds_h_bits: bounds.h.to_bits(),
                min_db,
                max_db,
                range_db,
                corner_x,
                corner_y,
                point_coords: point_coords_local,
                segments: segments_local,
                ext_in,
                ext_out,
            };
        }

        let cache = self.draw_cache.borrow();
        let min_db = cache.min_db;
        let max_db = cache.max_db;
        let range_db = cache.range_db.max(1.0);
        let corner_x = cache.corner_x;
        let corner_y = cache.corner_y;
        let point_coords = &cache.point_coords;
        let segments = &cache.segments;
        let ext_in = cache.ext_in;
        let ext_out = cache.ext_out;

        let output_db = {
            let uncapped = sample_curve_with_segments(detector as f64, segments, min_db);
            apply_extrapolation_caps(detector as f64, uncapped, min_db, max_db, range_db) as f32
        };

        let db_to_x =
            |db: f64| -> f32 { graph_display::db_to_local_x(db, bounds, min_db, range_db) };
        let db_to_y =
            |db: f64| -> f32 { graph_display::db_to_local_y(db, bounds, min_db, range_db) };
        let display_range = graph_display::display_range_db(range_db);
        let px_per_db = bounds.h / display_range as f32;

        let analog = super::EditorData::appearance.get(cx) == 2;
        if analog {
            let mut bg = Path::new();
            bg.rect(bounds.x, bounds.y, bounds.w, bounds.h);
            canvas.fill_path(
                &bg,
                &Paint::color(VgColor::rgbaf(0.12, 0.12, 0.12, 0.45 * opacity)),
            );
        }

        draw_histograms(canvas, bounds, &self.display, min_db, px_per_db, opacity);

        draw_grid(canvas, bounds, min_db, range_db, opacity);
        if analog {
            draw_out_of_scope_fade(canvas, bounds, min_db, max_db, range_db, opacity);
        }

        let active = self.active_threshold();
        threshold_lines::draw_input_level_threshold(
            canvas,
            bounds,
            &plugin_params,
            db_to_y,
            opacity,
            active,
        );

        let unity_paint = {
            let mut p = Paint::color(vg_color(theme::GRAPH_UNITY, opacity));
            p.set_line_width(1.0);
            p
        };
        let mut unity = Path::new();
        unity.move_to(db_to_x(min_db), db_to_y(min_db));
        unity.line_to(db_to_x(max_db), db_to_y(max_db));
        canvas.stroke_path(&unity, &unity_paint);

        if !segments.is_empty() {
            let curve_paint = {
                let mut p = Paint::color(vg_color(theme::GRAPH_CURVE, opacity));
                p.set_line_width(2.0);
                p
            };
            let mut curve = Path::new();
            curve.move_to(db_to_x(segments[0].x1), db_to_y(segments[0].y1));
            for seg in segments {
                curve.line_to(db_to_x(seg.x2), db_to_y(seg.y2));
            }
            canvas.stroke_path(&curve, &curve_paint);

            let last = segments.last().unwrap();
            let ext_paint = {
                let mut p = Paint::color(vg_color(theme::GRAPH_CURVE, opacity * 0.45));
                p.set_line_width(2.0);
                p
            };
            let mut ext = Path::new();
            ext.move_to(db_to_x(last.x2), db_to_y(last.y2));
            ext.line_to(db_to_x(ext_in), db_to_y(ext_out));
            canvas.stroke_path(&ext, &ext_paint);
        }

        draw_bl_margin_extension(
            canvas, &db_to_x, &db_to_y, segments, corner_x, corner_y, min_db, opacity,
        );

        for (i, (px, py)) in point_coords.iter().enumerate() {
            let selected = self.dragging_point == (i + 1) as i32;
            let dot_x = (bounds.x + px).clamp(bounds.x, bounds.x + bounds.w);
            let dot_y = (bounds.y + py).clamp(bounds.y, bounds.y + bounds.h);
            draw_dot(
                canvas,
                dot_x,
                dot_y,
                if selected {
                    GRAPH_POINT_RADIUS_SELECTED
                } else {
                    GRAPH_POINT_RADIUS
                },
                vg_color(
                    if selected {
                        theme::GRAPH_CURVE
                    } else {
                        theme::GRAPH_INPUT_DOT
                    },
                    opacity,
                ),
            );
        }

        draw_input_trail_dots(canvas, bounds, &self.display, min_db, range_db, opacity);

        let gr = self.gr_db.get(cx);

        draw_dot(
            canvas,
            db_to_x(detector as f64),
            db_to_y(output_db as f64),
            3.5,
            vg_color(theme::GRAPH_INPUT_DOT, opacity * 0.85),
        );

        if gr.abs() > 0.05 {
            draw_dot(
                canvas,
                db_to_x(detector as f64),
                db_to_y((output_db + gr) as f64),
                3.0,
                vg_color(theme::GRAPH_GR_DOT, opacity),
            );
        }

        if analog {
            draw_graph_reflection(canvas, bounds, detector, range_db as f32, opacity);
        }
    }
}
