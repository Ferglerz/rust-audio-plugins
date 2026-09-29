use super::*;

#[test]
fn view_scale_rejects_empty_and_non_finite_bounds() {
    assert_eq!(
        view_scale(BoundingBox {
            x: 0.0,
            y: 0.0,
            w: UI_W,
            h: UI_H
        }),
        Some(1.0)
    );
    assert!(view_scale(BoundingBox {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: UI_H
    })
    .is_none());
    assert!(view_scale(BoundingBox {
        x: 0.0,
        y: 0.0,
        w: UI_W,
        h: 0.0
    })
    .is_none());
    assert!(view_scale(BoundingBox {
        x: 0.0,
        y: 0.0,
        w: f32::NAN,
        h: UI_H
    })
    .is_none());
    assert!(view_scale(BoundingBox {
        x: 0.0,
        y: 0.0,
        w: f32::INFINITY,
        h: UI_H
    })
    .is_none());
    assert!(view_scale(BoundingBox {
        x: 0.0,
        y: 0.0,
        w: 4.0,
        h: 4.0
    })
    .is_none());
}

#[test]
fn hud_axis_row_sits_on_freq_axis_and_keeps_values_in_strip() {
    for range in SCALES {
        for freq in [20.0, 178.0, 1000.0, 20000.0] {
            for gain in -24..=24 {
                let b = Band {
                    freq,
                    gain: gain as f64,
                    ..Band::default()
                };
                let r = hud_rect_for(&b, range);
                assert!((r.1 - axis_strip_y()).abs() < 0.01);
                assert_eq!(r.3, AXIS_STRIP_H);
                assert!(r.0 >= GX && r.0 + r.2 <= GX + GW);
                assert!(r.1 >= GRAPH_BOTTOM);
                for i in 0..3 {
                    let value = hud_value_rect(&b, range, i);
                    assert!(inside(value.0, value.1, r));
                    assert!(inside(value.0 + value.2, value.1 + value.3, r));
                }
            }
        }
    }
    let wide = Band {
        freq: 1000.0,
        gain: 12.0,
        dynamic: true,
        ..Band::default()
    };
    let main = hud_rect_for(&wide, 24.0);
    assert_eq!(HUD_GAIN_W, 120.0);
    assert_eq!(HUD_Q_W, 84.0);
    assert_eq!(HUD_SHAPE_W, 172.0);
    assert_eq!(HUD_FREQ_W, 160.0);
    assert_eq!(
        HUD_DYN_FIELD_H,
        34.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA
    );
    assert_eq!(MODULE_TITLE_SIZE, 15.0);
    assert_eq!(HUD_LABEL_SIZE, MODULE_TITLE_SIZE);
    assert_eq!(HUD_VALUE_SIZE, HUD_LABEL_SIZE);
    assert_eq!(HUD_SHAPE_TEXT, HUD_LABEL_SIZE);
    assert_eq!(BandMenu::Shape.text_size(), HUD_SHAPE_TEXT);
    assert_eq!(BandMenu::Order.text_size(), HUD_LABEL_SIZE);
    assert_eq!(BandMenu::Shape.row_h(), dropdown_row_h(HUD_SHAPE_TEXT));
    assert_eq!(SCALE_BUTTON_TEXT, 13.0);
    assert_eq!(PROCESS_BUTTON_TEXT, 11.0);
    assert_eq!(HUD_DYN_TEXT, 15.0);
    assert!(HUD_BTN >= 32.0);
    assert_eq!(HUD_BYPASS, MODULE_HEADER_CTRL);
    assert_eq!(HUD_BYPASS, 24.0);
    for i in 1..5 {
        let field = hud_dyn_field_rect_at(&wide, 24.0, i, GX, GW);
        assert!(inside(field.0, field.1, main));
        assert!(inside(field.0 + field.2, field.1 + field.3, main));
        assert_eq!(
            field.3,
            if i == 1 {
                AXIS_STRIP_H
            } else {
                HUD_DYN_FIELD_H
            }
        );
        if i != 1 {
            assert!((field.1 + 16.0 - meter_value_y()).abs() < 0.01);
            assert!(field.1 + field.3 <= MODULE_Y + MODULE_H - 6.0);
        }
        let value = hud_dyn_value_rect_at(&wide, 24.0, i, GX, GW);
        assert!(inside(value.0, value.1, field));
        assert!(inside(value.0 + value.2, value.1 + value.3, field));
        assert!(value.0 > field.0 + 8.0);
    }
    let range = hud_dyn_field_rect_at(&wide, 24.0, 1, GX, GW);
    let attack = hud_dyn_field_rect_at(&wide, 24.0, 3, GX, GW);
    assert!(range.2 <= 116.0);
    assert!(range.2 < attack.2);
    for i in 0..3 {
        let value = hud_value_rect_at(&wide, 24.0, i, GX, GW);
        assert_eq!(value.1, main.1);
        assert_eq!(value.3, main.3);
    }
    let cog = hud_cog_rect_at(GX, GW, main.1, main.3);
    let dyn_r = hud_dyn_btn_rect_at(GX, GW, main.1, main.3);
    assert!(cog.0 + cog.2 <= dyn_r.0 - HUD_RIGHT_GAP + 0.01);
    assert!((dyn_r.0 + dyn_r.2 - (GX + GW)).abs() < 0.01);
    assert_eq!(dyn_r.2, HUD_DYN_W);
    assert_eq!(dyn_r.3, HUD_DYN_H);
    assert!((axis_strip_y() + AXIS_STRIP_H * 0.5 - EQ_AXIS_LABEL_Y).abs() < 0.01);
    let bell_baseline = EQ_AXIS_LABEL_Y + HUD_LABEL_SIZE * 0.32;
    assert!((hud_control_text_y() - bell_baseline).abs() < 0.01);
    assert!((meter_value_y() - bell_baseline).abs() < 0.01);
    let bypass = hud_bypass_rect(main.0, main.1);
    assert_eq!(bypass.2, HUD_BYPASS);
    assert_eq!(bypass.3, HUD_BYPASS);
    let shape = hud_shape_rect(main.0, main.1, main.2);
    assert!(
        ((shape.1 + shape.3 * 0.5 + HUD_SHAPE_TEXT * 0.32) - bell_baseline).abs() < 0.01,
        "Bell text Y is the HUD readout baseline"
    );
    let gap_top = GRAPH_BOTTOM;
    let gap_bot = MODULE_Y + MODULE_H;
    assert!((axis_strip_y() - (gap_top + (gap_bot - gap_top - AXIS_STRIP_H) * 0.5)).abs() < 0.01);
}

