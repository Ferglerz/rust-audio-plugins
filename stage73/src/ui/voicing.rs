//! Fixed, oblique view of one tone generator at rest. Dimensions at the tip
//! use G3's calibration. The surrounding hardware is shortened to fit the panel.
use crate::{
    engine::LOW,
    era::Era,
    field::{MAX_GAP, MIN_GAP, POLE_FLAT_HALF_HEIGHT_MM, POLE_RADIUS_MM},
};
use nih_plug_vizia::vizia::vg::{Color, Paint, Path};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, Callout},
    theme::{rgb, with_alpha, MUTED, TEAL, TEXT},
};
use std::f32::consts::{PI, TAU};

type Point = (f32, f32);

const PX_PER_MM: f32 = 10.0;
const AXIAL_X: f32 = 0.96;
const AXIAL_Y: f32 = -0.09;
const VERTICAL_Y: f32 = 0.94;
const DEPTH_X: f32 = 0.21;
const DEPTH_Y: f32 = 0.32;
const STEEL: Color = rgb(164, 182, 189);
const BRASS: Color = rgb(185, 153, 95);
const COPPER: Color = rgb(180, 100, 65);
const BOBBIN: Color = rgb(81, 77, 74);

// The free tine end is at x = 0. Height points up, depth towards the viewer.
fn project(x: f32, height: f32, depth: f32) -> Point {
    (
        340.0 + AXIAL_X * x + DEPTH_X * depth,
        173.0 + AXIAL_Y * x - VERTICAL_Y * height + DEPTH_Y * depth,
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Tine,
    Pickup,
}

impl Part {
    pub fn drag_mm(self, start: Point, now: Point) -> f32 {
        match self {
            Self::Tine => (start.1 - now.1) / (PX_PER_MM * VERTICAL_Y),
            Self::Pickup => (now.0 - start.0) / (PX_PER_MM * AXIAL_X),
        }
    }
}

pub struct Voicing {
    note: u8,
    elapsed: Option<f32>,
    height_mm: f32,
    gap_mm: f32,
}

impl Voicing {
    pub fn new(era: Era, height_offset: f32, gap_offset: f32) -> Self {
        Self::for_key(era, 55, height_offset, gap_offset, None)
    }

    pub fn for_key(
        era: Era,
        note: u8,
        height_offset: f32,
        gap_offset: f32,
        elapsed: Option<f32>,
    ) -> Self {
        let index = usize::from(note.clamp(LOW, crate::engine::HIGH) - LOW);
        let (height, gap) = era.geometry(index);
        Self {
            note,
            elapsed,
            height_mm: height as f32 + height_offset,
            gap_mm: (gap as f32 + gap_offset).clamp(MIN_GAP as f32, MAX_GAP as f32),
        }
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<Part> {
        // Undo the projection in the central plane. Pad the thin rod and
        // include the visible depth of the tonebar, bobbin and bracket.
        let axial = (x - project(0.0, 0.0, 0.0).0) / AXIAL_X;
        let above_axis = (project(axial, 0.0, 0.0).1 - y) / VERTICAL_Y;
        let above_tine = above_axis - self.height_mm * PX_PER_MM;
        if ((-287.0..=-212.0).contains(&axial) && (-16.0..=46.0).contains(&above_tine))
            || ((-225.0..=-88.0).contains(&axial) && (21.0..=47.0).contains(&above_tine))
            || ((-230.0..=3.0).contains(&axial) && above_tine.abs() <= 11.0)
        {
            return Some(Part::Tine);
        }
        let from_pole = axial - self.gap_mm * PX_PER_MM;
        let on_pole = (-8.0..=55.0).contains(&from_pole) && above_axis.abs() <= 43.0;
        let on_bobbin = (44.0..=178.0).contains(&from_pole) && above_axis.abs() <= 51.0;
        let on_foot = (42.0..=230.0).contains(&from_pole) && (-64.0..=-44.0).contains(&above_axis);
        (on_pole || on_bobbin || on_foot).then_some(Part::Pickup)
    }

    pub fn draw(&self, d: &mut Draw, active: Option<Part>) {
        let h = self.height_mm * PX_PER_MM;
        let gap = self.gap_mm * PX_PER_MM;
        action::panel(
            d,
            if self.elapsed.is_some() { 728. } else { 680. },
            "TINE ACTION",
            self.note,
        );

        // Fixed pickup rail and the centreline give both movements a datum.
        for x in (-144..45).step_by(10) {
            line(
                d,
                project(x as f32, 0.0, 0.0),
                project(x as f32 + 5.0, 0.0, 0.0),
                MUTED,
                0.8,
            );
        }
        // Keep light and shadow on the physical materials consistent in either UI theme.
        let mut hardware = Draw::new(d.c, false, d.s, d.ox, d.oy, d.font);
        hardware.offset_x = d.offset_x;
        block(
            &mut hardware,
            [50.0, -63.0, -27.0],
            [293.0, -55.0, 27.0],
            rgb(89, 74, 62),
        );
        pickup(&mut hardware, gap);
        let motion = self.elapsed.map_or(0., |t| 7. * action::vibration(t, 0.55));
        tine(&mut hardware, h, motion);

        let tip = project(0.0, h + motion, 0.0);
        let center = project(0.0, 0.0, 0.0);
        let height_color = if active == Some(Part::Tine) {
            TEXT
        } else {
            TEAL
        };
        let gap_color = if active == Some(Part::Pickup) {
            TEXT
        } else {
            TEAL
        };
        let dim_x = tip.0 - 15.0;
        let center_y = project(-15.0 / AXIAL_X, 0.0, 0.0).1;
        let tip_y = center_y + tip.1 - center.1;
        line(d, (dim_x, tip_y), (dim_x, center_y), height_color, 1.1);
        for y in [tip_y, center_y] {
            line(d, (dim_x - 3.0, y), (dim_x + 3.0, y), height_color, 1.1);
        }
        let pole = project(gap, 0.0, 0.0);
        d.circle(pole.0, pole.1, 2.4, rgb(35, 59, 60), true);
        d.circle(pole.0, pole.1, 1.3, TEAL, true);
        let dim_y = 222.0;
        for (x, y) in [tip, pole] {
            d.line(x, y + 4.0, x, dim_y + 3.0, with_alpha(gap_color, 0.40), 0.8);
            d.line(x, dim_y - 3.0, x, dim_y + 3.0, gap_color, 1.2);
        }
        d.line(tip.0, dim_y, pole.0, dim_y, gap_color, 1.2);

        let tine = Callout {
            label: "TINE HEIGHT",
            origin: (101., 105.),
            width: 78.,
            elbow: (170., 109.),
        };
        let pickup = Callout {
            label: "PICKUP GAP",
            origin: (481., 105.),
            width: 78.,
            elbow: (459., 109.),
        };
        tine.draw(
            d,
            tine.label,
            project(-135., h, 0.),
            active == Some(Part::Tine),
        );
        pickup.draw(
            d,
            pickup.label,
            project(gap + 95., 24., 0.),
            active == Some(Part::Pickup),
        );
    }
}

fn line(d: &mut Draw, a: Point, b: Point, color: Color, width: f32) {
    d.line(a.0, a.1, b.0, b.1, color, width);
}

fn shade(color: Color, light: f32) -> Color {
    Color {
        r: (color.r * light).min(1.0),
        g: (color.g * light).min(1.0),
        b: (color.b * light).min(1.0),
        ..color
    }
}

fn block(d: &mut Draw, min: [f32; 3], max: [f32; 3], color: Color) {
    let [x, y, z] = min;
    let [u, v, w] = max;
    let front = [
        project(x, v, w),
        project(u, v, w),
        project(u, y, w),
        project(x, y, w),
    ];
    let end = [
        project(x, v, z),
        project(x, v, w),
        project(x, y, w),
        project(x, y, z),
    ];
    let top = [
        project(x, v, z),
        project(u, v, z),
        project(u, v, w),
        project(x, v, w),
    ];
    d.fill_poly(&front, shade(color, 0.70));
    d.fill_poly(&end, shade(color, 0.45));
    d.fill_poly(&top, color);
    line(d, front[0], front[1], shade(color, 1.16), 0.9);
}

fn ring(x: f32, h: f32, radius: f32, angle: f32) -> Point {
    let (sin, cos) = angle.sin_cos();
    project(x, h + radius * cos, radius * sin)
}

// Paint revolved parts from the far end towards the exposed pole face.
fn cylinder(d: &mut Draw, far: f32, near: f32, h: f32, radii: (f32, f32), color: Color) {
    barrel(d, far, h, radii, color, |_| near);
    let face: [_; 40] = std::array::from_fn(|i| ring(near, h, radii.1, TAU * i as f32 / 40.0));
    d.fill_poly(&face, shade(color, 0.79));
}

fn barrel(
    d: &mut Draw,
    far: f32,
    h: f32,
    radii: (f32, f32),
    color: Color,
    near: impl Fn(f32) -> f32,
) {
    // The two silhouette edges occur at the extrema of the projected ring
    // measured perpendicular to the cylinder axis.
    let axis_len = AXIAL_X.hypot(AXIAL_Y);
    let down = (-AXIAL_Y / axis_len, AXIAL_X / axis_len);
    let height_span = VERTICAL_Y * down.1;
    let depth_span = DEPTH_X * down.0 + DEPTH_Y * down.1;
    let top_angle = -depth_span.atan2(height_span);
    let (r_far, r_near) = radii;
    let angles = (0..=32).map(|i| top_angle + PI * i as f32 / 32.0);
    let mut outline = Path::new();
    let start = ring(far, h, r_far, top_angle);
    outline.move_to(start.0, start.1);
    for a in angles.clone().skip(1) {
        let (x, y) = ring(far, h, r_far, a);
        outline.line_to(x, y);
    }
    for a in angles.rev() {
        let (x, y) = ring(near(a), h, r_near, a);
        outline.line_to(x, y);
    }
    outline.close();

    // Interpolate the lighting across one filled silhouette. A surface normal
    // rotated by phi projects to (1 - cos(phi)) / 2 across the diameter.
    let stops = (0..=24).map(|i| {
        let phi = PI * i as f32 / 24.0;
        let (sin, cos) = (top_angle + phi).sin_cos();
        let lit = (0.88 * cos + 0.47 * sin).max(0.0);
        let light = 0.40 + 0.59 * lit + 0.27 * lit.powi(16);
        ((1.0 - phi.cos()) * 0.5, d.color(shade(color, light)))
    });
    let (cx, cy) = project(far, h, 0.0);
    let radius = (r_far + r_near) * 0.5 * height_span.hypot(depth_span);
    let (dx, dy) = (down.0 * radius, down.1 * radius);
    let paint = Paint::linear_gradient_stops(cx - dx, cy - dy, cx + dx, cy + dy, stops);
    d.c.save();
    d.c.translate(d.ox + d.offset_x * d.s, d.oy);
    d.c.scale(d.s, d.s);
    d.c.fill_path(&outline, &paint);
    d.c.restore();
}

// At the nose the round pole is ground flat at the centre, with two 45° bevels.
// Build the front rim from the same cross section the field integral samples.
fn pole(d: &mut Draw, x: f32) {
    let r = POLE_RADIUS_MM as f32 * PX_PER_MM;
    let flat = POLE_FLAT_HALF_HEIGHT_MM as f32 * PX_PER_MM;
    let end = |a: f32| x + (r * a.cos().abs() - flat).max(0.0);
    barrel(d, x + 53.0, 0.0, (r, r), STEEL, end);
    let edge = |a: f32| ring(end(a), 0.0, r, a);
    let boundary = (flat / r).acos();
    let top: [_; 21] = std::array::from_fn(|i| edge(-boundary + 2.0 * boundary * i as f32 / 20.0));
    let flat_face: [_; 22] = std::array::from_fn(|i| {
        let side = if i < 11 { boundary } else { PI + boundary };
        edge(side + (PI - 2.0 * boundary) * (i % 11) as f32 / 10.0)
    });
    d.fill_poly(&flat_face, shade(STEEL, 0.83));
    d.fill_poly(&top, shade(STEEL, 1.22));
}

fn tine(d: &mut Draw, h: f32, motion: f32) {
    // Generator block, upper tonebar and lower spring-loaded tine.
    block(d, [-278.0, h - 9.0, -14.0], [-221.0, h + 22.0, 14.0], STEEL);
    block(
        d,
        [-281.0, h + 22.0, -15.0],
        [-237.0, h + 28.0, 15.0],
        BRASS,
    );
    block(d, [-244.0, h + 27.0, -15.0], [-97.0, h + 33.0, 15.0], BRASS);
    let screw = project(-258.0, h + 29.0, 0.0);
    d.circle(screw.0, screw.1, 4.8, shade(STEEL, 0.52), true);
    d.line(
        screw.0 - 2.4,
        screw.1 + 0.8,
        screw.0 + 2.4,
        screw.1 - 0.8,
        TEXT,
        1.0,
    );
    // Deflect the free tip around the fixed generator block in the animated view.
    let base = project(-222., h, 0.);
    d.c.save();
    let (bx, by) = (d.ox + (d.offset_x + base.0) * d.s, d.oy + base.1 * d.s);
    d.c.translate(bx, by);
    d.c.rotate((-motion * VERTICAL_Y / (222. * AXIAL_X)).atan());
    d.c.translate(-bx, -by);
    cylinder(d, 0.0, -222.0, h, (4.7, 5.7), STEEL);
    for turn in 0..9 {
        let start = -65.0 + 3.4 * turn as f32;
        let wire: [_; 11] = std::array::from_fn(|i| {
            let t = i as f32 / 10.0;
            ring(start + 1.7 * t, h, 6.7, -0.33 + t * PI)
        });
        d.poly(&wire, shade(STEEL, 0.55), 2.9);
        d.poly(&wire[..6], shade(STEEL, 1.30), 1.6);
    }
    d.c.restore();
}

fn pickup(d: &mut Draw, x: f32) {
    block(d, [x + 46.0, -54.0, -18.0], [x + 223.0, -49.0, 18.0], STEEL);
    block(
        d,
        [x + 72.0, -49.0, -10.0],
        [x + 130.0, -19.0, 10.0],
        BOBBIN,
    );
    // The bracket slides under a fixed screw in the rail.
    line(
        d,
        project(x + 164.0, -48.5, 0.0),
        project(x + 210.0, -48.5, 0.0),
        shade(BOBBIN, 0.5),
        4.8,
    );
    let screw = project(205.0, -48.0, 0.0);
    d.circle(screw.0, screw.1, 4.5, STEEL, true);
    d.line(screw.0 - 2.0, screw.1, screw.0 + 2.0, screw.1, BOBBIN, 1.1);
    cylinder(d, x + 168.0, x + 158.0, 0.0, (47.0, 47.0), BOBBIN);
    cylinder(d, x + 158.0, x + 60.0, 0.0, (42.0, 42.0), COPPER);
    for i in 0..38 {
        let at = x + 61.0 + i as f32 * 2.55;
        let turn: [_; 12] =
            std::array::from_fn(|j| ring(at, 0.0, 42.1, -0.33 + j as f32 * PI / 11.0));
        d.poly(&turn, with_alpha(shade(COPPER, 0.36), 0.64), 0.8);
    }
    cylinder(d, x + 60.0, x + 50.0, 0.0, (48.0, 48.0), BOBBIN);
    pole(d, x);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_each_part_follows_its_projected_axis_across_the_range() {
        let stock = Voicing::new(Era::LateI, 0.0, 0.0);
        let tine = |v: &Voicing| project(-50.0, v.height_mm * PX_PER_MM, 0.0);
        let coil = |v: &Voicing| project(92.0 + v.gap_mm * PX_PER_MM, 0.0, 0.0);
        for height in [-3.0, 0.0, 3.0] {
            for gap in [-1.0, 0.0, 1.0] {
                let model = Voicing::new(Era::LateI, height, gap);
                let travel =
                    (stock.gap_mm + gap).clamp(MIN_GAP as f32, MAX_GAP as f32) - stock.gap_mm;
                for (part, start, end, expected) in [
                    (Part::Tine, tine(&stock), tine(&model), height),
                    (Part::Pickup, coil(&stock), coil(&model), travel),
                ] {
                    assert!(model.hit(end.0, end.1) == Some(part));
                    assert!((part.drag_mm(start, end) - expected).abs() < 0.0001);
                }
            }
        }
    }
}
