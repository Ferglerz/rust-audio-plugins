use nih_plug::params::persist::PersistentField;
use scd_core::kit::MAX_ARTICULATIONS;
use scd_core::KitPieceId;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub const MAX_NODES: usize = 16;
pub const MAX_ARTS: usize = MAX_ARTICULATIONS;
const MIN_GAP: f32 = 1.0 / 126.0;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct VelHandle {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct VelNode {
    pub x: f32,
    pub y: f32,
    pub in_handle: VelHandle,
    pub out_handle: VelHandle,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VelCurve {
    pub nodes: Vec<VelNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VelMapBank {
    #[serde(deserialize_with = "deserialize_curves")]
    pub curves: [[VelCurve; MAX_ARTS]; KitPieceId::COUNT],
}

fn deserialize_curves<'de, D>(
    deserializer: D,
) -> Result<[[VelCurve; MAX_ARTS]; KitPieceId::COUNT], D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Wire {
        Nested(Vec<Vec<VelCurve>>),
        Flat(Vec<VelCurve>),
    }
    match Wire::deserialize(deserializer)? {
        Wire::Nested(rows) => Ok(pad_bank(rows)),
        Wire::Flat(flat) => {
            let mut rows = vec![Vec::new(); KitPieceId::COUNT];
            for (i, curve) in flat.into_iter().take(KitPieceId::COUNT).enumerate() {
                rows[i] = vec![curve];
            }
            Ok(pad_bank(rows))
        }
    }
}

fn pad_arts(mut arts: Vec<VelCurve>) -> [VelCurve; MAX_ARTS] {
    arts.truncate(MAX_ARTS);
    while arts.len() < MAX_ARTS {
        arts.push(VelCurve::identity());
    }
    std::array::from_fn(|i| arts[i].clone())
}

fn pad_bank(mut rows: Vec<Vec<VelCurve>>) -> [[VelCurve; MAX_ARTS]; KitPieceId::COUNT] {
    rows.truncate(KitPieceId::COUNT);
    while rows.len() < KitPieceId::COUNT {
        rows.push(Vec::new());
    }
    std::array::from_fn(|i| pad_arts(std::mem::take(&mut rows[i])))
}

pub struct VelMapState {
    curves: Mutex<[[VelCurve; MAX_ARTS]; KitPieceId::COUNT]>,
    tables: [[VelTable; MAX_ARTS]; KitPieceId::COUNT],
}

struct VelTable([AtomicU8; 128]);

impl VelHandle {
    fn new(x: f32, y: f32) -> Self {
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
            node.in_handle.y = node.in_handle.y.clamp(-1.0, 2.0);
            node.out_handle.y = node.out_handle.y.clamp(-1.0, 2.0);
        }
        self.constrain_handles();
        self
    }

    /// Keep each cubic's X control points ordered `P0 ≤ C0 ≤ C1 ≤ P1`.
    /// That makes `dx/dt ≥ 0`, so the graph cannot fold backwards.
    fn constrain_handles(&mut self) {
        let n = self.nodes.len();
        if n < 2 {
            return;
        }
        self.nodes[0].in_handle.x = self.nodes[0].in_handle.x.clamp(0.0, self.nodes[0].x);
        self.nodes[n - 1].out_handle.x = self.nodes[n - 1]
            .out_handle
            .x
            .clamp(self.nodes[n - 1].x, 1.0);
        for i in 0..n - 1 {
            let lo = self.nodes[i].x;
            let hi = self.nodes[i + 1].x;
            let hi = hi.max(lo);
            self.nodes[i].out_handle.x = self.nodes[i].out_handle.x.clamp(lo, hi);
            self.nodes[i + 1].in_handle.x = self.nodes[i + 1].in_handle.x.clamp(lo, hi);
            if self.nodes[i].out_handle.x > self.nodes[i + 1].in_handle.x {
                let mid = 0.5 * (self.nodes[i].out_handle.x + self.nodes[i + 1].in_handle.x);
                self.nodes[i].out_handle.x = mid;
                self.nodes[i + 1].in_handle.x = mid;
            }
        }
    }

    pub fn eval_y(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let nodes = &self.nodes;
        if nodes.len() < 2 {
            return x;
        }
        if x <= nodes[0].x {
            return nodes[0].y;
        }
        let last = nodes.len() - 1;
        if x >= nodes[last].x {
            return nodes[last].y;
        }
        for i in 0..last {
            let a = &nodes[i];
            let b = &nodes[i + 1];
            if x >= a.x && x <= b.x {
                let t = solve_t(a.x, a.out_handle.x, b.in_handle.x, b.x, x);
                return cubic(a.y, a.out_handle.y, b.in_handle.y, b.y, t).clamp(0.0, 1.0);
            }
        }
        x
    }

    pub fn rasterize(&self) -> [u8; 128] {
        let mut table = [1u8; 128];
        for vel in 1..=127u8 {
            table[vel as usize] = norm_to_midi(self.eval_y(midi_to_norm(vel)));
        }
        table[0] = table[1];
        table
    }

    pub fn lookup(&self, vel: u8) -> u8 {
        self.rasterize()[vel.min(127) as usize]
    }

    pub fn sample_points(&self, steps_per_segment: usize) -> Vec<(f32, f32)> {
        let steps = steps_per_segment.max(2);
        let mut pts = Vec::new();
        if self.nodes.len() < 2 {
            return pts;
        }
        for i in 0..self.nodes.len() - 1 {
            let a = &self.nodes[i];
            let b = &self.nodes[i + 1];
            let start = if i == 0 { 0 } else { 1 };
            for s in start..=steps {
                let t = s as f32 / steps as f32;
                let x = cubic(a.x, a.out_handle.x, b.in_handle.x, b.x, t);
                let y = cubic(a.y, a.out_handle.y, b.in_handle.y, b.y, t);
                pts.push((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)));
            }
        }
        pts
    }

    pub fn insert_at(&mut self, x: f32) -> Option<usize> {
        if self.nodes.len() >= MAX_NODES {
            return None;
        }
        let x = x.clamp(MIN_GAP, 1.0 - MIN_GAP);
        for node in &self.nodes {
            if (node.x - x).abs() < MIN_GAP {
                return None;
            }
        }
        let y = self.eval_y(x);
        let idx = self
            .nodes
            .iter()
            .position(|n| n.x > x)
            .unwrap_or(self.nodes.len());
        if idx == 0 || idx >= self.nodes.len() {
            return None;
        }
        let prev = self.nodes[idx - 1];
        let next = self.nodes[idx];
        let dx = (next.x - prev.x) / 6.0;
        let dy = (next.y - prev.y) / 6.0;
        self.nodes.insert(
            idx,
            VelNode {
                x,
                y,
                in_handle: VelHandle::new(x - dx, y - dy),
                out_handle: VelHandle::new(x + dx, y + dy),
            },
        );
        self.constrain_handles();
        Some(idx)
    }

    pub fn delete(&mut self, index: usize) -> bool {
        if index == 0 || index + 1 >= self.nodes.len() || self.nodes.len() <= 2 {
            return false;
        }
        self.nodes.remove(index);
        true
    }

    pub fn move_node(&mut self, index: usize, x: f32, y: f32) {
        if index >= self.nodes.len() {
            return;
        }
        let y = y.clamp(0.0, 1.0);
        let x = if index == 0 {
            0.0
        } else if index + 1 == self.nodes.len() {
            1.0
        } else {
            let lo = self.nodes[index - 1].x + MIN_GAP;
            let hi = self.nodes[index + 1].x - MIN_GAP;
            x.clamp(lo, hi)
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
        let lo = self.nodes[index].x;
        let mut hi = if index + 1 < self.nodes.len() {
            self.nodes[index + 1].x
        } else {
            1.0
        };
        if index + 1 < self.nodes.len() {
            hi = hi.min(self.nodes[index + 1].in_handle.x);
        }
        self.nodes[index].set_out_handle(x.clamp(lo, hi.max(lo)), y);
        self.constrain_handles();
    }

    pub fn drag_in_handle(&mut self, index: usize, x: f32, y: f32) {
        if index >= self.nodes.len() {
            return;
        }
        let hi = self.nodes[index].x;
        let mut lo = if index > 0 {
            self.nodes[index - 1].x
        } else {
            0.0
        };
        if index > 0 {
            lo = lo.max(self.nodes[index - 1].out_handle.x);
        }
        self.nodes[index].set_in_handle(x.clamp(lo.min(hi), hi), y);
        self.constrain_handles();
    }

    pub fn hit_node(&self, x: f32, y: f32, radius: f32) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let d = (n.x - x).hypot(n.y - y);
                (d <= radius).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
}

