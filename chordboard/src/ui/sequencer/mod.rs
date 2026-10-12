//! Pattern editor. All drawing and hit testing share the same control geometry.
use super::*;
use crate::sequencer::{Page, State, Step, FINE_RANGE, SPEED_NAMES, SPEED_STEPS};

const NUMBER_HOLD: f32 = 0.5;
const NUMBER_FADE: f32 = 0.35;

const VISIBLE_LANES: [usize; 2] = [0, 1];
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
    edit: Option<ValueEdit<Field>>,
    modifier: Option<Field>,
    slider_drag: Option<SliderDrag>,
    length_drag: Option<usize>,
}
/// Cell sliders move relative to the press point so a click alone never edits.
#[derive(Clone, Copy, Debug)]
struct SliderDrag {
    lane: usize,
    page: usize,
    step: usize,
    field: Field,
    last_y: f32,
    value: f32,
    reset: bool,
}
/// Mouse travel, in cell heights, that sweeps a cell slider's full range.
const SLIDER_FRICTION: f32 = 2.5;
const SLIDER_FINE: f32 = 0.25;
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
            edit: None,
            modifier: None,
            slider_drag: None,
            length_drag: None,
        }
    }
}
// Legacy expression fields remain usable by the saved-data editor tests.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Field {
    Velocity,
    Gate,
    Octave,
    Tone,
    Probability,
    Ratchets,
    Micro,
    Pressure,
    Timbre,
    Bend,
    Speed,
    Fine,
    Cc(usize),
    Mod(usize),
}
#[derive(Clone, Copy)]
enum Action {
    Modifier(Option<Field>),
    EditPage(usize, usize),
    SelectStep(usize, usize),
    LengthLock,
    Field(Field),
    Enabled,
    LaneEnabled(usize),
    PageRate,
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
const STEP_TOP: f32 = MODULE_HEADER_H;
const STEP_H: f32 = 56.0;
const STEP_GAP: f32 = 3.0;
const LENGTH_H: f32 = 18.0;
const LANE_PITCH: f32 = 78.0;
const LANE_LABEL_W: f32 = 60.0;
const PAGE_COLUMNS: usize = 4;
const PAGE_GAP: f32 = 4.0;
const PAGE_SIZE: f32 = (STEP_H - PAGE_GAP) / 2.0;
const STEPS_LEFT: f32 = 212.0;
fn lane_y(lane: usize) -> f32 {
    let row = VISIBLE_LANES.iter().position(|&l| l == lane).unwrap_or(0);
    surface().1 + STEP_TOP + row as f32 * LANE_PITCH
}
fn lane_label_rect(lane: usize) -> Rect {
    (surface().0 + 16.0, lane_y(lane), LANE_LABEL_W, PAGE_SIZE)
}
fn length_rect(lane: usize) -> Rect {
    let first = step_rect(lane, 0);
    let last = step_rect(lane, 31);
    (
        first.0,
        first.1 + first.3 + STEP_GAP,
        last.0 + last.2 - first.0,
        LENGTH_H,
    )
}
/// Eight page buttons in two rows whose combined height matches one step cell.
fn page_rect(lane: usize, page: usize) -> Rect {
    let block = page_block_rect(lane);
    (
        block.0 + (page % PAGE_COLUMNS) as f32 * (PAGE_SIZE + PAGE_GAP),
        block.1 + (page / PAGE_COLUMNS) as f32 * (PAGE_SIZE + PAGE_GAP),
        PAGE_SIZE,
        PAGE_SIZE,
    )
}
fn page_block_rect(lane: usize) -> Rect {
    let label = lane_label_rect(lane);
    (
        label.0 + label.2 + 8.0,
        lane_y(lane),
        PAGE_COLUMNS as f32 * (PAGE_SIZE + PAGE_GAP) - PAGE_GAP,
        STEP_H,
    )
}
fn step_rect(lane: usize, visible: usize) -> Rect {
    let left = surface().0 + STEPS_LEFT;
    let width = (surface().2 - STEPS_LEFT - 16.0 - 31.0 * STEP_GAP) / 32.0;
    (
        left + visible as f32 * (width + STEP_GAP),
        lane_y(lane),
        width,
        STEP_H,
    )
}
fn header_rect(x: f32, width: f32) -> Rect {
    (surface().0 + x, surface().1 + 8.0, width, 28.0)
}
fn lock_lengths_rect() -> Rect {
    header_rect(114.0, 104.0)
}
/// Step value selectors occupy two rows beneath the arpeggiator patterns.
fn header_slots(modifiers: usize, rest: usize) -> Vec<Rect> {
    let columns = 7;
    let gap = 8.0;
    let width = (ARP_PAD.2 - 16.0 - (columns - 1) as f32 * gap) / columns as f32;
    (0..modifiers + rest)
        .map(|index| (
            ARP_PAD.0 + 8.0 + (index % columns) as f32 * (width + gap),
            380.0 + (index / columns) as f32 * 44.0,
            width,
            36.0,
        ))
        .collect()
}
fn cells_per_group(numerator: u8) -> usize {
    numerator.clamp(1, 32) as usize
}
fn light_bar(step: usize, per_bar: usize) -> bool {
    (step / per_bar.max(1)).is_multiple_of(2)
}
fn first_step() -> usize {
    0
}
fn visible_steps() -> usize {
    32
}
fn step_at(lane: usize, x: f32, _y: f32) -> usize {
    let r = step_rect(lane, 0);
    ((x - r.0) / (r.2 + STEP_GAP)).floor().clamp(0.0, 31.0) as usize
}
fn inspector_rect() -> Rect {
    ARP_EDITOR
}
const MODIFIERS: [(&str, Option<Field>); 11] = [
    ("Steps", None),
    ("Gate", Some(Field::Gate)),
    ("Chance", Some(Field::Probability)),
    ("Octave", Some(Field::Octave)),
    ("Timing", Some(Field::Micro)),
    ("Repeats", Some(Field::Ratchets)),
    ("Speed", Some(Field::Speed)),
    ("Fine", Some(Field::Fine)),
    ("Velocity", Some(Field::Velocity)),
    ("Mod 1", Some(Field::Mod(0))),
    ("Mod 2", Some(Field::Mod(1))),
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
            | Field::Speed
            | Field::Fine
            | Field::Cc(_)
            | Field::Mod(_)
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
        Field::Speed => step.speed as f32,
        Field::Fine => step.fine as f32,
        Field::Cc(i) => step.cc[i] as f32,
        Field::Mod(i) => step.mods[i] as f32,
    }
}
/// Vertical fill endpoints, measured from the top of a cell.
fn slider_fill(field: Field, step: &Step) -> (f32, f32) {
    let (min, max) = slider_range(field);
    let value = slider_value(field, step).clamp(min, max);
    let end = 1.0 - (value - min) / (max - min);
    let origin = if min < 0.0 { 1.0 + min / (max - min) } else { 1.0 };
    (origin.min(end), origin.max(end))
}
fn set_slider_value(field: Field, value: f32, page: &mut Page, step: usize) {
    if field == Field::Tone {
        page.steps[step].tone = value.round().clamp(-2.0, 31.0) as i8;
    } else if field == Field::Speed {
        page.steps[step].speed = value.round().clamp(0.0, (SPEED_STEPS.len() - 1) as f32) as u8;
    } else {
        set_field(field, &value.to_string(), page, step, 0);
    }
}
fn slider_range(field: Field) -> (f32, f32) {
    match field {
        Field::Velocity => (0.0, 200.0),
        Field::Mod(_) => (-100.0, 100.0),
        Field::Gate => (1.0, 100.0),
        Field::Probability => (0.0, 100.0),
        Field::Octave => (-4.0, 4.0),
        Field::Micro => (-49.0, 49.0),
        Field::Ratchets => (1.0, 8.0),
        Field::Tone => (-2.0, 31.0),
        Field::Bend => (-48.0, 48.0),
        Field::Speed => (0.0, (SPEED_STEPS.len() - 1) as f32),
        Field::Fine => (-(FINE_RANGE as f32), FINE_RANGE as f32),
        _ => (-127.0, 127.0),
    }
}

