use crate::engine::{Command, Snapshot};
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Audio-thread-owned display cadence, independent of host automation sub-blocks.
#[derive(Default)]
pub(crate) struct SnapshotClock {
    elapsed_samples: u64,
    was_visible: bool,
}

impl SnapshotClock {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn should_publish(
        &mut self,
        visible: bool,
        samples: usize,
        sample_rate: f32,
    ) -> bool {
        if !visible {
            self.reset();
            return false;
        }
        let period = (f64::from(sample_rate) / 60.0).round().max(1.0) as u64;
        let elapsed = self.elapsed_samples.saturating_add(samples as u64);
        let publish = !self.was_visible || elapsed >= period;
        self.was_visible = true;
        // Keep phase across short sub-blocks and discard missed visual frames
        // in large buffers without constructing snapshots that nobody can see.
        self.elapsed_samples = elapsed % period;
        publish
    }
}

pub struct Bridge {
    pub commands: ArrayQueue<Command>,
    pub snapshots: ArrayQueue<Snapshot>,
    pub panic: AtomicBool,
    pub reset: AtomicBool,
    pub visible: AtomicBool,
    pub learned_split: AtomicU32,
    pub quick_learn: AtomicU32,
}
impl Default for Bridge {
    fn default() -> Self {
        Self {
            commands: ArrayQueue::new(128),
            snapshots: ArrayQueue::new(4),
            panic: AtomicBool::new(false),
            reset: AtomicBool::new(false),
            visible: AtomicBool::new(false),
            learned_split: AtomicU32::new(0),
            quick_learn: AtomicU32::new(0),
        }
    }
}
impl Bridge {
    /// Construct display telemetry only while an editor can consume it.
    pub fn publish_with(&self, snapshot: impl FnOnce() -> Snapshot) {
        if self.visible.load(Ordering::Relaxed) {
            self.publish(snapshot());
        }
    }

    pub fn send(&self, command: Command) {
        if self.commands.push(command).is_err() {
            self.panic.store(true, Ordering::Release);
        }
    }
    pub fn publish(&self, snapshot: Snapshot) {
        if !self.visible.load(Ordering::Relaxed) {
            return;
        }
        if let Err(snapshot) = self.snapshots.push(snapshot) {
            let _ = self.snapshots.pop();
            let _ = self.snapshots.push(snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn snapshot_clock_counts_variable_sub_blocks_at_audio_rate() {
        let mut clock = SnapshotClock::default();
        assert!(clock.should_publish(true, 0, 48_000.0));
        for samples in [128, 64, 256, 351] {
            assert!(!clock.should_publish(true, samples, 48_000.0));
        }
        assert!(clock.should_publish(true, 1, 48_000.0));
        assert!(!clock.should_publish(true, 0, 48_000.0));
        assert!(!clock.should_publish(true, 799, 48_000.0));
        assert!(clock.should_publish(true, 1, 48_000.0));
    }

    #[test]
    fn snapshot_clock_preserves_remainder_across_large_blocks() {
        let mut clock = SnapshotClock::default();
        assert!(clock.should_publish(true, 0, 44_100.0));
        assert!(clock.should_publish(true, 735 * 7 + 100, 44_100.0));
        assert!(!clock.should_publish(true, 634, 44_100.0));
        assert!(clock.should_publish(true, 1, 44_100.0));
        assert!(!clock.should_publish(true, 0, 44_100.0));
    }

    #[test]
    fn snapshot_clock_reopening_and_reset_publish_immediately() {
        let mut clock = SnapshotClock::default();
        assert!(!clock.should_publish(false, 48_000, 48_000.0));
        assert!(clock.should_publish(true, 64, 48_000.0));
        assert!(!clock.should_publish(true, 64, 48_000.0));
        assert!(!clock.should_publish(false, 1, 48_000.0));
        assert!(clock.should_publish(true, 1, 48_000.0));
        assert!(!clock.should_publish(true, 1, 48_000.0));
        clock.reset();
        assert!(clock.should_publish(true, 0, 96_000.0));
        assert!(!clock.should_publish(true, 1599, 96_000.0));
        assert!(clock.should_publish(true, 1, 96_000.0));
    }

    #[test]
    fn snapshot_clock_skips_construction_between_visual_frames() {
        let bridge = Bridge::default();
        let mut clock = SnapshotClock::default();
        let constructed = Cell::new(0);
        bridge.visible.store(true, Ordering::Relaxed);
        for _ in 0..1600 {
            if clock.should_publish(bridge.visible.load(Ordering::Relaxed), 1, 48_000.0) {
                bridge.publish_with(|| {
                    constructed.set(constructed.get() + 1);
                    Snapshot::default()
                });
            }
        }
        assert_eq!(constructed.get(), 3); // Initial state, then 800 and 1600 samples.
    }

    #[test]
    fn hidden_editor_skips_snapshot_and_reopening_publishes_current_state() {
        let bridge = Bridge::default();
        let constructed = Cell::new(0);
        let snapshot = || {
            constructed.set(constructed.get() + 1);
            Snapshot {
                root: 67,
                ..Snapshot::default()
            }
        };
        bridge.publish_with(snapshot);
        assert_eq!(constructed.get(), 0);
        assert!(bridge.snapshots.is_empty());
        bridge.visible.store(true, Ordering::Relaxed);
        bridge.publish_with(snapshot);
        assert_eq!(constructed.get(), 1);
        assert_eq!(bridge.snapshots.pop().unwrap().root, 67);
        bridge.visible.store(false, Ordering::Relaxed);
        bridge.publish_with(snapshot);
        assert_eq!(constructed.get(), 1);
    }
}
