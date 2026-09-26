//! Published graph variants: edits prepare every supported range off the audio thread.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use nih_plug::params::persist::PersistentField;

use crate::dsp::graph::CompressionGraph;
pub use crate::dsp::graph_snapshot::GraphSnapshot;
use crate::params::GraphRangeMode;

const ORIGINAL_RANGE: u8 = GraphRangeMode::ALL.len() as u8;

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
    // A saved graph may use an older arbitrary range. Preserve it until the
    // host's discrete range parameter selects a supported variant.
    original: Option<Arc<GraphSnapshot>>,
}

impl GraphVersions {
    fn new(graph: CompressionGraph) -> Self {
        let source_index = range_index(graph.range_db);
        let original = source_index
            .is_none()
            .then(|| Arc::new(GraphSnapshot::from_graph(graph)));
        let prepared = std::array::from_fn(|index| {
            let mut variant = graph;
            if Some(index as u8) != source_index {
                variant.remap_points_for_range(GraphRangeMode::ALL[index].range_db());
            }
            Arc::new(GraphSnapshot::from_graph(variant))
        });
        Self { prepared, original }
    }

    fn at(&self, index: u8) -> &Arc<GraphSnapshot> {
        if index == ORIGINAL_RANGE {
            self.original.as_ref().unwrap_or(&self.prepared[0])
        } else {
            &self.prepared[index as usize]
        }
    }

    fn has_external_reader(&self) -> bool {
        self.prepared
            .iter()
            .any(|snapshot| Arc::strong_count(snapshot) > 1)
            || self
                .original
                .as_ref()
                .is_some_and(|snapshot| Arc::strong_count(snapshot) > 1)
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
        Self {
            versions: Arc::new(ArcSwap::from_pointee(GraphVersions::new(graph))),
            selected_range: Arc::new(AtomicU8::new(0)),
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

    /// Match an arbitrary persisted graph range. The host's discrete modes
    /// take the prepared fast path; other values are prepared on this caller.
    /// This method must not be called from the audio callback.
    pub fn sync_range_if_needed(&self, range_db: f64) {
        match range_index(range_db) {
            Some(index) => self.select_range(GraphRangeMode::ALL[index as usize]),
            _ => {
                let mut graph = self.load().graph;
                if (graph.range_db - range_db).abs() > 0.01 {
                    graph.remap_points_for_range(range_db);
                    self.replace(graph);
                }
            }
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

    /// Replace from a preset or host state. Keep its original range until the
    /// range parameter is selected again during processing or editor redraw.
    pub fn replace(&self, graph: CompressionGraph) {
        let selected = range_index(graph.range_db).unwrap_or(ORIGINAL_RANGE);
        // Prepare and publish first. Otherwise an old generation without an
        // arbitrary-range snapshot could be read under ORIGINAL_RANGE while
        // the new LUTs are still being built.
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
    fn range_change_uses_prepared_graph_and_lut() {
        let store = GraphStore::default();
        let range20 = store.load();
        let before_gen = store.generation();
        store.select_range(GraphRangeMode::Range40);
        let range40 = store.load();
        assert_eq!(range40.graph.range_db, 40.0);
        assert_eq!(range40.graph.min_db, -40.0);
        assert!(range40.lut.threshold().is_finite());
        assert!(store.generation() > before_gen);
        store.select_range(GraphRangeMode::Range20);
        assert!(Arc::ptr_eq(&range20, &store.load()));
        assert!(Arc::ptr_eq(
            &range40,
            &store.load_for_range(GraphRangeMode::Range40)
        ));
    }

    #[test]
    fn callback_load_ignores_a_concurrent_legacy_selector() {
        let store = GraphStore::default();
        store
            .selected_range
            .store(ORIGINAL_RANGE, Ordering::Release);
        assert_eq!(
            store.load_for_range(GraphRangeMode::Range60).graph.range_db,
            60.0
        );
    }

    #[test]
    fn unchanged_range_does_not_advance_generation() {
        let store = GraphStore::default();
        let generation = store.generation();
        store.select_range(GraphRangeMode::Range20);
        assert_eq!(store.generation(), generation);
    }

    #[test]
    fn legacy_range_is_preserved_until_host_selects_a_prepared_mode() {
        let store = GraphStore::default();
        store.sync_range_if_needed(31.0);
        assert_eq!(store.load().graph.range_db, 31.0);
        store.select_range(GraphRangeMode::Range40);
        assert_eq!(store.load().graph.range_db, 40.0);
    }

    #[test]
    fn damian_ranges_are_negative_only_and_use_prepared_snapshots() {
        let store = GraphStore::default();
        for mode in GraphRangeMode::PLEASANT {
            let prepared = store.load_for_range(mode);
            store.sync_range_if_needed(mode.range_db());
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
