mod events;
mod layout;
mod menu;
mod strips;
mod tooltip;

use crate::{
    network::{self, Network, CAPACITY},
    params::AutomixerParams,
};
use layout::*;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    widgets::util::ModifiersExt,
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport},
    load_font_once, param,
    pointer::ValuePress,
    point_in_rect,
    readout::{self, EditAction, PressAction},
    start_frame_timer,
    value_edit::ValueEdit,
    FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL, TEXT,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

pleasant_ui::appearance_store!("Automixer");

struct AutomixerView {
    params: Arc<AutomixerParams>,
    network: Arc<Network>,
    local_slot: Option<usize>,
    font: Cell<Option<FontId>>,
    hover: Option<(f32, f32)>,
    page: usize,
    selected: usize,
    // A click targets the strip that was drawn, even if the registry has since changed.
    bypass_buttons: Cell<[Option<BypassButton>; PER_PAGE]>,
    drawn_strips: Cell<[Option<DrawnStrip>; PER_PAGE]>,
    drag: Option<(Control, (f32, f32), f32)>,
    press: Option<ValuePress<Control>>,
    edit: Option<ValueEdit<Target>>,
    menu: Option<OpenMenu>,
    group_press: Option<usize>,
}

impl AutomixerView {
    fn new(params: Arc<AutomixerParams>, network: Arc<Network>, local_slot: Option<usize>) -> Self {
        let mut slots_buf = [0usize; CAPACITY];
        let count = Self::collect_slots(&network, &mut slots_buf);
        let page = slots_buf[..count]
            .iter()
            .position(|&index| Some(index) == local_slot)
            .unwrap_or(0)
            / PER_PAGE;
        let drawn_strips = Self::compute_drawn_strips(&network, page);
        Self {
            selected: params.mic_index(),
            params,
            network,
            local_slot,
            font: Cell::new(None),
            hover: None,
            page,
            bypass_buttons: Cell::new([None; PER_PAGE]),
            drawn_strips: Cell::new(drawn_strips),
            drag: None,
            press: None,
            edit: None,
            menu: None,
            group_press: None,
        }
    }

    fn parameter(&self, id: Control) -> Parameter<'_> {
        let p = &self.params.shared;
        match id {
            Control::Amount => Parameter::Float(&p.amount),
            Control::VoiceCap => Parameter::Float(&p.cap),
            Control::OtherCap => Parameter::Float(&p.other_cap),
            Control::Voices => Parameter::Int(&p.voices),
            Control::Threshold => Parameter::Float(&p.threshold),
            Control::Hold => Parameter::Float(&p.hold),
            Control::Fade => Parameter::Float(&p.fade),
            Control::Assist => Parameter::Float(&p.assist),
            Control::Mic => Parameter::Int(&self.params.mic),
            Control::Weight(i) => Parameter::Float(&p.mics[i].weight),
        }
    }
    fn normalized(&self, id: Control) -> f32 {
        if let Control::Weight(i) = id {
            (self.params.shared.mics[i].effective_weight() + 12.0) / 24.0
        } else {
            self.parameter(id).normalized()
        }
    }
    fn prepare_edit(&self, id: Control, normalized: f32) {
        if let Control::Weight(i) = id {
            let mic = &self.params.shared.mics[i];
            // Changed values clear MIDI from the parameter callback, when the
            // host actually applies them. A no-op base value has no callback.
            if mic.weight.preview_plain(normalized.clamp(0.0, 1.0)) == mic.weight.value() {
                mic.manual_edit();
            }
        }
    }
}

impl View for AutomixerView {
    fn element(&self) -> Option<&'static str> {
        Some("automixer-view")
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let editing = self.edit.is_some();
        self.handle_event(cx, event);
        pleasant_ui::value_edit::sync_text_input(cx, editing, self.edit.is_some());
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let font = load_font_once(canvas, &self.font, FONT_JETBRAINS_MONO);
        let b = cx.bounds();
        if let Some(mut d) = Draw::new_fitted(canvas, prefs().light(), (b.x, b.y, b.w, b.h), SIZE, font)
        {
            self.paint(&mut d);
        }
    }
}

pub fn create(
    params: Arc<AutomixerParams>,
    network: Arc<Network>,
    local_slot: Option<usize>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            AutomixerView::new(params.clone(), network.clone(), local_slot)
                .build(cx, |cx| {
                    start_frame_timer(cx, Duration::from_millis(33), |cx| cx.needs_redraw());
                })
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        },
    )
}

#[cfg(all(test, target_os = "macos"))]
#[path = "../ui_preview.rs"]
mod preview;
