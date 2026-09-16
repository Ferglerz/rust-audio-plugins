use nih_plug_vizia::vizia::vg::Color;

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub const BG: Color = rgb(19, 22, 27);
pub const PANEL: Color = rgb(27, 31, 37);
pub const LINE: Color = rgb(45, 50, 58);
pub const TEXT: Color = rgb(229, 233, 237);
pub const MUTED: Color = rgb(143, 153, 164);
pub const GOLD: Color = rgb(225, 187, 101);
pub const TEAL: Color = rgb(106, 213, 189);

pub const COLORS: [Color; 6] = [
    rgb(111, 210, 188),
    rgb(171, 151, 238),
    rgb(236, 177, 107),
    rgb(108, 176, 242),
    rgb(228, 130, 164),
    rgb(193, 214, 118),
];

/// Transform a dark-mode palette color to light-mode if `light` is true.
pub fn transform_color(c: Color, light: bool) -> Color {
    if !light {
        return c;
    }
    let mut result = if c == BG {
        rgb(245, 243, 238)
    } else if c == PANEL {
        rgb(233, 230, 223)
    } else if c == LINE {
        rgb(201, 201, 195)
    } else if c == TEXT {
        rgb(30, 37, 44)
    } else if c == MUTED {
        rgb(95, 105, 114)
    } else if c == GOLD {
        rgb(145, 98, 24)
    } else if c == TEAL {
        rgb(21, 122, 104)
    } else if c.r.max(c.g).max(c.b) < 0.35 {
        Color {
            r: 0.90 - c.r * 0.35,
            g: 0.90 - c.g * 0.35,
            b: 0.88 - c.b * 0.35,
            a: c.a,
        }
    } else {
        Color {
            r: c.r * 0.64,
            g: c.g * 0.64,
            b: c.b * 0.64,
            a: c.a,
        }
    };
    result.a = c.a;
    result
}
