//! Compression graph ported from `02_graph_data_core.jsfx-inc` and `03_graph_curves.jsfx-inc`.

use super::constants::{
    BEZIER_STEPS, GRAPH_MAX_DB, GRAPH_MIN_DB, GRAPH_RANGE_DB, MAX_CURVE_SEGMENTS, MAX_POINTS,
    MIN_POINTS,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GraphPoint {
    pub input_db: f64,
    pub output_db: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CurveSegment {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "PersistedGraph")]
pub struct CompressionGraph {
    points: [f64; MAX_POINTS * 2],
    pub num_points: usize,
    #[serde(skip)]
    segments: [CurveSegment; MAX_CURVE_SEGMENTS],
    #[serde(skip)]
    num_segments: usize,
    #[serde(skip)]
    pub segments_dirty: bool,
    pub range_db: f64,
    pub min_db: f64,
    pub max_db: f64,
}

#[derive(Deserialize)]
struct PersistedGraph {
    points: [f64; MAX_POINTS * 2],
    num_points: usize,
    range_db: f64,
    min_db: f64,
    max_db: f64,
}

impl TryFrom<PersistedGraph> for CompressionGraph {
    type Error = &'static str;

    fn try_from(saved: PersistedGraph) -> Result<Self, Self::Error> {
        let graph = Self {
            points: saved.points,
            num_points: saved.num_points,
            segments: default_segments(),
            num_segments: 0,
            segments_dirty: true,
            range_db: saved.range_db,
            min_db: saved.min_db,
            max_db: saved.max_db,
        };
        graph.validate()?;
        Ok(graph)
    }
}

fn default_segments() -> [CurveSegment; MAX_CURVE_SEGMENTS] {
    [CurveSegment::default(); MAX_CURVE_SEGMENTS]
}

impl Default for CompressionGraph {
    fn default() -> Self {
        let mut g = Self {
            points: [0.0; MAX_POINTS * 2],
            num_points: 6,
            segments: [CurveSegment::default(); MAX_CURVE_SEGMENTS],
            num_segments: 0,
            segments_dirty: true,
            range_db: GRAPH_RANGE_DB,
            min_db: GRAPH_MIN_DB,
            max_db: GRAPH_MAX_DB,
        };
        g.init_default_points();
        g
    }
}

impl CompressionGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !(MIN_POINTS..=MAX_POINTS).contains(&self.num_points) {
            return Err("compression graph point count is outside supported limits");
        }
        let span = self.max_db - self.min_db;
        if !self.range_db.is_finite()
            || self.range_db <= 0.0
            || !self.min_db.is_finite()
            || !self.max_db.is_finite()
            || !span.is_finite()
            || span <= 0.0
            || (span - self.range_db).abs() > self.range_db * 1.0e-9
        {
            return Err("compression graph range is invalid");
        }
        if self.points.iter().any(|value| !value.is_finite()) {
            return Err("compression graph coordinates must be finite");
        }
        Ok(())
    }

    pub fn set_range_db(&mut self, range_db: f64) {
        self.range_db = range_db;
        self.min_db = self.max_db - range_db;
    }

    /// Remap interior points by normalized position when graph range changes (JSFX parity).
    pub fn remap_points_for_range(&mut self, new_range_db: f64) {
        if self.num_points < 2 {
            return;
        }
        let old_min_db = self.min_db;
        let old_range_db = self.range_db.max(0.001);

        self.range_db = new_range_db;
        self.max_db = 0.0;
        self.min_db = self.max_db - new_range_db;

        for i in 1..self.num_points - 1 {
            let point_idx = i * 2;
            let norm_x = (self.points[point_idx] - old_min_db) / old_range_db;
            let norm_y = (self.points[point_idx + 1] - old_min_db) / old_range_db;
            self.points[point_idx] = self.min_db + norm_x.clamp(0.0, 1.0) * new_range_db;
            self.points[point_idx + 1] = self.min_db + norm_y.clamp(0.0, 1.0) * new_range_db;
        }

        self.sort_points();
        self.update_corner_points();
        self.invalidate();
    }

    pub fn graph_points(&self) -> &[f64] {
        &self.points
    }

    pub fn graph_points_mut(&mut self) -> &mut [f64] {
        self.invalidate();
        &mut self.points
    }

    pub fn get_point_x(&self, point_idx: usize) -> f64 {
        self.points[point_idx * 2]
    }

    pub fn get_point_y(&self, point_idx: usize) -> f64 {
        self.points[point_idx * 2 + 1]
    }

    pub fn is_valid_curve_point(&self, point_idx: usize) -> bool {
        point_idx > 0 && point_idx < self.num_points - 1
    }

    pub fn init_default_points(&mut self) {
        let gmin = self.min_db;
        let gmax = self.max_db;
        let gr = self.range_db;
        self.points[0] = gmin;
        self.points[1] = gmin;
        self.points[10] = gmax;
        self.points[11] = gmax;
        self.points[2] = gmin + gr * 0.2;
        self.points[3] = self.points[2];
        self.points[4] = gmin + gr * 0.4;
        self.points[5] = self.points[4];
        self.points[6] = gmin + gr * 0.6;
        self.points[7] = self.points[6];
        self.points[8] = gmin + gr * 0.8;
        self.points[9] = self.points[8];
        self.update_corner_points();
        self.invalidate();
    }

    pub fn invalidate(&mut self) {
        self.segments_dirty = true;
    }

    /// Compare graph content for preset sync (ignores cached segments).
    pub fn content_eq(&self, other: &Self) -> bool {
        self.num_points == other.num_points
            && self.range_db == other.range_db
            && self.min_db == other.min_db
            && self.max_db == other.max_db
            && self.points == other.points
    }

    pub fn update_corner_points(&mut self) {
        if self.num_points >= 3 {
            let p1_x = self.points[2];
            let p1_y = self.points[3];
            let p2_x = self.points[4];
            let p2_y = self.points[5];
            let dx = p2_x - p1_x;
            if dx.abs() > 0.001 {
                let slope = (p2_y - p1_y) / dx;
                // Horizontal segment (expansion plateau): treat like dx≈0 — snap to unity corner.
                if slope.abs() <= 0.001 || p1_y > p2_y {
                    self.points[0] = self.min_db;
                    self.points[1] = self.min_db;
                } else {
                    self.points[1] = self.min_db;
                    self.points[0] = p1_x + (self.min_db - p1_y) / slope;
                    if self.points[0] < self.min_db {
                        self.points[0] = self.min_db;
                        self.points[1] = p1_y + slope * (self.min_db - p1_x);
                        if self.points[1] > self.min_db {
                            self.points[1] = self.min_db;
                        }
                    } else if self.points[0] > self.min_db {
                        self.points[0] = self.min_db;
                        self.points[1] = p1_y + slope * (self.min_db - p1_x);
                        if self.points[1] > self.min_db {
                            self.points[1] = self.min_db;
                        }
                    }
                    // Prevent corner Y below graph floor (e.g. -40 when min is -20).
                    if self.points[1] < self.min_db {
                        self.points[1] = self.min_db;
                    }
                }
            } else {
                self.points[0] = self.min_db;
                self.points[1] = self.min_db;
            }
        } else {
            self.points[0] = self.min_db;
            self.points[1] = self.min_db;
        }

        let last_idx = self.num_points - 1;
        if self.num_points >= 3 {
            let p1_x = self.points[(last_idx - 2) * 2];
            let p1_y = self.points[(last_idx - 2) * 2 + 1];
            let p2_x = self.points[(last_idx - 1) * 2];
            let p2_y = self.points[(last_idx - 1) * 2 + 1];
            let dx = p2_x - p1_x;
            if dx.abs() > 0.001 {
                let slope = (p2_y - p1_y) / dx;
                self.points[last_idx * 2] = self.max_db;
                let calculated_y = p2_y + slope * (self.max_db - p2_x);
                self.points[last_idx * 2 + 1] = calculated_y.min(self.max_db);
            } else {
                self.points[last_idx * 2] = self.max_db;
                self.points[last_idx * 2 + 1] = p2_y.min(self.max_db);
            }
        } else {
            self.points[last_idx * 2] = self.max_db;
            self.points[last_idx * 2 + 1] = self.max_db;
        }
    }

    fn calculate_bezier_control_points(
        &self,
        point_index: usize,
    ) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
        let prev_x = self.get_point_x(point_index - 1);
        let prev_y = self.get_point_y(point_index - 1);
        let curr_x = self.get_point_x(point_index);
        let curr_y = self.get_point_y(point_index);
        let next_x = self.get_point_x(point_index + 1);
        let next_y = self.get_point_y(point_index + 1);
        // Every node uses the former maximum (100%) curve setting.
        let curve_factor = 1.0;
        let invisible1_x = curr_x + (prev_x - curr_x) * curve_factor;
        let invisible1_y = curr_y + (prev_y - curr_y) * curve_factor;
        let invisible2_x = curr_x + (next_x - curr_x) * curve_factor;
        let invisible2_y = curr_y + (next_y - curr_y) * curve_factor;
        (
            invisible1_x,
            invisible1_y,
            curr_x,
            curr_y,
            curr_x,
            curr_y,
            invisible2_x,
            invisible2_y,
        )
    }

    fn evaluate_bezier_at_t(
        t: f64,
        p0_x: f64,
        p0_y: f64,
        p1_x: f64,
        p1_y: f64,
        p2_x: f64,
        p2_y: f64,
        p3_x: f64,
        p3_y: f64,
    ) -> (f64, f64) {
        let u = 1.0 - t;
        let uuu = u * u * u;
        let uu = u * u;
        let tt = t * t;
        let ttt = tt * t;
        let rx = uuu * p0_x + 3.0 * uu * t * p1_x + 3.0 * u * tt * p2_x + ttt * p3_x;
        let ry = uuu * p0_y + 3.0 * uu * t * p1_y + 3.0 * u * tt * p2_y + ttt * p3_y;
        (rx, ry)
    }

    /// Rebuild Bezier segments when dirty (read-lock friendly after this call).
    pub fn ensure_segments_cached(&mut self) {
        if self.segments_dirty {
            self.generate_curve_segments_db();
        }
    }

    pub fn segments_ref(&self) -> &[CurveSegment] {
        &self.segments[..self.num_segments]
    }

    pub fn generate_curve_segments_db(&mut self) {
        self.num_segments = 0;
        if self.num_points < 2 {
            self.segments_dirty = false;
            return;
        }

        let mut prev_x_db = self.get_point_x(0);
        let mut prev_y_db = self.get_point_y(0);
        let t_step = 1.0 / BEZIER_STEPS as f64;
        let mut i = 0;
        while i < self.num_points - 1 {
            let next_idx = i + 1;
            let next_point_has_curve = self.is_valid_curve_point(next_idx);

            if next_point_has_curve {
                let (
                    mut p0_x_db,
                    mut p0_y_db,
                    p1_x_db,
                    p1_y_db,
                    p2_x_db,
                    p2_y_db,
                    mut p3_x_db,
                    mut p3_y_db,
                ) = self.calculate_bezier_control_points(next_idx);

                // Check overlap with previous curve
                if i > 0 && self.is_valid_curve_point(i) {
                    let (_, _, _, _, _, _, prev_p3_x, prev_p3_y) =
                        self.calculate_bezier_control_points(i);
                    if prev_p3_x > p0_x_db {
                        p0_x_db = (prev_p3_x + p0_x_db) / 2.0;
                        p0_y_db = (prev_p3_y + p0_y_db) / 2.0;
                    }
                }

                let next_next_idx = i + 2;
                let next_next_point_has_curve = self.is_valid_curve_point(next_next_idx);
                if next_next_point_has_curve {
                    let (n0_x, n0_y, _, _, _, _, _, _) =
                        self.calculate_bezier_control_points(next_next_idx);
                    if p3_x_db > n0_x {
                        p3_x_db = (p3_x_db + n0_x) / 2.0;
                        p3_y_db = (p3_y_db + n0_y) / 2.0;
                    }
                }

                let mut t = 0.0;
                while t < 1.0 && self.num_segments < MAX_CURVE_SEGMENTS - 1 {
                    let (current_x_db, current_y_db) = Self::evaluate_bezier_at_t(
                        t, p0_x_db, p0_y_db, p1_x_db, p1_y_db, p2_x_db, p2_y_db, p3_x_db, p3_y_db,
                    );
                    self.segments[self.num_segments] = CurveSegment {
                        x1: prev_x_db,
                        y1: prev_y_db,
                        x2: current_x_db,
                        y2: current_y_db,
                    };
                    self.num_segments += 1;
                    prev_x_db = current_x_db;
                    prev_y_db = current_y_db;
                    t += t_step;
                }
            } else {
                let end_x_db = self.get_point_x(next_idx);
                let end_y_db = self.get_point_y(next_idx);
                self.segments[self.num_segments] = CurveSegment {
                    x1: prev_x_db,
                    y1: prev_y_db,
                    x2: end_x_db,
                    y2: end_y_db,
                };
                self.num_segments += 1;
                prev_x_db = end_x_db;
                prev_y_db = end_y_db;
            }
            i += 1;
        }

        self.segments_dirty = false;
    }

    fn sample_curve_at_db_internal(&mut self, input_db: f64) -> f64 {
        if self.segments_dirty {
            self.generate_curve_segments_db();
        }
        sample_curve_with_segments(input_db, self.segments_ref(), self.min_db)
    }

    pub fn apply_extrapolation_caps(&self, input_db: f64, output_db: f64) -> f64 {
        apply_extrapolation_caps(input_db, output_db, self.min_db, self.max_db, self.range_db)
    }

    /// Sort knots, refresh corners, and mark curve segments dirty.
    pub fn finalize_after_edit(&mut self) {
        self.sort_points();
        self.update_corner_points();
        self.invalidate();
    }

    /// Refresh corners and mark dirty without re-sorting knots.
    pub fn finalize_after_edit_no_sort(&mut self) {
        self.update_corner_points();
        self.invalidate();
    }

    /// Adjust output dB of an interior knot (`delta_db` negative = more compression).
    pub fn adjust_interior_output_y(&mut self, point_index: usize, delta_db: f64) {
        if point_index > 0 && point_index < self.num_points - 1 {
            self.points[point_index * 2 + 1] -= delta_db;
        }
    }

    pub fn sample_curve_at_db(&mut self, input_db: f64) -> f64 {
        let output_db = self.sample_curve_at_db_internal(input_db);
        self.apply_extrapolation_caps(input_db, output_db)
    }

    pub fn sort_points(&mut self) {
        if self.num_points < 3 {
            return;
        }
        let mut i = 1;
        while i < self.num_points - 2 {
            let mut j = i + 1;
            while j < self.num_points - 1 {
                if self.points[i * 2] > self.points[j * 2] {
                    self.points.swap(i * 2, j * 2);
                    self.points.swap(i * 2 + 1, j * 2 + 1);
                }
                j += 1;
            }
            i += 1;
        }
        self.update_corner_points();
    }

    pub fn add_point(&mut self, input_db: f64, output_db: f64) -> Option<usize> {
        if self.num_points >= MAX_POINTS {
            return None;
        }
        let mut insert_pos = self.num_points - 1;
        for i in 1..self.num_points - 1 {
            if self.points[i * 2] > input_db {
                insert_pos = i;
                break;
            }
        }
        for i in (insert_pos + 1..=self.num_points).rev() {
            self.points[i * 2] = self.points[(i - 1) * 2];
            self.points[i * 2 + 1] = self.points[(i - 1) * 2 + 1];
        }
        self.points[insert_pos * 2] = input_db;
        self.points[insert_pos * 2 + 1] = output_db;
        self.num_points += 1;
        self.update_corner_points();
        self.invalidate();
        Some(insert_pos)
    }

    pub fn delete_point(&mut self, point_index: usize) -> bool {
        if point_index == 0 || point_index >= self.num_points - 1 || self.num_points <= MIN_POINTS {
            return false;
        }
        for i in point_index..self.num_points - 1 {
            self.points[i * 2] = self.points[(i + 1) * 2];
            self.points[i * 2 + 1] = self.points[(i + 1) * 2 + 1];
        }
        self.num_points -= 1;
        self.update_corner_points();
        self.invalidate();
        true
    }

    pub fn move_interior_point(&mut self, point_index: usize, input_db: f64, mut output_db: f64) {
        if point_index == 0 || point_index >= self.num_points - 1 {
            return;
        }
        if (output_db - input_db).abs() < 0.5 {
            output_db = input_db;
        }
        self.points[point_index * 2] = input_db;
        self.points[point_index * 2 + 1] = output_db;
        self.sort_points();
        self.invalidate();
    }

    pub fn reset_point_to_unity(&mut self, point_index: usize) {
        if point_index == 0 || point_index >= self.num_points - 1 {
            return;
        }
        let input_db = self.points[point_index * 2];
        self.points[point_index * 2 + 1] = input_db;
        self.update_corner_points();
        self.invalidate();
    }

    /// Return cached Bezier segments, rebuilding when dirty.
    pub fn segments(&mut self) -> &[CurveSegment] {
        if self.segments_dirty {
            self.generate_curve_segments_db();
        }
        self.segments_ref()
    }
}

