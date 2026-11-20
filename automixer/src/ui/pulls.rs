use super::*;
use crate::spectral::{band_centers, PullSnapshot, BANDS};
use nih_plug_vizia::vizia::vg::Color;
use pleasant_ui::{bezier::CubicBezier, with_alpha};

const MAX_TRAILS: usize = 3;
const MIN_CUT_DB: f32 = 0.25;

#[derive(Clone, Copy)]
struct Source {
    owner: u64,
    band: usize,
    db: f32,
}

fn ranked_sources(snapshot: &PullSnapshot) -> Vec<Source> {
    let mut sources: Vec<Source> = Vec::new();
    for (band, contribution) in snapshot.sources.iter().enumerate() {
        if contribution.owner == 0
            || contribution.db < MIN_CUT_DB
            || contribution.db < snapshot.cut[band] * 0.75
        {
            continue;
        }
        let source = Source {
            owner: contribution.owner,
            band,
            db: contribution.db,
        };
        if let Some(previous) = sources.iter_mut().find(|s| s.owner == source.owner) {
            if source.db > previous.db {
                *previous = source;
            }
        } else {
            sources.push(source);
        }
    }
    sources.sort_unstable_by(|a, b| b.db.total_cmp(&a.db).then_with(|| a.owner.cmp(&b.owner)));
    sources
}

fn live_slot(slot: &network::Slot, owner: u64, now: u64, conflicts: u64) -> bool {
    let updated = slot.updated_ms.load(Ordering::Relaxed);
    let mic = slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
    owner != 0
        && slot.owner.load(Ordering::Acquire) == owner
        && conflicts & (1 << mic) == 0
        && slot.signal.load(Ordering::Acquire) != 0
        && updated != 0
        && now.saturating_sub(updated) < 750
        && !slot
            .editor_params(owner)
            .is_some_and(|params| params.bypassed())
}

pub(super) fn cut_point(bounds: Rect, band: usize, cut_db: f32) -> (f32, f32) {
    let (left, top, width, height) = bounds;
    (
        left + 1.0 + band as f32 / (BANDS - 1) as f32 * (width - 2.0),
        top + 1.0 + (cut_db / 24.0).clamp(0.0, 1.0) * (height - 2.0),
    )
}

#[derive(Clone, Copy, Default)]
struct Trail {
    owner: u64,
    phase: f32,
    last_transient: f32,
    kick: f32,
    kick_progress: f32,
}

impl Trail {
    fn advance(&mut self, transient: f32, dt: f32) {
        self.phase = (self.phase + dt * (0.72 + 0.55 * transient)) % 1.0;
        self.kick_progress += dt * (2.7 + 1.8 * self.kick);
        if transient > self.last_transient + 0.06 {
            self.kick = transient;
            self.kick_progress = 0.0;
        }
        self.last_transient = transient;
    }

    fn draw(self, d: &mut Draw, from: Rect, to: (f32, f32), db: f32, band: usize) {
        let from_left = from.0 + from.2 * 0.5 < to.0;
        let sign = if from_left { 1.0 } else { -1.0 };
        let start = (
            if from_left {
                from.0 + from.2 - 12.0
            } else {
                from.0 + 12.0
            },
            from.1 + from.3 * (0.35 + 0.22 * band as f32 / (BANDS - 1) as f32),
        );
        let distance = (to.0 - start.0).abs();
        let intensity = (db / (db + 2.0)).sqrt();
        let bend = 12.0 + 10.0 * (self.phase * std::f32::consts::TAU).sin();
        let path = CubicBezier([
            start,
            (start.0 + sign * distance * 0.30, start.1 + bend),
            (to.0 - sign * distance * 0.23, to.1 + 18.0),
            to,
        ]);
        let tint = TEAL;
        for (width, alpha) in [(21.0, 0.025), (14.0, 0.044), (8.0, 0.068), (3.0, 0.09)] {
            d.stroke_cubic(
                path,
                with_alpha(tint, alpha * intensity),
                width * (0.5 + intensity),
                true,
            );
        }
        for offset in [0.0, 0.53] {
            let phase = (self.phase + offset) % 1.0;
            let t = phase * (0.55 + 0.45 * phase);
            let opacity = 0.33 * intensity * (std::f32::consts::PI * phase).sin();
            puff(
                d,
                path,
                t,
                (10.0 + 6.0 * intensity) * (1.0 - 0.68 * t),
                tint,
                opacity,
            );
        }
        if self.kick > 0.0 && self.kick_progress < 1.0 {
            let phase = self.kick_progress;
            let t = phase * (0.35 + 0.65 * phase);
            let size = (12.0 + 20.0 * self.kick) * (1.0 - 0.77 * t);
            let opacity =
                0.65 * intensity * self.kick * (std::f32::consts::PI * phase).sin().sqrt();
            puff(d, path, t, size, tint, opacity);
        }
    }
}

