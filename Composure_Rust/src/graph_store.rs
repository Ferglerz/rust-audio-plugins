//! RCU-style published compression graph + LUT — UI writes; audio loads `Arc` snapshots.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use arc_swap::ArcSwap;
use nih_plug::params::persist::PersistentField;

use crate::dsp::compression_lut::CompressionLUT;
use crate::dsp::graph::CompressionGraph;

/// Pre-built graph and LUT published atomically to the audio thread.
#[derive(Debug, Clone, Copy)]
pub struct GraphSnapshot {
    pub graph: CompressionGraph,
    pub lut: CompressionLUT,
}

impl GraphSnapshot {
    pub fn from_graph(mut graph: CompressionGraph) -> Self {
        graph.ensure_segments_cached();
        let mut lut = CompressionLUT::new();
        lut.build_lut(&graph);
        Self { graph, lut }
    }
}

impl Default for GraphSnapshot {
    fn default() -> Self {
        Self::from_graph(CompressionGraph::new())
    }
}

/// Lock-free published graph snapshot (serde via inner `CompressionGraph`).
#[derive(Clone)]
pub struct GraphStore {
    snapshot: Arc<ArcSwap<GraphSnapshot>>,
    generation: Arc<AtomicU32>,
}

impl Default for GraphStore {
    fn default() -> Self {
        Self {
            snapshot: Arc::new(ArcSwap::from_pointee(GraphSnapshot::default())),
            generation: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl GraphStore {
    pub fn load(&self) -> Arc<GraphSnapshot> {
        self.snapshot.load_full()
    }

    pub fn generation(&self) -> u32 {
        self.generation.load(Ordering::Acquire)
    }

    fn publish_graph(&self, graph: CompressionGraph) {
        let snapshot = GraphSnapshot::from_graph(graph);
        self.snapshot.store(Arc::new(snapshot));
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Copy-on-write edit, cache segments, build LUT, publish.
    pub fn mutate_and_publish<F>(&self, f: F)
    where
        F: FnOnce(&mut CompressionGraph),
    {
        let mut graph = self.snapshot.load_full().graph;
        f(&mut graph);
        self.publish_graph(graph);
    }

    /// Remap interior points when graph dB range changes, then publish.
    pub fn sync_range_if_needed(&self, range_db: f64) {
        let current = self.snapshot.load();
        if (current.graph.range_db - range_db).abs() <= 0.01 {
            return;
        }
        let mut graph = current.graph;
        graph.remap_points_for_range(range_db);
        self.publish_graph(graph);
    }

    /// Replace snapshot (preset load / init).
    pub fn replace(&self, graph: CompressionGraph) {
        self.publish_graph(graph);
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
        f(&self.snapshot.load().graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_range_if_needed_remaps_and_publishes() {
        let store = GraphStore::default();
        let before_gen = store.generation();
        store.sync_range_if_needed(40.0);
        let after = store.load();
        assert!((after.graph.range_db - 40.0).abs() < 0.01);
        assert!((after.graph.min_db - (-40.0)).abs() < 0.01);
        assert!(store.generation() > before_gen);
    }

    #[test]
    fn sync_range_if_needed_noop_when_unchanged() {
        let store = GraphStore::default();
        let gen = store.generation();
        store.sync_range_if_needed(20.0);
        assert_eq!(store.generation(), gen);
    }
}