#[inline]
fn segment_contains(seg: &CurveSegment, input_db: f64) -> bool {
    (input_db >= seg.x1 - 0.0001 && input_db <= seg.x2 + 0.0001)
        || (input_db >= seg.x2 - 0.0001 && input_db <= seg.x1 + 0.0001)
}

#[inline]
fn interpolate_segment(seg: &CurveSegment, input_db: f64) -> f64 {
    let seg_width = seg.x2 - seg.x1;
    if seg_width.abs() > 0.0001 {
        let t = ((input_db - seg.x1) / seg_width).clamp(0.0, 1.0);
        seg.y1 + t * (seg.y2 - seg.y1)
    } else {
        seg.y1
    }
}

pub fn sample_curve_with_segments(input_db: f64, segments: &[CurveSegment], min_db: f64) -> f64 {
    if segments.is_empty() {
        return input_db;
    }

    if input_db < min_db {
        return input_db;
    }

    let first = &segments[0];
    if input_db <= first.x1 {
        let seg_width = first.x2 - first.x1;
        if seg_width.abs() > 0.0001 {
            let slope = (first.y2 - first.y1) / seg_width;
            return first.y1 + slope * (input_db - first.x1);
        }
        return first.y1;
    }

    let last_idx = segments.len() - 1;
    let last_x = segments[last_idx].x2;
    if input_db >= last_x {
        let last = &segments[last_idx];
        // A knot on the right edge coincides with the hidden corner. Skip
        // zero-width trailing segments so the extension keeps the incoming tangent.
        if let Some(tangent) = segments
            .iter()
            .rev()
            .find(|seg| (seg.x2 - seg.x1).abs() > 0.0001)
        {
            let slope = (tangent.y2 - tangent.y1) / (tangent.x2 - tangent.x1);
            return last.y2 + slope * (input_db - last.x2);
        }
        return last.y2;
    }

    // Segments are generated in monotonic x order — binary search then check neighbors.
    let mut lo = 0usize;
    let mut hi = last_idx;
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if segments[mid].x1 <= input_db + 0.0001 {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    for idx in lo..=lo.saturating_add(1).min(last_idx) {
        let seg = &segments[idx];
        if segment_contains(seg, input_db) {
            return interpolate_segment(seg, input_db);
        }
    }

    input_db
}

#[inline]
pub fn apply_extrapolation_caps(
    input_db: f64,
    output_db: f64,
    min_db: f64,
    max_db: f64,
    range_db: f64,
) -> f64 {
    if input_db < min_db {
        return input_db;
    }
    if input_db > max_db {
        let upper_cap_db = input_db;
        let lower_cap_db = input_db - range_db;
        return output_db.clamp(lower_cap_db, upper_cap_db);
    }
    output_db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_curve_settings_cannot_restore_linear_elbows() {
        let mut graph = CompressionGraph::new();
        graph.move_interior_point(2, -12.0, -16.0);
        let expected = graph.segments().to_vec();
        assert!(expected.len() > graph.num_points);
        for amount in [0.0, 35.0, 100.0] {
            let mut saved = serde_json::to_value(graph).unwrap();
            saved["curve_amounts"] = serde_json::json!(vec![amount; MAX_POINTS]);
            let mut restored: CompressionGraph = serde_json::from_value(saved).unwrap();
            assert_eq!(restored.segments(), expected.as_slice());
            assert!(serde_json::to_value(restored)
                .unwrap()
                .get("curve_amounts")
                .is_none());
        }
    }

    #[test]
    fn new_and_reset_nodes_keep_maximum_curvature() {
        let mut graph = CompressionGraph::new();
        let idx = graph.add_point(-10.0, -14.0).unwrap();
        for reset in [false, true] {
            if reset {
                graph.reset_point_to_unity(idx);
            }
            let controls = graph.calculate_bezier_control_points(idx);
            assert_eq!(
                (controls.0, controls.1),
                (graph.get_point_x(idx - 1), graph.get_point_y(idx - 1))
            );
            assert_eq!(
                (controls.6, controls.7),
                (graph.get_point_x(idx + 1), graph.get_point_y(idx + 1))
            );
        }
    }

    #[test]
    fn default_graph_is_unity_line() {
        let mut g = CompressionGraph::new();
        for inp in [-15.0, -10.0, -5.0, -1.0] {
            let out = g.sample_curve_at_db(inp);
            assert!((out - inp).abs() < 0.01, "inp={inp} out={out}");
        }
    }

    #[test]
    fn right_edge_knot_keeps_its_tangent_in_sampler_and_lut() {
        let mut graph = CompressionGraph::new();
        graph.num_points = 4;
        graph.points[2..6].copy_from_slice(&[-8.0, -8.0, 0.0, -4.0]);
        graph.finalize_after_edit_no_sort();
        assert!((graph.sample_curve_at_db(0.0) + 4.0).abs() < 1e-9);
        assert!((graph.sample_curve_at_db(2.0) + 3.0).abs() < 1e-9);
        let mut lut = super::super::compression_lut::CompressionLUT::new();
        lut.build_lut(&graph);
        assert!((lut.lookup(2.0) + 3.0).abs() < 1e-9);
    }

    #[test]
    fn corner_points_extend_tangents() {
        let mut g = CompressionGraph::new();
        g.points[3] -= 5.0;
        g.update_corner_points();
        g.invalidate();
        let out = g.sample_curve_at_db(-16.0);
        assert!(out < -16.0);
    }

    #[test]
    fn add_and_delete_point() {
        let mut g = CompressionGraph::new();
        let n0 = g.num_points;
        let idx = g.add_point(-8.0, -10.0).unwrap();
        assert_eq!(g.num_points, n0 + 1);
        assert!((g.get_point_x(idx) - (-8.0)).abs() < 0.01);
        assert!(g.delete_point(idx));
        assert_eq!(g.num_points, n0);
    }

    #[test]
    fn sample_curve_binary_search_matches_linear_scan() {
        let mut g = CompressionGraph::new();
        g.adjust_interior_output_y(1, 8.0);
        g.adjust_interior_output_y(2, 4.0);
        g.finalize_after_edit();
        let segments = g.segments().to_vec();

        for i in 0..400 {
            let input_db = -19.0 + (i as f64) * 0.1;
            let expected = segments
                .iter()
                .find(|seg| segment_contains(seg, input_db))
                .map(|seg| interpolate_segment(seg, input_db))
                .unwrap_or(input_db);
            let actual = sample_curve_with_segments(input_db, &segments, g.min_db);
            assert!(
                (actual - expected).abs() < 1e-9,
                "input={input_db} expected={expected} actual={actual}"
            );
        }
    }

    #[test]
    fn remap_preserves_normalized_interior_positions() {
        let mut g = CompressionGraph::new();
        g.points[6] = -12.0;
        g.points[7] = -14.0;
        g.remap_points_for_range(40.0);
        assert!((g.range_db - 40.0).abs() < 0.01);
        assert!((g.min_db - (-40.0)).abs() < 0.01);
        let norm_x = (g.points[6] - g.min_db) / g.range_db;
        let norm_y = (g.points[7] - g.min_db) / g.range_db;
        assert!((norm_x - 0.4).abs() < 0.02);
        assert!((norm_y - 0.3).abs() < 0.02);
    }
}
