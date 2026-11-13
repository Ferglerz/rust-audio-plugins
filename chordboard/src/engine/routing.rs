//! Fixed-capacity routing with bounded source-to-source resolution.
use super::{Config, Engine, Out};
pub const LOOP_OCTAVE_IDS: [&str; 4] = [
    "arp_loop_1_octave",
    "arp_loop_2_octave",
    "arp_loop_3_octave",
    "arp_loop_4_octave",
];
pub const ROUTE_COUNT: usize = 16;
pub const SOURCE_COUNT: usize = 14;
pub const ARP_MOD_SOURCES: std::ops::Range<usize> = 9..11;
pub const ARP_POSITION_SOURCE: usize = 11;
pub const PERF_MOD_SOURCES: std::ops::Range<usize> = 12..14;
// Saved route source numbers are one-based. Append new sources at the end.
pub const SOURCES: [&str; SOURCE_COUNT + 1] = [
    "Off",
    "Pitch wheel",
    "Modwheel",
    "Expression",
    "Breath",
    "Pressure",
    "Timbre",
    "Input Vel",
    "Strum X",
    "Strum Y",
    "Tracker Mod 1",
    "Tracker Mod 2",
    "Arp Position",
    "Perf Mod 1",
    "Perf Mod 2",
];
pub const SOURCE_SHORT: [&str; SOURCE_COUNT] = [
    "Pitch",
    "Modwheel",
    "Expr",
    "Breath",
    "Press",
    "Timbre",
    "Input Vel",
    "X",
    "Y",
    "Mod 1",
    "Mod 2",
    "Arp Pos",
    "Perf Mod 1",
    "Perf Mod 2",
];
pub const SOURCE_DEFAULTS: [f32; SOURCE_COUNT] =
    [0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.8, 0.0, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0];

pub fn bipolar_source(index: usize) -> bool {
    ARP_MOD_SOURCES.contains(&index) || PERF_MOD_SOURCES.contains(&index)
}

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
    pub fn additive(self) -> bool {
        self.source
            .checked_sub(1)
            .is_some_and(|index| bipolar_source(index as usize))
    }
    /// Bipolar routes preserve zero and use the contour's span as their depth.
    pub fn output(self, input: f32) -> f32 {
        if self.additive() {
            let input = input.clamp(-1.0, 1.0);
            input.signum()
                * (self.max - self.min)
                * input
                    .abs()
                    .powf(2.0_f32.powf(self.curve.clamp(-1.0, 1.0) * 3.0))
        } else {
            self.value(input)
        }
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
#[derive(Clone, Copy)]
enum Format {
    Plain,
    Names(&'static [&'static str]),
    OnOff,
    Ms,
    SignedOct,
    SignedSt,
    St,
    OneBased,
    SignedPercentDiv75,
    Percent,
}

const Y_TARGET_NAMES: &[&str] = &[
    "Velocity",
    "Gate",
    "Pressure",
    "Timbre",
    "Bend",
    "Custom CC",
];
const FILTER_NAMES: &[&str] = &[
    "All",
    "Bass",
    "Top",
    "Bass + Top",
    "Odd tones",
    "Even tones",
];

