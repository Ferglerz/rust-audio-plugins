use nih_plug::params::persist::PersistentField;
use scd_core::kit::MAX_ARTICULATIONS;
use scd_core::KitPieceId;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub use pleasant_curves::{VelCurve, VelHandle, VelNode, MAX_NODES};

pub const MAX_ARTS: usize = MAX_ARTICULATIONS;
/// The extra persisted slot is a kit-piece curve applied after articulation mapping.
pub const ALL_ART: usize = MAX_ARTS;
const MAP_SLOTS: usize = MAX_ARTS + 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VelMapBank {
    #[serde(deserialize_with = "deserialize_curves")]
    pub curves: [[VelCurve; MAP_SLOTS]; KitPieceId::COUNT],
}

fn deserialize_curves<'de, D>(
    deserializer: D,
) -> Result<[[VelCurve; MAP_SLOTS]; KitPieceId::COUNT], D::Error>
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

fn pad_arts(mut arts: Vec<VelCurve>) -> [VelCurve; MAP_SLOTS] {
    arts.truncate(MAP_SLOTS);
    while arts.len() < MAP_SLOTS {
        arts.push(VelCurve::identity());
    }
    std::array::from_fn(|i| arts[i].clone())
}

fn pad_bank(mut rows: Vec<Vec<VelCurve>>) -> [[VelCurve; MAP_SLOTS]; KitPieceId::COUNT] {
    rows.truncate(KitPieceId::COUNT);
    while rows.len() < KitPieceId::COUNT {
        rows.push(Vec::new());
    }
    std::array::from_fn(|i| pad_arts(std::mem::take(&mut rows[i])))
}

pub struct VelMapState {
    curves: Mutex<[[VelCurve; MAP_SLOTS]; KitPieceId::COUNT]>,
    tables: [[VelTable; MAP_SLOTS]; KitPieceId::COUNT],
}

struct VelTable([AtomicU8; 128]);

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
        let tables = &self.tables[kit_piece as usize];
        tables[ALL_ART].get(tables[art.min(MAX_ARTS - 1)].get(vel))
    }

    pub fn curve(&self, kit_piece: KitPieceId, art: usize) -> VelCurve {
        self.lock_curves()[kit_piece as usize][art.min(ALL_ART)].clone()
    }

    pub fn set_curve(&self, kit_piece: KitPieceId, art: usize, curve: VelCurve) {
        let art = art.min(ALL_ART);
        let curve = curve.sanitize();
        let raster = curve.rasterize();
        self.lock_curves()[kit_piece as usize][art] = curve;
        self.tables[kit_piece as usize][art].store(&raster);
    }

    pub fn reset_art(&self, kit_piece: KitPieceId, art: usize) {
        self.set_curve(kit_piece, art, VelCurve::identity());
    }

    pub fn reset_piece(&self, kit_piece: KitPieceId) {
        for art in 0..MAP_SLOTS {
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
        let curves =
            std::array::from_fn(|i| std::array::from_fn(|a| bank.curves[i][a].clone().sanitize()));
        for (i, arts) in curves.iter().enumerate() {
            for (a, curve) in arts.iter().enumerate() {
                self.tables[i][a].store(&curve.rasterize());
            }
        }
        *self.lock_curves() = curves;
    }

    fn lock_curves(&self) -> std::sync::MutexGuard<'_, [[VelCurve; MAP_SLOTS]; KitPieceId::COUNT]> {
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

trait VelCurveMidiExt {
    fn rasterize(&self) -> [u8; 128];
    #[cfg_attr(not(test), allow(dead_code))]
    fn lookup(&self, velocity: u8) -> u8;
}

impl VelCurveMidiExt for VelCurve {
    fn rasterize(&self) -> [u8; 128] {
        let mut table = [1_u8; 128];
        for velocity in 1..=127_u8 {
            table[velocity as usize] = norm_to_midi(self.eval_y(midi_to_norm(velocity)));
        }
        table[0] = table[1];
        table
    }

    fn lookup(&self, velocity: u8) -> u8 {
        self.rasterize()[velocity.min(127) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_mapping_composes_after_articulation_and_persists() {
        let state = VelMapState::identity();
        let mut art = VelCurve::identity();
        art.move_node(1, 1.0, 0.5);
        let mut all = VelCurve::identity();
        all.move_node(0, 0.0, 0.25);
        state.set_curve(KitPieceId::Kick, 0, art.clone());
        state.set_curve(KitPieceId::Kick, ALL_ART, all.clone());
        for velocity in 1..=127 {
            assert_eq!(
                state.lookup(KitPieceId::Kick, 0, velocity),
                all.lookup(art.lookup(velocity))
            );
            assert_eq!(
                state.lookup(KitPieceId::Kick, 1, velocity),
                all.lookup(velocity)
            );
            assert_eq!(state.lookup(KitPieceId::Snare, 0, velocity), velocity);
        }
        let json = serde_json::to_string(&state.snapshot()).unwrap();
        let restored = VelMapState::identity();
        restored.load_bank(serde_json::from_str(&json).unwrap());
        assert_eq!(restored.snapshot(), state.snapshot());
        assert_eq!(
            restored.lookup(KitPieceId::Kick, 0, 100),
            all.lookup(art.lookup(100))
        );
        restored.reset_art(KitPieceId::Kick, ALL_ART);
        assert_eq!(restored.curve(KitPieceId::Kick, 0), art);
        assert_eq!(restored.lookup(KitPieceId::Kick, 0, 100), art.lookup(100));
        state.reset_piece(KitPieceId::Kick);
        assert_eq!(state.lookup(KitPieceId::Kick, 0, 100), 100);
    }

    #[test]
    fn legacy_bank_defaults_all_to_identity() {
        let mut art = VelCurve::identity();
        art.move_node(1, 1.0, 0.5);
        let old_rows = vec![vec![art.clone(); MAX_ARTS]; KitPieceId::COUNT];
        let bank: VelMapBank =
            serde_json::from_value(serde_json::json!({"curves": old_rows})).unwrap();
        let state = VelMapState::identity();
        state.load_bank(bank);
        assert_eq!(state.curve(KitPieceId::Kick, ALL_ART), VelCurve::identity());
        assert_eq!(state.lookup(KitPieceId::Kick, 0, 100), art.lookup(100));
    }

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
