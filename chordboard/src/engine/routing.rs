//! Fixed-capacity, audio-thread routing. Sources never feed back from destinations.
use super::{Config, Engine, Out};
pub const ROUTE_COUNT: usize = 16;
pub const SOURCE_COUNT: usize = 9;
pub const SOURCES: [&str; SOURCE_COUNT + 1] = [
    "Off",
    "Pitch wheel",
    "Mod wheel",
    "Expression",
    "Breath",
    "Pressure",
    "Timbre",
    "Velocity",
    "Strum X",
    "Strum Y",
];
pub const SOURCE_SHORT: [&str; SOURCE_COUNT] = [
    "Pitch", "Mod", "Expr", "Breath", "Press", "Timbre", "Vel", "X", "Y",
];
pub const SOURCE_DEFAULTS: [f32; SOURCE_COUNT] = [0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.8, 0.0, 0.8];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Route {
    pub enabled: bool,
    pub source: u8,
    pub target: u8,
    pub min: f32,
    pub max: f32,
    pub curve: f32,
}
impl Default for Route {
    fn default() -> Self {
        Self {
            enabled: true,
            source: 0,
            target: 0,
            min: 0.0,
            max: 1.0,
            curve: 0.0,
        }
    }
}
impl Route {
    pub fn active(self) -> bool {
        self.enabled
            && (1..=SOURCE_COUNT as u8).contains(&self.source)
            && TARGETS
                .get(self.target as usize)
                .is_some_and(Target::available)
    }
    pub fn value(self, input: f32) -> f32 {
        let shape = input
            .clamp(0.0, 1.0)
            .powf(2.0_f32.powf(self.curve.clamp(-1.0, 1.0) * 3.0));
        (self.min + (self.max - self.min) * shape).clamp(0.0, 1.0)
    }
}
pub struct Target {
    pub name: &'static str,
    pub id: &'static str,
    pub min: f32,
    pub max: f32,
    pub discrete: bool,
}
// Indices are saved in host state. Append destinations; never reorder this list.
pub const TARGETS: &[Target] = &[
    Target {
        name: "Strings",
        id: "strings",
        min: 3.0,
        max: 12.0,
        discrete: true,
    },
    Target {
        name: "Spread",
        id: "spread",
        min: 0.0,
        max: 2.0,
        discrete: true,
    },
    Target {
        name: "Quality",
        id: "quality",
        min: 0.0,
        max: 11.0,
        discrete: true,
    },
    Target {
        name: "Inversion",
        id: "inversion",
        min: 0.0,
        max: 5.0,
        discrete: true,
    },
    Target {
        name: "Transpose",
        id: "transpose",
        min: -24.0,
        max: 24.0,
        discrete: true,
    },
    Target {
        name: "Velocity",
        id: "velocity",
        min: 0.01,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "Note length",
        id: "length_ms",
        min: 20.0,
        max: 3000.0,
        discrete: false,
    },
    Target {
        name: "Sweep",
        id: "strum_ms",
        min: 0.0,
        max: 1500.0,
        discrete: false,
    },
    Target {
        name: "Sweep sync",
        id: "strum_sync",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "Sweep beats",
        id: "strum_beats",
        min: 0.0625,
        max: 2.0,
        discrete: false,
    },
    Target {
        name: "Direction",
        id: "direction",
        min: 0.0,
        max: 2.0,
        discrete: true,
    },
    Target {
        name: "Velocity contour",
        id: "contour",
        min: -1.0,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "Arp pattern",
        id: "arp_pattern",
        min: 0.0,
        max: 4.0,
        discrete: true,
    },
    Target {
        name: "Arp rate",
        id: "rate",
        min: 0.0625,
        max: 2.0,
        discrete: false,
    },
    Target {
        name: "Gate",
        id: "gate",
        min: 0.05,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "Swing",
        id: "swing",
        min: -0.75,
        max: 0.75,
        discrete: false,
    },
    Target {
        name: "Octaves",
        id: "octaves",
        min: 1.0,
        max: 4.0,
        discrete: true,
    },
    Target {
        name: "Humanize",
        id: "humanize",
        min: 0.0,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "Root on select",
        id: "root_on_select",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "Hold chord",
        id: "latch",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "Note filter",
        id: "filter",
        min: 0.0,
        max: 5.0,
        discrete: true,
    },
    Target {
        name: "Play mode",
        id: "mode",
        min: 1.0,
        max: 3.0,
        discrete: true,
    },
    Target {
        name: "Y destination",
        id: "y_target",
        min: 0.0,
        max: 5.0,
        discrete: true,
    },
    Target {
        name: "Y CC",
        id: "y_cc",
        min: 0.0,
        max: 119.0,
        discrete: true,
    },
    Target {
        name: "Reverse X",
        id: "x_reverse",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "Reverse Y",
        id: "y_reverse",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "X minimum",
        id: "x_min",
        min: 0.0,
        max: 0.99,
        discrete: false,
    },
    Target {
        name: "X maximum",
        id: "x_max",
        min: 0.01,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "Y minimum",
        id: "y_min",
        min: 0.0,
        max: 0.99,
        discrete: false,
    },
    Target {
        name: "Y maximum",
        id: "y_max",
        min: 0.01,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "MPE members",
        id: "members",
        min: 1.0,
        max: 15.0,
        discrete: true,
    },
    Target {
        name: "Member bend",
        id: "bend_range",
        min: 1.0,
        max: 96.0,
        discrete: false,
    },
    Target {
        name: "Master bend",
        id: "master_range",
        min: 1.0,
        max: 24.0,
        discrete: false,
    },
    Target {
        name: "Output channel",
        id: "output_channel",
        min: 0.0,
        max: 15.0,
        discrete: true,
    },
    Target {
        name: "Split channels",
        id: "split_channels",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "Bass channel",
        id: "bass_channel",
        min: 0.0,
        max: 15.0,
        discrete: true,
    },
    Target {
        name: "Upper channel",
        id: "upper_channel",
        min: 0.0,
        max: 15.0,
        discrete: true,
    },
    Target {
        name: "Strings played",
        id: "strings_played",
        min: 1.0,
        max: 12.0,
        discrete: true,
    },
    Target {
        name: "Strum X / sweep",
        id: "x",
        min: 0.0,
        max: 1.0,
        discrete: false,
    },
];
impl Target {
    // Keep saved destination indices stable; touch bounds are no longer routable.
    pub fn available(&self) -> bool {
        !matches!(self.id, "x_min" | "x_max" | "y_min" | "y_max")
    }

