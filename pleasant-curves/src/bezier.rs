#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const MAX_NODES: usize = 16;
const MIN_GAP: f32 = 1.0 / 126.0;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelHandle {
    pub x: f32,
    pub y: f32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelNode {
    pub x: f32,
    pub y: f32,
    pub in_handle: VelHandle,
    pub out_handle: VelHandle,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct VelCurve {
    pub nodes: Vec<VelNode>,
}

pub type Handle = VelHandle;
pub type Node = VelNode;
pub type BezierCurve = VelCurve;

impl VelHandle {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl VelNode {
    fn identity_start() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            in_handle: VelHandle::new(0.0, 0.0),
            out_handle: VelHandle::new(1.0 / 3.0, 1.0 / 3.0),
        }
    }

    fn identity_end() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            in_handle: VelHandle::new(2.0 / 3.0, 2.0 / 3.0),
            out_handle: VelHandle::new(1.0, 1.0),
        }
    }

    fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
        self.in_handle.x += dx;
        self.in_handle.y += dy;
        self.out_handle.x += dx;
        self.out_handle.y += dy;
    }

    fn set_out_handle(&mut self, x: f32, y: f32) {
        self.out_handle.x = x;
        self.out_handle.y = y;
        self.in_handle.x = 2.0 * self.x - x;
        self.in_handle.y = 2.0 * self.y - y;
    }

    fn set_in_handle(&mut self, x: f32, y: f32) {
        self.in_handle.x = x;
        self.in_handle.y = y;
        self.out_handle.x = 2.0 * self.x - x;
        self.out_handle.y = 2.0 * self.y - y;
    }
}

impl VelCurve {
    pub fn identity() -> Self {
        Self {
            nodes: vec![VelNode::identity_start(), VelNode::identity_end()],
        }
    }

    pub fn sanitize(mut self) -> Self {
        self.nodes
            .retain(|node| node.x.is_finite() && node.y.is_finite());
        if self.nodes.len() < 2 {
            return Self::identity();
        }
        self.nodes.truncate(MAX_NODES);
        self.nodes.sort_by(|a, b| a.x.total_cmp(&b.x));
        self.nodes[0].x = 0.0;
        let last = self.nodes.len() - 1;
        self.nodes[last].x = 1.0;
        for node in &mut self.nodes {
            node.x = node.x.clamp(0.0, 1.0);
            node.y = node.y.clamp(0.0, 1.0);
            for handle in [&mut node.in_handle, &mut node.out_handle] {
                if !handle.x.is_finite() {
                    handle.x = node.x;
                }
                handle.y = if handle.y.is_finite() {
                    handle.y.clamp(-1.0, 2.0)
                } else {
                    node.y
                };
            }
        }
        // Keep both endpoints and discard interior knots without room to drag.
        let mut index = 0;
        let mut previous_x = 0.0;
        self.nodes.retain(|node| {
            let keep = index == 0
                || index == last
                || (node.x >= previous_x + MIN_GAP && node.x <= 1.0 - MIN_GAP);
            index += 1;
            if keep {
                previous_x = node.x;
            }
            keep
        });
        self.constrain_handles();
        self
    }

    fn constrain_handles(&mut self) {
        let count = self.nodes.len();
        if count < 2 {
            return;
        }
        self.nodes[0].in_handle.x = self.nodes[0].in_handle.x.clamp(0.0, self.nodes[0].x);
        self.nodes[count - 1].out_handle.x = self.nodes[count - 1]
            .out_handle
            .x
            .clamp(self.nodes[count - 1].x, 1.0);
        for index in 0..count - 1 {
            let low = self.nodes[index].x;
            let high = self.nodes[index + 1].x.max(low);
            self.nodes[index].out_handle.x = self.nodes[index].out_handle.x.clamp(low, high);
            self.nodes[index + 1].in_handle.x = self.nodes[index + 1].in_handle.x.clamp(low, high);
            if self.nodes[index].out_handle.x > self.nodes[index + 1].in_handle.x {
                let midpoint =
                    0.5 * (self.nodes[index].out_handle.x + self.nodes[index + 1].in_handle.x);
                self.nodes[index].out_handle.x = midpoint;
                self.nodes[index + 1].in_handle.x = midpoint;
            }
        }
    }

    pub fn eval_y(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        if self.nodes.len() < 2 {
            return x;
        }
        if x <= self.nodes[0].x {
            return self.nodes[0].y;
        }
        let last = self.nodes.len() - 1;
        if x >= self.nodes[last].x {
            return self.nodes[last].y;
        }
        for index in 0..last {
            let start = &self.nodes[index];
            let end = &self.nodes[index + 1];
            if x >= start.x && x <= end.x {
                let t = solve_t(start.x, start.out_handle.x, end.in_handle.x, end.x, x);
                return cubic(start.y, start.out_handle.y, end.in_handle.y, end.y, t)
                    .clamp(0.0, 1.0);
            }
        }
        x
    }

