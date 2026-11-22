//! Published graph variants: edits prepare every supported range off the audio thread.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use nih_plug::params::persist::PersistentField;

use crate::dsp::graph::CompressionGraph;
pub use crate::dsp::graph_snapshot::GraphSnapshot;
use crate::params::GraphRangeMode;

fn range_index(range_db: f64) -> Option<u8> {
    GraphRangeMode::ALL
        .iter()
        .position(|mode| (range_db - mode.range_db()).abs() <= 0.01)
        .map(|index| index as u8)
}

fn mode_index(mode: GraphRangeMode) -> u8 {
    mode as u8
}

struct GraphVersions {
    prepared: [Arc<GraphSnapshot>; GraphRangeMode::ALL.len()],
}

impl GraphVersions {
    fn new(graph: CompressionGraph) -> Self {
        let source_index = range_index(graph.range_db);
        let prepared = std::array::from_fn(|index| {
            let mut variant = graph;
            if Some(index as u8) != source_index {
                variant.remap_points_for_range(GraphRangeMode::ALL[index].range_db());
            }
            Arc::new(GraphSnapshot::from_graph(variant))
        });
        Self { prepared }
    }

    fn at(&self, index: u8) -> &Arc<GraphSnapshot> {
        &self.prepared[index as usize]
    }

    fn has_external_reader(&self) -> bool {
        self.prepared
            .iter()
            .any(|snapshot| Arc::strong_count(snapshot) > 1)
    }
}

/// Copy-on-write graph publication with constant-time range selection.
#[derive(Clone)]
pub struct GraphStore {
    versions: Arc<ArcSwap<GraphVersions>>,
    selected_range: Arc<AtomicU8>,
    generation: Arc<AtomicU32>,
    // Publishing threads own reclamation. A snapshot still held by the audio
    // chain stays here until a later edit or explicit collection.
    retired: Arc<Mutex<Vec<Arc<GraphVersions>>>>,
}

