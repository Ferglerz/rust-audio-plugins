use super::super::*;
use super::support::*;

fn pad_center() -> (f32, f32) {
    (PLAY_PAD.0 + PLAY_PAD.2 / 2.0, PLAY_PAD.1 + PLAY_PAD.3 / 2.0)
}
fn drain_commands(view: &ChordboardView) -> Vec<Command> {
    std::iter::from_fn(|| view.bridge.commands.pop()).collect()
}
#[test]
fn trackpad_latch_button_toggles_hover_strum_without_a_click() {
    let mut view = view(2, false);
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, STRUM_LATCH);
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.strum_latch.as_ptr(), 1.0))
    );

    let (x, y) = pad_center();
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert!(drain_commands(&view).is_empty());

    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert!(view.pad_hover);
    assert!(view.drag.is_none());
    let commands = drain_commands(&view);
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::BeginGesture(px, py) if (px - 0.5).abs() < 0.001 && (py - 0.5).abs() < 0.001)),
        "{commands:?}"
    );
    assert_eq!(
        changes.borrow().last().copied(),
        Some((view.params.y.as_ptr(), 0.5))
    );

    let nx = PLAY_PAD.0 + PLAY_PAD.2 * 0.75;
    event(
        &mut view,
        &mut cx,
        target,
        nx,
        y,
        WindowEvent::MouseMove(nx, y),
    );
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, value)| { *ptr == view.params.x.as_ptr() && (*value - 0.75).abs() < 0.001 }));
    assert!(drain_commands(&view)
        .iter()
        .all(|c| !matches!(c, Command::BeginGesture(_, _) | Command::EndGesture)));

    event(
        &mut view,
        &mut cx,
        target,
        PLAY_PAD.0 - 20.0,
        y,
        WindowEvent::MouseMove(PLAY_PAD.0 - 20.0, y),
    );
    assert!(!view.pad_hover);
    assert!(drain_commands(&view)
        .iter()
        .any(|c| matches!(c, Command::EndGesture)));
}
#[test]
fn trackpad_latch_hover_promotes_to_a_captured_drag_once() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    let (mut cx, target, _) = context();
    let (x, y) = pad_center();
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert_eq!(
        drain_commands(&view)
            .iter()
            .filter(|c| matches!(c, Command::BeginGesture(_, _)))
            .count(),
        1
    );
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::Pad)));
    assert!(!view.pad_hover);
    assert!(drain_commands(&view)
        .iter()
        .all(|c| !matches!(c, Command::BeginGesture(_, _) | Command::EndGesture)));
}
#[test]
fn trackpad_latch_skips_range_grips_and_still_lets_them_calibrate() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    let (mut cx, target, _) = context();
    let r = strum_bound_rect(0.0);
    let x = r.0 + r.2 / 2.0;
    let y = r.1 + r.3 / 2.0;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert!(!view.pad_hover);
    assert!(drain_commands(&view).is_empty());
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseDown(MouseButton::Left),
    );
    assert!(matches!(view.drag, Some(Drag::StrumBound(false, false))));
    assert!(drain_commands(&view).is_empty());
}
#[test]
fn trackpad_latch_hover_follows_an_expanded_strum_field() {
    let mut view = view(2, false);
    view.params = Arc::new(ChordboardParams {
        mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 0, max: 3 }),
        strum_latch: BoolParam::new("Trackpad latch", true),
        ..ChordboardParams::default()
    });
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.expand_elapsed = pleasant_ui::page_slide::DURATION;
    let (mut cx, target, _) = context();
    let play = view.play_pad();
    let x = play.0 + play.2 / 2.0;
    let y = play.1 + play.3 / 2.0;
    event(
        &mut view,
        &mut cx,
        target,
        x,
        y,
        WindowEvent::MouseMove(x, y),
    );
    assert_eq!(play, EXPANDED_PLAY_PAD);
    assert!(view.pad_hover);
    assert!(drain_commands(&view)
        .iter()
        .any(|c| matches!(c, Command::BeginGesture(_, _))));
}
#[test]
fn collapsed_strum_geometry_is_unchanged() {
    assert_eq!(pad_rect(0.0), PAD);
    assert_eq!(play_pad_rect(0.0), PLAY_PAD);
    assert_eq!(auto_field_rect(0.0), AUTO_FIELD);
    assert_eq!(expand_rect(0.0), EXPAND);
    assert_eq!(perf_surface_rect(0.0), PERF_SURFACE);
    assert_eq!(strum_sync_rect(0.0), STRUM_SYNC);
    assert_eq!(strum_latch_rect(0.0), STRUM_LATCH);
    let expand = expand_rect(0.0);
    let sync = strum_sync_rect(0.0);
    let latch = strum_latch_rect(0.0);
    assert!(
        expand.0 + expand.2 <= sync.0 || sync.0 + sync.2 <= expand.0,
        "expand button overlaps sweep sync"
    );
    assert!(
        expand.0 + expand.2 <= latch.0 || latch.0 + latch.2 <= expand.0,
        "expand button overlaps trackpad latch"
    );
}
#[test]
fn expanded_strum_field_fills_the_body_below_the_header() {
    let pad = pad_rect(1.0);
    let play = play_pad_rect(1.0);
    let performance = perf_surface_rect(1.0);
    assert_eq!(pad, EXPANDED_PAD);
    assert_eq!(play, EXPANDED_PLAY_PAD);
    assert!(performance.1 >= HEADER_H);
    assert!(pad.1 >= HEADER_H);
    assert!(pad.0 + pad.2 <= W);
    assert!(pad.1 + pad.3 <= H);
    assert!(play.0 >= pad.0 && play.0 + play.2 <= pad.0 + pad.2);
    assert!(play.1 >= pad.1 && play.1 + play.3 <= pad.1 + pad.3);
    let expand = expand_rect(1.0);
    let latch = strum_latch_rect(1.0);
    assert!(
        expand.0 + expand.2 <= latch.0 || latch.0 + latch.2 <= expand.0,
        "expanded expand button overlaps trackpad latch"
    );
    assert!(pad.2 > PAD.2);
    assert!(pad.3 > PAD.3);
    assert!(pad.0 < PAD.0);
    assert_eq!(shrink_width(CHORDS_SURFACE, 1.0).2, 0.0);
}
#[test]
fn strum_expand_slides_like_composure_pages() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    click(&mut view, &mut cx, target, expand_rect(0.0));
    assert_eq!(view.expand_target, 1.0);
    view.last_frame = Instant::now() - Duration::from_millis(100);
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert!(view.expand_progress > 0.0 && view.expand_progress < 1.0);
    let interrupted = view.expand_progress;
    let collapse = view.expand_button();
    click(&mut view, &mut cx, target, collapse);
    assert_eq!(view.expand_target, 0.0);
    assert_eq!(view.expand_start, interrupted);
    for _ in 0..3 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.expand_progress, 0.0);
}
#[test]
fn expanded_strum_consumes_covered_chord_clicks_and_keeps_the_play_pad() {
    let mut view = view(2, false);
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.expand_elapsed = pleasant_ui::page_slide::DURATION;
    let (mut cx, target, changes) = context();
    click(&mut view, &mut cx, target, key_rect(0));
    assert!(std::iter::from_fn(|| view.bridge.commands.pop())
        .all(|c| !matches!(c, Command::KeyDown(..))));
    let play = view.play_pad();
    click(&mut view, &mut cx, target, play);
    assert!(std::iter::from_fn(|| view.bridge.commands.pop())
        .any(|c| matches!(c, Command::BeginGesture(..))));
    assert!(changes
        .borrow()
        .iter()
        .any(|(ptr, _)| *ptr == view.params.x.as_ptr()));
}
#[test]
fn arp_mode_collapses_an_open_strum_field() {
    let mut view = view(2, false);
    let (mut cx, target, _) = context();
    view.expand_progress = 1.0;
    view.expand_target = 1.0;
    view.expand_elapsed = pleasant_ui::page_slide::DURATION;
    view.params = super::support::view(3, false).params.clone();
    view.last_frame = Instant::now();
    view.tick(&mut EventContext::new_with_current(&mut cx, target));
    assert_eq!(view.expand_target, 0.0);
    for _ in 0..3 {
        view.last_frame = Instant::now() - Duration::from_millis(100);
        view.tick(&mut EventContext::new_with_current(&mut cx, target));
    }
    assert_eq!(view.expand_progress, 0.0);
}