    pub fn sample_points(&self, steps_per_segment: usize) -> Vec<(f32, f32)> {
        let steps = steps_per_segment.max(2);
        let mut points = Vec::new();
        if self.nodes.len() < 2 {
            return points;
        }
        for index in 0..self.nodes.len() - 1 {
            let start_node = &self.nodes[index];
            let end_node = &self.nodes[index + 1];
            let first_step = if index == 0 { 0 } else { 1 };
            for step in first_step..=steps {
                let t = step as f32 / steps as f32;
                let x = cubic(
                    start_node.x,
                    start_node.out_handle.x,
                    end_node.in_handle.x,
                    end_node.x,
                    t,
                );
                let y = cubic(
                    start_node.y,
                    start_node.out_handle.y,
                    end_node.in_handle.y,
                    end_node.y,
                    t,
                );
                points.push((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)));
            }
        }
        points
    }

    pub fn insert_at(&mut self, x: f32) -> Option<usize> {
        if self.nodes.len() >= MAX_NODES {
            return None;
        }
        let x = x.clamp(MIN_GAP, 1.0 - MIN_GAP);
        if self.nodes.iter().any(|node| (node.x - x).abs() < MIN_GAP) {
            return None;
        }
        let y = self.eval_y(x);
        let index = self
            .nodes
            .iter()
            .position(|node| node.x > x)
            .unwrap_or(self.nodes.len());
        if index == 0 || index >= self.nodes.len() {
            return None;
        }
        let previous = self.nodes[index - 1];
        let next = self.nodes[index];
        let dx = (next.x - previous.x) / 6.0;
        let dy = (next.y - previous.y) / 6.0;
        self.nodes.insert(
            index,
            VelNode {
                x,
                y,
                in_handle: VelHandle::new(x - dx, y - dy),
                out_handle: VelHandle::new(x + dx, y + dy),
            },
        );
        self.constrain_handles();
        Some(index)
    }

    pub fn delete(&mut self, index: usize) -> bool {
        if index == 0 || index + 1 >= self.nodes.len() || self.nodes.len() <= 2 {
            return false;
        }
        self.nodes.remove(index);
        true
    }

    pub fn move_node(&mut self, index: usize, x: f32, y: f32) {
        if index >= self.nodes.len() || !x.is_finite() || !y.is_finite() {
            return;
        }
        let y = y.clamp(0.0, 1.0);
        let x = if index == 0 {
            0.0
        } else if index + 1 == self.nodes.len() {
            1.0
        } else {
            let low = self.nodes[index - 1].x + MIN_GAP;
            let high = self.nodes[index + 1].x - MIN_GAP;
            if !low.is_finite() || !high.is_finite() || low > high {
                return;
            }
            x.clamp(low, high)
        };
        let dx = x - self.nodes[index].x;
        let dy = y - self.nodes[index].y;
        self.nodes[index].translate(dx, dy);
        self.nodes[index].y = self.nodes[index].y.clamp(0.0, 1.0);
        self.nodes[index].in_handle.y = self.nodes[index].in_handle.y.clamp(-1.0, 2.0);
        self.nodes[index].out_handle.y = self.nodes[index].out_handle.y.clamp(-1.0, 2.0);
        self.constrain_handles();
    }

    pub fn drag_out_handle(&mut self, index: usize, x: f32, y: f32) {
        if index >= self.nodes.len() {
            return;
        }
        let low = self.nodes[index].x;
        let mut high = if index + 1 < self.nodes.len() {
            self.nodes[index + 1].x
        } else {
            1.0
        };
        if index + 1 < self.nodes.len() {
            high = high.min(self.nodes[index + 1].in_handle.x);
        }
        self.nodes[index].set_out_handle(x.clamp(low, high.max(low)), y);
        self.constrain_handles();
    }

    pub fn drag_in_handle(&mut self, index: usize, x: f32, y: f32) {
        if index >= self.nodes.len() {
            return;
        }
        let high = self.nodes[index].x;
        let mut low = if index > 0 {
            self.nodes[index - 1].x
        } else {
            0.0
        };
        if index > 0 {
            low = low.max(self.nodes[index - 1].out_handle.x);
        }
        self.nodes[index].set_in_handle(x.clamp(low.min(high), high), y);
        self.constrain_handles();
    }

    pub fn hit_node(&self, x: f32, y: f32, radius: f32) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let distance = (node.x - x).hypot(node.y - y);
                (distance <= radius).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }
}

fn cubic(p0: f32, c0: f32, c1: f32, p1: f32, t: f32) -> f32 {
    let inverse = 1.0 - t;
    inverse * inverse * inverse * p0
        + 3.0 * inverse * inverse * t * c0
        + 3.0 * inverse * t * t * c1
        + t * t * t * p1
}