#[test]
fn module_settings_return_after_two_seconds_outside() {
    let away = Cell::new(None);
    let t0 = Instant::now();
    assert!(!module_settings_should_return(true, true, &away, t0));
    assert!(away.get().is_none());
    assert!(!module_settings_should_return(false, true, &away, t0));
    assert_eq!(away.get(), Some(t0));
    assert!(!module_settings_should_return(
        false,
        true,
        &away,
        t0 + Duration::from_millis(1999)
    ));
    assert!(module_settings_should_return(
        false,
        true,
        &away,
        t0 + Duration::from_secs(2)
    ));
    assert!(!module_settings_should_return(false, false, &away, t0));
    assert!(away.get().is_none());
}

#[test]
fn hud_cog_hidden_when_band_dynamics_off() {
    let off = Band {
        dynamic: false,
        ..Band::default()
    };
    let on = Band {
        dynamic: true,
        ..Band::default()
    };
    assert!(band_allows_dyn(&off));
    assert!(band_allows_dyn(&on));
    let g_off = hud_geom_for_at(&off, 24.0, GX, GW);
    let g_on = hud_geom_for_at(&on, 24.0, GX, GW);
    assert!(g_off.show_dyn_btn);
    assert!(!g_off.show_cog);
    assert!(g_on.show_dyn_btn);
    assert!(g_on.show_cog);
    // Reclaim cog width so FREQ/GAIN/Q/Bell have no dead click gap.
    assert!(g_off.bw > g_on.bw + HUD_COG * 0.5);
    let cog = hud_cog_rect_at(GX, GW, g_off.by, g_off.bh);
    let cx = cog.0 + cog.2 * 0.5;
    let cy = cog.1 + cog.3 * 0.5;
    assert!(!hud_cog_hit(g_off, cx, cy, GX, GW));
    assert!(hud_cog_hit(g_on, cx, cy, GX, GW));
    let (dx, dy) = hud_dyn_btn_center(g_off, GX, GW);
    assert!(hud_dyn_btn_hit(g_off, dx, dy, GX, GW));
}

#[test]
fn band_dyn_page_stays_only_for_bands_that_have_dyn() {
    let live = Band {
        dynamic: true,
        ..Band::default()
    };
    let quiet = Band::default();
    let cut = Band {
        shape: Shape::LowCut,
        dynamic: true,
        ..Band::default()
    };
    let sc = Band {
        id: SC_EQ_ID_BASE + 1,
        dynamic: true,
        ..Band::default()
    };
    assert!(band_keeps_dyn_page(Some(&live)));
    assert!(!band_keeps_dyn_page(Some(&quiet)));
    assert!(!band_keeps_dyn_page(Some(&cut)));
    assert!(!band_keeps_dyn_page(Some(&sc)));
    assert!(!band_keeps_dyn_page(None));
}

