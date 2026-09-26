use crate::engine::{Command, Snapshot};
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, Ordering};
pub struct Bridge {
    pub commands: ArrayQueue<Command>,
    pub snapshots: ArrayQueue<Snapshot>,
    pub panic: AtomicBool,
    pub reset: AtomicBool,
    pub visible: AtomicBool,
}
impl Default for Bridge {
    fn default() -> Self {
        Self {
            commands: ArrayQueue::new(128),
            snapshots: ArrayQueue::new(4),
            panic: AtomicBool::new(false),
            reset: AtomicBool::new(false),
            visible: AtomicBool::new(false),
        }
    }
}
impl Bridge {
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