impl VelMapBank {
    pub fn identity() -> Self {
        Self {
            curves: std::array::from_fn(|_| std::array::from_fn(|_| VelCurve::identity())),
        }
    }
}

impl VelTable {
    fn identity() -> Self {
        Self(std::array::from_fn(|i| {
            AtomicU8::new(if i == 0 { 1 } else { i as u8 })
        }))
    }

    fn store(&self, table: &[u8; 128]) {
        for (slot, value) in self.0.iter().zip(table) {
            slot.store(*value, Ordering::Relaxed);
        }
    }

    fn get(&self, vel: u8) -> u8 {
        self.0[vel.min(127) as usize].load(Ordering::Relaxed)
    }
}

impl VelMapState {
    pub fn identity() -> Self {
        Self {
            curves: Mutex::new(std::array::from_fn(|_| {
                std::array::from_fn(|_| VelCurve::identity())
            })),
            tables: std::array::from_fn(|_| std::array::from_fn(|_| VelTable::identity())),
        }
    }

    pub fn lookup(&self, kit_piece: KitPieceId, art: usize, vel: u8) -> u8 {
        self.tables[kit_piece as usize][art.min(MAX_ARTS - 1)].get(vel)
    }

    pub fn curve(&self, kit_piece: KitPieceId, art: usize) -> VelCurve {
        self.lock_curves()[kit_piece as usize][art.min(MAX_ARTS - 1)].clone()
    }

