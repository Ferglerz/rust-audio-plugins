use crate::strength::{PreparedStrengthCurve, StrengthNode};
use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, RwLock};

pub struct Shared {
    pub sample_rate: AtomicF32,
    pub fft_size: AtomicUsize,
    pub is_bypassed: AtomicBool,
    pub low_cut_hz: AtomicF32,
    pub high_cut_hz: AtomicF32,
    pub tilt: AtomicF32,
    pub tilt_freq_hz: AtomicF32,
    pub max_boost_db: AtomicF32,
    pub max_cut_db: AtomicF32,
    pub output_gain_db: AtomicF32,
    pub spectrum_mags_db: RwLock<Vec<f32>>,
    pub filter_display: RwLock<Vec<(f32, f32)>>, // (center_hz, gain_db)
    boost_nodes: RwLock<Arc<PreparedStrengthCurve>>,
    cut_nodes: RwLock<Arc<PreparedStrengthCurve>>,
    retired_nodes: ArrayQueue<Arc<PreparedStrengthCurve>>,
}

impl Default for Shared {
    fn default() -> Self {
        Self::new()
    }
}

impl Shared {
    pub fn new() -> Self {
        Self {
            sample_rate: AtomicF32::new(44100.0),
            fft_size: AtomicUsize::new(512),
            is_bypassed: AtomicBool::new(false),
            low_cut_hz: AtomicF32::new(20.0),
            high_cut_hz: AtomicF32::new(20000.0),
            tilt: AtomicF32::new(0.0),
            tilt_freq_hz: AtomicF32::new(1800.0),
            max_boost_db: AtomicF32::new(12.0),
            max_cut_db: AtomicF32::new(12.0),
            output_gain_db: AtomicF32::new(0.0),
            spectrum_mags_db: RwLock::new(Vec::with_capacity(4096)),
            filter_display: RwLock::new(Vec::with_capacity(1024)),
            boost_nodes: RwLock::new(Arc::default()),
            cut_nodes: RwLock::new(Arc::default()),
            retired_nodes: ArrayQueue::new(8),
        }
    }

    pub fn publish_nodes(&self, boost: bool, nodes: Arc<[StrengthNode]>) {
        // Sorting, allocation, and reclamation belong to the publishing thread.
        let prepared = Arc::new(PreparedStrengthCurve::new(nodes));
        while self.retired_nodes.pop().is_some() {}
        let lock = if boost {
            &self.boost_nodes
        } else {
            &self.cut_nodes
        };
        match lock.write() {
            Ok(mut current) => *current = prepared,
            Err(poisoned) => *poisoned.into_inner() = prepared,
        }
    }

    pub fn node_snapshot(&self, boost: bool) -> Option<Arc<PreparedStrengthCurve>> {
        let lock = if boost {
            &self.boost_nodes
        } else {
            &self.cut_nodes
        };
        lock.try_read().ok().map(|nodes| Arc::clone(&nodes))
    }

    pub fn refresh_node_snapshot(
        &self,
        boost: bool,
        current: &mut Arc<PreparedStrengthCurve>,
    ) -> bool {
        if self.retired_nodes.is_full() {
            return false;
        }
        let lock = if boost {
            &self.boost_nodes
        } else {
            &self.cut_nodes
        };
        let Ok(published) = lock.try_read() else {
            return false;
        };
        if Arc::ptr_eq(current, &published) {
            return false;
        }
        // Keep the read guard until adoption (or rollback) completes. A publisher
        // must not retire `next` while this thread could still drop its last Arc.
        let next = Arc::clone(&published);
        let previous = std::mem::replace(current, next);
        if let Err(previous) = self.retired_nodes.push(previous) {
            *current = previous;
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_defers_while_writer_holds_snapshot() {
        let shared = Shared::new();
        let mut current = shared.node_snapshot(true).unwrap();
        shared.publish_nodes(true, Arc::from([StrengthNode::new(1, 1000.0, 0.5)]));
        let writer = shared.boost_nodes.write().unwrap();
        assert!(!shared.refresh_node_snapshot(true, &mut current));
        assert!(current.nodes().is_empty());
        drop(writer);
        assert!(shared.refresh_node_snapshot(true, &mut current));
        assert_eq!(current.nodes()[0].id, 1);
        assert!(!shared.refresh_node_snapshot(true, &mut current));
    }

    #[test]
    fn full_retirement_queue_defers_refresh_until_publisher_reclaims() {
        let shared = Shared::new();
        let mut readers: Vec<_> = (0..9)
            .map(|_| shared.node_snapshot(true).unwrap())
            .collect();
        shared.publish_nodes(true, Arc::from([StrengthNode::new(1, 1000.0, 0.5)]));
        for reader in &mut readers[..8] {
            assert!(shared.refresh_node_snapshot(true, reader));
        }
        assert!(!shared.refresh_node_snapshot(true, &mut readers[8]));
        assert!(readers[8].nodes().is_empty());
        shared.publish_nodes(true, Arc::from([StrengthNode::new(2, 2000.0, 2.0)]));
        assert!(shared.refresh_node_snapshot(true, &mut readers[8]));
        assert_eq!(readers[8].nodes()[0].id, 2);
    }
}