    pub fn plain(&self, norm: f32) -> f32 {
        let v = self.min + norm.clamp(0.0, 1.0) * (self.max - self.min);
        if self.discrete {
            v.round()
        } else {
            v
        }
    }
    pub fn label(&self, norm: f32) -> String {
        let v = self.plain(norm);
        match self.id {
            "spread" => ["Close", "Open", "Wide"][v as usize].into(),
            "quality" => crate::harmony::QUALITY_NAMES[v as usize].into(),
            "mode" => ["Auto Strum", "Manual Strum", "Arpeggiator"][v as usize - 1].into(),
            "direction" => ["Up", "Down", "Alternate"][v as usize].into(),
            "arp_pattern" => ["Up", "Down", "Up/Down", "Played order", "Random"][v as usize].into(),
            "y_target" => [
                "Velocity",
                "Gate",
                "Pressure",
                "Timbre",
                "Bend",
                "Custom CC",
            ][v as usize]
                .into(),
            "filter" => [
                "All",
                "Bass",
                "Top",
                "Bass + Top",
                "Odd tones",
                "Even tones",
            ][v as usize]
                .into(),
            "strum_sync" | "root_on_select" | "latch" | "x_reverse" | "y_reverse"
            | "split_channels" => if v >= 0.5 { "On" } else { "Off" }.into(),
            "length_ms" | "strum_ms" => format!("{v:.0} ms"),
            "transpose" => format!("{v:+.0} st"),
            "bend_range" | "master_range" => format!("{v:.0} st"),
            "output_channel" | "bass_channel" | "upper_channel" => format!("{:.0}", v + 1.0),
            "velocity" | "humanize" | "gate" | "contour" | "swing" => format!("{:.0}%", v * 100.0),
            _ if self.discrete => format!("{v:.0}"),
            _ => format!("{v:.2}"),
        }
    }
    fn apply(&self, c: &mut Config, norm: f32) {
        let v = self.plain(norm);
        match self.id {
            "strings" => c.strings = v as u8,
            "strings_played" => c.strings_played = v as u8,
            "x" => c.routed_x = Some(v),
            "spread" => c.spread = v as u8,
            "quality" => c.quality = v as u8,
            "inversion" => c.inversion = v as u8,
            "transpose" => c.transpose = v as i8,
            "velocity" => c.velocity = v,
            "length_ms" => c.length_ms = v,
            "strum_ms" => c.strum_ms = v,
            "strum_sync" => c.strum_sync = v >= 0.5,
            "strum_beats" => c.strum_beats = v,
            "direction" => c.direction = v as u8,
            "contour" => c.contour = v,
            "arp_pattern" => c.arp_pattern = v as u8,
            "rate" => c.rate = v,
            "gate" => c.gate = v,
            "swing" => c.swing = v,
            "octaves" => c.octaves = v as u8,
            "humanize" => c.humanize = v,
            "root_on_select" => c.root_on_select = v >= 0.5,
            "latch" => c.latch = v >= 0.5,
            "filter" => c.filter = v as u8,
            "mode" => c.mode = v as u8,
            "y_target" => c.y_target = v as u8,
            "y_cc" => c.y_cc = v as u8,
            "x_reverse" => c.x_reverse = v >= 0.5,
            "y_reverse" => c.y_reverse = v >= 0.5,
            "x_min" => c.x_min = v,
            "x_max" => c.x_max = v,
            "y_min" => c.y_min = v,
            "y_max" => c.y_max = v,
            "members" => c.members = v as u8,
            "bend_range" => c.bend_range = v,
            "master_range" => c.master_range = v,
            "output_channel" => c.output_channel = v as u8,
            "split_channels" => c.split_channels = v >= 0.5,
            "bass_channel" => c.bass_channel = v as u8,
            "upper_channel" => c.upper_channel = v as u8,
            _ => {}
        }
    }
}
impl Engine {
    pub(super) fn apply_routes(&self, config: &mut Config) {
        for route in self.host_config.routes {
            if route.active() {
                TARGETS[route.target as usize]
                    .apply(config, route.value(self.sources[route.source as usize - 1]));
            }
        }
    }
    pub(super) fn source(&mut self, index: usize, value: f32, out: &mut impl FnMut(Out)) {
        if !value.is_finite() || index >= SOURCE_COUNT {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        if self.sources[index] == value {
            return;
        }
        self.sources[index] = value;
        if self
            .host_config
            .routes
            .iter()
            .any(|r| r.active() && r.source as usize == index + 1)
        {
            self.configure(self.host_config, out);
        }
    }
    pub fn routed_values(&self) -> [Option<f32>; TARGET_COUNT] {
        let mut values = [None; TARGET_COUNT];
        for route in self.host_config.routes {
            if route.active() {
                values[route.target as usize] =
                    Some(route.value(self.sources[route.source as usize - 1]));
            }
        }
        values
    }
}
pub fn available_targets() -> impl Iterator<Item = (usize, &'static Target)> {
    TARGETS
        .iter()
        .enumerate()
        .filter(|(_, target)| target.available())
}
pub const TARGET_COUNT: usize = TARGETS.len();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Command, ARP, AUTO, MANUAL};
    fn target(id: &str) -> u8 {
        TARGETS.iter().position(|t| t.id == id).unwrap() as u8
    }
    fn route(source: u8, id: &str) -> Route {
        Route {
            source,
            target: target(id),
            ..Route::default()
        }
    }
    fn ons(events: &[Out]) -> Vec<u8> {
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

    #[test]
    fn root_on_select_plays_immediately_in_manual_and_releases() {
        let mut e = Engine::default();
        let mut events = Vec::new();
        e.configure(
            Config {
                mode: MANUAL,
                root_on_select: true,
                transpose: 12,
                inversion: 1,
                ..Config::default()
            },
            &mut |v| events.push(v),
        );
        e.midi_note(true, 0, 60, 0.8, &mut |v| events.push(v));
        assert_eq!(ons(&events), vec![72]);
        e.midi_note(false, 0, 60, 0.0, &mut |v| events.push(v));
        assert!(!events.iter().any(|v| matches!(v, Out::Off(_, 72, _))));
        e.now += e.duration();
        e.tick(&mut |v| events.push(v));
        assert!(events.iter().any(|v| matches!(v, Out::Off(_, 72, _))));
        assert!(e.voices.iter().all(Option::is_none));
    }
    #[test]
    fn root_is_not_doubled_by_auto_or_first_arp_step() {
        for mode in [AUTO, ARP] {
            let mut e = Engine::default();
            e.configure(
                Config {
                    mode,
                    direction: 1,
                    root_on_select: true,
                    ..Config::default()
                },
                &mut |_| {},
            );
            let mut events = Vec::new();
            e.midi_note(true, 0, 60, 0.8, &mut |v| events.push(v));
            assert_eq!(ons(&events), vec![60]);
            for _ in 0..6000 {
                e.tick(&mut |v| events.push(v));
            }
            assert_eq!(ons(&events).iter().filter(|&&n| n == 60).count(), 1);
        }
    }
    #[test]
    fn disabled_root_preserves_manual_silence() {
        let mut e = Engine::default();
        e.configure(
            Config {
                mode: MANUAL,
                ..Config::default()
            },
            &mut |_| {},
        );
        let mut events = Vec::new();
        e.midi_note(true, 0, 60, 0.8, &mut |v| events.push(v));
        assert!(ons(&events).is_empty());
    }
    #[test]
    fn midi_and_field_sources_are_captured_even_with_legacy_axis_assignments() {
        let mut e = Engine::default();
        e.configure(
            Config {
                mode: MANUAL,
                ..Config::default()
            },
            &mut |_| {},
        );
        e.bend(0, 0.9, &mut |_| {});
        e.control(0, 1, 0.2, &mut |_| {});
        e.control(0, 11, 0.3, &mut |_| {});
        e.control(0, 2, 0.4, &mut |_| {});
        e.pressure(0, None, 0.6, &mut |_| {});
        e.control(0, 74, 0.7, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.position(0.1, 0, &mut |_| {});
        e.position(0.5, 1, &mut |_| {});
        assert_eq!(
            e.snapshot().sources,
            [0.9, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.1, 0.5]
        );
    }
    #[test]
    fn one_controller_drives_strings_and_spread_and_disable_restores_base() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: MANUAL,
            strings: 8,
            spread: 1,
            ..Config::default()
        };
        c.routes[0] = route(2, "strings");
        c.routes[1] = route(2, "spread");
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.control(0, 1, 1.0, &mut |_| {});
        assert_eq!((e.config.strings, e.config.spread), (12, 2));
        assert_eq!((e.host_config.strings, e.host_config.spread), (8, 1));
        c.routes[0].enabled = false;
        c.routes[1].enabled = false;
        e.configure(c, &mut |_| {});
        assert_eq!((e.config.strings, e.config.spread), (8, 1));
    }
    #[test]
    fn reversed_and_curved_ranges_keep_endpoints() {
        let r = Route {
            min: 0.8,
            max: 0.2,
            curve: 1.0,
            ..route(1, "strings")
        };
        assert!((r.value(0.0) - 0.8).abs() < 1e-6);
        assert!((r.value(1.0) - 0.2).abs() < 1e-6);
        assert!(r.value(0.5) > 0.7);
        assert!((Route { curve: 0.0, ..r }.value(0.5) - 0.5).abs() < 1e-6);
    }
    #[test]
    fn later_routes_win_and_never_feed_destination_values_back_into_sources() {
        let mut e = Engine::default();
        let mut c = Config::default();
        c.routes[0] = route(2, "strings");
        c.routes[1] = Route {
            min: 0.0,
            max: 0.0,
            ..route(3, "strings")
        };
        e.configure(c, &mut |_| {});
        e.control(0, 1, 1.0, &mut |_| {});
        assert_eq!(e.config.strings, 3);
        assert_eq!(e.sources[1], 1.0);
        assert_eq!(e.sources[2], 0.0);
    }
    #[test]
    fn routing_cannot_resurrect_a_released_chord() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: AUTO,
            ..Config::default()
        };
        c.routes[0] = route(2, "transpose");
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.midi_note(false, 0, 60, 0.0, &mut |_| {});
        let mut events = Vec::new();
        e.control(0, 1, 1.0, &mut |v| events.push(v));
        for _ in 0..100 {
            e.tick(&mut |v| events.push(v));
        }
        assert!(ons(&events).is_empty());
    }
    #[test]
    fn explicit_velocity_route_does_not_square_incoming_velocity() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: MANUAL,
            root_on_select: true,
            ..Config::default()
        };
        c.routes[0] = route(7, "velocity");
        e.configure(c, &mut |_| {});
        let mut events = Vec::new();
        e.midi_note(true, 0, 60, 0.5, &mut |v| events.push(v));
        assert!(events
            .iter()
            .any(|e| matches!(e,Out::On(_,60,v) if (*v-0.505).abs()<1e-5)));
        e.command(Command::Panic, &mut |_| {});
        assert!(e.voices.iter().all(Option::is_none));
    }
    #[test]
    fn modwheel_controls_strings_played_without_changing_total() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: AUTO,
            strings: 12,
            strum_ms: 0.0,
            ..Config::default()
        };
        c.routes[0] = route(2, "strings_played");
        e.configure(c, &mut |_| {});
        for (value, count) in [(0.0, 1), (0.5, 7), (1.0, 12)] {
            e.control(0, 1, value, &mut |_| {});
            assert_eq!(e.config.strings, 12);
            assert_eq!(e.config.strings_played, count);
            e.midi_note(true, 0, 63, 0.8, &mut |_| {});
            let mut events = Vec::new();
            e.tick(&mut |v| events.push(v));
            assert_eq!(ons(&events).len(), count as usize);
            e.midi_note(false, 0, 63, 0.0, &mut |_| {});
        }
    }
    #[test]
    fn routed_x_sweeps_once_and_overrides_legacy_mapping_without_feedback() {
        for source in [2, 4, 8] {
            let mut e = Engine::default();
            let mut c = Config {
                mode: MANUAL,
                ..Config::default()
            };
            c.routes[0] = route(source, "x");
            e.configure(c, &mut |_| {});
            e.midi_note(true, 0, 63, 0.8, &mut |_| {});
            e.position(0.0, 0, &mut |_| {});
            let mut events = Vec::new();
            match source {
                2 => e.control(0, 1, 1.0, &mut |v| events.push(v)),
                4 => e.control(0, 2, 1.0, &mut |v| events.push(v)),
                _ => e.position(1.0, 0, &mut |v| events.push(v)),
            }
            assert_eq!(ons(&events), vec![67, 70, 75, 79, 82, 87, 91]);
            events.clear();
            e.configure(c, &mut |v| events.push(v));
            assert!(ons(&events).is_empty());
            assert_eq!(e.x, 1.0);
            if source == 4 {
                e.control(0, 1, 0.0, &mut |v| events.push(v));
                assert_eq!(e.x, 1.0);
                assert!(ons(&events).is_empty());
            }
        }
    }
    #[test]
    fn touch_bounds_are_not_modulation_destinations() {
        let mut e = Engine::default();
        let mut c = Config::default();
        for (i, id) in ["x_min", "x_max", "y_min", "y_max"].iter().enumerate() {
            c.routes[i] = route(2, id);
            assert!(!c.routes[i].active());
            assert!(!available_targets().any(|(_, t)| t.id == *id));
        }
        e.configure(c, &mut |_| {});
        e.control(0, 1, 1.0, &mut |_| {});
        assert_eq!(
            (
                e.config.x_min,
                e.config.x_max,
                e.config.y_min,
                e.config.y_max
            ),
            (c.x_min, c.x_max, c.y_min, c.y_max)
        );
        assert!(route(2, "x").active());
    }
}