#[test]
fn hud_rect_for_lift_stays_in_graph() {
    for range in SCALES {
        for freq in [400.0, 1000.0, 8000.0, 20000.0] {
            for gain in [-100.0, -75.0, -50.0, -24.0, -12.0, 0.0] {
                let b = LiftBand {
                    freq,
                    gain,
                    ..LiftBand::default()
                };
                let r = hud_rect_for_lift(&b, range);
                assert!((r.1 - axis_strip_y()).abs() < 0.01);
                assert_eq!(r.3, AXIS_STRIP_H);
                assert!(r.0 >= GX && r.0 + r.2 <= GX + GW);
                assert!(r.1 >= GRAPH_BOTTOM);
                for i in 0..3 {
                    let value = hud_value_rect_lift(&b, range, i);
                    assert!(inside(value.0, value.1, r));
                    assert!(inside(value.0 + value.2, value.1 + value.3, r));
                }
            }
        }
    }
}

#[test]
fn dual_eq_animation_math_and_id_ranges() {
    assert_eq!(EQ2_ID_BASE, 10_000);
    assert_eq!(SC_EQ_ID_BASE, 20_000);
    assert!(EQ2_ID_BASE < SC_EQ_ID_BASE);
    assert!(SC_EQ_ID_BASE < LIFT_ID_BASE);
    assert_eq!(GRAPH_CLIP_W, EQ_W);
    assert_eq!(band_display_num(3), 3);
    assert_eq!(band_display_num(EQ2_ID_BASE + 2), 2);
    assert_eq!(band_display_num(SC_EQ_ID_BASE + 1), 1);

    let p0 = 0.0_f32;
    let eased0 = quintic_page_progress(p0);
    assert_eq!((0.0 - eased0) * GRAPH_CLIP_W, 0.0);
    assert_eq!((1.0 - eased0) * GRAPH_CLIP_W, GRAPH_CLIP_W);
    assert_eq!((2.0 - eased0) * GRAPH_CLIP_W, 2.0 * GRAPH_CLIP_W);

    let p1 = 1.0_f32;
    let eased1 = quintic_page_progress(p1);
    assert_eq!((0.0 - eased1) * GRAPH_CLIP_W, -GRAPH_CLIP_W);
    assert_eq!((1.0 - eased1) * GRAPH_CLIP_W, 0.0);
    assert_eq!((2.0 - eased1) * GRAPH_CLIP_W, GRAPH_CLIP_W);

    let p2 = 2.0_f32;
    let eased2 = quintic_page_progress(p2);
    assert_eq!((0.0 - eased2) * GRAPH_CLIP_W, -2.0 * GRAPH_CLIP_W);
    assert_eq!((1.0 - eased2) * GRAPH_CLIP_W, -GRAPH_CLIP_W);
    assert_eq!((2.0 - eased2) * GRAPH_CLIP_W, 0.0);
    assert_eq!((3.0 - eased2) * GRAPH_CLIP_W, GRAPH_CLIP_W);

    let p3 = 3.0_f32;
    let eased3 = quintic_page_progress(p3);
    assert_eq!((3.0 - eased3) * GRAPH_CLIP_W, 0.0);

    let p_mid = 0.5_f32;
    let eased_mid = quintic_page_progress(p_mid);
    assert!((eased_mid - 0.5).abs() < 1e-6);
    let p_sc_mid = 1.5_f32;
    let eased_sc = quintic_page_progress(p_sc_mid);
    assert!((eased_sc - 1.5).abs() < 1e-6);
}