impl Default for GraphStore {
    fn default() -> Self {
        let graph = CompressionGraph::new();
        let selected = range_index(graph.range_db).unwrap_or(mode_index(GraphRangeMode::Range48));
        Self {
            versions: Arc::new(ArcSwap::from_pointee(GraphVersions::new(graph))),
            selected_range: Arc::new(AtomicU8::new(selected)),
            generation: Arc::new(AtomicU32::new(0)),
            retired: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl GraphStore {
    pub fn load(&self) -> Arc<GraphSnapshot> {
        let versions = self.versions.load_full();
        versions
            .at(self.selected_range.load(Ordering::Acquire))
            .clone()
    }

    /// Load one prepared range directly. The callback uses its captured host
    /// mode so a concurrent preset publication cannot expose a different
    /// selector alongside the previous graph generation.
    pub fn load_for_range(&self, mode: GraphRangeMode) -> Arc<GraphSnapshot> {
        self.versions.load_full().at(mode_index(mode)).clone()
    }

    pub fn generation(&self) -> u32 {
        self.generation.load(Ordering::Acquire)
    }

    /// Select a graph and LUT prepared on the UI/preset thread. This method
    /// only performs atomic operations and may be called from `process`.
    pub fn select_range(&self, mode: GraphRangeMode) {
        let next = mode_index(mode);
        if self.selected_range.swap(next, Ordering::AcqRel) != next {
            self.generation.fetch_add(1, Ordering::Release);
        }
    }

    fn publish_graph(&self, graph: CompressionGraph) {
        let versions = Arc::new(GraphVersions::new(graph));
        let old = self.versions.swap(versions);
        // Keep the old generation alive before collecting, so a callback that
        // replaces its Arc cannot become the final owner of the old graph.
        let mut retired = self.retired.lock().unwrap();
        retired.push(old);
        retired.retain(|version| Arc::strong_count(version) > 1 || version.has_external_reader());
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Reclaim snapshots after the audio chain has released them. Call only
    /// from a non-realtime owner such as the editor or preset publisher.
    pub fn collect_retired(&self) {
        let mut retired = self.retired.lock().unwrap();
        retired.retain(|version| Arc::strong_count(version) > 1 || version.has_external_reader());
    }

    /// Copy-on-write edit, cache segments, build all LUTs, then publish.
    pub fn mutate_and_publish<F>(&self, f: F)
    where
        F: FnOnce(&mut CompressionGraph),
    {
        let mut graph = self.load().graph;
        f(&mut graph);
        self.publish_graph(graph);
    }

    /// Replace from a preset or host state.
    pub fn replace(&self, graph: CompressionGraph) {
        if graph.validate().is_err() {
            return;
        }
        let selected = range_index(graph.range_db).unwrap_or(mode_index(GraphRangeMode::Range48));
        self.publish_graph(graph);
        self.selected_range.store(selected, Ordering::Release);
    }
}

impl<'a> PersistentField<'a, CompressionGraph> for GraphStore {
    fn set(&self, new_value: CompressionGraph) {
        self.replace(new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&CompressionGraph) -> R,
    {
        f(&self.load().graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_persisted_graph_does_not_replace_current_state() {
        let store = GraphStore::default();
        let before = store.load();
        let generation = store.generation();
        for count in [0, 1, 3, 14, usize::MAX] {
            let mut graph = before.graph;
            graph.num_points = count;
            let json = serde_json::to_string(&graph).unwrap();
            assert!(serde_json::from_str::<CompressionGraph>(&json).is_err());
            PersistentField::set(&store, graph);
            assert!(Arc::ptr_eq(&before, &store.load()));
        }
        for range in [0.0, -20.0, f64::INFINITY, f64::NAN] {
            let mut graph = before.graph;
            graph.range_db = range;
            store.replace(graph);
            assert!(Arc::ptr_eq(&before, &store.load()));
        }
        let mut graph = before.graph;
        graph.graph_points_mut()[2] = f64::NAN;
        store.replace(graph);
        assert!(Arc::ptr_eq(&before, &store.load()));
        assert_eq!(store.generation(), generation);
    }

    #[test]
    fn persisted_graph_round_trip_keeps_range_and_curve() {
        let store = GraphStore::default();
        store.select_range(GraphRangeMode::Range36);
        store.mutate_and_publish(|graph| {
            graph.adjust_interior_output_y(1, 2.0);
            graph.finalize_after_edit();
        });
        let before = store.load();
        let json = serde_json::to_string(&before.graph).unwrap();
        let restored: CompressionGraph = serde_json::from_str(&json).unwrap();
        let reopened = GraphStore::default();
        PersistentField::set(&reopened, restored);
        let after = reopened.load();
        assert!(before.graph.content_eq(&after.graph));
        for input in [-35.0, -20.0, -10.0, 0.0, 2.0] {
            assert!((before.lut.lookup(input) - after.lut.lookup(input)).abs() < 1.0e-9);
        }
    }

    #[test]
    fn range_change_uses_prepared_graph_and_lut() {
        let store = GraphStore::default();
        let range48 = store.load();
        let before_gen = store.generation();
        store.select_range(GraphRangeMode::Range36);
        let range36 = store.load();
        assert_eq!(range36.graph.range_db, 36.0);
        assert_eq!(range36.graph.min_db, -36.0);
        assert!(range36.lut.threshold().is_finite());
        assert!(store.generation() > before_gen);
        store.select_range(GraphRangeMode::Range48);
        assert!(Arc::ptr_eq(&range48, &store.load()));
        assert!(Arc::ptr_eq(
            &range36,
            &store.load_for_range(GraphRangeMode::Range36)
        ));
    }

    #[test]
    fn unchanged_range_does_not_advance_generation() {
        let store = GraphStore::default();
        let generation = store.generation();
        store.select_range(GraphRangeMode::Range48);
        assert_eq!(store.generation(), generation);
    }

    #[test]
    fn ranges_are_negative_only_and_use_prepared_snapshots() {
        let store = GraphStore::default();
        for mode in GraphRangeMode::ALL {
            let prepared = store.load_for_range(mode);
            store.select_range(mode);
            let selected = store.load();
            assert!(Arc::ptr_eq(&prepared, &selected));
            assert_eq!(selected.graph.min_db, -mode.range_db());
            assert_eq!(selected.graph.max_db, 0.0);
            assert!(selected.lut.threshold().is_finite());
        }
    }

    #[test]
    fn retired_snapshot_waits_for_audio_reader() {
        let store = GraphStore::default();
        let audio_snapshot = store.load();
        store.mutate_and_publish(|graph| graph.adjust_interior_output_y(1, -1.0));
        assert_eq!(store.retired.lock().unwrap().len(), 1);
        store.mutate_and_publish(|graph| graph.adjust_interior_output_y(1, -2.0));
        assert_eq!(store.retired.lock().unwrap().len(), 1);
        drop(audio_snapshot);
        store.collect_retired();
        assert!(store.retired.lock().unwrap().is_empty());
    }
}