fn cubic_derivative(p0: f32, c0: f32, c1: f32, p1: f32, t: f32) -> f32 {
    let inverse = 1.0 - t;
    3.0 * inverse * inverse * (c0 - p0) + 6.0 * inverse * t * (c1 - c0) + 3.0 * t * t * (p1 - c1)
}

fn solve_t(p0: f32, c0: f32, c1: f32, p1: f32, x: f32) -> f32 {
    let span = p1 - p0;
    let mut t = if span.abs() < 1.0e-6 {
        0.5
    } else {
        ((x - p0) / span).clamp(0.0, 1.0)
    };
    for _ in 0..12 {
        let error = cubic(p0, c0, c1, p1, t) - x;
        if error.abs() < 1.0e-5 {
            break;
        }
        let derivative = cubic_derivative(p0, c0, c1, p1, t);
        if derivative.abs() < 1.0e-6 {
            break;
        }
        t = (t - error / derivative).clamp(0.0, 1.0);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parametric_x_monotonic(curve: &VelCurve) {
        let points = curve.sample_points(32);
        assert!(points.len() >= 2);
        for pair in points.windows(2) {
            assert!(pair[1].0 + 1.0e-4 >= pair[0].0);
        }
    }

    #[test]
    fn restored_duplicate_and_crowded_nodes_remain_editable() {
        let mut curve = VelCurve::identity();
        curve.nodes = [0.0, 0.001, 0.5, 0.5, 0.5, 0.999, 1.0]
            .map(|x| VelNode {
                x,
                y: x,
                in_handle: VelHandle::new(x, x),
                out_handle: VelHandle::new(x, x),
            })
            .to_vec();
        let saved = serde_json::to_string(&curve).unwrap();
        let mut restored = serde_json::from_str::<VelCurve>(&saved).unwrap().sanitize();
        assert_eq!(restored.nodes.len(), 3);
        for index in 0..restored.nodes.len() {
            restored.move_node(index, 0.5, 0.7);
        }
        assert_eq!(restored.nodes.first().unwrap().x, 0.0);
        assert_eq!(restored.nodes.last().unwrap().x, 1.0);
        assert_parametric_x_monotonic(&restored);
        assert!(restored.nodes.windows(2).all(|pair| pair[1].x > pair[0].x));
    }

    #[test]
    fn moving_unvalidated_crowded_nodes_does_not_panic() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.5);
        curve.nodes[0].x = 0.5;
        curve.nodes[2].x = 0.5;
        let before = curve.clone();
        curve.move_node(1, 0.6, 0.7);
        assert_eq!(curve, before);
    }

    #[test]
    fn minimum_gap_edits_survive_sanitizing_and_restoration() {
        for target in [0.25, 1.0] {
            let mut curve = VelCurve::identity();
            curve.insert_at(0.25);
            curve.insert_at(0.5);
            curve.move_node(2, target, 0.4);
            let saved = serde_json::to_string(&curve).unwrap();
            let restored = serde_json::from_str::<VelCurve>(&saved).unwrap().sanitize();
            assert_eq!(restored, curve, "minimum gap near {target}");
        }
    }

    #[test]
    fn sanitize_removes_nonfinite_nodes_and_repairs_handles() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.5);
        curve.nodes[1].x = f32::NAN;
        curve.nodes[0].out_handle.x = f32::INFINITY;
        curve.nodes[1].in_handle.y = f32::NEG_INFINITY;
        curve.nodes[2].in_handle.y = f32::NAN;
        let curve = curve.sanitize();
        assert_eq!(curve.nodes.len(), 2);
        assert_parametric_x_monotonic(&curve);
        for index in 0..=126 {
            assert!(curve.eval_y(index as f32 / 126.0).is_finite());
        }
    }

    #[test]
    fn edits_preserve_node_order() {
        let mut curve = VelCurve::identity();
        let index = curve.insert_at(0.5).expect("insert");
        curve.move_node(index, 0.99, 0.2);
        assert!(curve.nodes[index].x < curve.nodes[index + 1].x);
        assert!(!curve.delete(0));
        assert!(curve.delete(index));
    }

    #[test]
    fn persistence_keeps_handles() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.35);
        curve.drag_out_handle(1, 0.5, 0.8);
        let json = serde_json::to_string(&curve).unwrap();
        let restored: VelCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, curve);
    }

    #[test]
    fn crossed_handles_cannot_fold_x() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.5);
        curve.nodes[0].out_handle = VelHandle::new(0.95, 1.2);
        curve.nodes[1].in_handle = VelHandle::new(0.05, -0.4);
        curve = curve.sanitize();
        assert_parametric_x_monotonic(&curve);
        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((0.0..=1.0).contains(&curve.eval_y(x)));
        }
    }
}