fn test_view() -> StripView {
    let params = Arc::new(StripParams::default());
    let shared = Shared::new(
        params.bands.clone(),
        params.eq2_bands.clone(),
        params.sc_eq_bands.clone(),
        params.lift_bands.clone(),
    );
    StripView {
        params: params.clone(),
        shared,
        selected: None,
        dyn_page: Cell::new(DynPage::Main),
        pse_page: Cell::new(PsePage::Main),
        wall_page: Cell::new(WallPage::Main),
        band_dyn_page: Cell::new(false),
        drag: None,
        hover: None,
        shift_down: false,
        font: Cell::new(None),
        signature: Cell::new(None),
        graph_db: 24.0,
        scale_menu: false,
        processing_menu: None,
        edit: None,
        value_press: None,
        down: (0.0, 0.0),
        last_drag: (0.0, 0.0),
        pending_create: None,
        menu: None,
        active_eq: Cell::new(0),
        anim_progress: Cell::new(0.0),
        anim_target: Cell::new(0.0),
        anim_start: Cell::new(0.0),
        anim_time: Cell::new(1.0),
        last_tick: Cell::new(None),
        knee_bulge: Cell::new(0.0),
        pse_knee_bulge: Cell::new(0.0),
        dyn_anim_progress: Cell::new(0.0),
        dyn_anim_target: Cell::new(0.0),
        pse_anim_progress: Cell::new(0.0),
        pse_anim_target: Cell::new(0.0),
        wall_anim_progress: Cell::new(0.0),
        wall_anim_target: Cell::new(0.0),
        band_dyn_anim_progress: Cell::new(0.0),
        band_dyn_anim_target: Cell::new(0.0),
        dyn_away_since: Cell::new(None),
        pse_away_since: Cell::new(None),
        wall_away_since: Cell::new(None),
        pending_solo: None,
        alt_solo_restore: None,
        eq_bypass_anim: ButtonAnim::new(),
        pse_bypass_anim: ButtonAnim::new(),
        dyn_bypass_anim: ButtonAnim::new(),
        wall_bypass_anim: ButtonAnim::new(),
        dyn_band_anim: ButtonAnim::new(),
        band_bypass_anim: ButtonAnim::new(),
    }
}

