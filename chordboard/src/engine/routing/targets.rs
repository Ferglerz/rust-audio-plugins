use super::{Config, Route, ROUTE_COUNT};

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
    pub(super) fn normalized(&self, c: &Config) -> f32 {
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

    pub(super) fn apply(&self, c: &mut Config, norm: f32) {
        (self.set)(c, self.plain(norm));
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