    pub fn set_curve(&self, kit_piece: KitPieceId, art: usize, curve: VelCurve) {
        let art = art.min(MAX_ARTS - 1);
        let curve = curve.sanitize();
        let raster = curve.rasterize();
        self.lock_curves()[kit_piece as usize][art] = curve;
        self.tables[kit_piece as usize][art].store(&raster);
    }

    pub fn reset_art(&self, kit_piece: KitPieceId, art: usize) {
        self.set_curve(kit_piece, art, VelCurve::identity());
    }

    pub fn reset_piece(&self, kit_piece: KitPieceId) {
        for art in 0..MAX_ARTS {
            self.reset_art(kit_piece, art);
        }
    }

    pub fn reset_all(&self) {
        for kit_piece in KitPieceId::ALL {
            self.reset_piece(kit_piece);
        }
    }

    fn bank(&self) -> VelMapBank {
        VelMapBank {
            curves: self.lock_curves().clone(),
        }
    }

    pub fn snapshot(&self) -> VelMapBank {
        self.bank()
    }

    pub fn load_bank(&self, bank: VelMapBank) {
        self.replace_bank(bank);
    }

    fn replace_bank(&self, bank: VelMapBank) {
        let curves = std::array::from_fn(|i| {
            std::array::from_fn(|a| bank.curves[i][a].clone().sanitize())
        });
        for (i, arts) in curves.iter().enumerate() {
            for (a, curve) in arts.iter().enumerate() {
                self.tables[i][a].store(&curve.rasterize());
            }
        }
        *self.lock_curves() = curves;
    }

    fn lock_curves(&self) -> std::sync::MutexGuard<'_, [[VelCurve; MAX_ARTS]; KitPieceId::COUNT]> {
        self.curves.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Default for VelMapState {
    fn default() -> Self {
        Self::identity()
    }
}

impl<'a> PersistentField<'a, VelMapBank> for Arc<VelMapState> {
    fn set(&self, new_value: VelMapBank) {
        self.replace_bank(new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&VelMapBank) -> R,
    {
        f(&self.bank())
    }
}

pub fn midi_to_norm(vel: u8) -> f32 {
    (vel.clamp(1, 127) as f32 - 1.0) / 126.0
}

pub fn norm_to_midi(norm: f32) -> u8 {
    (norm.clamp(0.0, 1.0) * 126.0).round() as u8 + 1
}

fn cubic(p0: f32, c0: f32, c1: f32, p1: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    u * u * u * p0 + 3.0 * u * u * t * c0 + 3.0 * u * t * t * c1 + t * t * t * p1
}

fn cubic_dt(p0: f32, c0: f32, c1: f32, p1: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * (c0 - p0) + 6.0 * u * t * (c1 - c0) + 3.0 * t * t * (p1 - c1)
}

fn solve_t(p0: f32, c0: f32, c1: f32, p1: f32, x: f32) -> f32 {
    let span = p1 - p0;
    let mut t = if span.abs() < 1e-6 {
        0.5
    } else {
        ((x - p0) / span).clamp(0.0, 1.0)
    };
    for _ in 0..12 {
        let f = cubic(p0, c0, c1, p1, t) - x;
        if f.abs() < 1e-5 {
            break;
        }
        let d = cubic_dt(p0, c0, c1, p1, t);
        if d.abs() < 1e-6 {
            break;
        }
        t = (t - f / d).clamp(0.0, 1.0);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_raster_is_one_to_127() {
        let table = VelCurve::identity().rasterize();
        assert_eq!(table[0], 1);
        for vel in 1..=127u8 {
            assert_eq!(table[vel as usize], vel, "vel {vel}");
        }
    }

    #[test]
    fn identity_lookup_passthrough() {
        let curve = VelCurve::identity();
        assert_eq!(curve.lookup(1), 1);
        assert_eq!(curve.lookup(64), 64);
        assert_eq!(curve.lookup(127), 127);
    }

    #[test]
    fn insert_keeps_x_increasing() {
        let mut curve = VelCurve::identity();
        let idx = curve.insert_at(0.5).expect("insert");
        assert_eq!(idx, 1);
        assert_eq!(curve.nodes.len(), 3);
        for pair in curve.nodes.windows(2) {
            assert!(pair[1].x > pair[0].x);
        }
        assert_eq!(curve.nodes.first().unwrap().x, 0.0);
        assert_eq!(curve.nodes.last().unwrap().x, 1.0);
    }

    #[test]
    fn cannot_delete_endpoints() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.4);
        assert!(!curve.delete(0));
        assert!(!curve.delete(curve.nodes.len() - 1));
        assert!(curve.delete(1));
        assert_eq!(curve.nodes.len(), 2);
        assert!(!curve.delete(1));
    }

