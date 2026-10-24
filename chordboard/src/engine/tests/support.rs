use super::super::*;

pub(super) fn send(e: &mut Engine, c: Command) -> Vec<Out> {
    let mut out = Vec::new();
    e.command(c, &mut |v| out.push(v));
    out
}
pub(super) fn on(e: &mut Engine, n: u8, ch: u8) -> Vec<Out> {
    let mut out = Vec::new();
    e.midi_note(true, ch, n, 0.8, &mut |v| out.push(v));
    out
}
pub(super) fn off(e: &mut Engine, n: u8, ch: u8) -> Vec<Out> {
    let mut out = Vec::new();
    e.midi_note(false, ch, n, 0.3, &mut |v| out.push(v));
    out
}
pub(super) fn configure(e: &mut Engine, mut c: Config) {
    if c.mode == ARP {
        c.mode = MANUAL;
        c.arp_enabled = true;
        c.arp_settings = [crate::arpeggiator::Settings {
            mode: crate::arpeggiator::ArpMode::SyncLoop,
            pattern: c.arp_pattern,
            octaves: c.octaves,
            sync: c.strum_sync,
            rate: c.rate,
            free_ms: c.strum_ms,
            swing: c.swing,
            humanize: c.humanize,
        }; 8];
    }
    e.configure(c, &mut |_| {});
}
pub(super) fn tick(e: &mut Engine, count: usize) -> Vec<Out> {
    let mut out = Vec::new();
    for _ in 0..count {
        e.tick(&mut |v| out.push(v));
    }
    out
}
pub(super) fn notes(events: &[Out]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|e| {
            if let Out::On(_, n, _) = e {
                Some(*n)
            } else {
                None
            }
        })
        .collect()
}
