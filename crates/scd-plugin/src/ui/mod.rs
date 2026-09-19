use crate::params::ScdParams;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color, FontId},
    },
    widgets::RawParamEvent,
    ViziaTheming,
};
use pleasant_ui::{
    draw::Draw,
    preferences::AppearanceStore,
    theme::{rgb, BG, LINE, MUTED, PANEL, TEXT},
    FONT_JETBRAINS_MONO,
};
use scd_core::{KitPieceId, MicChannel};
use std::cell::Cell;
use std::sync::{Arc, OnceLock};

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("SoundChef Drums"))
}

pub const WINDOW_W: f32 = 1181.0;
pub const WINDOW_H: f32 = 611.0;

pub const MIC_COLORS: [Color; 6] = [
    rgb(184, 244, 171), // Close (soft green)
    rgb(244, 171, 184), // XY (soft pink)
    rgb(171, 220, 244), // Mono (soft blue)
    rgb(231, 171, 244), // Wide (soft violet)
    rgb(244, 195, 171), // Front MS (soft peach)
    rgb(244, 231, 171), // Room (soft yellow)
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragTarget {
    Pitch(KitPieceId),
    Pan(KitPieceId),
    Fader(KitPieceId),
    Punch(KitPieceId),
    SubKickVol,
    SubKickLength,
    SubKickDive,
    SubKickSpeed,
    SubKickOffset,
}

pub struct ScdEditorView {
    params: Arc<ScdParams>,
    active_sof: Option<MicChannel>,
    sub_kick_open: bool,
    drag: Option<DragTarget>,
    mouse: (f32, f32),
    font: Cell<Option<FontId>>,
}

impl ScdEditorView {
    pub fn new(cx: &mut Context, params: Arc<ScdParams>) -> Handle<'_, Self> {
        Self {
            params,
            active_sof: None,
            sub_kick_open: false,
            drag: None,
            mouse: (0.0, 0.0),
            font: Cell::new(None),
        }
        .build(cx, |_| {})
    }

    fn emit_norm(&self, cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        let norm = norm.clamp(0.0, 1.0);
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }
}

impl View for ScdEditorView {
    fn element(&self) -> Option<&'static str> {
        Some("scd-editor")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            let bounds = cx.bounds();
            let scale = if bounds.w > 0.0 { bounds.w / WINDOW_W } else { 1.0 };
            let mouse_x = (cx.mouse().cursorx - bounds.x) / scale;
            let mouse_y = (cx.mouse().cursory - bounds.y) / scale;
            let prev_y = self.mouse.1;
            self.mouse = (mouse_x, mouse_y);

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    let x = mouse_x;
                    let y = mouse_y;

                    // Check SOF buttons click: centered below mixer (y: 530..562)
                    let sof_w = 70.0;
                    let sof_h = 32.0;
                    let sof_spacing = 10.0;
                    let total_sof_w = 6.0 * sof_w + 5.0 * sof_spacing;
                    let sof_start_x = (WINDOW_W - total_sof_w) * 0.5;
                    let sof_y = 530.0;

