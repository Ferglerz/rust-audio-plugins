use super::*;

pub(super) const WIDTH: f32 = super::super::graph_pages::PAGE_W;
pub(super) const HEIGHT: f32 = super::super::appearance::SIDE_H;
pub(super) const GRAPH: (f32, f32, f32, f32) = (36.0, 44.0, WIDTH - 52.0, HEIGHT - 116.0);
pub(super) const CENTER_X: f32 = WIDTH * 0.5;
pub(super) const ACTIONS_Y: f32 = HEIGHT - 36.0;
pub(super) const FOOTER_X: f32 = (WIDTH - 434.0) * 0.5;
pub(super) const BYPASS_RECT: (f32, f32, f32, f32) = (FOOTER_X, ACTIONS_Y, 26.0, 28.0);
pub(super) const SHAPE_RECT: (f32, f32, f32, f32) = (FOOTER_X + 32.0, ACTIONS_Y, 100.0, 28.0);
pub(super) const REMOVE_RECT: (f32, f32, f32, f32) = (FOOTER_X + 416.0, ACTIONS_Y, 18.0, 28.0);
pub(super) const LISTEN_RECT: (f32, f32, f32, f32) = (WIDTH - 262.0, 4.0, 28.0, 26.0);
pub(super) const SIDECHAIN_RECT: (f32, f32, f32, f32) = (WIDTH - 226.0, 4.0, 210.0, 26.0);
pub(super) const MENU_ROW_H: f32 = 26.0;
pub(super) const SHAPE_MENU: (f32, f32, f32, f32) = (
    SHAPE_RECT.0,
    ACTIONS_Y - MENU_ROW_H * DetectorShape::ALL.len() as f32 - 4.0,
    160.0,
    MENU_ROW_H * DetectorShape::ALL.len() as f32,
);
pub(super) const GAIN_DB: f32 = 24.0;
pub(super) const FIELDS_Y: f32 = ACTIONS_Y;

pub(super) fn menu_shape(point: (f32, f32)) -> Option<DetectorShape> {
    if !point_in_rect(point.0, point.1, SHAPE_MENU) {
        return None;
    }
    DetectorShape::ALL
        .get(((point.1 - SHAPE_MENU.1) / MENU_ROW_H) as usize)
        .copied()
}

pub(super) fn frequency_x(frequency: f32) -> f32 {
    GRAPH.0 + GRAPH.2 * (frequency.clamp(20.0, 20_000.0) / 20.0).ln() / 1000.0_f32.ln()
}

pub(super) fn x_frequency(x: f32) -> f32 {
    20.0 * 1000.0_f32.powf(((x - GRAPH.0) / GRAPH.2).clamp(0.0, 1.0))
}

pub(super) fn gain_y(gain: f32) -> f32 {
    GRAPH.1 + GRAPH.3 * (0.5 - gain.clamp(-GAIN_DB, GAIN_DB) / (GAIN_DB * 2.0))
}

pub(super) fn y_gain(y: f32) -> f32 {
    ((0.5 - (y - GRAPH.1) / GRAPH.3) * GAIN_DB * 2.0).clamp(-GAIN_DB, GAIN_DB)
}

pub(super) fn field_rect(field: Field, _fields: &[Field]) -> (f32, f32, f32, f32) {
    let (offset, width) = match field {
        Field::Frequency => (138.0, 96.0),
        Field::Gain => (240.0, 100.0),
        Field::Q => (346.0, 64.0),
    };
    (FOOTER_X + offset, FIELDS_Y, width, 28.0)
}

pub(super) fn response_frequency(x: f32, sample_rate: f64) -> f64 {
    (x_frequency(x) as f64).min(sample_rate.max(1.0) * 0.49)
}