fn puff(d: &mut Draw, path: CubicBezier, progress: f32, radius: f32, tint: Color, alpha: f32) {
    let (x, y) = path.point(progress);
    let (tail_x, tail_y) = path.point((progress - 0.10).max(0.0));
    d.soft_circle(
        tail_x,
        tail_y + radius * 0.18,
        radius * 1.13,
        with_alpha(tint, alpha * 0.56),
    );
    d.soft_circle(x, y, radius, with_alpha(tint, alpha));
}

#[derive(Clone, Copy, Default)]
pub(super) struct PullMotion {
    target: u64,
    last_paint: Option<Instant>,
    trails: [Trail; MAX_TRAILS],
}

impl AutomixerView {
    pub(super) fn paint_pulls(
        &self,
        d: &mut Draw,
        strips: &PageStrips,
        now: u64,
        conflicts: u64,
        frame_time: Instant,
    ) -> Option<String> {
        let target = d
            .hover
            .filter(|_| self.menu.is_none() && self.press.is_none())
            .and_then(|point| {
                strips.iter().find_map(|(slot, rect)| {
                    let graph = StripLayout::new(rect, false).spectrum();
                    inside(point, graph).then_some((slot, graph))
                })
            });
        let Some((target_index, target_graph)) = target else {
            self.pull_motion.set(PullMotion::default());
            return None;
        };
        let target_slot = &self.network.slots[target_index];
        let target_owner = target_slot.owner.load(Ordering::Acquire);
        let snapshot = live_slot(target_slot, target_owner, now, conflicts)
            .then(|| target_slot.spectrum.read_pulls())
            .flatten()
            .filter(|_| target_slot.owner.load(Ordering::Acquire) == target_owner);
        let Some(snapshot) = snapshot else {
            self.pull_motion.set(PullMotion::default());
            return None;
        };

        let previous = self.pull_motion.get();
        let dt = previous
            .last_paint
            .filter(|_| previous.target == target_owner)
            .map_or(0.0, |time| {
                frame_time
                    .saturating_duration_since(time)
                    .as_secs_f32()
                    .min(0.1)
            });
        let mut motion = PullMotion {
            target: target_owner,
            last_paint: Some(frame_time),
            ..Default::default()
        };
        let mut hint = String::new();
        d.scissor(32.0, target_graph.1, SIZE.0 - 64.0, target_graph.3);
        for (i, (source, index)) in ranked_sources(&snapshot)
            .into_iter()
            .filter_map(|source| {
                let index = self
                    .network
                    .slots
                    .iter()
                    .position(|slot| slot.owner.load(Ordering::Acquire) == source.owner)?;
                (index != target_index
                    && live_slot(&self.network.slots[index], source.owner, now, conflicts))
                .then_some((source, index))
            })
            .take(MAX_TRAILS)
            .enumerate()
        {
            let slot = &self.network.slots[index];
            let mic = slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
            let host_name = slot
                .labels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .name
                .clone();
            if slot.owner.load(Ordering::Acquire) != source.owner {
                continue;
            }
            let on_page = strips.iter().find(|&(visible, _)| visible == index);
            if let Some((_, rect)) = on_page {
                let mut trail = previous
                    .trails
                    .iter()
                    .copied()
                    .find(|trail| previous.target == target_owner && trail.owner == source.owner)
                    .unwrap_or(Trail {
                        owner: source.owner,
                        phase: 0.22 + i as f32 * 0.19,
                        ..Default::default()
                    });
                let pulse = slot
                    .spectrum
                    .read_transient()
                    .unwrap_or(trail.last_transient);
                if slot.owner.load(Ordering::Acquire) != source.owner {
                    continue;
                }
                trail.advance(pulse, dt);
                trail.draw(
                    d,
                    StripLayout::new(rect, false).spectrum(),
                    cut_point(target_graph, source.band, snapshot.cut[source.band]),
                    source.db,
                    source.band,
                );
                motion.trails[i] = trail;
            }
            use std::fmt::Write as _;
            if !hint.is_empty() {
                hint.push_str("   ·   ");
            }
            let name: String = self
                .display_name(mic, host_name.as_deref())
                .chars()
                .take(21)
                .collect();
            let hz = band_centers()[source.band];
            let frequency = if hz < 1000.0 {
                format!("{hz:.0} Hz")
            } else {
                format!("{:.1} kHz", hz / 1000.0)
            };
            let location = if on_page.is_some() { "" } else { " (off page)" };
            let _ = write!(hint, "{name}{location}  {frequency}  {:.1} dB", source.db);
        }
        d.reset_scissor();
        self.pull_motion.set(motion);
        if !hint.is_empty() {
            return Some(format!("Spectral pull from {hint}"));
        }
        snapshot
            .cut
            .iter()
            .any(|&cut| cut >= MIN_CUT_DB)
            .then(|| "Spectral cut is mixed, releasing, or lacks a current source.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::Contribution;

    #[test]
    fn ranks_distinct_sources_by_strongest_attributed_band_and_ignores_ambiguous_bands() {
        let mut snapshot = PullSnapshot {
            cut: [4.0; BANDS],
            sources: [Contribution::default(); BANDS],
        };
        snapshot.sources[2] = Contribution { owner: 1, db: 3.2 };
        snapshot.sources[3] = Contribution { owner: 2, db: 3.8 };
        snapshot.sources[4] = Contribution { owner: 1, db: 3.9 };
        snapshot.sources[5] = Contribution { owner: 3, db: 2.9 };
        let sources: Vec<_> = ranked_sources(&snapshot)
            .iter()
            .map(|s| (s.owner, s.band))
            .collect();
        assert_eq!(sources, [(1, 4), (2, 3)]);
    }

    #[test]
    fn invalidates_stale_and_reused_sender_slots() {
        let network = Arc::new(Network::default());
        let first = network.register();
        let index = first.index.unwrap();
        let old_owner = first.token;
        let slot = &network.slots[index];
        slot.publish(0.1, true, true);
        slot.updated_ms.store(10, Ordering::Relaxed);
        assert!(live_slot(slot, old_owner, 20, 0));
        assert!(!live_slot(slot, old_owner, 760, 0));
        drop(first);
        let next = network.register();
        assert_eq!(next.index, Some(index));
        slot.publish(0.1, true, true);
        slot.updated_ms.store(20, Ordering::Relaxed);
        assert!(!live_slot(slot, old_owner, 20, 0));
        assert!(live_slot(slot, next.token, 20, 0));
    }

    #[test]
    fn transient_kick_travels_faster_than_a_small_onset() {
        let mut large = Trail::default();
        let mut small = Trail::default();
        large.advance(1.0, 0.0);
        small.advance(0.2, 0.0);
        large.advance(0.95, 0.1);
        small.advance(0.19, 0.1);
        assert!(large.kick_progress > small.kick_progress);
        assert!(large.kick > small.kick);
    }
}