#[test]
fn dynamics_routing_and_layout_geometry() {
    let mut view = test_view();

    // Post mode (default): PSE on the left, EQ in the middle, Dynamics on the right
    assert!(!view.is_pre());
    assert_eq!(view.pse_bounds(), (MARGIN, MODULE_Y, PSE_W, MODULE_H));
    assert_eq!(
        view.eq_bounds(),
        (MARGIN + PSE_W + GAP, MODULE_Y, EQ_W, MODULE_H)
    );
    assert_eq!(
        view.dyn_bounds(),
        (
            UI_W - MARGIN - WALL_W - GAP - DYN_W,
            MODULE_Y,
            DYN_W,
            MODULE_H
        )
    );
    assert_eq!(
        view.wall_bounds(),
        (UI_W - MARGIN - WALL_W, MODULE_Y, WALL_W, MODULE_H)
    );
    assert!(view.pse_bounds().0 + view.pse_bounds().2 <= view.eq_bounds().0);
    assert!(view.eq_bounds().0 + view.eq_bounds().2 <= view.dyn_bounds().0);
    assert!(view.dyn_bounds().0 + view.dyn_bounds().2 <= view.wall_bounds().0);
    assert_eq!(view.gx(), MARGIN + PSE_W + GAP + EQ_GRAPH_PAD_LEFT);
    assert_eq!(view.global_controls(), &[0]);
    assert_eq!(PSE_CONTROLS_KNOBS, [8, 10, 2]);

    let tab2 = view.eq_tab_2_rect();
    let tab_lift = view.eq_tab_lift_rect();
    let tab_sc = view.eq_tab_sc_rect();
    let bypass = view.eq_power_rect();
    assert_eq!(EQ_W, 866.0);
    assert_eq!(bypass.0 - view.eq_bounds().0, MODULE_HEADER_INSET);
    assert_eq!(
        view.pse_power_button_rect().0 - view.pse_bounds().0,
        MODULE_HEADER_INSET
    );
    assert_eq!(
        view.dyn_power_button_rect().0 - view.dyn_bounds().0,
        MODULE_HEADER_INSET
    );
    assert_eq!(
        view.wall_power_button_rect().0 - view.wall_bounds().0,
        MODULE_HEADER_INSET
    );
    let header_listen_sc = view.eq_header_listen_sc_rect();
    assert!(tab2.0 + tab2.2 < tab_lift.0);
    assert!(tab_lift.0 + tab_lift.2 < tab_sc.0);
    assert!(tab_sc.0 + tab_sc.2 < header_listen_sc.0);
    assert!(header_listen_sc.0 + header_listen_sc.2 <= view.eq_bounds().0 + view.eq_bounds().2);
    assert_eq!(tab_lift.1, tab2.1);
    assert_eq!(tab_sc.1, tab2.1);
    assert_eq!(bypass.1, tab2.1);
    assert_eq!(header_listen_sc.1, tab2.1);
    assert_eq!(header_listen_sc.3, MODULE_HEADER_CTRL);
    assert!((bypass.1 + bypass.3 * 0.5 - module_header_mid()).abs() < 0.01);
    assert_eq!(axis_db_text(12), "12");
    assert_eq!(axis_db_text(0), "0");
    assert_eq!(axis_db_text(-12), "-12");

    let slider = view.dyn_main_thresh_slider_rect();
    let gr_meter = view.dyn_main_gr_meter_rect();
    assert_eq!(gr_meter.0, slider.0 + slider.2 + 16.0);
    assert_eq!(gr_meter.1, slider.1);
    assert_eq!(gr_meter.3, slider.3);
    assert_eq!(slider.1, GY);
    assert_eq!(slider.1 + slider.3, GRAPH_BOTTOM);
    assert_eq!(gr_meter.1 + gr_meter.3, GRAPH_BOTTOM);
    assert_eq!(meter_value_y(), hud_control_text_y());
    assert!((meter_value_y() - (EQ_AXIS_LABEL_Y + HUD_LABEL_SIZE * 0.32)).abs() < 0.01);
    let knee_at_top = view.dyn_main_knee_rect(slider.1, 0.0);
    assert_eq!(knee_at_top.1, slider.1 - KNEE_METER_OVERHANG);
    assert!(knee_at_top.1 + knee_at_top.3 <= slider.1 + slider.3 + KNEE_METER_OVERHANG);
    let knee_at_bottom = view.dyn_main_knee_rect(slider.1 + slider.3, 0.0);
    assert_eq!(
        knee_at_bottom.1 + knee_at_bottom.3,
        slider.1 + slider.3 + KNEE_METER_OVERHANG
    );
    assert!(knee_at_bottom.1 >= slider.1 - KNEE_METER_OVERHANG);
    let pse_knee_at_top = view.pse_main_knee_rect(slider.1, 0.0);
    assert_eq!(pse_knee_at_top.1, slider.1 - KNEE_METER_OVERHANG);
    let cog = view.dyn_cog_button_rect();
    assert_eq!(cog.1, module_header_ctrl_y());
    assert_eq!(cog.1, bypass.1);
    assert_eq!(cog.3, MODULE_HEADER_CTRL);
    assert_eq!(
        cog.0,
        view.dyn_bounds().0 + DYN_W - MODULE_HEADER_INSET - 24.0
    );
    assert!(cog.0 > view.dyn_power_button_rect().0 + view.dyn_power_button_rect().2);
    let pse_cog = view.pse_cog_button_rect();
    assert_eq!(pse_cog.1, cog.1);
    assert_eq!(
        pse_cog.0,
        view.pse_bounds().0 + PSE_W - MODULE_HEADER_INSET - 24.0
    );
    let scale_btn = view.scale_button_rect();
    assert_eq!(
        scale_btn.0 + scale_btn.2 * 0.5,
        view.gx() - EQ_GRAPH_PAD_LEFT * 0.5
    );
    assert_eq!(scale_btn.1, GY - 10.0);
    assert_eq!(scale_btn.3, 20.0);
    assert_eq!(FREQ_AXIS[5], (2500.0, "2.5k"));
    assert!(FREQ_AXIS
        .iter()
        .all(|(f, label)| *f != 20000.0 && *label != "0" && *label != "20k"));
    let handle = view.dyn_main_thresh_handle_rect(slider.1 + slider.3 * 0.5);
    assert!(handle.2 <= slider.2 + 12.0);
    assert!(handle.3 >= 12.0);
    let knee = view.dyn_main_knee_rect(slider.1 + slider.3 * 0.5, 0.0);
    let knee_offset = view.dyn_main_knee_offset();
    assert_eq!(knee.1, handle.1 - knee_offset);
    assert_eq!(knee.3, handle.3 + 2.0 * knee_offset);

    let pse_slider = view.pse_main_thresh_slider_rect();
    let pse_gr = view.pse_main_gr_meter_rect();
    assert_eq!(pse_slider.1, slider.1);
    assert_eq!(pse_gr.0, pse_slider.0 + pse_slider.2 + 16.0);
    let depth_handle = view.dyn_main_depth_handle_rect(gr_meter.1 + gr_meter.3 * 0.5);
    assert!(inside(
        depth_handle.0 + 4.0,
        depth_handle.1 + 4.0,
        view.dyn_bounds()
    ));

    assert!(GRAPH_BOTTOM + 22.0 <= MODULE_Y + MODULE_H);
    assert!(MODULE_Y + MODULE_H < FOOTER_LINE_Y);
    assert!(FOOTER_LINE_Y < FOOTER_BTN_Y);
    assert!(FOOTER_BTN_Y + 28.0 <= UI_H);
    assert_eq!(HEADER_H, 82.0);

    let out_r = output_gain_rect();
    assert_eq!(
        out_r,
        (
            UI_W - MARGIN - WALL_W,
            FOOTER_BTN_Y,
            WALL_W,
            28.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA
        )
    );
    assert_eq!(THEME_BUTTON, (32.0, FOOTER_BTN_Y, 88.0, 28.0));
    assert_eq!(PROCESS_BUTTON.0, 128.0);
    let thresh_val = view.global_value_rect(0);
    assert_eq!(thresh_val, view.global_rect(0));
    assert!(thresh_val.1 + thresh_val.3 <= FOOTER_LINE_Y);
    assert!(
        (rect_center_x(thresh_val) - rect_center_x(slider)).abs() < 0.5,
        "DYN threshold value must sit under the meter"
    );
    let dyn_depth_val = view.global_value_rect(14);
    assert!(
        (rect_center_x(dyn_depth_val) - rect_center_x(gr_meter)).abs() < 0.5,
        "DYN depth value must sit under the GR meter"
    );
    let pse_thresh_val = view.pse_value_rect(1);
    assert!(
        (rect_center_x(pse_thresh_val) - rect_center_x(pse_slider)).abs() < 0.5,
        "PSE threshold value must sit under the meter"
    );
    assert!(view
        .value_at(out_r.0 + 20.0, out_r.1 + out_r.3 * 0.5)
        .is_none());
    let routing = view.footer_comp_routing_rect();
    assert!(view
        .value_at(routing.0 + routing.2 * 0.5, routing.1 + routing.3 * 0.5)
        .is_none());

    view.dyn_page.set(DynPage::Controls);
    assert_eq!(view.global_controls(), &[5, 11, 6, 3, 4]);
    let attack = view.global_rect(5);
    let release = view.global_rect(11);
    let ratio = view.global_rect(6);
    assert!(attack.2 > 90.0);
    assert!(attack.3 > 70.0);
    assert_eq!(attack.0, release.0);
    assert!(release.1 > attack.1 + attack.3 - 1.0);
    assert!(ratio.1 > release.1);
    assert!(view.global_rect(4).1 > view.global_rect(3).1);
    let routing = view.footer_comp_routing_rect();
    let auto = view.footer_auto_rect();
    let link = view.footer_link_rect();
    assert_eq!(auto.1, routing.1);
    assert_eq!(link.1, routing.1);
    assert!(routing.0 + routing.2 <= auto.0);
    assert!(auto.0 + auto.2 <= link.0);
    assert!((out_r.0 - (link.0 + link.2) - GAP).abs() < f32::EPSILON);
    assert!(
        !view.global_controls().contains(&2),
        "SC HPF must not appear on the compressor controls page"
    );

    view.pse_page.set(PsePage::Controls);
    assert_eq!(view.pse_controls(), &[8, 10, 2]);
    let time_r = view.pse_knob_rect(10);
    let det_r = view.pse_detect_mode_rect();
    assert_eq!(det_r.0 + det_r.2, time_r.0 + time_r.2);
    assert!(time_r.1 + time_r.3 <= det_r.1);
    assert!(inside(
        view.pse_bounds().0 + 20.0,
        view.pse_knob_rect(8).1 + 10.0,
        view.pse_bounds()
    ));

    view.wall_page.set(WallPage::Main);
    assert_eq!(view.wall_controls(), &[18]);
    let wall_slider = view.wall_main_thresh_slider_rect();
    assert_eq!(wall_slider.1, slider.1);
    assert_eq!(wall_slider.2, slider.2);
    assert_eq!(wall_slider.3, slider.3);
    let wall_handle = view.wall_main_thresh_handle_rect(wall_slider.1 + wall_slider.3 * 0.5);
    assert!(wall_handle.2 <= wall_slider.2 + 12.0);
    assert!(wall_handle.3 >= 12.0);
    assert!(inside(
        wall_slider.0 + 4.0,
        wall_slider.1 + 4.0,
        view.wall_bounds()
    ));
    let wall_thresh_val = view.wall_value_rect(18);
    assert_eq!(wall_thresh_val.1, meter_value_y() - 14.0);
    assert!(
        (rect_center_x(wall_slider) - rect_center_x(view.wall_bounds())).abs() < 0.5,
        "WALL meter must sit on the module midline"
    );
    assert!(
        (rect_center_x(wall_thresh_val) - rect_center_x(wall_slider)).abs() < 0.5,
        "WALL threshold value must sit under the meter"
    );
    view.wall_page.set(WallPage::Controls);
    assert_eq!(view.wall_controls(), &[16, 17]);
    let even_r = view.wall_knob_rect(16);
    let odd_r = view.wall_knob_rect(17);
    assert!(even_r.2 > 90.0);
    assert!(even_r.3 > 160.0);
    assert_eq!(even_r.0, odd_r.0);
    assert!(odd_r.1 > even_r.1 + even_r.3 - 1.0);
    assert!(inside(even_r.0 + 8.0, even_r.1 + 8.0, view.wall_bounds()));
    let wall_cog = view.wall_cog_button_rect();
    assert_eq!(wall_cog.1, cog.1);
    assert_eq!(
        wall_cog.0,
        view.wall_bounds().0 + WALL_W - MODULE_HEADER_INSET - 24.0
    );
    assert!(wall_cog.0 > view.wall_power_button_rect().0 + view.wall_power_button_rect().2);

    let params_pre = Arc::new(StripParams {
        comp_pre: BoolParam::new("Dynamics routing", true),
        ..StripParams::default()
    });
    view.params = params_pre;
    assert!(view.is_pre());
    assert_eq!(view.pse_bounds(), (MARGIN, MODULE_Y, PSE_W, MODULE_H));
    assert_eq!(
        view.dyn_bounds(),
        (MARGIN + PSE_W + GAP, MODULE_Y, DYN_W, MODULE_H)
    );
    assert_eq!(
        view.eq_bounds(),
        (MARGIN + PSE_W + GAP + DYN_W + GAP, MODULE_Y, EQ_W, MODULE_H)
    );
    assert_eq!(
        view.gx(),
        MARGIN + PSE_W + GAP + DYN_W + GAP + EQ_GRAPH_PAD_LEFT
    );
    assert_eq!(
        view.wall_bounds(),
        (UI_W - MARGIN - WALL_W, MODULE_Y, WALL_W, MODULE_H)
    );
    assert!(view.eq_bounds().0 + view.eq_bounds().2 <= view.wall_bounds().0);
    view.dyn_page.set(DynPage::Main);
    let pre_slider = view.dyn_main_thresh_slider_rect();
    assert_eq!(pre_slider.0, MARGIN + PSE_W + GAP + 30.0);
    assert_eq!(
        view.dyn_main_gr_meter_rect().0,
        pre_slider.0 + pre_slider.2 + 16.0
    );
}

