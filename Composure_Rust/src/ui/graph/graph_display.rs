//! Graph widget display coordinates (operational square + symmetric margin).

use nih_plug_vizia::vizia::prelude::BoundingBox;

/// Extra dB shown around the operational graph square.
pub const DISPLAY_PAD_DB: f64 = 1.5;
/// Grid and axis label spacing in the graph editor.
pub fn grid_step_db(range_db: f64) -> f64 {
    if range_db == 20.0 || range_db == 40.0 {
        5.0
    } else if range_db <= 24.0 {
        6.0
    } else {
        12.0
    }
}

pub fn display_min_db(operational_min_db: f64) -> f64 {
    operational_min_db - DISPLAY_PAD_DB
}

pub fn display_max_db(operational_max_db: f64) -> f64 {
    operational_max_db + DISPLAY_PAD_DB
}

pub fn pixel_to_db_with_pad(
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    min_db: f64,
    max_db: f64,
    range_db: f64,
    pad_db: f64,
) -> (f64, f64) {
    let x_norm = local_x as f64 / width as f64;
    let y_norm = 1.0 - (local_y as f64 / height as f64);
    (
        norm_to_db_with_pad(x_norm, min_db, range_db, pad_db).clamp(min_db, max_db),
        norm_to_db_with_pad(y_norm, min_db, range_db, pad_db).clamp(min_db, max_db),
    )
}

pub fn db_to_pixel_with_pad(
    input_db: f64,
    output_db: f64,
    width: f32,
    height: f32,
    min_db: f64,
    range_db: f64,
    pad_db: f64,
) -> (f32, f32) {
    let x_norm = db_to_norm_with_pad(input_db, min_db, range_db, pad_db);
    let y_norm = db_to_norm_with_pad(output_db, min_db, range_db, pad_db);
    (
        (x_norm * width as f64) as f32,
        height - (y_norm * height as f64) as f32,
    )
}

pub fn db_to_local_x(db: f64, bounds: BoundingBox, min_db: f64, range_db: f64) -> f32 {
    let norm = db_to_norm(db, min_db, range_db);
    (bounds.x + norm as f32 * bounds.w).clamp(bounds.x, bounds.x + bounds.w)
}

pub fn db_to_local_y(db: f64, bounds: BoundingBox, min_db: f64, range_db: f64) -> f32 {
    let norm = db_to_norm(db, min_db, range_db);
    (bounds.y + bounds.h - norm as f32 * bounds.h).clamp(bounds.y, bounds.y + bounds.h)
}

pub fn local_y_to_db(local_y: f32, bounds: BoundingBox, min_db: f64, range_db: f64) -> f64 {
    let norm = 1.0 - ((local_y - bounds.y) / bounds.h.max(1.0)) as f64;
    norm_to_db(norm, min_db, range_db)
}

pub fn display_range_db(operational_range_db: f64) -> f64 {
    operational_range_db + 2.0 * DISPLAY_PAD_DB
}

fn db_to_norm_with_pad(db: f64, min_db: f64, range_db: f64, pad_db: f64) -> f64 {
    (db - (min_db - pad_db)) / (range_db + 2.0 * pad_db).max(0.001)
}

fn norm_to_db_with_pad(norm: f64, min_db: f64, range_db: f64, pad_db: f64) -> f64 {
    min_db - pad_db + norm * (range_db + 2.0 * pad_db).max(0.001)
}

pub fn db_to_norm(db: f64, operational_min_db: f64, operational_range_db: f64) -> f64 {
    db_to_norm_with_pad(db, operational_min_db, operational_range_db, DISPLAY_PAD_DB)
}

pub fn norm_to_db(norm: f64, operational_min_db: f64, operational_range_db: f64) -> f64 {
    norm_to_db_with_pad(
        norm,
        operational_min_db,
        operational_range_db,
        DISPLAY_PAD_DB,
    )
}

pub fn grid_line_count(operational_range_db: f64) -> usize {
    (operational_range_db / grid_step_db(operational_range_db)).floor() as usize
}

/// Axis labels at -5, -10, … down to the operational floor (e.g. -20 / -40 / -60).
pub fn axis_label_db_values(operational_range_db: f64) -> Vec<i32> {
    let n = grid_line_count(operational_range_db);
    let mut labels: Vec<i32> = (1..=n)
        .map(|i| -(i as f64 * grid_step_db(operational_range_db)).round() as i32)
        .collect();
    let floor = -operational_range_db.round() as i32;
    if labels.last() != Some(&floor) {
        labels.push(floor);
    }
    labels
}

pub fn axis_label_y(
    db: f64,
    operational_min_db: f64,
    operational_range_db: f64,
    graph_height: f32,
) -> f32 {
    let norm = db_to_norm(db, operational_min_db, operational_range_db);
    graph_height - (norm * graph_height as f64) as f32
}

pub fn graph_y_to_db(
    local_y: f32,
    graph_height: f32,
    operational_min_db: f64,
    operational_range_db: f64,
) -> f64 {
    let norm = 1.0 - (local_y as f64 / graph_height as f64);
    norm_to_db(norm, operational_min_db, operational_range_db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_coordinates_round_trip_with_and_without_padding() {
        for pad_db in [0.0, DISPLAY_PAD_DB] {
            for (input, output) in [(-20.0, -20.0), (-16.0, -12.0), (0.0, 0.0)] {
                let (x, y) = db_to_pixel_with_pad(input, output, 400.0, 400.0, -20.0, 20.0, pad_db);
                let (back_input, back_output) =
                    pixel_to_db_with_pad(x, y, 400.0, 400.0, -20.0, 0.0, 20.0, pad_db);
                assert!((back_input - input).abs() < 0.05);
                assert!((back_output - output).abs() < 0.05);
            }
        }
    }

    #[test]
    fn damian_axis_labels_end_at_the_selected_negative_floor() {
        for mode in crate::params::GraphRangeMode::PLEASANT {
            let labels = axis_label_db_values(mode.range_db());
            assert_eq!(labels.last(), Some(&-(mode.range_db() as i32)));
            assert!(labels
                .iter()
                .all(|db| *db < 0 && *db >= -(mode.range_db() as i32)));
        }
    }

    #[test]
    fn range20_labels_are_five_db_steps_to_floor() {
        let labels = axis_label_db_values(20.0);
        assert_eq!(labels, vec![-5, -10, -15, -20]);
    }

    #[test]
    fn range40_labels_step_by_five() {
        let labels = axis_label_db_values(40.0);
        assert_eq!(labels.len(), 8);
        assert_eq!(labels[0], -5);
        assert_eq!(labels[7], -40);
    }

    #[test]
    fn symmetric_pad_extends_both_sides() {
        assert!((display_min_db(-20.0) - (-21.5)).abs() < 0.001);
        assert!((display_max_db(0.0) - 1.5).abs() < 0.001);
        assert!((display_range_db(20.0) - 23.0).abs() < 0.001);
    }

    #[test]
    fn operational_floor_maps_above_widget_bottom() {
        let y = axis_label_y(-20.0, -20.0, 20.0, 230.0);
        let bottom = 230.0;
        let pad_px = bottom * (DISPLAY_PAD_DB / display_range_db(20.0)) as f32;
        assert!((y - (bottom - pad_px)).abs() < 1.0);
    }
}
