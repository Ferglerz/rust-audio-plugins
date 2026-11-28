use super::{controls::*, *};

impl TapeStopView {
    pub(super) fn tooltip(&self) -> &'static str {
        if self.edit.is_some() {
            return "Type a value · Enter applies · Escape cancels";
        }
        let Some((x, y)) = self.hover else {
            return "Hover a control for help · Drag to adjust · Shift for fine adjustment";
        };
        for (rect, text) in [
            (position_cc_slot(), "Drag or type a CC (11 for expression). Move it to play the curve. Zero releases to live. Type Off to disable."),
            (midi_slot(), "Click the MIDI label to switch CC / Note; drag its number or click the value to type."),
            (axis_slot(0), "Crossfade: smooth the transition between live and slowed audio."),
            (axis_slot(1), "Stereo divergence: offset the left and right tape curves."),
            (trigger_column(), "Audio trigger: set the input level that restarts playback; use the power icon to bypass."),
        ] {
            if Self::inside(x, y, rect) { return text; }
        }
        for (rect, text) in [
            (THEME_BUTTON, "Cycle Dark, Light, and Auto appearance."),
            (AFFECT_MIDI, "Apply the tape slowdown and pitch changes to MIDI output as well as audio."),
            (TIME_SYNC, "Switch Drop and Return times between free timing and tempo-synced divisions."),
            (TEMPO_HEADER.source, "Switch between host tempo and the internal tempo."),
            (TEMPO_HEADER.bpm, "Internal tempo: drag to adjust or click the value to type."),
            (STOP_BUTTON, "Click to stop the tape; click again to return to normal playback."),
            (DROP_TIME_READOUT, "Drop time: click the value to type; Shift for fine adjustment."),
            (RETURN_TIME_READOUT, "Return time: click the value to type; Shift for fine adjustment."),
            (DROP_TIME_SLIDER, "Drop time: drag to set how long the tape takes to slow down."),
            (RETURN_TIME_SLIDER, "Return time: drag to set how long the tape takes to return to full speed."),
            (XY_AXIS, "Choose Curve or Return time for the pad's horizontal axis."),
            (XY_MODE, "Choose absolute pad positioning or relative dragging."),
            (XY_PAD, "Drag the pad: vertical movement adjusts Drop time; horizontal movement adjusts the chosen X parameter."),
            ((graph::GRAPH_X, graph::GRAPH_Y, graph::GRAPH_W, graph::GRAPH_H), "Drag the curve node to shape the tape slowdown; Shift for fine adjustment."),
        ] {
            if Self::inside(x, y, rect) { return text; }
        }
        "Hover a control for help · Drag to adjust · Shift for fine adjustment"
    }
}
