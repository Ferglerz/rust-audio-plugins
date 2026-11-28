use super::*;
use crate::telemetry::{TransportFrame, TransportStage};
use pleasant_ui::{bezier::CubicBezier, with_alpha};
use std::{
    f32::consts::TAU,
    time::{Duration, Instant},
};

const LEFT: f32 = 218.0;
const RIGHT: f32 = 295.0;
const CENTER_Y: f32 = 28.0;
const STALE: Duration = Duration::from_millis(250);
const JOIN_FLASH: Duration = Duration::from_millis(180);

#[derive(Clone, Copy, Default)]
pub(super) struct Reels {
    phase: [f32; 2],
    last_paint: Option<Instant>,
    last_block: Option<Instant>,
    last_crossfade: Option<Instant>,
    sequence: Option<u64>,
    frame: TransportFrame,
}

impl Reels {
    pub(super) fn advance(
        &mut self,
        now: Instant,
        snapshot: Option<(u64, TransportFrame)>,
        enabled: bool,
    ) -> TransportFrame {
        let dt = self.last_paint.replace(now).map_or(0.0, |previous| {
            now.saturating_duration_since(previous)
                .as_secs_f32()
                .min(0.05)
        });
        if let Some((sequence, frame)) =
            snapshot.filter(|(sequence, _)| Some(*sequence) != self.sequence)
        {
            // A snapshot may predate the editor. Wait for the first new audio block.
            let had_snapshot = self.sequence.replace(sequence).is_some();
            self.frame = frame;
            self.last_block = (had_snapshot && frame.stage != TransportStage::Idle).then_some(now);
        }
        if !enabled
            || self
                .last_block
                .is_none_or(|last| now.saturating_duration_since(last) >= STALE)
        {
            self.last_crossfade = None;
            return TransportFrame::default();
        }
        if self.frame.stage == TransportStage::Crossfading {
            self.last_crossfade = Some(now);
        } else if matches!(
            self.frame.stage,
            TransportStage::Braking | TransportStage::Position
        ) {
            self.last_crossfade = None;
        }
        let average_speed = (self.frame.speed[0] + self.frame.speed[1]) * 0.5;
        for (angle, ratio) in self.phase.iter_mut().zip([0.9, 1.12]) {
            *angle = (*angle + (average_speed * ratio).clamp(0.0, 3.0) * dt * TAU) % TAU;
        }
        self.frame
    }

    pub(super) fn draw(&self, d: &mut Draw, frame: TransportFrame, enabled: bool, now: Instant) {
        let label = match frame.stage {
            TransportStage::Idle if !enabled => "BYPASSED",
            TransportStage::Idle => "IDLE",
            TransportStage::Live => "LIVE",
            TransportStage::Position => "POSITION CC",
            TransportStage::Braking if frame.speed.iter().all(|&value| value <= 0.005) => "STOPPED",
            TransportStage::Braking => "BRAKING",
            TransportStage::Returning => "RETURNING",
            TransportStage::Crossfading => "CROSSFADE",
        };
        let accent = match frame.stage {
            TransportStage::Braking | TransportStage::Position => GOLD,
            TransportStage::Returning => COLORS[0],
            TransportStage::Crossfading => TEAL,
            _ => MUTED,
        };
        let span = CubicBezier([
            (LEFT, CENTER_Y + 11.0),
            (LEFT + 24.0, CENTER_Y + 23.0),
            (RIGHT - 22.0, CENTER_Y + 20.0),
            (RIGHT, CENTER_Y + 9.0),
        ]);
        let flash = self.last_crossfade.map_or(0.0, |last| {
            1.0 - (now.saturating_duration_since(last).as_secs_f32() / JOIN_FLASH.as_secs_f32())
                .clamp(0.0, 1.0)
        });
        d.stroke_cubic(span, with_alpha(accent, 0.72), 2.0, false);
        if flash > 0.0 {
            d.stroke_cubic(span, with_alpha(TEAL, 0.22 * flash), 8.0, true);
            d.stroke_cubic(span, with_alpha(TEAL, flash), 2.5, false);
        }
        self.draw_reel(d, LEFT, 17.5, self.phase[0], accent);
        self.draw_reel(d, RIGHT, 15.5, self.phase[1] - 0.53, accent);
        d.text_centered((LEFT + RIGHT) * 0.5, 62.0, label, 9.5, accent);
    }

    fn draw_reel(&self, d: &mut Draw, x: f32, radius: f32, angle: f32, accent: Color) {
        d.circle(x, CENTER_Y, radius, pleasant_ui::BG, true);
        d.circle(x, CENTER_Y, radius, with_alpha(accent, 0.78), false);
        d.circle(x, CENTER_Y, radius - 3.0, LINE, false);
        for cutout in 0..3 {
            let (sin, cos) = (angle + cutout as f32 * TAU / 3.0).sin_cos();
            let orbit = radius * 0.57;
            d.circle(
                x + cos * orbit,
                CENTER_Y + sin * orbit,
                radius * 0.15,
                with_alpha(accent, if cutout == 0 { 0.90 } else { 0.52 }),
                true,
            );
        }
        d.circle(x, CENTER_Y, 3.2, accent, false);
        d.circle(x, CENTER_Y, 1.25, accent, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reels_follow_speed_and_freeze_on_stop_bypass_reset_or_stale_audio() {
        let start = Instant::now();
        let mut reels = Reels::default();
        let live = TransportFrame {
            stage: TransportStage::Live,
            speed: [1.0; 2],
            ..Default::default()
        };
        let returning = TransportFrame {
            stage: TransportStage::Returning,
            speed: [4.0, 6.0],
            ..Default::default()
        };
        let stopped = TransportFrame {
            stage: TransportStage::Braking,
            ..Default::default()
        };
        assert_eq!(
            reels.advance(start, Some((2, live)), true).stage,
            TransportStage::Idle
        );
        let at = |millis| start + Duration::from_millis(millis);
        reels.advance(at(16), Some((4, live)), true);
        let live_step = reels.phase[0];
        reels.advance(at(32), Some((6, returning)), true);
        assert!(reels.phase[0] - live_step > 2.0 * live_step);
        let phase = reels.phase;
        assert_eq!(
            reels.advance(at(48), Some((8, stopped)), true).stage,
            TransportStage::Braking
        );
        assert_eq!(reels.phase, phase);
        assert_eq!(
            reels.advance(at(64), Some((10, live)), false).stage,
            TransportStage::Idle
        );
        assert_eq!(
            reels.advance(at(350), Some((10, live)), true).stage,
            TransportStage::Idle
        );
        assert_eq!(
            reels
                .advance(at(366), Some((12, TransportFrame::default())), true)
                .stage,
            TransportStage::Idle
        );
        assert_eq!(reels.phase, phase);
    }

    #[test]
    fn crossfade_leaves_a_short_flash_after_the_live_handoff() {
        let now = Instant::now();
        let mut reels = Reels::default();
        reels.advance(now, Some((0, TransportFrame::default())), true);
        let crossfade = TransportFrame {
            stage: TransportStage::Crossfading,
            speed: [1.0; 2],
            ..Default::default()
        };
        reels.advance(now, Some((2, crossfade)), true);
        let live = TransportFrame {
            stage: TransportStage::Live,
            ..crossfade
        };
        reels.advance(now + Duration::from_millis(16), Some((4, live)), true);
        assert_eq!(reels.last_crossfade, Some(now));
    }
}
