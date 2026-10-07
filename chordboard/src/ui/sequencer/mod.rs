//! Pattern editor. All drawing and hit testing share the same control geometry.
use super::*;
use crate::sequencer::{Page, State, Step};

const NUMBER_HOLD: f32 = 0.5;
const NUMBER_FADE: f32 = 0.35;

const VISIBLE_LANES: [usize; 3] = [0, 1, 3];
const LANES: [&str; 4] = ["Arp", "Chords", "Bass", "Harmony"];
fn number_alpha(age: f32) -> f32 {
    (1.0 - (age - NUMBER_HOLD).max(0.0) / NUMBER_FADE).clamp(0.0, 1.0)
}
fn lane_color(lane: usize) -> Color {
    match lane {
        0 => TEAL,
        2 => GOLD,
        _ => COLORS[1],
    }
}
const TABS: [&str; 2] = ["Step", "Pattern"];

pub(super) struct SequencerUi {
    pub progress: f32,
    pub edit_pages: [usize; 4],
    page_positions: [f32; 4],
    page_starts: [f32; 4],
    page_elapsed: [f32; 4],
    number_age: [[f32; 32]; 4],
    pub(super) open: bool,
    start: f32,
    elapsed: f32,
    lane: usize,
    step: usize,
    arp_step: usize,
    pub(super) inspector: bool,
    tab: usize,
    box_drag: Option<(usize, usize, usize)>,
    edit: Option<ValueEdit<Field>>,
    modifier: Option<Field>,
    slider_drag: Option<(usize, usize, usize, Field)>,
    length_drag: Option<usize>,
    loop_anchor: Option<usize>,
}
impl Default for SequencerUi {
    fn default() -> Self {
        Self {
            progress: 0.0,
            edit_pages: [0; 4],
            page_positions: [0.0; 4],
            page_starts: [0.0; 4],
            page_elapsed: [pleasant_ui::page_slide::DURATION; 4],
            number_age: [[NUMBER_HOLD + NUMBER_FADE; 32]; 4],
            open: false,
            start: 0.0,
            elapsed: pleasant_ui::page_slide::DURATION,
            lane: 0,
            step: 0,
            arp_step: 0,
            inspector: false,
            tab: 0,
            box_drag: None,
            edit: None,
            modifier: None,
            slider_drag: None,
            length_drag: None,
            loop_anchor: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Field {
    Span,
    Start,
    Rate,
    Interlock,
    Velocity,
    Gate,
    Octave,
    Tone,
    Probability,
    Ratchets,
    Micro,
    Root,
    Quality,
    Inversion,
    Spread,
    Pressure,
    Timbre,
    Bend,
    Cc(usize),
}
#[derive(Clone, Copy)]
enum Action {
    Tab(usize),
    Modifier(Option<Field>),
    EditPage(usize, usize),
    PlayPage(usize),
    SelectStep(usize, usize),
    LengthLock,
    LoopAll,
    Field(Field),
    Choice(Field),
    Enabled,
    DeleteBox,
    Tie,
    LaneEnabled(usize),
    Ghosts,
}
struct Item {
    rect: Rect,
    label: String,
    action: Action,
    on: bool,
}
fn item(items: &mut Vec<Item>, rect: Rect, label: impl Into<String>, action: Action, on: bool) {
    items.push(Item {
        rect,
        label: label.into(),
        action,
        on,
    });
}
fn surface() -> Rect {
    PIANO_SURFACE
}
fn lane_y(lane: usize) -> f32 {
    let row = VISIBLE_LANES.iter().position(|&l| l == lane).unwrap_or(0);
    surface().1 + 42.0 + row as f32 * 52.5
}
fn length_rect(lane: usize) -> Rect {
    let first = step_rect(lane, 0);
    let last = step_rect(lane, 31);
    (
        first.0,
        first.1 + first.3 + 3.0,
        last.0 + last.2 - first.0,
        12.0,
    )
}
fn page_row_rect(lane: usize) -> Rect {
    (
        surface().0 + 92.0,
        lane_y(lane),
        7.0 * 26.0 + 23.0,
        27.0,
    )
}
fn step_rect(lane: usize, visible: usize) -> Rect {
    let left = surface().0 + 316.0;
    let width = (surface().2 - 332.0 - 31.0 * 3.0) / 32.0;
    (
        left + visible as f32 * (width + 3.0),
        lane_y(lane),
        width,
        width,
    )
}
fn header_rect(x: f32, width: f32) -> Rect {
    (surface().0 + x, surface().1 + 10.0, width, 24.0)
}
fn lock_lengths_rect() -> Rect {
    header_rect(114.0, 104.0)
}
fn loop_all_rect() -> Rect {
    header_rect(224.0, 76.0)
}
fn loop_step_rect(visible: usize) -> Rect {
    let step = step_rect(0, visible);
    (step.0, surface().1 + 10.0, step.2, 24.0)
}
fn loop_strip_rect() -> Rect {
    let first = loop_step_rect(0);
    let last = loop_step_rect(31);
    (first.0, first.1, last.0 + last.2 - first.0, first.3)
}
fn first_step() -> usize {
    0
}
fn visible_steps() -> usize {
    32
}
fn step_at(lane: usize, x: f32, _y: f32) -> usize {
    let r = step_rect(lane, 0);
    ((x - r.0) / (r.2 + 3.0)).floor().clamp(0.0, 31.0) as usize
}
fn span_rects(start: usize, end: usize) -> Vec<Rect> {
    let end = end.min(visible_steps());
    if start >= end {
        return Vec::new();
    }
    let a = step_rect(3, start);
    let b = step_rect(3, end - 1);
    vec![(a.0, a.1, b.0 + b.2 - a.0, a.3)]
}
fn inspector_rect() -> Rect {
    ARP_EDITOR
}
const MODIFIERS: [(&str, Option<Field>); 6] = [
    ("Steps", None),
    ("Gate", Some(Field::Gate)),
    ("Chance", Some(Field::Probability)),
    ("Octave", Some(Field::Octave)),
    ("Timing", Some(Field::Micro)),
    ("Repeats", Some(Field::Ratchets)),
];
const EXPRESSION_MODES: [(usize, &str, Field); 4] = [
    (0, "Bend", Field::Bend),
    (4, "Pressure", Field::Pressure),
    (5, "Timbre", Field::Timbre),
    (6, "Velocity", Field::Velocity),
];
fn is_step_value(field: Field) -> bool {
    matches!(
        field,
        Field::Velocity
            | Field::Gate
            | Field::Probability
            | Field::Octave
            | Field::Tone
            | Field::Micro
            | Field::Ratchets
            | Field::Pressure
            | Field::Timbre
            | Field::Bend
            | Field::Cc(_)
    )
}
fn slider_value(field: Field, step: &Step) -> f32 {
    match field {
        Field::Velocity => step.velocity as f32,
        Field::Gate => step.gate as f32,
        Field::Probability => step.probability as f32,
        Field::Octave => step.octave as f32,
        Field::Tone => step.tone as f32,
        Field::Micro => step.micro as f32,
        Field::Ratchets => step.ratchets as f32,
        Field::Pressure => step.pressure as f32,
        Field::Timbre => step.timbre as f32,
        Field::Bend => step.bend,
        Field::Cc(i) => step.cc[i] as f32,
        _ => 0.0,
    }
}
fn set_slider_value(field: Field, value: f32, page: &mut Page, step: usize) {
    if field == Field::Tone {
        page.steps[step].tone = value.round().clamp(-2.0, 31.0) as i8;
    } else {
        set_field(field, &value.to_string(), page, step, 0);
    }
}
fn slider_range(field: Field) -> (f32, f32) {
    match field {
        Field::Velocity => (0.0, 200.0),
        Field::Gate => (1.0, 100.0),
        Field::Probability => (0.0, 100.0),
        Field::Octave => (-4.0, 4.0),
        Field::Micro => (-49.0, 49.0),
        Field::Ratchets => (1.0, 8.0),
        Field::Tone => (-2.0, 31.0),
        Field::Bend => (-48.0, 48.0),
        _ => (-127.0, 127.0),
    }
}

fn field_value(field: Field, p: &Page, s: &Step, ui: &SequencerUi) -> String {
    match field {
        Field::Span => p.box_at(ui.step).map_or(1, |(_, len)| len).to_string(),
        Field::Start => (p.start + 1).to_string(),
        Field::Rate => p.rate.to_string(),
        Field::Interlock => {
            p.interlock_override.map_or("Global", |v| {
                crate::params::INTERLOCK_NAMES[v.min(2) as usize]
            })
            .into()
        }
        Field::Velocity => s.velocity.to_string(),
        Field::Gate => s.gate.to_string(),
        Field::Octave => s.octave.to_string(),
        Field::Tone => match s.tone {
            -2 => "Fifth".into(),
            -1 if ui.lane == 2 => "Root".into(),
            -1 => "Pattern".into(),
            n => (n + 1).to_string(),
        },
        Field::Probability => s.probability.to_string(),
        Field::Ratchets => s.ratchets.to_string(),
        Field::Micro => s.micro.to_string(),
        Field::Root => s.root_offset.to_string(),
        Field::Quality => {
            if s.quality < 0 {
                "Live".into()
            } else {
                harmony::QUALITY_NAMES[s.quality.min(11) as usize].into()
            }
        }
        Field::Inversion => {
            if s.inversion < 0 {
                "Live".into()
            } else {
                ["Root", "1st", "2nd", "3rd", "4th", "5th"][s.inversion.min(5) as usize].into()
            }
        }
        Field::Spread => {
            if s.spread < 0 {
                "Live".into()
            } else {
                harmony::VOICING_NAMES[s.spread.min(4) as usize].into()
            }
        }
        Field::Pressure => s.pressure.to_string(),
        Field::Timbre => s.timbre.to_string(),
        Field::Bend => s.bend.to_string(),
        Field::Cc(i) => s.cc[i].to_string(),
    }
}
fn set_field(field: Field, text: &str, p: &mut Page, index: usize, _tone: usize) -> bool {
    if field == Field::Tone {
        let tone = match text.trim().to_ascii_lowercase().as_str() {
            "pattern" | "root" => -1,
            "fifth" => -2,
            number => {
                let Ok(number) = number.parse::<i32>() else {
                    return false;
                };
                if !(1..=32).contains(&number) {
                    return false;
                }
                (number - 1) as i8
            }
        };
        p.steps[index].tone = tone;
        return true;
    }
    let Ok(v) = text.trim().parse::<f32>() else {
        return false;
    };
    if !v.is_finite() {
        return false;
    }
    let n = v.round() as i32;
    let s = &mut p.steps[index];
    match field {
        Field::Span => p.draw_box(index, index + n.clamp(1, 32) as usize - 1),
        Field::Start => p.start = (n - 1).clamp(0, p.end as i32) as u8,
        Field::Rate => p.rate = v.clamp(1.0 / 64.0, 16.0),
        Field::Interlock => p.interlock_override = (n >= 0).then(|| n.min(2) as u8),
        Field::Velocity => s.velocity = n.clamp(0, 200) as u8,
        Field::Gate => s.gate = n.clamp(1, 100) as u8,
        Field::Octave => s.octave = n.clamp(-4, 4) as i8,
        Field::Tone => s.tone = n.clamp(-2, 31) as i8,
        Field::Probability => s.probability = n.clamp(0, 100) as u8,
        Field::Ratchets => s.ratchets = n.clamp(1, 8) as u8,
        Field::Micro => s.micro = n.clamp(-49, 49) as i16,
        Field::Root => s.root_offset = n.clamp(-48, 48) as i8,
        Field::Quality => s.quality = n.clamp(-1, 11) as i8,
        Field::Inversion => s.inversion = n.clamp(-1, 5) as i8,
        Field::Spread => s.spread = n.clamp(-1, 4) as i8,
        Field::Pressure => s.pressure = n.clamp(-127, 127) as i16,
        Field::Timbre => s.timbre = n.clamp(-127, 127) as i16,
        Field::Bend => s.bend = v.clamp(-48.0, 48.0),
        Field::Cc(i) => s.cc[i] = n.clamp(-127, 127) as i16,
    }
    true
}

mod draw;
mod session;

#[cfg(test)]
mod tests;