pub struct Target {
    pub name: &'static str,
    pub id: &'static str,
    pub min: f32,
    pub max: f32,
    pub discrete: bool,
    get: Option<fn(&Config) -> f32>,
    set: fn(&mut Config, f32),
    format: Format,
}
fn noop_set(_: &mut Config, _: f32) {}
// Indices are saved in host state. Append destinations; never reorder this list.
pub const TARGETS: &[Target] = &[
    Target {
        name: "Strings",
        id: "strings",
        min: 3.0,
        max: 12.0,
        discrete: true,
        get: Some(|c| c.strings as f32),
        set: |c, v| c.strings = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Voicing",
        id: "spread",
        min: 0.0,
        max: (crate::harmony::VOICING_NAMES.len() - 1) as f32,
        discrete: true,
        get: Some(|c| c.spread as f32),
        set: |c, v| c.spread = v as u8,
        format: Format::Names(&crate::harmony::VOICING_NAMES),
    },
    Target {
        name: "Quality",
        id: "quality",
        min: 0.0,
        max: 11.0,
        discrete: true,
        get: Some(|c| c.quality as f32),
        set: |c, v| c.quality = v as u8,
        format: Format::Names(&crate::harmony::QUALITY_NAMES),
    },
    Target {
        name: "Inversion",
        id: "inversion",
        min: 0.0,
        max: 11.0,
        discrete: true,
        get: Some(|c| c.inversion as f32),
        set: |c, v| c.inversion = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Transpose",
        id: "transpose",
        min: -24.0,
        max: 24.0,
        discrete: true,
        get: Some(|c| c.transpose as f32),
        set: |c, v| c.transpose = v as i8,
        format: Format::SignedSt,
    },
    Target {
        name: "Velocity",
        id: "velocity",
        min: 0.01,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.velocity),
        set: |c, v| c.velocity = v,
        format: Format::Percent,
    },
    Target {
        name: "Note length",
        id: "length_ms",
        min: 20.0,
        max: 3000.0,
        discrete: false,
        get: Some(|c| c.length_ms),
        set: |c, v| c.length_ms = v,
        format: Format::Ms,
    },
    Target {
        name: "Rate (free)",
        id: "strum_ms",
        min: 0.0,
        max: 1500.0,
        discrete: false,
        get: Some(|c| c.strum_ms),
        set: |c, v| c.strum_ms = v,
        format: Format::Ms,
    },
    Target {
        name: "Rate tempo sync",
        id: "strum_sync",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.strum_sync as u8 as f32),
        set: |c, v| c.strum_sync = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "Sweep beats",
        id: "strum_beats",
        min: 0.0625,
        max: 2.0,
        discrete: false,
        get: Some(|c| c.strum_beats),
        set: |c, v| {
            c.strum_beats = v;
            c.rate = v;
        },
        format: Format::Plain,
    },
    Target {
        name: "Direction",
        id: "direction",
        min: 0.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.direction as f32),
        set: |c, v| {
            c.direction = v as u8;
            c.arp_pattern = v as u8;
        },
        format: Format::Names(&crate::arpeggiator::STRUM_DIRECTION_NAMES),
    },
    Target {
        name: "Velocity contour",
        id: "contour",
        min: -1.0,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.contour),
        set: |c, v| c.contour = v,
        format: Format::Percent,
    },
    Target {
        name: "Arp pattern",
        id: "arp_pattern",
        min: 0.0,
        max: 6.0,
        discrete: true,
        get: Some(|c| c.arp_pattern as f32),
        set: |c, v| c.arp_pattern = v as u8,
        format: Format::Names(&crate::arpeggiator::PATTERN_NAMES),
    },
    Target {
        name: "Rate (synced)",
        id: "rate",
        min: 0.0625,
        max: 2.0,
        discrete: false,
        get: Some(|c| c.rate),
        set: |c, v| c.rate = v,
        format: Format::Plain,
    },
    Target {
        name: "Gate",
        id: "gate",
        min: 0.05,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.gate),
        set: |c, v| c.gate = v,
        format: Format::Percent,
    },
    Target {
        name: "Swing",
        id: "swing",
        min: -0.75,
        max: 0.75,
        discrete: false,
        get: Some(|c| c.swing),
        set: |c, v| c.swing = v,
        format: Format::SignedPercentDiv75,
    },
    Target {
        name: "Octaves",
        id: "octaves",
        min: 1.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.octaves as f32),
        set: |c, v| c.octaves = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Humanize",
        id: "humanize",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.humanize),
        set: |c, v| c.humanize = v,
        format: Format::Percent,
    },
    Target {
        name: "Root on select",
        id: "root_on_select",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.root_on_select as u8 as f32),
        set: |c, v| c.root_on_select = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "Latch",
        id: "latch",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.latch as u8 as f32),
        set: |c, v| c.latch = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "Auto strum hold",
        id: "strum_hold",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.strum_hold as u8 as f32),
        set: |c, v| c.strum_hold = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "Note filter",
        id: "filter",
        min: 0.0,
        max: 5.0,
        discrete: true,
        get: Some(|c| c.filter as f32),
        set: |c, v| c.filter = v as u8,
        format: Format::Names(FILTER_NAMES),
    },
    Target {
        name: "Arp repeat",
        id: "arp_mode",
        min: 0.0,
        max: 3.0,
        discrete: true,
        get: Some(|c| c.arp_mode as u8 as f32),
        set: |c, v| c.arp_mode = crate::arpeggiator::ArpMode::from_index(v as u8),
        format: Format::Names(&crate::arpeggiator::MODE_NAMES),
    },
    Target {
        name: "Y destination",
        id: "y_target",
        min: 0.0,
        max: 5.0,
        discrete: true,
        get: Some(|c| c.y_target as f32),
        set: |c, v| c.y_target = v as u8,
        format: Format::Names(Y_TARGET_NAMES),
    },
    Target {
        name: "Y CC",
        id: "y_cc",
        min: 0.0,
        max: 119.0,
        discrete: true,
        get: Some(|c| c.y_cc as f32),
        set: |c, v| c.y_cc = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Reverse X",
        id: "x_reverse",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.x_reverse as u8 as f32),
        set: |c, v| c.x_reverse = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "Reverse Y",
        id: "y_reverse",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.y_reverse as u8 as f32),
        set: |c, v| c.y_reverse = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "X minimum",
        id: "x_min",
        min: 0.0,
        max: 0.99,
        discrete: false,
        get: Some(|c| c.x_min),
        set: |c, v| c.x_min = v,
        format: Format::Plain,
    },
    Target {
        name: "X maximum",
        id: "x_max",
        min: 0.01,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.x_max),
        set: |c, v| c.x_max = v,
        format: Format::Plain,
    },
    Target {
        name: "Y minimum",
        id: "y_min",
        min: 0.0,
        max: 0.99,
        discrete: false,
        get: Some(|c| c.y_min),
        set: |c, v| c.y_min = v,
        format: Format::Plain,
    },
    Target {
        name: "Y maximum",
        id: "y_max",
        min: 0.01,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.y_max),
        set: |c, v| c.y_max = v,
        format: Format::Plain,
    },
    Target {
        name: "MPE members",
        id: "members",
        min: 1.0,
        max: 15.0,
        discrete: true,
        get: Some(|c| c.members as f32),
        set: |c, v| c.members = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Member bend",
        id: "bend_range",
        min: 1.0,
        max: 96.0,
        discrete: false,
        get: Some(|c| c.bend_range),
        set: |c, v| c.bend_range = v,
        format: Format::St,
    },
    Target {
        name: "Master bend",
        id: "master_range",
        min: 1.0,
        max: 24.0,
        discrete: false,
        get: Some(|c| c.master_range),
        set: |c, v| c.master_range = v,
        format: Format::St,
    },
    Target {
        name: "Output channel",
        id: "output_channel",
        min: 0.0,
        max: 15.0,
        discrete: true,
        get: Some(|c| c.output_channel as f32),
        set: |c, v| c.output_channel = v as u8,
        format: Format::OneBased,
    },
    Target {
        name: "Bass channel",
        id: "bass_channel",
        min: 0.0,
        max: 15.0,
        discrete: true,
        get: Some(|c| c.bass_channel as f32),
        set: |c, v| c.bass_channel = v as u8,
        format: Format::OneBased,
    },
    Target {
        name: "Upper channel",
        id: "upper_channel",
        min: 0.0,
        max: 15.0,
        discrete: true,
        get: Some(|c| c.upper_channel as f32),
        set: |c, v| c.upper_channel = v as u8,
        format: Format::OneBased,
    },
    Target {
        name: "Strings played",
        id: "strings_played",
        min: 1.0,
        max: 12.0,
        discrete: true,
        get: Some(|c| c.strings_played as f32),
        set: |c, v| c.strings_played = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Strumfield position",
        id: "x",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: Some(|c| c.routed_x.unwrap_or(0.0)),
        set: |c, v| c.routed_x = Some(v),
        format: Format::Plain,
    },
    Target {
        name: "Arp page",
        id: "arp_page",
        min: 0.0,
        max: 7.0,
        discrete: true,
        get: Some(|c| c.arp_page as f32),
        set: |c, v| c.arp_page = v as u8,
        format: Format::OneBased,
    },
    Target {
        name: "Pitch wheel",
        id: "source_0",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Modwheel",
        id: "source_1",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Expression",
        id: "source_2",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Breath",
        id: "source_3",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Pressure",
        id: "source_4",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Timbre",
        id: "source_5",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Input velocity",
        id: "source_6",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Strum X",
        id: "source_7",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Strum Y",
        id: "source_8",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Tracker Mod 1",
        id: "source_9",
        min: -1.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Tracker Mod 2",
        id: "source_10",
        min: -1.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Arp position",
        id: "source_11",
        min: 0.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Arp loop length",
        id: "arp_octaves",
        min: 1.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.arp_octaves as f32),
        set: |c, v| c.arp_octaves = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Arp sync",
        id: "arp_sync",
        min: 0.0,
        max: 1.0,
        discrete: true,
        get: Some(|c| c.arp_sync as u8 as f32),
        set: |c, v| c.arp_sync = v >= 0.5,
        format: Format::OnOff,
    },
    Target {
        name: "Arp rate (free)",
        id: "arp_free_ms",
        min: 1.0,
        max: 1500.0,
        discrete: false,
        get: Some(|c| c.arp_free_ms),
        set: |c, v| c.arp_free_ms = v,
        format: Format::Ms,
    },
    Target {
        name: "Arp loop 1 octave",
        id: "arp_loop_1_octave",
        min: -4.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.arp_loop_octaves[0] as f32),
        set: |c, v| c.arp_loop_octaves[0] = v as i8,
        format: Format::SignedOct,
    },
    Target {
        name: "Arp loop 2 octave",
        id: "arp_loop_2_octave",
        min: -4.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.arp_loop_octaves[1] as f32),
        set: |c, v| c.arp_loop_octaves[1] = v as i8,
        format: Format::SignedOct,
    },
    Target {
        name: "Arp loop 3 octave",
        id: "arp_loop_3_octave",
        min: -4.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.arp_loop_octaves[2] as f32),
        set: |c, v| c.arp_loop_octaves[2] = v as i8,
        format: Format::SignedOct,
    },
    Target {
        name: "Arp loop 4 octave",
        id: "arp_loop_4_octave",
        min: -4.0,
        max: 4.0,
        discrete: true,
        get: Some(|c| c.arp_loop_octaves[3] as f32),
        set: |c, v| c.arp_loop_octaves[3] = v as i8,
        format: Format::SignedOct,
    },
    Target {
        name: "Base inversion",
        id: "base_inversion",
        min: -12.0,
        max: 12.0,
        discrete: true,
        get: Some(|c| c.base_inversion as f32),
        set: |c, v| c.base_inversion = v as i8,
        format: Format::Plain,
    },
    Target {
        name: "Complexity",
        id: "complexity",
        min: 0.0,
        max: 5.0,
        discrete: true,
        get: Some(|c| c.complexity as f32),
        set: |c, v| c.complexity = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Global voicing",
        id: "voicing_override",
        min: 0.0,
        max: 5.0,
        discrete: true,
        get: Some(|c| c.voicing_override as f32),
        set: |c, v| c.voicing_override = v as u8,
        format: Format::Plain,
    },
    Target {
        name: "Gate scale",
        id: "gate_scale",
        min: 0.01,
        max: 2.0,
        discrete: false,
        get: Some(|c| c.gate_scale),
        set: |c, v| c.gate_scale = v,
        format: Format::Percent,
    },
    Target {
        name: "Velocity scale",
        id: "velocity_scale",
        min: 0.0,
        max: 2.0,
        discrete: false,
        get: Some(|c| c.velocity_scale),
        set: |c, v| c.velocity_scale = v,
        format: Format::Percent,
    },
    Target {
        name: "Perf Mod 1",
        id: "source_12",
        min: -1.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
    Target {
        name: "Perf Mod 2",
        id: "source_13",
        min: -1.0,
        max: 1.0,
        discrete: false,
        get: None,
        set: noop_set,
        format: Format::Plain,
    },
];
pub fn maximum_for_routes(id: &str, base: f32, routes: &[Route]) -> f32 {
    let Some((index, target)) = TARGETS.iter().enumerate().find(|(_, t)| t.id == id) else {
        return base;
    };
    let mut maximum = (base - target.min) / (target.max - target.min);
    for route in routes
        .iter()
        .filter(|r| r.active() && r.target as usize == index && !r.additive())
    {
        maximum = route.min.max(route.max);
    }
    for route in routes
        .iter()
        .filter(|r| r.active() && r.target as usize == index && r.additive())
    {
        maximum += (route.max - route.min).abs();
    }
    target.plain(maximum.clamp(0.0, 1.0)).max(base)
}
impl Target {
    fn normalized(&self, c: &Config) -> f32 {
        let Some(get) = self.get else {
            return 0.0;
        };
        let v = get(c);
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
                | "arp_pattern"
                | "rate"
                | "gate"
                | "arp_page"
                | "arp_octaves"
                | "arp_sync"
                | "arp_free_ms"
                | "arp_loop_1_octave"
                | "arp_loop_2_octave"
                | "arp_loop_3_octave"
                | "arp_loop_4_octave"
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
        match self.format {
            Format::Names(names) => names[v as usize].into(),
            Format::OnOff => if v >= 0.5 { "On" } else { "Off" }.into(),
            Format::Ms => format!("{v:.0} ms"),
            Format::SignedOct => format!("{v:+.0} oct"),
            Format::SignedSt => format!("{v:+.0} st"),
            Format::St => format!("{v:.0} st"),
            Format::OneBased => format!("{:.0}", v + 1.0),
            Format::SignedPercentDiv75 => format!("{:+.0}%", v / 0.75 * 100.0),
            Format::Percent => format!("{:.0}%", v * 100.0),
            Format::Plain if self.discrete => format!("{v:.0}"),
            Format::Plain => format!("{v:.2}"),
        }
    }
    fn apply(&self, c: &mut Config, norm: f32) {
        (self.set)(c, self.plain(norm));
    }
}
impl Engine {
    fn route_output(&self, slot: usize, input: f32) -> f32 {
        let route = self.host_config.routes[slot];
        let smoother = self.route_smoothers[slot];
        if route.smoothing_ms > 0.0
            && smoother
                .route
                .is_some_and(|old| old.source == route.source && old.target == route.target)
        {
            smoother.value
        } else {
            route.output(input)
        }
    }
    pub(super) fn prepare_route_smoothing(&mut self) {
        let sources = self.effective_sources();
        for (slot, route) in self.host_config.routes.iter().copied().enumerate() {
            let smoother = &mut self.route_smoothers[slot];
            if !route.active() {
                *smoother = RouteSmoother::default();
                continue;
            }
            let target = route.output(sources[route.source as usize - 1]);
            let fresh = smoother
                .route
                .is_none_or(|old| old.source != route.source || old.target != route.target);
            let time_changed = smoother
                .route
                .is_some_and(|old| old.smoothing_ms != route.smoothing_ms);
            smoother.route = Some(route);
            if fresh || route.smoothing_ms <= 0.0 {
                smoother.value = target;
                smoother.target = target;
                smoother.remaining = 0;
            } else if smoother.target != target || time_changed {
                smoother.target = target;
                smoother.remaining = (route.smoothing_ms.clamp(0.0, 2000.0) * self.sample_rate
                    / 1000.0)
                    .round()
                    .max(1.0) as u64;
                smoother.increment = (target - smoother.value) / smoother.remaining as f32;
            }
        }
    }
    pub(super) fn smoothing_active(&self) -> bool {
        self.route_smoothers.iter().any(|s| s.remaining > 0)
    }
    pub(super) fn tick_route_smoothing(&mut self, out: &mut impl FnMut(Out)) {
        if !self.smoothing_active() {
            return;
        }
        for smoother in &mut self.route_smoothers {
            if smoother.remaining > 0 {
                smoother.remaining -= 1;
                smoother.value = if smoother.remaining == 0 {
                    smoother.target
                } else {
                    smoother.value + smoother.increment
                };
            }
        }
        // Advance ramps per sample, but refresh MIDI/parameter destinations at
        // most once per millisecond so smoothing cannot flood the host queue.
        let interval = (self.sample_rate / 1000.0).round().max(1.0) as u64;
        if self.now.is_multiple_of(interval) || !self.smoothing_active() {
            self.apply_smoothed_targets(out);
        }
    }
    /// Push smoothed route values into the live config. Structural changes still
    /// take the full configure path so voice and harmony resets stay intact.
    pub(super) fn apply_smoothed_targets(&mut self, out: &mut impl FnMut(Out)) {
        let old = self.config;
        let previous_sources = self.source_overrides;
        let mut config = self.host_config;
        for page in 0..8 {
            if let Some(settings) = self.arp.paste_settings[page] {
                if config.arp_settings[page] != settings {
                    config.arp_settings[page] = settings;
                }
            }
        }
        let initialized = self.overlay_arp_settings(&mut config);
        self.apply_routes(&mut config);
        config.arp_octaves = config.arp_octaves.clamp(1, initialized);
        if smoothed_needs_configure(&old, &config) {
            self.configure(self.host_config, out);
            return;
        }
        self.config = config;
        self.source_overrides = self.resolved_source_overrides();
        self.publish_source_overrides(previous_sources, out);
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
            if let Some(value) = self.source_overrides[index] {
                out(Out::Cc(channel, cc, value));
            }
        }
    }
    /// Resolve each input once. A back edge reads the original input, so cycles
    /// stay deterministic and bounded instead of accumulating across samples.
    pub fn effective_sources(&self) -> [f32; SOURCE_COUNT] {
        fn resolve(
            engine: &Engine,
            index: usize,
            values: &mut [f32; SOURCE_COUNT],
            marks: &mut [u8; SOURCE_COUNT],
        ) -> f32 {
            if marks[index] == 2 {
                return values[index];
            }
            if marks[index] == 1 {
                return engine.sources[index];
            }
            marks[index] = 1;
            if let Some((slot, route)) =
                engine
                    .host_config
                    .routes
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, route)| {
                        route.active()
                            && !route.additive()
                            && TARGETS[route.target as usize].source_index() == Some(index)
                    })
            {
                let input = resolve(engine, route.source as usize - 1, values, marks);
                values[index] =
                    TARGETS[route.target as usize].plain(engine.route_output(slot, input));
            }
            let mut value = values[index];
            for (slot, route) in engine.host_config.routes.iter().enumerate() {
                if route.active()
                    && route.additive()
                    && TARGETS[route.target as usize].source_index() == Some(index)
                {
                    let input = resolve(engine, route.source as usize - 1, values, marks);
                    value += engine.route_output(slot, input);
                }
            }
            values[index] = value.clamp(if bipolar_source(index) { -1.0 } else { 0.0 }, 1.0);
            marks[index] = 2;
            values[index]
        }
        let mut values = self.sources;
        let mut marks = [0; SOURCE_COUNT];
        for index in 0..SOURCE_COUNT {
            resolve(self, index, &mut values, &mut marks);
        }
        values
    }
    pub(super) fn apply_routes(&self, config: &mut Config) {
        let sources = self.effective_sources();
        let mut values = std::array::from_fn::<_, TARGET_COUNT, _>(|i| {
            if TARGETS[i].id == "x" && config.routed_x.is_none() {
                sources[7]
            } else {
                TARGETS[i].normalized(config)
            }
        });
        let mut touched = [false; TARGET_COUNT];
        for additive in [false, true] {
            for (slot, route) in self.host_config.routes.iter().enumerate() {
                if !route.active() || route.additive() != additive {
                    continue;
                }
                let index = route.target as usize;
                if TARGETS[index].source_index().is_some() {
                    continue;
                }
                let output = self.route_output(slot, sources[route.source as usize - 1]);
                if additive {
                    values[index] += output;
                } else {
                    values[index] = output;
                }
                touched[index] = true;
            }
        }
        for (index, target) in TARGETS.iter().enumerate() {
            if touched[index] {
                target.apply(config, values[index].clamp(0.0, 1.0));
            }
        }
    }
    pub(super) fn source(&mut self, index: usize, value: f32, out: &mut impl FnMut(Out)) {
        if !value.is_finite() || index >= SOURCE_COUNT {
            return;
        }
        let value = value.clamp(if bipolar_source(index) { -1.0 } else { 0.0 }, 1.0);
        if self.sources[index] == value {
            return;
        }
        self.sources[index] = value;
        if self.host_config.routes.iter().any(|r| {
            r.active()
                && (r.source as usize == index + 1
                    || TARGETS[r.target as usize].source_index() == Some(index))
        }) {
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
                    ((sources[source] - TARGETS[index].min)
                        / (TARGETS[index].max - TARGETS[index].min))
                        .clamp(0.0, 1.0)
                } else {
                    TARGETS[index].normalized(&self.config)
                });
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
    fn swing_and_humanize_are_routable_on_both_arpeggiators() {
        let mut e = Engine::default();
        let mut config = Config {
            tracker_enabled: true,
            arp_enabled: true,
            ..Config::default()
        };
        config.routes[0] = route(2, "swing");
        config.routes[1] = route(2, "humanize");
        e.configure(config, &mut |_| {});
        e.source(1, 1.0, &mut |_| {});
        assert_eq!((e.config.swing, e.config.humanize), (0.75, 1.0));
        e.with_performance_arp(|e| {
            assert_eq!((e.config.swing, e.config.humanize), (0.75, 1.0));
        });
    }

    #[test]
    fn bipolar_offsets_preserve_bases_sum_before_clamping_and_never_accumulate() {
        let mut e = Engine::default();
        let mut c = Config {
            transpose: 3,
            velocity: 0.6,
            ..Config::default()
        };
        c.routes[0] = route(10, "transpose");
        c.routes[1] = route(11, "transpose");
        c.routes[2] = route(10, "velocity");
        e.configure(c, &mut |_| {});
        assert_eq!(e.config.transpose, 3);
        assert!((e.config.velocity - 0.6).abs() < 1e-6);
        e.source(9, -0.25, &mut |_| {});
        e.source(10, 0.125, &mut |_| {});
        assert_eq!(e.config.transpose, -3);
        let expected_velocity = TARGETS[target("velocity") as usize]
            .plain(TARGETS[target("velocity") as usize].normalized(&c) - 0.25);
        assert!((e.config.velocity - expected_velocity).abs() < 1e-6);
        for _ in 0..20 {
            e.configure(c, &mut |_| {});
        }
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
            c.routes[0] = Route {
                smoothing_ms: 10.0,
                ..route(source as u8, "velocity")
            };
            let mut e = Engine {
                sample_rate: 1000.0,
                ..Engine::default()
            };
            e.configure(c, &mut |_| {});
            let initial = e.config.velocity;
            let value = if bipolar_source(source - 1) {
                -0.2
            } else {
                0.75
            };
            e.source(source - 1, value, &mut |_| {});
            assert_eq!(e.config.velocity, initial);
            e.advance(5, &mut |_, _| {});
            let midpoint = e.config.velocity;
            let mut reference = Engine {
                sample_rate: 1000.0,
                ..Engine::default()
            };
            reference.configure(c, &mut |_| {});
            reference.source(source - 1, value, &mut |_| {});
            for _ in 0..5 {
                reference.tick(&mut |_| {});
            }
            assert_eq!(midpoint, reference.config.velocity);
            e.advance(5, &mut |_, _| {});
            assert!(!e.smoothing_active());
            let destination = &TARGETS[target("velocity") as usize];
            let norm = if bipolar_source(source - 1) {
                destination.normalized(&c) + value
            } else {
                value
            };
            let expected = destination.plain(norm);
            assert!(
                (e.config.velocity - expected).abs() < 1e-6,
                "source {source}"
            );
            assert!(
                (midpoint - (initial + expected) * 0.5).abs() < 1e-5,
                "source {source}"
            );
            c.routes[0].enabled = false;
            e.configure(c, &mut |_| {});
            assert!(!e.smoothing_active());
            assert_eq!(e.config.velocity, c.velocity);
        }
    }

    #[test]
    fn smoothing_controller_output_is_bounded_and_reaches_the_final_value() {
        let mut e = Engine {
            sample_rate: 48000.0,
            ..Engine::default()
        };
        let mut c = Config {
            mode: MANUAL,
            mpe: false,
            ..Config::default()
        };
        c.routes[0] = Route {
            smoothing_ms: 10.0,
            ..route(10, "source_4")
        };
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.strike(60, 0.8, 1000, &mut |_| {});
        e.source(9, 1.0, &mut |_| {});
        let mut pressure = Vec::new();
        e.advance(480, &mut |_, out| {
            if let Out::Pressure(_, value) = out {
                pressure.push(value);
            }
        });
        assert!(
            pressure.len() > 2 && pressure.len() <= 12,
            "{} pressure events",
            pressure.len()
        );
        assert_eq!(pressure.last(), Some(&1.0));
        assert!(pressure.windows(2).all(|pair| pair[1] >= pair[0]));
    }

    #[test]
    fn routed_velocity_and_axis_sources_affect_performance_without_default_x_route() {
        let mut e = Engine::default();
        let mut c = Config {
            mode: MANUAL,
            ..Config::default()
        };
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
        let mut c = Config {
            mode: MANUAL,
            mpe: false,
            ..Config::default()
        };
        c.routes[0] = route(10, "source_4");
        c.routes[1] = route(10, "source_0");
        c.routes[2] = route(10, "source_1");
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 60, 0.8, &mut |_| {});
        e.sources[9] = 0.75;
        e.configure(c, &mut |_| {});
        let mut events = Vec::new();
        e.strike(60, 0.8, 1000, &mut |event| events.push(event));
        assert!(events
            .iter()
            .any(|event| matches!(event, Out::Pressure(_, value) if *value == 0.75)));
        assert!(events
            .iter()
            .any(|event| matches!(event, Out::Bend(_, value) if *value == 1.0)));
        assert!(events
            .iter()
            .any(|event| matches!(event, Out::Cc(_, 1, value) if *value == 0.75)));
        assert_eq!(e.expression.pressure, 0.0);
        events.clear();
        c.routes = [Route::default(); ROUTE_COUNT];
        e.configure(c, &mut |event| events.push(event));
        assert!(events
            .iter()
            .any(|event| matches!(event, Out::Pressure(_, value) if *value == 0.0)));
        assert!(events
            .iter()
            .any(|event| matches!(event, Out::Bend(_, value) if *value == 0.5)));
        assert!(events
            .iter()
            .any(|event| matches!(event, Out::Cc(_, 1, value) if *value == 0.0)));
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
        assert!(e.effective_sources()[9..11]
            .iter()
            .all(|v| (*v - 0.4).abs() < 1e-6));
        assert!((e.config.velocity - 1.0).abs() < 0.0001);
        assert_eq!(e.sources[9..11], [0.0, 0.0]);
        assert!((e.snapshot().sources[10] - 0.4).abs() < 1e-6);
        c.routes[2].enabled = false;
        e.configure(c, &mut |_| {});
        assert_eq!(e.effective_sources()[9..11], [0.0, 0.0]);
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
        assert_eq!(e.sources[9..11], [0.25, 0.75]);
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
            [0.9, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.1, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
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

    #[test]
    fn every_target_round_trips_apply_normalized_and_formats_labels() {
        for target in TARGETS {
            for norm in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                let label = target.label(norm);
                assert!(!label.is_empty(), "empty label for {}", target.id);
                let mut config = Config::default();
                target.apply(&mut config, norm);
                if target.source_index().is_some() {
                    assert_eq!(target.normalized(&config), 0.0);
                    continue;
                }
                let expected = ((target.plain(norm) - target.min) / (target.max - target.min))
                    .clamp(0.0, 1.0);
                let got = target.normalized(&config);
                assert!(
                    (got - expected).abs() < 1.0e-5,
                    "round-trip mismatch for {} at norm={norm}: got {got}, expected {expected}",
                    target.id
                );
                if target.id == "strum_beats" {
                    assert_eq!(config.rate, target.plain(norm));
                }
                if target.id == "direction" {
                    assert_eq!(config.arp_pattern, target.plain(norm) as u8);
                }
            }
        }
    }
}

fn smoothed_needs_configure(old: &Config, new: &Config) -> bool {
    fn structure(config: Config) -> Config {
        Config {
            velocity: 0.0,
            velocity_scale: 0.0,
            gate_scale: 0.0,
            length_ms: 0.0,
            gliss_ms: 0.0,
            strum_ms: 0.0,
            strum_beats: 0.0,
            contour: 0.0,
            contour_curve: 0.0,
            rate: 0.0,
            gate: 0.0,
            swing: 0.0,
            humanize: 0.0,
            arp_free_ms: 0.0,
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            ..config
        }
    }
    structure(*old) != structure(*new)
}