#[test]
fn node_takes_priority_over_solo_close_gap() {
    let mut view = test_view();
    let b = Band {
        id: 1,
        freq: 1000.0,
        gain: -18.0,
        range: 1.0,
        dynamic: true,
        ..Band::default()
    };
    *view.params.bands.lock().unwrap() = vec![b.clone()];
    view.select(Some(b.id));
    let x = view.freq_x(b.freq);
    let y = db_y(b.gain, view.graph_db);
    let layout = band_chrome_layout(&b, view.graph_db, view.gx(), view.gw(), true);
    assert_eq!(layout, NodeChromeLayout::Flank);
    for (dx, dy) in [
        (0.0, 0.0),
        (-12.0, 0.0),
        (12.0, 0.0),
        (0.0, -12.0),
        (0.0, 12.0),
    ] {
        assert!(
            !view.over_selected_hud(x + dx, y + dy),
            "chrome must not block the node"
        );
    }
    // The narrow gap outside the grab circle still consumes background clicks.
    assert!(view.over_selected_hud(x + 17.0, y));
    let (solo, close) = node_chrome_rects(x, y, layout, view.gx(), view.gw());
    for r in [solo, close] {
        assert!(view.over_selected_hud(r.0 + r.2 * 0.5, r.1 + r.3 * 0.5));
    }
}

