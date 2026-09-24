//! Embedded UI images copied from `../Composure/Images/`.

use nih_plug_vizia::vizia::image;
use nih_plug_vizia::vizia::prelude::*;

pub const KNOB: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ui/knob.png"));
pub const SWITCH_DN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/switch_dn.png"
));
pub const SWITCH_UP: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/switch_up.png"
));
pub const FEEDBACK_ON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/Feed/feedback_on.png"
));
pub const FEEDFWRD_OFF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/Feed/feedfwrd_off.png"
));
pub const FADER_TOP: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/Paralax_Fader/para_horz_fader_top.png"
));
pub const FADER_BOT_LEFT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/Paralax_Fader/para_horz_fader_bot_left.png"
));
pub const FADER_BG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/Paralax_Fader/para_horz_fader_bg.png"
));

fn load_png(cx: &mut Context, name: &str, bytes: &'static [u8]) {
    if let Ok(img) = image::load_from_memory_with_format(bytes, image::ImageFormat::Png) {
        cx.load_image(name, img, ImageRetentionPolicy::Forever);
    }
}

/// Register images for CSS `background-image` (editor BG + any styled elements).
pub fn register_editor_images(cx: &mut Context) {
    let bg = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ui/BG.png"));
    load_png(cx, super::EDITOR_BG_IMAGE, bg);
    load_png(cx, "ui_knob", KNOB);
    load_png(cx, "ui_switch_dn", SWITCH_DN);
    load_png(cx, "ui_switch_up", SWITCH_UP);
}