fn field_value(field: Field, _page: &Page, s: &Step, ui: &SequencerUi) -> String {
    match field {
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
        Field::Pressure => s.pressure.to_string(),
        Field::Timbre => s.timbre.to_string(),
        Field::Bend => s.bend.to_string(),
        Field::Speed => SPEED_NAMES[(s.speed as usize).min(SPEED_NAMES.len() - 1)].into(),
        Field::Fine => match s.fine {
            0 => "0%".into(),
            n => format!("{n:+}%"),
        },
        Field::Cc(i) => s.cc[i].to_string(),
        Field::Mod(i) => if s.mods[i] == 0 { "0%".into() } else { format!("{:+}%", s.mods[i]) },
    }
}
fn parse_speed(text: &str) -> Option<u8> {
    let text = text.trim();
    if let Some(index) = SPEED_NAMES.iter().position(|name| *name == text) {
        return Some(index as u8);
    }
    let factor = match text.trim_start_matches(['×', 'x', 'X']).trim() {
        "¼" | "1/4" => 0.25,
        "½" | "1/2" => 0.5,
        number => number.parse::<f32>().ok().filter(|v| v.is_finite() && *v > 0.0)?,
    };
    SPEED_STEPS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - factor).abs().total_cmp(&(b.1 - factor).abs()))
        .map(|(index, _)| index as u8)
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
    if field == Field::Speed {
        let Some(speed) = parse_speed(text) else {
            return false;
        };
        p.steps[index].speed = speed;
        return true;
    }
    let Ok(v) = text.trim().trim_end_matches('%').trim().parse::<f32>() else {
        return false;
    };
    if !v.is_finite() {
        return false;
    }
    let n = v.round() as i32;
    let s = &mut p.steps[index];
    match field {
        Field::Velocity => s.velocity = n.clamp(0, 200) as u8,
        Field::Gate => s.gate = n.clamp(1, 100) as u8,
        Field::Octave => s.octave = n.clamp(-4, 4) as i8,
        Field::Tone => s.tone = n.clamp(-2, 31) as i8,
        Field::Probability => s.probability = n.clamp(0, 100) as u8,
        Field::Ratchets => s.ratchets = n.clamp(1, 8) as u8,
        Field::Micro => s.micro = n.clamp(-49, 49) as i16,
        Field::Pressure => s.pressure = n.clamp(-127, 127) as i16,
        Field::Timbre => s.timbre = n.clamp(-127, 127) as i16,
        Field::Bend => s.bend = v.clamp(-48.0, 48.0),
        Field::Speed => s.speed = n.clamp(0, SPEED_STEPS.len() as i32 - 1) as u8,
        Field::Fine => s.fine = n.clamp(-(FINE_RANGE as i32), FINE_RANGE as i32) as i8,
        Field::Cc(i) => s.cc[i] = n.clamp(-127, 127) as i16,
        Field::Mod(i) => s.mods[i] = n.clamp(-100, 100) as i16,
    }
    true
}

mod draw;
mod session;

#[cfg(test)]
mod tests;