#[test]
fn graph_expands_on_release_near_either_endpoint() {
    let mut view = test_view();
    for (gain, range, dynamic, expand) in [
        (20.9, 0.0, false, false),
        (21.0, 0.0, false, true),
        (-21.0, 0.0, false, true),
        (6.0, 27.0, true, true),
        (-6.0, -27.0, true, true),
        (6.0, 27.0, false, false),
    ] {
        *view.params.bands.lock().unwrap() = vec![Band {
            id: 1,
            gain,
            range,
            dynamic,
            ..Band::default()
        }];
        view.set_graph_range(24.0);
        view.drag = Some(Target::Range(1));
        view.expand_graph_after_drag();
        assert_eq!(view.graph_db, if expand { 36.0 } else { 24.0 });
        assert_eq!(*view.params.graph_range.lock().unwrap(), view.graph_db);
    }
    view.params.bands.lock().unwrap()[0].gain = 6.0;
    view.set_graph_range(6.0);
    view.drag = Some(Target::Node(1));
    view.expand_graph_after_drag();
    assert_eq!(view.graph_db, 18.0);
    view.set_graph_range(72.0);
    view.params.bands.lock().unwrap()[0].gain = 72.0;
    view.expand_graph_after_drag();
    assert_eq!(view.graph_db, 72.0);
}

