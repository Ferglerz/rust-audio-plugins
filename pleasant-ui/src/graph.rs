//! Domain-neutral graph coordinates and semantic edit actions.

use pleasant_dsp::axis::{flattery_freq_to_pos, flattery_pos_to_freq};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_tuple(rect: (f32, f32, f32, f32)) -> Self {
        Self::new(rect.0, rect.1, rect.2, rect.3)
    }

    pub fn to_normalized(self, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.x) / self.width.max(1.0),
            1.0 - (y - self.y) / self.height.max(1.0),
        )
    }

    pub fn to_normalized_clamped(self, x: f32, y: f32) -> (f32, f32) {
        let (x, y) = self.to_normalized(x, y);
        (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
    }

    pub fn from_normalized(self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.width, self.y + (1.0 - y) * self.height)
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

pub trait AxisMapping {
    fn value_to_position(&self, value: f64) -> f64;
    fn position_to_value(&self, position: f64) -> f64;
}

#[derive(Clone, Copy, Debug)]
pub struct LinearAxis {
    pub minimum: f64,
    pub maximum: f64,
}

impl AxisMapping for LinearAxis {
    fn value_to_position(&self, value: f64) -> f64 {
        ((value - self.minimum) / (self.maximum - self.minimum).max(f64::EPSILON)).clamp(0.0, 1.0)
    }

    fn position_to_value(&self, position: f64) -> f64 {
        self.minimum + position.clamp(0.0, 1.0) * (self.maximum - self.minimum)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LogFrequencyAxis {
    pub minimum_hz: f64,
    pub maximum_hz: f64,
}

impl AxisMapping for LogFrequencyAxis {
    fn value_to_position(&self, value: f64) -> f64 {
        let minimum = self.minimum_hz.max(1.0e-9);
        ((value.max(minimum).ln() - minimum.ln())
            / (self.maximum_hz.max(minimum).ln() - minimum.ln()).max(f64::EPSILON))
        .clamp(0.0, 1.0)
    }

    fn position_to_value(&self, position: f64) -> f64 {
        let minimum = self.minimum_hz.max(1.0e-9);
        (minimum.ln()
            + position.clamp(0.0, 1.0) * (self.maximum_hz.max(minimum).ln() - minimum.ln()))
        .exp()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlatteryFrequencyAxis {
    pub minimum_hz: f64,
    pub maximum_hz: f64,
}

impl AxisMapping for FlatteryFrequencyAxis {
    fn value_to_position(&self, value: f64) -> f64 {
        flattery_freq_to_pos(value, self.minimum_hz, self.maximum_hz)
    }

    fn position_to_value(&self, position: f64) -> f64 {
        flattery_pos_to_freq(position, self.minimum_hz, self.maximum_hz)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphAction<T> {
    Begin(T),
    Move(T),
    Insert(T),
    Delete(T),
    End(T),
}

pub fn nearest_point(
    points: impl IntoIterator<Item = (usize, f32, f32)>,
    x: f32,
    y: f32,
    radius: f32,
) -> Option<usize> {
    points
        .into_iter()
        .filter_map(|(index, point_x, point_y)| {
            let distance = (point_x - x).hypot(point_y - y);
            (distance <= radius).then_some((index, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_round_trips_and_flips_y() {
        let viewport = Viewport::new(10.0, 20.0, 200.0, 100.0);
        let point = viewport.from_normalized(0.25, 0.75);
        assert_eq!(point, (60.0, 45.0));
        assert_eq!(viewport.to_normalized(point.0, point.1), (0.25, 0.75));
    }

    #[test]
    fn axes_round_trip() {
        let logarithmic = LogFrequencyAxis {
            minimum_hz: 20.0,
            maximum_hz: 20_000.0,
        };
        let flattery = FlatteryFrequencyAxis {
            minimum_hz: 20.0,
            maximum_hz: 20_000.0,
        };
        for frequency in [20.0, 100.0, 1_000.0, 20_000.0] {
            let log_back = logarithmic.position_to_value(logarithmic.value_to_position(frequency));
            assert!((log_back - frequency).abs() / frequency < 1.0e-10);
            let flattery_back = flattery.position_to_value(flattery.value_to_position(frequency));
            assert!((flattery_back - frequency).abs() / frequency < 1.0e-8);
        }
    }
}
