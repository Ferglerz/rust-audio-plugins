use super::*;

pub(super) fn create(params: Arc<FlatteryParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            FlatteryView::new(params.clone(), shared.clone())
                .build(cx, |cx| {
                    start_frame_timer(cx, FRAME_INTERVAL, |cx| cx.needs_redraw());
                })
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        },
    )
}

impl FlatteryView {
    pub(super) fn new(params: Arc<FlatteryParams>, shared: Arc<Shared>) -> Self {
        Self {
            params,
            shared,
            layout: GraphLayout::default(),
            font: Cell::new(None),
            label_font: Cell::new(None),
            display: RefCell::new(probe::DisplayFrame::default()),
            drag: None,
            hover_low_cut: false,
            hover_high_cut: false,
            hover_max_boost: false,
            hover_max_cut: false,
            hover_strength: None,
            hover_curve: None,
            hover_node: None,
            selected: None,
            mouse: (0.0, 0.0),
            hover: None,
            edit: None,
            value_press: None,
            graph_zoomed: false,
        }
    }
}