#[test]
fn lift_dock_is_above_the_graph_and_pse_controls_stack() {
    let view = test_view();
    for i in 0..5 {
        let r = view.band_rect(i);
        assert_eq!(r.1, GY);
        assert!(r.0 >= view.gx() && r.0 + r.2 <= view.gx() + view.gw());
        assert!(r.1 + r.3 <= GY + LIFT_DOCK_H);
    }
    let hysteresis = view.pse_knob_rect(8);
    let time = view.pse_knob_rect(10);
    let mode = view.pse_detect_mode_rect();
    let voice = view.pse_knob_rect(2);
    let meter = view.pse_vad_meter_rect();
    assert_eq!(hysteresis.0, time.0);
    assert_eq!(time.0, voice.0);
    assert!(hysteresis.1 + hysteresis.3 <= time.1);
    assert!(time.1 + time.3 <= mode.1);
    assert!(mode.1 + mode.3 <= voice.1);
    assert!(voice.1 + voice.3 <= meter.1);
    assert!(meter.1 + meter.3 + 18.0 < MODULE_Y + MODULE_H);
}

#[test]
fn readout_underlines_follow_text_and_slider_tracks_leave_clearance() {
    let view = test_view();
    for page in [PsePage::Controls] {
        view.pse_page.set(page);
        for &i in view.pse_controls() {
            let r = view.pse_knob_rect(i);
            let text = pleasant_ui::draw::KnobLayout::new(r).value_y;
            assert_eq!(view.value_baseline(ValueTarget::Global(i)), text);
            assert!(pleasant_ui::value_edit::value_underline_y(text) + 0.5 < r.1 + r.3);
        }
    }
    view.dyn_page.set(DynPage::Controls);
    for &i in view.global_controls() {
        let r = view.global_rect(i);
        let text = pleasant_ui::draw::KnobLayout::new(r).value_y;
        assert_eq!(view.value_baseline(ValueTarget::Global(i)), text);
        assert!(pleasant_ui::value_edit::value_underline_y(text) + 0.5 < r.1 + r.3);
    }
    let b = Band {
        dynamic: true,
        ..Band::default()
    };
    for i in 2..5 {
        let r = hud_dyn_field_rect_at(&b, 24.0, i, GX, GW);
        let underline = pleasant_ui::value_edit::value_underline_y(hud_control_text_y());
        let track_y = r.1 + 24.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA;
        assert!(track_y - underline >= 3.0);
        assert!(track_y + 5.0 < r.1 + r.3);
    }
}

#[test]
fn command_node_drag_moves_range_endpoint_and_preserves_gain_and_ratio() {
    for dynamic in [false, true] {
        for direction in [-1.0, 1.0] {
            let mut view = test_view();
            let band = Band {
                id: 1,
                gain: 6.0,
                range: 8.0,
                dynamic,
                ratio: 4.0,
                ..Band::default()
            };
            *view.params.bands.lock().unwrap() = vec![band.clone()];
            view.select(Some(1));
            let x = view.freq_x(band.freq);
            let y = db_y(band.gain, view.graph_db);
            view.drag = Some(Target::Node(1));
            view.last_drag = (x, y);
            let mut context = Context::default();
            let mut cx = EventContext::new(&mut context);
            let dy = GH * 0.1 * direction;
            view.apply_drag(&mut cx, x + 20.0, y + dy, false, true, false);
            let changed = view.find_band(1).unwrap();
            let initial_endpoint = if dynamic {
                band.gain - band.range
            } else {
                band.gain
            };
            let endpoint_delta = -(dy as f64) * 2.0 * view.graph_db / GH as f64;
            assert!(
                (changed.gain - changed.range - initial_endpoint - endpoint_delta).abs() < 1e-5
            );
            assert!(changed.dynamic);
            assert_eq!(changed.gain, band.gain);
            assert_eq!(changed.freq, band.freq);
            assert_eq!(changed.ratio, band.ratio);
            assert_eq!(changed.threshold, band.threshold);
        }
    }
}
