//! Fixed-capacity routing with bounded source-to-source resolution.
use super::{Config, Engine, Out};
pub const ROUTE_COUNT: usize = 16;
pub const SOURCE_COUNT: usize = 11;
pub const SOURCES: [&str; SOURCE_COUNT + 1] = [
    "Off",
    "Pitch wheel",
    "Modwheel",
    "Expression",
    "Breath",
    "Pressure",
    "Timbre",
    "Input Vel",
    "X controller",
    "Y controller",
    "Mod 1",
    "Mod 2",
];
pub const SOURCE_SHORT: [&str; SOURCE_COUNT] = [
    "Pitch", "Modwheel", "Expr", "Breath", "Press", "Timbre", "Input Vel", "X", "Y", "Mod 1", "Mod 2",
];
pub const SOURCE_DEFAULTS: [f32; SOURCE_COUNT] = [0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.8, 0.0, 0.8, 0.0, 0.0];

#[derive(Clone, Copy, Default)]
pub(super) struct RouteSmoother {
    route: Option<Route>,
    value: f32,
    target: f32,
    increment: f32,
    remaining: u64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Route {
    pub enabled: bool,
    pub source: u8,
    pub target: u8,
    pub min: f32,
    pub max: f32,
    pub curve: f32,
    pub smoothing_ms: f32,
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
            smoothing_ms: 0.0,
        }
    }
}
impl Route {
    pub fn additive(self) -> bool { self.source >= 10 }
    /// Bipolar routes preserve zero and use the contour's span as their depth.
    pub fn output(self, input: f32) -> f32 {
        if self.additive() {
            let input = input.clamp(-1.0, 1.0);
            input.signum() * (self.max - self.min)
                * input.abs().powf(2.0_f32.powf(self.curve.clamp(-1.0, 1.0) * 3.0))
        } else { self.value(input) }
    }
    pub fn active(self) -> bool {
        self.enabled
            && (1..=SOURCE_COUNT as u8).contains(&self.source)
            && TARGETS
                .get(self.target as usize)
                .is_some_and(|target| target.accepts_source(self.source))
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
        name: "Voicing",
        id: "spread",
        min: 0.0,
        max: (crate::harmony::VOICING_NAMES.len() - 1) as f32,
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
        max: 11.0,
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
        name: "Rate (free)",
        id: "strum_ms",
        min: 0.0,
        max: 1500.0,
        discrete: false,
    },
    Target {
        name: "Rate tempo sync",
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
        max: 4.0,
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
        max: 6.0,
        discrete: true,
    },
    Target {
        name: "Rate (synced)",
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
        name: "Latch",
        id: "latch",
        min: 0.0,
        max: 1.0,
        discrete: true,
    },
    Target {
        name: "Auto strum hold",
        id: "strum_hold",
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
        name: "Strumfield position",
        id: "x",
        min: 0.0,
        max: 1.0,
        discrete: false,
    },
    Target {
        name: "Arp page",
        id: "seq_page_0",
        min: 0.0,
        max: 7.0,
        discrete: true,
    },
    Target {
        name: "Chords page",
        id: "seq_page_1",
        min: 0.0,
        max: 7.0,
        discrete: true,
    },
    Target {
        name: "Bass page",
        id: "seq_page_2",
        min: 0.0,
        max: 7.0,
        discrete: true,
    },
    Target {
        name: "Harmony page",
        id: "seq_page_3",
        min: 0.0,
        max: 7.0,
        discrete: true,
    },
    Target { name: "Pitch wheel", id: "source_0", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Modwheel", id: "source_1", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Expression", id: "source_2", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Breath", id: "source_3", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Pressure", id: "source_4", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Timbre", id: "source_5", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Input velocity", id: "source_6", min: 0.0, max: 1.0, discrete: false },
    Target { name: "X controller", id: "source_7", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Y controller", id: "source_8", min: 0.0, max: 1.0, discrete: false },
    Target { name: "Mod 1", id: "source_9", min: -1.0, max: 1.0, discrete: false },
    Target { name: "Mod 2", id: "source_10", min: -1.0, max: 1.0, discrete: false },
];
impl Target {
    fn normalized(&self, c: &Config) -> f32 {
        let v = match self.id {
            "seq_page_0" => c.seq_pages[0] as f32,
            "seq_page_1" => c.seq_pages[1] as f32,
            "seq_page_2" => c.seq_pages[2] as f32,
            "seq_page_3" => c.seq_pages[3] as f32,
            "strings" => c.strings as f32,
            "strings_played" => c.strings_played as f32,
            "x" => c.routed_x.unwrap_or(0.0),
            "spread" => c.spread as f32,
            "quality" => c.quality as f32,
            "inversion" => c.inversion as f32,
            "transpose" => c.transpose as f32,
            "velocity" => c.velocity,
            "length_ms" => c.length_ms,
            "strum_ms" => c.strum_ms,
            "strum_sync" => c.strum_sync as u8 as f32,
            "strum_beats" => c.strum_beats,
            "strum_hold" => c.strum_hold as u8 as f32,
            "direction" => c.direction as f32,
            "contour" => c.contour,
            "arp_pattern" => c.arp_pattern as f32,
            "rate" => c.rate,
            "gate" => c.gate,
            "swing" => c.swing,
            "octaves" => c.octaves as f32,
            "humanize" => c.humanize,
            "root_on_select" => c.root_on_select as u8 as f32,
            "latch" => c.latch as u8 as f32,
            "filter" => c.filter as f32,
            "mode" => if c.seq_loop { super::ARP as f32 } else { super::AUTO as f32 },
            "y_target" => c.y_target as f32,
            "y_cc" => c.y_cc as f32,
            "x_reverse" => c.x_reverse as u8 as f32,
            "y_reverse" => c.y_reverse as u8 as f32,
            "x_min" => c.x_min,
            "x_max" => c.x_max,
            "y_min" => c.y_min,
            "y_max" => c.y_max,
            "members" => c.members as f32,
            "bend_range" => c.bend_range,
            "master_range" => c.master_range,
            "output_channel" => c.output_channel as f32,
            "bass_channel" => c.bass_channel as f32,
            "upper_channel" => c.upper_channel as f32,
            _ => self.min,
        };
        ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
    pub fn source_index(&self) -> Option<usize> {
        self.id.strip_prefix("source_").and_then(|n| n.parse().ok())
    }

    // Keep saved destination indices stable; touch bounds are no longer routable.
    pub fn available(&self) -> bool {
        !matches!(
            self.id,
            "x_min"
                | "x_max"
                | "y_min"
                | "y_max"
                | "direction"
                | "strum_beats"
                | "contour"
                | "strings"
        )
    }

    // Controller setup belongs to the input, so it cannot be changed by that
    // same input. The Strumfield destination is separate from the X controller.
    pub fn accepts_source(&self, source: u8) -> bool {
        self.available()
            && !(source > 0 && self.source_index() == Some(source as usize - 1))
            && !(source == 8 && self.id == "x_reverse")
            && !(source == 9 && matches!(self.id, "y_reverse" | "y_target" | "y_cc"))
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
            "spread" => crate::harmony::VOICING_NAMES[v as usize].into(),
            "quality" => crate::harmony::QUALITY_NAMES[v as usize].into(),
            "mode" => ["Arp Once", "Arp Loop (legacy Manual)", "Arp Loop"][v as usize - 1].into(),
            "direction" => {
                ["Up", "Down", "Alternate", "Up in 3rds", "Down in 3rds"][v as usize].into()
            }
            "arp_pattern" => [
                "Up",
                "Down",
                "Up/Down",
                "Up in 3rds",
                "Down in 3rds",
                "Played order",
                "Random",
            ][v as usize]
                .into(),
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
            "strum_sync" | "strum_hold" | "root_on_select" | "latch" | "x_reverse"
            | "y_reverse" => if v >= 0.5 { "On" } else { "Off" }.into(),
            "length_ms" | "strum_ms" => format!("{v:.0} ms"),
            "transpose" => format!("{v:+.0} st"),
            "bend_range" | "master_range" => format!("{v:.0} st"),
            "seq_page_0" | "seq_page_1" | "seq_page_2" | "seq_page_3" => format!("{:.0}", v + 1.0),
            "output_channel" | "bass_channel" | "upper_channel" => format!("{:.0}", v + 1.0),
            "velocity" | "humanize" | "gate" | "contour" | "swing" => format!("{:.0}%", v * 100.0),
            _ if self.discrete => format!("{v:.0}"),
            _ => format!("{v:.2}"),
        }
    }
    fn apply(&self, c: &mut Config, norm: f32) {
        let v = self.plain(norm);
        match self.id {
            "seq_page_0" => c.seq_pages[0] = v as u8,
            "seq_page_1" => c.seq_pages[1] = v as u8,
            "seq_page_2" => c.seq_pages[2] = v as u8,
            "seq_page_3" => c.seq_pages[3] = v as u8,
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
            "strum_beats" => {
                c.strum_beats = v;
                c.rate = v;
            }
            "strum_hold" => c.strum_hold = v >= 0.5,
            "direction" => {
                c.direction = v as u8;
                c.arp_pattern = v as u8;
            }
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
            "mode" => c.seq_loop = v as u8 != super::AUTO,
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
            "bass_channel" => c.bass_channel = v as u8,
            "upper_channel" => c.upper_channel = v as u8,
            _ => {}
        }
    }
}
impl Engine {
    fn route_output(&self, slot: usize, input: f32) -> f32 {
        let route = self.host_config.routes[slot];
        let smoother = self.route_smoothers[slot];
        if route.smoothing_ms > 0.0 && smoother.route.is_some_and(|old| old.source == route.source && old.target == route.target) { smoother.value }
        else { route.output(input) }
    }
    pub(super) fn prepare_route_smoothing(&mut self) {
        let sources = self.effective_sources();
        for (slot, route) in self.host_config.routes.iter().copied().enumerate() {
            let smoother = &mut self.route_smoothers[slot];
            if !route.active() { *smoother = RouteSmoother::default(); continue; }
            let target = route.output(sources[route.source as usize - 1]);
            let fresh = smoother.route.is_none_or(|old| old.source != route.source || old.target != route.target);
            let time_changed = smoother.route.is_some_and(|old| old.smoothing_ms != route.smoothing_ms);
            smoother.route = Some(route);
            if fresh || route.smoothing_ms <= 0.0 {
                smoother.value = target;
                smoother.target = target;
                smoother.remaining = 0;
            } else if smoother.target != target || time_changed {
                smoother.target = target;
                smoother.remaining = (route.smoothing_ms.clamp(0.0, 2000.0) * self.sample_rate / 1000.0).round().max(1.0) as u64;
                smoother.increment = (target - smoother.value) / smoother.remaining as f32;
            }
        }
    }
    pub(super) fn smoothing_active(&self) -> bool {
        self.route_smoothers.iter().any(|s| s.remaining > 0)
    }
    pub(super) fn tick_route_smoothing(&mut self, out: &mut impl FnMut(Out)) {
        if !self.smoothing_active() { return; }
        for smoother in &mut self.route_smoothers {
            if smoother.remaining > 0 {
                smoother.remaining -= 1;
                smoother.value = if smoother.remaining == 0 { smoother.target } else { smoother.value + smoother.increment };
            }
        }
        // Advance ramps per sample, but refresh MIDI/parameter destinations at
        // most once per millisecond so smoothing cannot flood the host queue.
        let interval = (self.sample_rate / 1000.0).round().max(1.0) as u64;
        if self.now.is_multiple_of(interval) || !self.smoothing_active() {
            self.configure(self.host_config, out);
        }
    }
    pub(super) fn resolved_source_overrides(&self) -> [Option<f32>; SOURCE_COUNT] {
        let effective = self.effective_sources();
        let mut values = [None; SOURCE_COUNT];
        for route in self.host_config.routes {
            if route.active() {
                if let Some(index) = TARGETS[route.target as usize].source_index() {
                    values[index] = Some(effective[index]);
                }
            }
        }
        values
    }
    pub(super) fn routed_expression(&self) -> super::Expression {
        super::Expression {
            bend: self.source_overrides[0].unwrap_or(self.expression.bend),
            pressure: self.source_overrides[4].unwrap_or(self.expression.pressure),
            timbre: self.source_overrides[5].unwrap_or(self.expression.timbre),
        }
    }
    pub(super) fn emit_routed_controllers(&self, channel: u8, out: &mut impl FnMut(Out)) {
        for (index, cc) in [(1, 1), (2, 11), (3, 2)] {
            if let Some(value) = self.source_overrides[index] { out(Out::Cc(channel, cc, value)); }
        }
    }
    /// Resolve each input once. A back edge reads the original input, so cycles
    /// stay deterministic and bounded instead of accumulating across samples.
    pub fn effective_sources(&self) -> [f32; SOURCE_COUNT] {
        fn resolve(engine: &Engine, index: usize, values: &mut [f32; SOURCE_COUNT], marks: &mut [u8; SOURCE_COUNT]) -> f32 {
            if marks[index] == 2 { return values[index]; }
            if marks[index] == 1 { return engine.sources[index]; }
            marks[index] = 1;
            if let Some((slot, route)) = engine.host_config.routes.iter().enumerate().rev().find(|(_, route)| {
                route.active() && !route.additive() && TARGETS[route.target as usize].source_index() == Some(index)
            }) {
                let input = resolve(engine, route.source as usize - 1, values, marks);
                values[index] = TARGETS[route.target as usize].plain(engine.route_output(slot, input));
            }
            let mut value = values[index];
            for (slot, route) in engine.host_config.routes.iter().enumerate() {
                if route.active() && route.additive() && TARGETS[route.target as usize].source_index() == Some(index) {
                    let input = resolve(engine, route.source as usize - 1, values, marks);
                    value += engine.route_output(slot, input);
                }
            }
            values[index] = value.clamp(if index >= 9 { -1.0 } else { 0.0 }, 1.0);
            marks[index] = 2;
            values[index]
        }
        let mut values = self.sources;
        let mut marks = [0; SOURCE_COUNT];
        for index in 0..SOURCE_COUNT { resolve(self, index, &mut values, &mut marks); }
        values
    }
    pub(super) fn apply_routes(&self, config: &mut Config) {
        let sources = self.effective_sources();
        let mut values = std::array::from_fn::<_, TARGET_COUNT, _>(|i| {
            if TARGETS[i].id == "x" && config.routed_x.is_none() { sources[7] }
            else { TARGETS[i].normalized(config) }
        });
        let mut touched = [false; TARGET_COUNT];
        for additive in [false, true] {
            for (slot, route) in self.host_config.routes.iter().enumerate() {
                if !route.active() || route.additive() != additive { continue; }
                let index = route.target as usize;
                if TARGETS[index].source_index().is_some() { continue; }
                let output = self.route_output(slot, sources[route.source as usize - 1]);
                if additive { values[index] += output; } else { values[index] = output; }
                touched[index] = true;
            }
        }
        for (index, target) in TARGETS.iter().enumerate() {
            if touched[index] { target.apply(config, values[index].clamp(0.0, 1.0)); }
        }
    }
    pub(super) fn source(&mut self, index: usize, value: f32, out: &mut impl FnMut(Out)) {
        if !value.is_finite() || index >= SOURCE_COUNT {
            return;
        }
        let value = value.clamp(if index >= 9 { -1.0 } else { 0.0 }, 1.0);
        if self.sources[index] == value {
            return;
        }
        self.sources[index] = value;
        if self
            .host_config
            .routes
            .iter()
            .any(|r| r.active() && (r.source as usize == index + 1 || TARGETS[r.target as usize].source_index() == Some(index)))
        {
            self.configure(self.host_config, out);
        }
    }
    pub fn routed_values(&self) -> [Option<f32>; TARGET_COUNT] {
        let sources = self.effective_sources();
        let mut values = [None; TARGET_COUNT];
        for route in self.host_config.routes {
            if route.active() {
                let index = route.target as usize;
                values[index] = Some(if let Some(source) = TARGETS[index].source_index() {
                    ((sources[source] - TARGETS[index].min) / (TARGETS[index].max - TARGETS[index].min)).clamp(0.0, 1.0)
                } else { TARGETS[index].normalized(&self.config) });
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

pub fn default_routes() -> [Route; ROUTE_COUNT] {
    let mut routes = [Route::default(); ROUTE_COUNT];
    routes[0] = Route {
        source: 8,
        target: TARGETS
            .iter()
            .position(|t| t.id == "x")
            .expect("Strumfield destination") as u8,
        ..Route::default()
    };
    routes
}

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
    fn bipolar_offsets_preserve_bases_sum_before_clamping_and_never_accumulate() {
        let mut e = Engine::default();
        let mut c = Config { transpose: 3, velocity: 0.6, ..Config::default() };
        c.routes[0] = route(10, "transpose");
        c.routes[1] = route(11, "transpose");
        c.routes[2] = route(10, "velocity");
        e.configure(c, &mut |_| {});
        assert_eq!(e.config.transpose, 3);
        assert!((e.config.velocity - 0.6).abs() < 1e-6);
        e.source(9, -0.25, &mut |_| {});
        e.source(10, 0.125, &mut |_| {});
        assert_eq!(e.config.transpose, -3);
        let expected_velocity = TARGETS[target("velocity") as usize].plain(
            TARGETS[target("velocity") as usize].normalized(&c) - 0.25);
        assert!((e.config.velocity - expected_velocity).abs() < 1e-6);
        for _ in 0..20 { e.configure(c, &mut |_| {}); }
        assert_eq!(e.config.transpose, -3);
        assert_eq!(e.host_config.transpose, 3);
        e.source(9, 0.0, &mut |_| {});
        e.source(10, 0.0, &mut |_| {});
        assert_eq!(e.config.transpose, 3);
        assert!((e.config.velocity - 0.6).abs() < 1e-6);
    }

    #[test]
    fn additive_modulators_offset_live_controllers_and_signed_modulators() {
        let mut e = Engine::default();
        let mut c = Config::default();
        c.routes[0] = route(10, "source_0");
        c.routes[1] = route(10, "source_10");
        e.sources[0] = 0.5;
        e.sources[10] = -0.1;
        e.configure(c, &mut |_| {});
        e.source(9, -0.25, &mut |_| {});
        assert_eq!(e.effective_sources()[0], 0.25);
        assert!((e.effective_sources()[10] + 0.35).abs() < 1e-6);
        assert_eq!(e.sources[0], 0.5);
        assert_eq!(e.sources[10], -0.1);
        e.source(0, 0.6, &mut |_| {});
        assert!((e.source_overrides[0].unwrap() - 0.35).abs() < 1e-6);
        c.routes[0].enabled = false;
        e.configure(c, &mut |_| {});
        assert_eq!(e.effective_sources()[0], 0.6);
    }

    #[test]
    fn smoothing_ramps_in_milliseconds_and_advance_matches_ticks() {
        for source in 1..=SOURCE_COUNT {
            let mut c = Config::default();
            c.routes[0] = Route { smoothing_ms: 10.0, ..route(source as u8, "velocity") };
            let mut e = Engine { sample_rate: 1000.0, ..Engine::default() };
            e.configure(c, &mut |_| {});
            let initial = e.config.velocity;
            let value = if source >= 10 { -0.2 } else { 0.75 };
            e.source(source - 1, value, &mut |_| {});
            assert_eq!(e.config.velocity, initial);
            e.advance(5, &mut |_, _| {});
            let midpoint = e.config.velocity;
            let mut reference = Engine { sample_rate: 1000.0, ..Engine::default() };
            reference.configure(c, &mut |_| {});
            reference.source(source - 1, value, &mut |_| {});
            for _ in 0..5 { reference.tick(&mut |_| {}); }
            assert_eq!(midpoint, reference.config.velocity);
            e.advance(5, &mut |_, _| {});
            assert!(!e.smoothing_active());
            let destination = &TARGETS[target("velocity") as usize];
            let norm = if source >= 10 { destination.normalized(&c) + value } else { value };
            let expected = destination.plain(norm);
            assert!((e.config.velocity - expected).abs() < 1e-6, "source {source}");
            assert!((midpoint - (initial + expected) * 0.5).abs() < 1e-5, "source {source}");
            c.routes[0].enabled = false;
            e.configure(c, &mut |_| {});
            assert!(!e.smoothing_active());
            assert_eq!(e.config.velocity, c.velocity);
        }
    }

    #[test]
    fn smoothing_controller_output_is_bounded_and_reaches_the_final_value() {
        let mut e = Engine { sample_rate: 48000.0, ..Engine::default() };
        let mut c = Config { mode: MANUAL, mpe: false, ..Config::default() };
        c.routes[0] = Route { smoothing_ms: 10.0, ..route(10, "source_4") };
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.strike(60, 0.8, 1000, &mut |_| {});
        e.source(9, 1.0, &mut |_| {});
        let mut pressure = Vec::new();
        e.advance(480, &mut |_, out| {
            if let Out::Pressure(_, value) = out { pressure.push(value); }
        });
        assert!(pressure.len() > 2 && pressure.len() <= 12, "{} pressure events", pressure.len());
        assert_eq!(pressure.last(), Some(&1.0));
        assert!(pressure.windows(2).all(|pair| pair[1] >= pair[0]));
    }

    #[test]
    fn routed_velocity_and_axis_sources_affect_performance_without_default_x_route() {
        let mut e = Engine::default();
        let mut c = Config { mode: MANUAL, ..Config::default() };
        c.routes[0] = route(10, "source_6");
        c.routes[1] = route(10, "source_7");
        c.routes[2] = route(10, "source_8");
        e.configure(c, &mut |_| {});
        e.sources[9] = 0.25;
        e.configure(c, &mut |_| {});
        assert!((e.base_strike_velocity() - 1.0).abs() < 0.0001);
        assert_eq!(e.x, 0.25);
        assert_eq!(e.y, 1.0);
        assert_eq!(e.sources[6], 0.8);
    }

    #[test]
    fn routed_midi_modulators_reach_owned_notes_and_restore_live_baselines() {
        let mut e = Engine::default();
        let mut c = Config { mode: MANUAL, mpe: false, ..Config::default() };
        c.routes[0] = route(10, "source_4");
        c.routes[1] = route(10, "source_0");
        c.routes[2] = route(10, "source_1");
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.sources[9] = 0.75;
        e.configure(c, &mut |_| {});
        let mut events = Vec::new();
        e.strike(60, 0.8, 1000, &mut |event| events.push(event));
        assert!(events.iter().any(|event| matches!(event, Out::Pressure(_, value) if *value == 0.75)));
        assert!(events.iter().any(|event| matches!(event, Out::Bend(_, value) if *value == 1.0)));
        assert!(events.iter().any(|event| matches!(event, Out::Cc(_, 1, value) if *value == 0.75)));
        assert_eq!(e.expression.pressure, 0.0);
        events.clear();
        c.routes = [Route::default(); ROUTE_COUNT];
        e.configure(c, &mut |event| events.push(event));
        assert!(events.iter().any(|event| matches!(event, Out::Pressure(_, value) if *value == 0.0)));
        assert!(events.iter().any(|event| matches!(event, Out::Bend(_, value) if *value == 0.5)));
        assert!(events.iter().any(|event| matches!(event, Out::Cc(_, 1, value) if *value == 0.0)));
    }

    #[test]
    fn modulator_chains_resolve_independent_of_route_order_and_preserve_inputs() {
        let mut e = Engine::default();
        let mut c = Config::default();
        c.routes = [Route::default(); ROUTE_COUNT];
        c.routes[0] = route(11, "velocity");
        c.routes[1] = route(10, "source_10");
        c.routes[2] = route(2, "source_9");
        e.configure(c, &mut |_| {});
        e.source(1, 0.7, &mut |_| {});
        assert!(e.effective_sources()[9..].iter().all(|v| (*v - 0.4).abs() < 1e-6));
        assert!((e.config.velocity - 1.0).abs() < 0.0001);
        assert_eq!(e.sources[9..], [0.0, 0.0]);
        assert!((e.snapshot().sources[10] - 0.4).abs() < 1e-6);
        c.routes[2].enabled = false;
        e.configure(c, &mut |_| {});
        assert_eq!(e.effective_sources()[9..], [0.0, 0.0]);
    }

    #[test]
    fn circular_modulator_routes_are_bounded_and_do_not_accumulate() {
        let mut e = Engine::default();
        let mut c = Config::default();
        c.routes[0] = route(10, "source_10");
        c.routes[1] = route(11, "source_9");
        e.sources[9] = 0.25;
        e.sources[10] = 0.75;
        e.configure(c, &mut |_| {});
        let expected = e.effective_sources();
        for _ in 0..100 {
            e.configure(c, &mut |_| {});
            assert_eq!(e.effective_sources(), expected);
        }
        assert_eq!(e.sources[9..], [0.25, 0.75]);
        assert!(!route(10, "source_9").active());
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
            [0.9, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.1, 0.5, 0.0, 0.0]
        );
    }
    #[test]
    fn one_controller_drives_octaves_and_spread_and_disable_restores_base() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: MANUAL,
            octaves: 2,
            spread: 1,
            ..Config::default()
        };
        c.routes[0] = route(2, "octaves");
        c.routes[1] = route(2, "spread");
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.control(0, 1, 1.0, &mut |_| {});
        assert_eq!((e.config.octaves, e.config.spread), (4, 4));
        assert_eq!((e.host_config.octaves, e.host_config.spread), (2, 1));
        c.routes[0].enabled = false;
        c.routes[1].enabled = false;
        e.configure(c, &mut |_| {});
        assert_eq!((e.config.octaves, e.config.spread), (2, 1));
    }
    #[test]
    fn legacy_strings_routes_stay_inert() {
        let mut e = Engine::default();
        let mut c = Config {
            strings: 8,
            ..Config::default()
        };
        c.routes[0] = route(2, "strings");
        assert!(!c.routes[0].active());
        assert!(available_targets().all(|(_, t)| t.id != "strings"));
        e.configure(c, &mut |_| {});
        e.control(0, 1, 1.0, &mut |_| {});
        assert_eq!(e.config.strings, 8);
    }
    #[test]
    fn reversed_and_curved_ranges_keep_endpoints() {
        let r = Route {
            min: 0.8,
            max: 0.2,
            curve: 1.0,
            ..route(1, "octaves")
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
        c.routes[0] = route(2, "octaves");
        c.routes[1] = Route {
            min: 0.0,
            max: 0.0,
            ..route(3, "octaves")
        };
        e.configure(c, &mut |_| {});
        e.control(0, 1, 1.0, &mut |_| {});
        assert_eq!(e.config.octaves, 1);
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
            octaves: 4,
            strum_ms: 0.0,
            strum_sync: false,
            ..Config::default()
        };
        c.routes[0] = route(2, "strings_played");
        e.configure(c, &mut |_| {});
        for (value, count) in [(0.0, 1), (0.5, 7), (1.0, 12)] {
            e.control(0, 1, value, &mut |_| {});
            assert_eq!(e.config.octaves, 4);
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
                octaves: 3,
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
            assert_eq!(ons(&events), vec![67, 70, 75, 79, 82, 87, 91, 94]);
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
    fn strumfield_connection_is_editable_but_direct_input_survives_removal() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: MANUAL,
            ..Config::default()
        };
        c.routes[0].max = 0.5;
        c.routes[0].curve = 1.0;
        e.configure(c, &mut |_| {});
        e.position(0.5, 0, &mut |_| {});
        assert!((e.x - 0.5 * 0.5_f32.powi(8)).abs() < 1e-6);
        assert_eq!(e.sources[7], 0.5);
        e.position(1.0, 0, &mut |_| {});
        assert_eq!(e.x, 0.5);
        c.routes[0].enabled = false;
        e.configure(c, &mut |_| {});
        e.position(0.0, 0, &mut |_| {});
        assert_eq!(e.x, 0.0);
        c.routes[0].source = 0;
        c.routes[0].enabled = true;
        e.configure(c, &mut |_| {});
        e.position(1.0, 0, &mut |_| {});
        assert_eq!(e.x, 1.0);
    }

    #[test]
    fn controllers_cannot_modulate_their_own_input_setup() {
        let mut e = Engine::default();
        let mut c = Config::default();
        for (slot, source, id) in [
            (1, 8, "x_reverse"),
            (2, 9, "y_reverse"),
            (3, 9, "y_target"),
            (4, 9, "y_cc"),
        ] {
            c.routes[slot] = route(source, id);
            assert!(!c.routes[slot].active());
        }
        e.configure(c, &mut |_| {});
        e.position(1.0, 0, &mut |_| {});
        e.position(1.0, 1, &mut |_| {});
        assert_eq!(
            (
                e.config.x_reverse,
                e.config.y_reverse,
                e.config.y_target,
                e.config.y_cc
            ),
            (c.x_reverse, c.y_reverse, c.y_target, c.y_cc)
        );
        assert!(route(8, "x").active());
        assert!(route(2, "x_reverse").active());
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
