use crate::engine::{Command, Snapshot};
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