    #[test]
    fn move_node_cannot_cross_neighbors() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.5);
        curve.move_node(1, 0.99, 0.2);
        assert!(curve.nodes[1].x < curve.nodes[2].x);
        curve.move_node(1, 0.0, 0.2);
        assert!(curve.nodes[1].x > curve.nodes[0].x);
        curve.move_node(0, 0.4, 0.8);
        assert_eq!(curve.nodes[0].x, 0.0);
        assert!((curve.nodes[0].y - 0.8).abs() < 1e-5);
    }

    #[test]
    fn persist_round_trip_keeps_handles() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.35);
        curve.drag_out_handle(1, 0.5, 0.8);
        let json = serde_json::to_string(&curve).unwrap();
        let back: VelCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(back.nodes.len(), 3);
        assert!((back.nodes[1].out_handle.x - 0.5).abs() < 1e-5);
        assert!((back.nodes[1].out_handle.y - 0.8).abs() < 1e-5);
        assert!((back.nodes[1].in_handle.x - curve.nodes[1].in_handle.x).abs() < 1e-5);
        assert_parametric_x_monotonic(&curve);
    }

    #[test]
    fn skew_changes_lookup() {
        let mut curve = VelCurve::identity();
        curve.move_node(0, 0.0, 1.0);
        curve.move_node(1, 1.0, 1.0);
        assert!(curve.lookup(40) > 40);
    }

    #[test]
    fn bank_persist_restores_atomics() {
        let state = Arc::new(VelMapState::identity());
        let mut curve = VelCurve::identity();
        curve.move_node(0, 0.0, 1.0);
        curve.move_node(1, 1.0, 1.0);
        state.set_curve(KitPieceId::Kick, 0, curve.clone());
        assert!(state.lookup(KitPieceId::Kick, 0, 40) > 40);

        let mut bank = VelMapBank::identity();
        bank.curves[KitPieceId::Kick as usize][0] = curve.clone();
        let restored = Arc::new(VelMapState::identity());
        PersistentField::set(&restored, bank);
        assert_eq!(
            restored.lookup(KitPieceId::Kick, 0, 40),
            state.lookup(KitPieceId::Kick, 0, 40)
        );
        assert_eq!(restored.lookup(KitPieceId::Snare, 0, 40), 40);
        assert_eq!(restored.lookup(KitPieceId::Kick, 1, 40), 40);
    }

    fn assert_parametric_x_monotonic(curve: &VelCurve) {
        let pts = curve.sample_points(32);
        assert!(pts.len() >= 2);
        for pair in pts.windows(2) {
            assert!(
                pair[1].0 + 1e-4 >= pair[0].0,
                "x went backwards: {} -> {}",
                pair[0].0,
                pair[1].0
            );
        }
        for i in 0..curve.nodes.len() - 1 {
            let a = &curve.nodes[i];
            let b = &curve.nodes[i + 1];
            assert!(a.out_handle.x + 1e-5 >= a.x);
            assert!(b.in_handle.x + 1e-5 >= a.out_handle.x);
            assert!(b.x + 1e-5 >= b.in_handle.x);
        }
    }

    #[test]
    fn crossed_handles_cannot_fold_x() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.5);
        curve.nodes[0].out_handle = VelHandle::new(0.95, 1.2);
        curve.nodes[1].in_handle = VelHandle::new(0.05, -0.4);
        curve.nodes[1].out_handle = VelHandle::new(-0.2, 0.9);
        curve = curve.sanitize();
        assert_parametric_x_monotonic(&curve);
        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let y = curve.eval_y(x);
            assert!((0.0..=1.0).contains(&y), "eval_y({x})={y}");
        }
    }

    #[test]
    fn dragging_handle_past_neighbors_stays_monotonic() {
        let mut curve = VelCurve::identity();
        curve.insert_at(0.5);
        curve.drag_out_handle(0, 2.0, -1.0);
        curve.drag_in_handle(1, -1.0, 2.0);
        curve.drag_out_handle(1, -1.0, 1.5);
        curve.drag_in_handle(2, 2.0, -1.0);
        assert_parametric_x_monotonic(&curve);
    }
}
