//! Run the shared scheduler with a separate live clock and voice owner.
//! Swapping fixed-size runtimes keeps the single prepared publication on the audio thread.
use super::*;
use routing::{ARP_POSITION_SOURCE, PERF_MOD_SOURCES, SOURCE_COUNT};

impl Engine {
    pub(super) fn performance_arp_snapshot(&self) -> ArpPlayback {
        let program = &self.arp.data.performance;
        let settings = self
            .performance_arp
            .timing
            .map_or(program.settings, |t| t.apply(program.settings));
        let mut globals = self.config.global_harmony();
        globals.transpose = 0;
        let (notes, len) = arpeggiator::program_cycle(
            self.notes,
            self.memory,
            globals,
            settings,
            &program.page,
            self.performance_arp.seed,
            self.performance_arp.display_phase,
        );
        ArpPlayback {
            notes,
            len: len as u8,
            phase: self.performance_arp.display_phase,
            step: self.performance_arp.step,
            running: self.performance_arp.running && !self.performance_arp.finished,
            aligned: self.host_config.daw_alignment
                && settings.sync
                && self.host_config.host_sync
                && self.playing
                && self.host_beat_available
                && self.host_tempo_available,
        }
    }
    pub(super) fn automatic_arp_owner(&self) -> u8 {
        if self.in_performance_arp {
            arpeggiator::PERFORMANCE_ARP_OWNER
        } else {
            arpeggiator::ARP_OWNER
        }
    }
    pub(super) fn performance_arp_enabled(&self) -> bool {
        self.arp.data.performance.enabled
    }
    pub(super) fn with_performance_arp<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        debug_assert!(!self.in_performance_arp);
        let config = self.config;
        let overrides = self.source_overrides;
        std::mem::swap(&mut self.arp, &mut self.performance_arp);
        // The prepared data stays shared; only the scheduler state changes owners.
        std::mem::swap(&mut self.arp.data, &mut self.performance_arp.data);
        self.in_performance_arp = true;
        // Keep the existing cycle-position route local to the current arp.
        std::mem::swap(&mut self.sources[ARP_POSITION_SOURCE], &mut self.performance_arp_position);
        self.configure_performance_arp();
        let result = f(self);
        std::mem::swap(&mut self.sources[ARP_POSITION_SOURCE], &mut self.performance_arp_position);
        self.in_performance_arp = false;
        std::mem::swap(&mut self.arp.data, &mut self.performance_arp.data);
        std::mem::swap(&mut self.arp, &mut self.performance_arp);
        self.config = config;
        self.source_overrides = overrides;
        result
    }
    pub(super) fn configure_performance_arp(&mut self) {
        let program = &self.arp.data.performance;
        let settings = self
            .arp
            .timing
            .map_or(program.settings, |t| t.apply(program.settings));
        let mut config = self.host_config;
        config.tracker_enabled = false;
        config.arp_page = 0;
        config.arp_enabled = program.enabled;
        arpeggiator::apply_arp_playback(
            &mut config,
            settings,
            &program.page,
            self.host_config.swing,
            self.host_config.humanize,
        );
        self.apply_routes(&mut config);
        self.source_overrides = self.resolved_source_overrides();
        self.config = config;
    }
    pub(super) fn performance_arp_deadline(&mut self) -> Option<u64> {
        if !self.performance_arp_enabled() && !self.performance_arp.running {
            return None;
        }
        self.with_performance_arp(|e| {
            e.refresh_arp_events();
            e.arp_deadline()
        })
    }
    pub(super) fn tick_performance_arp(&mut self, out: &mut impl FnMut(Out)) {
        if !self.performance_arp_enabled() && !self.performance_arp.running {
            return;
        }
        let previous = self.sources;
        self.with_performance_arp(|e| {
            if !e.arp_gated() {
                e.reset_automatic_arp(out);
            } else {
                e.tick_automatic_arp(out);
            }
        });
        self.refresh_perf_mod_routes(previous, out);
    }

    pub(super) fn refresh_perf_mod_routes(
        &mut self,
        previous: [f32; SOURCE_COUNT],
        out: &mut impl FnMut(Out),
    ) {
        if previous[PERF_MOD_SOURCES] != self.sources[PERF_MOD_SOURCES]
            && self.host_config.routes.iter().any(|route| {
                route.active() && PERF_MOD_SOURCES.contains(&(route.source as usize - 1))
            })
        {
            self.configure(self.host_config, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arpeggiator::{ArpMode, Settings};
    use routing::{Route, ARP_MOD_SOURCES, TARGETS};

    fn engine() -> Engine {
        let mut e = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        e.arp.data.performance.enabled = true;
        e.arp.data.performance.settings = Settings {
            mode: ArpMode::Loop,
            sync: false,
            free_ms: 50.0,
            ..Settings::default()
        };
        e.configure(
            Config {
                mode: MANUAL,
                tracker_enabled: true,
                bass_enabled: false,
                ..Config::default()
            },
            &mut |_| {},
        );
        e
    }
    fn attacks(e: &mut Engine, count: usize) -> Vec<(usize, u8)> {
        let mut events = Vec::new();
        e.advance(count, &mut |at, event| {
            if let Out::On(_, note, _) = event {
                events.push((at, note));
            }
        });
        events
    }
    #[test]
    fn performance_arp_runs_without_tracker_and_stops_on_release_even_with_strum_latch() {
        let mut e = engine();
        e.configure(
            Config {
                strum_chord_latch: true,
                ..e.host_config
            },
            &mut |_| {},
        );
        e.note_on(60, 60, 0, 0.8, Some(0), &mut |_| {});
        e.playing = true;
        let played = attacks(&mut e, 101);
        assert_eq!(
            played.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
            [0, 50, 100]
        );
        assert!(!e.arp.running);
        e.note_off(60, 0.0, &mut |_| {});
        assert!(e.notes.len > 0, "Strumfield keeps its chord");
        assert!(
            attacks(&mut e, 200).is_empty(),
            "live arp must follow key release"
        );
        assert!(!e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == arpeggiator::PERFORMANCE_ARP_OWNER));
    }
    #[test]
    fn performance_arp_and_tracker_have_independent_clocks_and_pause_ownership() {
        let mut e = engine();
        e.arp.data.rows[0].chord = Some(harmony::SavedChord {
            root: 48,
            second: None,
            additions: 0,
            quality: 0,
            inversion: 0,
            spread: 0,
            transpose: 0,
        });
        e.arp.data.rows[0].settings = Settings {
            mode: ArpMode::Loop,
            sync: false,
            free_ms: 100.0,
            ..Settings::default()
        };
        e.configure(
            Config {
                arp_enabled: true,
                ..e.host_config
            },
            &mut |_| {},
        );
        e.note_on(72, 72, 0, 0.8, Some(0), &mut |_| {});
        let played = attacks(&mut e, 101);
        assert_eq!(
            played
                .iter()
                .filter(|(_, n)| *n < 60)
                .map(|(t, _)| *t)
                .collect::<Vec<_>>(),
            [0, 100]
        );
        assert_eq!(
            played
                .iter()
                .filter(|(_, n)| *n >= 72)
                .map(|(t, _)| *t)
                .collect::<Vec<_>>(),
            [0, 50, 100]
        );
        e.tracker_pause(&mut |_| {});
        assert!(e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == arpeggiator::PERFORMANCE_ARP_OWNER));
        assert!(!e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == arpeggiator::ARP_OWNER));
        assert!(attacks(&mut e, 150).iter().all(|(_, n)| *n >= 72));
        e.panic(&mut |_| {});
        assert!(attacks(&mut e, 300).is_empty());
    }

    #[test]
    fn performance_lanes_meter_and_route_independently_of_the_tracker() {
        let mut e = engine();
        let row = &mut e.arp.data.rows[0];
        row.chord = Some(harmony::SavedChord {
            root: 48,
            second: None,
            additions: 0,
            quality: 0,
            inversion: 0,
            spread: 0,
            transpose: 0,
        });
        row.page.loops[0].steps[0].mods = [-30, 20];
        e.arp.data.performance.page.loops[0].steps[0].mods = [50, -25];
        let mut config = Config {
            arp_enabled: true,
            velocity: 0.5,
            ..e.host_config
        };
        for (source, id) in PERF_MOD_SOURCES.zip(["velocity", "velocity_scale"]) {
            config.routes[source - PERF_MOD_SOURCES.start] = Route {
                source: (source + 1) as u8,
                target: TARGETS.iter().position(|t| t.id == id).unwrap() as u8,
                max: 0.4,
                ..Route::default()
            };
        }
        e.configure(config, &mut |_| {});
        e.note_on(72, 72, 0, 0.8, Some(0), &mut |_| {});
        let mut performance_velocity = None;
        e.advance(1, &mut |_, event| {
            if let Out::On(_, 72, velocity) = event {
                performance_velocity = Some(velocity);
            }
        });
        let snapshot = e.snapshot();
        assert_eq!(snapshot.sources[ARP_MOD_SOURCES], [-0.3, 0.2]);
        assert_eq!(snapshot.sources[PERF_MOD_SOURCES], [0.5, -0.25]);
        assert!((performance_velocity.unwrap() - 0.5584).abs() < 1e-6);

        e.tracker_pause(&mut |_| {});
        assert_eq!(e.snapshot().sources[PERF_MOD_SOURCES], [0.5, -0.25]);
        e.note_off(72, 0.0, &mut |_| {});
        e.advance(1, &mut |_, _| {});
        assert_eq!(e.snapshot().sources[PERF_MOD_SOURCES], [0.0; 2]);
        assert_eq!(e.config.velocity, config.velocity);
        assert_eq!(e.config.velocity_scale, config.velocity_scale);
    }

    #[test]
    fn performance_lane_routes_smooth_from_step_changes_and_clear_on_bypass() {
        let mut e = engine();
        e.arp.data.performance.page.loops[0].steps[0].mods = [100, 0];
        let mut config = Config {
            velocity: 0.5,
            ..e.host_config
        };
        config.routes[0] = Route {
            source: (PERF_MOD_SOURCES.start + 1) as u8,
            target: TARGETS.iter().position(|t| t.id == "velocity").unwrap() as u8,
            max: 0.4,
            smoothing_ms: 10.0,
            ..Route::default()
        };
        e.configure(config, &mut |_| {});
        e.note_on(60, 60, 0, 0.8, Some(0), &mut |_| {});
        e.advance(1, &mut |_, _| {});
        assert_eq!(e.config.velocity, 0.5);
        e.advance(5, &mut |_, _| {});
        assert!((e.config.velocity - 0.698).abs() < 1e-6);

        e.arp.data.performance.enabled = false;
        e.accept_arp_generation(&mut |_| {});
        assert_eq!(e.snapshot().sources[PERF_MOD_SOURCES], [0.0; 2]);
        e.advance(10, &mut |_, _| {});
        assert_eq!(e.config.velocity, config.velocity);
    }
    #[test]
    fn tracker_bypass_uses_performance_program_without_overwriting_row() {
        let mut e = engine();
        e.arp.data.rows[0].chord = Some(harmony::SavedChord {
            root: 48,
            second: None,
            additions: 0,
            quality: 0,
            inversion: 0,
            spread: 0,
            transpose: 0,
        });
        e.arp.data.rows[0].settings.pattern = 1;
        e.arp.data.rows[0].enabled = false;
        e.arp.data.performance.page.loops[0].octave = 1;
        e.configure(
            Config {
                arp_enabled: true,
                ..e.host_config
            },
            &mut |_| {},
        );
        assert_eq!(attacks(&mut e, 101), [(0, 60), (50, 64), (100, 67)]);
        assert_eq!(e.arp.data.rows[0].settings.pattern, 1);
        e.arp.data.rows[0].enabled = true;
        e.configure(e.host_config, &mut |_| {});
        assert_eq!(e.active_arp_page().loops[0].octave, 0);
    }
    #[test]
    fn performance_arp_suppresses_chord_glissando_and_bypass_restores_it() {
        let mut e = engine();
        e.configure(
            Config {
                always_chord: true,
                gliss_ms: 500.0,
                ..e.host_config
            },
            &mut |_| {},
        );
        e.note_on(60, 60, 0, 0.8, Some(0), &mut |_| {});
        assert!(e.layer_pending.iter().all(Option::is_none));
        assert_eq!(attacks(&mut e, 101).len(), 3);
        e.arp.data.performance.enabled = false;
        e.accept_arp_generation(&mut |_| {});
        assert!(e.layer_pending.iter().any(Option::is_some));
        assert!(!e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == arpeggiator::PERFORMANCE_ARP_OWNER));
    }
    #[test]
    fn strum_chord_latch_retains_harmony_and_ringing_notes_without_latching_chord_layer() {
        let mut e = engine();
        e.arp.data.performance.enabled = false;
        e.configure(
            Config {
                strum_chord_latch: true,
                always_chord: true,
                ..e.host_config
            },
            &mut |_| {},
        );
        e.note_on(60, 60, 0, 0.8, Some(0), &mut |_| {});
        e.note_on(64, 64, 0, 0.8, None, &mut |_| {});
        let chord = e.notes;
        e.strike(60, 0.8, 350, &mut |_| {});
        e.note_off(64, 0.0, &mut |_| {});
        e.note_off(60, 0.0, &mut |_| {});
        assert_eq!(e.notes, chord);
        assert!(e
            .voices
            .iter()
            .flatten()
            .any(|v| v.note == 60 && v.off > e.now));
        assert!(!e.voices.iter().flatten().any(|v| v.layer));
        e.strike(e.notes.values[1], 0.8, 350, &mut |_| {});
        e.configure(
            Config {
                strum_chord_latch: false,
                ..e.host_config
            },
            &mut |_| {},
        );
        assert_eq!(e.notes.len, 0);
    }
    #[test]
    fn performance_arp_stop_keeps_delay_echo() {
        let mut e = engine();
        let mut delay = crate::midi_fx::DelaySettings {
            enabled: true,
            sync: false,
            sources: crate::midi_fx::source::ARP,
            ..crate::midi_fx::DelaySettings::default()
        };
        delay.taps[0].time = 10.0;
        delay.taps[0].velocity = 1.0;
        delay.taps[0].feedback = 0.0;
        e.arp.data.midi_fx.delay = delay;
        e.note_on(60, 60, 0, 0.8, Some(0), &mut |_| {});
        e.advance(15, &mut |_, _| {});
        assert!(
            e.voices.iter().flatten().any(|v| v.owner == DELAY_OWNER),
            "echo should be sounding before the arp stops"
        );
        assert!(e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == arpeggiator::PERFORMANCE_ARP_OWNER));
        e.note_off(60, 0.0, &mut |_| {});
        e.advance(1, &mut |_, _| {});
        assert!(!e
            .voices
            .iter()
            .flatten()
            .any(|v| v.owner == arpeggiator::PERFORMANCE_ARP_OWNER));
        assert!(
            e.voices.iter().flatten().any(|v| v.owner == DELAY_OWNER),
            "stopping the live arp must not release delay echoes"
        );
    }
    #[test]
    fn performance_arp_releases_old_output_channels_when_routing_changes() {
        let mut e = engine();
        e.note_on(60, 60, 0, 0.8, Some(0), &mut |_| {});
        assert_eq!(attacks(&mut e, 1).len(), 1);
        let mut released = Vec::new();
        e.configure(Config { output_channel: 4, ..e.host_config }, &mut |event| released.push(event));
        assert!(released.iter().any(|event| matches!(event, Out::Off(0, 60, _))));
        assert!(!e.voices.iter().flatten().any(|v| v.owner == arpeggiator::PERFORMANCE_ARP_OWNER));
        let mut next = Vec::new();
        e.advance(1, &mut |_, event| next.push(event));
        assert!(next.iter().any(|event| matches!(event, Out::On(4, _, _))));
    }

}