                    for (i, mic) in MicChannel::ALL.iter().enumerate() {
                        let bx = sof_start_x + i as f32 * (sof_w + sof_spacing);
                        if x >= bx && x <= bx + sof_w && y >= sof_y && y <= sof_y + sof_h {
                            if self.active_sof == Some(*mic) {
                                self.active_sof = None;
                            } else {
                                self.active_sof = Some(*mic);
                            }
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                    }

                    // Sub Kick button click (top right header)
                    let sub_btn_x = WINDOW_W - 200.0;
                    let sub_btn_y = 4.0;
                    if x >= sub_btn_x && x <= sub_btn_x + 90.0 && y >= sub_btn_y && y <= sub_btn_y + 24.0 {
                        self.sub_kick_open = !self.sub_kick_open;
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    // Fader Lock toggle button (top right header)
                    let lock_btn_x = WINDOW_W - 100.0;
                    let lock_btn_y = 4.0;
                    if x >= lock_btn_x && x <= lock_btn_x + 90.0 && y >= lock_btn_y && y <= lock_btn_y + 24.0 {
                        let cur = self.params.fader_lock.value();
                        let ptr = self.params.fader_lock.as_ptr();
                        self.emit_norm(cx, ptr, if !cur { 1.0 } else { 0.0 });
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    // Sub Kick Modal knobs interaction
                    if self.sub_kick_open {
                        let modal_x = (WINDOW_W - 440.0) * 0.5;
                        let modal_y = 180.0;
                        let modal_w = 440.0;
                        let modal_h = 160.0;

                        if x >= modal_x && x <= modal_x + modal_w && y >= modal_y && y <= modal_y + modal_h {
                            let knob_y = modal_y + 50.0;
                            let knob_radius: f32 = 22.0;
                            let knob_xs = [
                                modal_x + 50.0,
                                modal_x + 130.0,
                                modal_x + 210.0,
                                modal_x + 290.0,
                                modal_x + 370.0,
                            ];

                            for (k_idx, &kx) in knob_xs.iter().enumerate() {
                                let dist_sq = (x - kx).powi(2) + (y - knob_y).powi(2);
                                if dist_sq <= knob_radius.powi(2) {
                                    match k_idx {
                                        0 => self.drag = Some(DragTarget::SubKickVol),
                                        1 => self.drag = Some(DragTarget::SubKickLength),
                                        2 => self.drag = Some(DragTarget::SubKickDive),
                                        3 => self.drag = Some(DragTarget::SubKickSpeed),
                                        4 => self.drag = Some(DragTarget::SubKickOffset),
                                        _ => {}
                                    }
                                    cx.capture();
                                    meta.consume();
                                    return;
                                }
                            }
                        } else {
                            self.sub_kick_open = false;
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                    }

                    // Mixer strips interaction
                    let strip_w = 74.0;
                    let mixer_start_x = (WINDOW_W - 13.0 * strip_w) * 0.5;
                    let mixer_y = 60.0;

                    for (idx, &kit_piece) in KitPieceId::ALL.iter().enumerate() {
                        let sx = mixer_start_x + idx as f32 * strip_w;
                        if x >= sx && x <= sx + strip_w {
                            // Pitch knob
                            if y >= mixer_y + 5.0 && y <= mixer_y + 35.0 {
                                self.drag = Some(DragTarget::Pitch(kit_piece));
                                cx.capture();
                                meta.consume();
                                return;
                            }
                            // Pan knob
                            if y >= mixer_y + 40.0 && y <= mixer_y + 70.0 {
                                self.drag = Some(DragTarget::Pan(kit_piece));
                                cx.capture();
                                meta.consume();
                                return;
                            }
                            // Fader area
                            if y >= mixer_y + 80.0 && y <= mixer_y + 360.0 {
                                self.drag = Some(DragTarget::Fader(kit_piece));
                                cx.capture();
                                meta.consume();
                                return;
                            }
                            // Punch knob
                            if y >= mixer_y + 380.0 && y <= mixer_y + 430.0 {
                                self.drag = Some(DragTarget::Punch(kit_piece));
                                cx.capture();
                                meta.consume();
                                return;
                            }
                        }
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    if self.drag.is_some() {
                        self.drag = None;
                        cx.release();
                        cx.needs_redraw();
                    }
                }
                WindowEvent::MouseMove(..) => {
                    if let Some(drag) = self.drag {
                        let dy = mouse_y - prev_y;
                        let delta = -dy * 0.005;

                        match drag {
                            DragTarget::Pitch(kit_piece) => {
                                let param = &self.params.get_strip(kit_piece).pitch;
                                let norm = (param.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
                                self.emit_norm(cx, param.as_ptr(), norm);
                            }
                            DragTarget::Pan(kit_piece) => {
                                let param = &self.params.get_strip(kit_piece).pan;
                                let norm = (param.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
                                self.emit_norm(cx, param.as_ptr(), norm);
                            }
                            DragTarget::Punch(kit_piece) => {
                                let param = &self.params.get_strip(kit_piece).punch;
                                let norm = (param.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
                                self.emit_norm(cx, param.as_ptr(), norm);
                            }
                            DragTarget::Fader(kit_piece) => {
                                let strip = self.params.get_strip(kit_piece);
                                let ptr = match self.active_sof {
                                    None => strip.gain.as_ptr(),
                                    Some(MicChannel::Close) => strip.send_close.as_ptr(),
                                    Some(MicChannel::XY) => strip.send_xy.as_ptr(),
                                    Some(MicChannel::Mono) => strip.send_mono.as_ptr(),
                                    Some(MicChannel::Wide) => strip.send_wide.as_ptr(),
                                    Some(MicChannel::FrontMS) => strip.send_front_ms.as_ptr(),
                                    Some(MicChannel::Room) => strip.send_room.as_ptr(),
                                };
                                let norm = match self.active_sof {
                                    None => strip.gain.unmodulated_normalized_value(),
                                    Some(MicChannel::Close) => strip.send_close.unmodulated_normalized_value(),
                                    Some(MicChannel::XY) => strip.send_xy.unmodulated_normalized_value(),
                                    Some(MicChannel::Mono) => strip.send_mono.unmodulated_normalized_value(),
                                    Some(MicChannel::Wide) => strip.send_wide.unmodulated_normalized_value(),
                                    Some(MicChannel::FrontMS) => strip.send_front_ms.unmodulated_normalized_value(),
                                    Some(MicChannel::Room) => strip.send_room.unmodulated_normalized_value(),
                                };
                                self.emit_norm(cx, ptr, norm + delta);
                            }
                            DragTarget::SubKickVol => {
                                let p = &self.params.sub_kick.vol;
                                let norm = p.unmodulated_normalized_value() + delta;
                                self.emit_norm(cx, p.as_ptr(), norm);
                            }
                            DragTarget::SubKickLength => {
                                let p = &self.params.sub_kick.length;
                                let norm = p.unmodulated_normalized_value() + delta;
                                self.emit_norm(cx, p.as_ptr(), norm);
                            }
                            DragTarget::SubKickDive => {
                                let p = &self.params.sub_kick.dive;
                                let norm = p.unmodulated_normalized_value() + delta;
                                self.emit_norm(cx, p.as_ptr(), norm);
                            }
                            DragTarget::SubKickSpeed => {
                                let p = &self.params.sub_kick.speed;
                                let norm = p.unmodulated_normalized_value() + delta;
                                self.emit_norm(cx, p.as_ptr(), norm);
                            }
                            DragTarget::SubKickOffset => {
                                let p = &self.params.sub_kick.offset;
                                let norm = p.unmodulated_normalized_value() + delta;
                                self.emit_norm(cx, p.as_ptr(), norm);
                            }
                        }
                        cx.needs_redraw();
                    }
                }
                _ => {}
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }

        let light = prefs().light();
        let s = cx.scale_factor();
        let b = cx.bounds();
        let font = self.font.get();

        let mut draw = Draw::new(canvas, light, s, b.x, b.y, font);

        // Background
        draw.rect(0.0, 0.0, WINDOW_W, WINDOW_H, BG);

        // Top Header
        draw.rect(0.0, 0.0, WINDOW_W, 32.0, PANEL);
        draw.line(0.0, 32.0, WINDOW_W, 32.0, LINE, 1.0);

        // Header Title
        draw.text(16.0, 10.0, "SOUNDCHEF DRUMS", 13.0, TEXT);

        // Sub Kick Button
        let sub_btn_x = WINDOW_W - 200.0;
        let sub_active = self.sub_kick_open;
        draw.rounded_rect(
            sub_btn_x,
            4.0,
            90.0,
            24.0,
            4.0,
            if sub_active { LINE } else { BG },
        );
        draw.text(sub_btn_x + 16.0, 9.0, "SUB KICK", 11.0, TEXT);

        // Fader Lock Button
        let lock_btn_x = WINDOW_W - 100.0;
        let locked = self.params.fader_lock.value();
        draw.rounded_rect(
            lock_btn_x,
            4.0,
            90.0,
            24.0,
            4.0,
            if locked { LINE } else { BG },
        );
        draw.text(
            lock_btn_x + 12.0,
            9.0,
            if locked { "LOCK ON" } else { "LOCK OFF" },
            11.0,
            if locked { TEXT } else { MUTED },
        );

        // Mixer Channel Strips
        let strip_w = 74.0;
        let mixer_start_x = (WINDOW_W - 13.0 * strip_w) * 0.5;
        let mixer_y = 50.0;

        for (idx, &kit_piece) in KitPieceId::ALL.iter().enumerate() {
            let sx = mixer_start_x + idx as f32 * strip_w;
            let strip = self.params.get_strip(kit_piece);

            // Strip separator line
            draw.line(sx, mixer_y, sx, mixer_y + 440.0, LINE, 0.5);

            // Pitch Knob
            let pitch_val = strip.pitch.value();
            draw.text(sx + 10.0, mixer_y + 4.0, &format!("{:+.1} st", pitch_val), 10.0, MUTED);

            // Pan Knob
            let pan_val = strip.pan.value();
            draw.text(sx + 10.0, mixer_y + 36.0, &format!("{:.0}%", pan_val), 10.0, MUTED);

            // Fader Track
            let fader_track_x = sx + strip_w * 0.5 - 1.0;
            let fader_track_y = mixer_y + 70.0;
            let fader_track_h = 270.0;
            draw.rect(fader_track_x, fader_track_y, 2.0, fader_track_h, LINE);

            // Fader Handle with Kit Piece Name
            let gain_norm = match self.active_sof {
                None => strip.gain.unmodulated_normalized_value(),
                Some(MicChannel::Close) => strip.send_close.unmodulated_normalized_value(),
                Some(MicChannel::XY) => strip.send_xy.unmodulated_normalized_value(),
                Some(MicChannel::Mono) => strip.send_mono.unmodulated_normalized_value(),
                Some(MicChannel::Wide) => strip.send_wide.unmodulated_normalized_value(),
                Some(MicChannel::FrontMS) => strip.send_front_ms.unmodulated_normalized_value(),
                Some(MicChannel::Room) => strip.send_room.unmodulated_normalized_value(),
            };

            let handle_w = 60.0;
            let handle_h = 24.0;
            let handle_x = sx + (strip_w - handle_w) * 0.5;
            let handle_y = fader_track_y + (1.0 - gain_norm) * (fader_track_h - handle_h);

            // Handle color (Master = Text / SOF = Mic Color)
            let handle_color = match self.active_sof {
                None => TEXT,
                Some(mic) => MIC_COLORS[mic as usize],
            };

            draw.rounded_rect(handle_x, handle_y, handle_w, handle_h, 4.0, handle_color);
            draw.text(handle_x + 8.0, handle_y + 6.0, kit_piece.name(), 10.0, BG);

            // Punch Knob
            let punch_val = strip.punch.value();
            draw.text(sx + 10.0, mixer_y + 370.0, &format!("P {:+.2}", punch_val), 10.0, MUTED);
        }

        // Sends-On-Fader (SOF) Buttons
        let sof_w = 70.0;
        let sof_h = 32.0;
        let sof_spacing = 10.0;
        let total_sof_w = 6.0 * sof_w + 5.0 * sof_spacing;
        let sof_start_x = (WINDOW_W - total_sof_w) * 0.5;
        let sof_y = 530.0;

        for (i, mic) in MicChannel::ALL.iter().enumerate() {
            let bx = sof_start_x + i as f32 * (sof_w + sof_spacing);
            let is_active = self.active_sof == Some(*mic);

            let btn_color = if is_active {
                MIC_COLORS[*mic as usize]
            } else {
                PANEL
            };

            let text_color = if is_active {
                BG
            } else {
                TEXT
            };

            draw.rounded_rect(bx, sof_y, sof_w, sof_h, 4.0, btn_color);
            draw.text(bx + 12.0, sof_y + 10.0, mic.name(), 11.0, text_color);
        }

        // Sub Kick Modal Overlay
        if self.sub_kick_open {
            let modal_w = 440.0;
            let modal_h = 160.0;
            let modal_x = (WINDOW_W - modal_w) * 0.5;
            let modal_y = 180.0;

            draw.rounded_rect(modal_x, modal_y, modal_w, modal_h, 8.0, PANEL);
            draw.rounded_rect(modal_x + 1.0, modal_y + 1.0, modal_w - 2.0, modal_h - 2.0, 7.0, BG);

            draw.text(modal_x + 20.0, modal_y + 14.0, "SUB KICK SETTINGS", 12.0, TEXT);

            let knob_xs = [
                (modal_x + 50.0, "VOL", format!("{:.1} dB", self.params.sub_kick.vol.value())),
                (modal_x + 130.0, "LEN", format!("{:.0}%", self.params.sub_kick.length.value() * 100.0)),
                (modal_x + 210.0, "DIVE", format!("{:.1} st", self.params.sub_kick.dive.value())),
                (modal_x + 290.0, "SPEED", format!("{:.0}%", self.params.sub_kick.speed.value() * 100.0)),
                (modal_x + 370.0, "OFFSET", format!("{:.1} ms", self.params.sub_kick.offset.value())),
            ];

            for (kx, label, val_str) in knob_xs.iter() {
                let ky = modal_y + 60.0;
                draw.rounded_rect(*kx - 20.0, ky - 20.0, 40.0, 40.0, 20.0, PANEL);
                draw.text(*kx - 14.0, ky + 26.0, label, 10.0, MUTED);
                draw.text(*kx - 18.0, ky + 40.0, val_str, 9.0, TEXT);
            }
        }
    }
}

pub fn create(params: Arc<ScdParams>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            ScdEditorView::new(cx, params.clone())
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        },
    )
}
