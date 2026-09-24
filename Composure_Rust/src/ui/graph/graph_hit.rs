//! Point hit testing in the editor's padded graph viewport.

use crate::dsp::graph::CompressionGraph;

use super::graph_display;

const MOUSE_CLICK_RADIUS: f32 = 18.0;

fn closest_interior_point(
    graph: &CompressionGraph,
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    pad_db: f64,
) -> Option<(usize, f32)> {
    let mut closest = None;
    for index in 1..graph.num_points - 1 {
        let (x, y) = graph_display::db_to_pixel_with_pad(
            graph.get_point_x(index),
            graph.get_point_y(index),
            width,
            height,
            graph.min_db,
            graph.range_db,
            pad_db,
        );
        let distance_sq = (local_x - x).powi(2) + (local_y - y).powi(2);
        if closest.is_none_or(|(_, best)| distance_sq < best) {
            closest = Some((index, distance_sq));
        }
    }
    closest
}

pub(super) fn find_interior_point(
    graph: &CompressionGraph,
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    pad_db: f64,
) -> Option<usize> {
    closest_interior_point(graph, local_x, local_y, width, height, pad_db)
        .filter(|(_, distance_sq)| *distance_sq < MOUSE_CLICK_RADIUS.powi(2))
        .map(|(index, _)| index)
}

pub(super) fn too_close_to_points(
    graph: &CompressionGraph,
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    pad_db: f64,
) -> bool {
    closest_interior_point(graph, local_x, local_y, width, height, pad_db)
        .is_some_and(|(_, distance_sq)| distance_sq < (MOUSE_CLICK_RADIUS * 2.0).powi(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_uses_the_same_padded_coordinates_as_drawing() {
        let graph = CompressionGraph::new();
        let pad = graph_display::DISPLAY_PAD_DB;
        let (x, y) = graph_display::db_to_pixel_with_pad(
            graph.get_point_x(1),
            graph.get_point_y(1),
            400.0,
            400.0,
            graph.min_db,
            graph.range_db,
            pad,
        );
        assert_eq!(
            find_interior_point(&graph, x, y, 400.0, 400.0, pad),
            Some(1)
        );
        assert!(too_close_to_points(&graph, x, y, 400.0, 400.0, pad));
    }
}
