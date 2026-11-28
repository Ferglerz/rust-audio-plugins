use super::*;

fn view(model: PianoType, era: Era) -> PianoView {
    let mut p = OpenWurliUiParams {
        piano: EnumParam::new("Piano", model),
        ..Default::default()
    };
    p.stage.era = EnumParam::new("Era", era);
    let keyboard = Rc::new(PlayableKeyboard::new(p.keyboard.clone(), || {}));
    PianoView {
        slide: Slide::new(model),
        params: Arc::new(p),
        keyboard,
        strikes: Rc::default(),
        font: Cell::default(),
        labels: Cell::default(),
        hover: None,
        drag: None,
        era_open: false,
        hohner_open: false,
        regulation_open: false,
    }
}

#[test]
fn cp70b_diagram_and_model_buttons_follow_the_selected_grand() {
    let mut v = view(PianoType::Grand, Era::LateI);
    Arc::get_mut(&mut v.params).unwrap().grand_model =
        EnumParam::new("Grand Model", GrandModel::Cp70B);
    for model in [GrandModel::Cp80, GrandModel::Cp70B] {
        let (x, y, w, h) = grand_model_button(model);
        assert_eq!(
            v.hit(x + w / 2., y + h / 2.),
            Some(Target::GrandModel(model))
        );
    }
    assert_eq!(v.hit(220., 285.), Some(Target::OriginalCp70));
    assert_eq!(v.hit(267., 285.), None);
    Arc::get_mut(&mut v.params).unwrap().grand_70_original = BoolParam::new("Original CP-70", true);
    assert_eq!(v.hit(620., 195.), Some(Target::String(StringPart::Piezo)));
    assert!(v.help(v.hit(620., 195.)).contains("own lead"));
    assert_eq!(v.hit(327., 349.), None);
    assert_eq!(v.hit(554., 166.), Some(Target::String(StringPart::Piezo)));
}

#[test]
fn controls_and_era_dropdown_remain_reachable_in_short_hosts() {
    for model in PianoType::ALL {
        for era in [Era::Sparkletop, Era::LateI] {
            let mut v = view(model, era);
            for bounds in [(WIDTH, HEIGHT), (720., 340.), (360., 550.)] {
                let fit = EditorViewport::fit((0., 0., bounds.0, bounds.1), SIZE).unwrap();
                let check = |view: &PianoView, r: Rect, want: Target| {
                    assert!(r.0 >= 0. && r.1 >= 0. && r.0 + r.2 <= WIDTH && r.1 + r.3 <= HEIGHT);
                    let (x, y) = fit.to_local(
                        fit.x + (r.0 + r.2 * 0.5) * fit.scale,
                        fit.y + (r.1 + r.3 * 0.5) * fit.scale,
                    );
                    assert_eq!(view.hit(x, y), Some(want));
                };
                for i in 0..3 {
                    check(&v, tab(i), Target::Piano(i));
                }
                for &k in visible(model, era, false) {
                    check(&v, k.rect(), Target::Dial(k));
                }
                check(&v, Knob::Volume.rect(), Target::Dial(Knob::Volume));
                check(&v, EXTENDED, Target::Extended);
                check(&v, THEME, Target::Theme);
                if model == PianoType::Stage {
                    check(&v, ERA, Target::Era);
                    v.era_open = true;
                    for i in 0..6 {
                        check(&v, era_row(i), Target::EraChoice(i));
                    }
                    v.era_open = false;
                }
                if model == PianoType::Wurli {
                    for ((x, y), k) in [
                        ((234., 185.), Knob::HammerHardness),
                        ((314., 156.), Knob::ReedDecay),
                        ((546., 151.), Knob::PickupHeight),
                    ] {
                        assert_eq!(v.hit(x, y), Some(Target::Reed(k)));
                    }
                }
            }
        }
    }
}

#[test]
fn output_switches_show_the_controls_for_the_selected_instrument() {
    let mut stage = view(PianoType::Stage, Era::LateI);
    assert_eq!(stage.hit(260., 285.), Some(Target::Peterson));
    Arc::get_mut(&mut stage.params).unwrap().stage.peterson =
        BoolParam::new("Peterson Suitcase", true);
    for k in [Knob::StageVolume, Knob::Cable] {
        let (x, y, w, h) = k.rect();
        assert_ne!(stage.hit(x + w / 2., y + h / 2.), Some(Target::Dial(k)));
    }
    for k in [Knob::VibratoSpeed, Knob::Intensity] {
        let (x, y, w, h) = k.rect();
        assert_eq!(stage.hit(x + w / 2., y + h / 2.), Some(Target::Dial(k)));
    }
    assert_ne!(
        view(PianoType::Stage, Era::Sparkletop).hit(260., 285.),
        Some(Target::Peterson)
    );
    assert_eq!(
        view(PianoType::Grand, Era::LateI).hit(GRAND_FRAME.0 + GRAND_FRAME.2 / 2., 285.),
        Some(Target::FrameRing)
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "offscreen editor previews in the temporary directory"]
fn render_piano_previews() {
    let mut gl = pleasant_ui::preview::OffscreenGl::new();
    for (model, era, menu, peterson, grand) in [
        (PianoType::Wurli, Era::LateI, false, false, GrandModel::Cp80),
        (PianoType::Stage, Era::LateI, false, false, GrandModel::Cp80),
        (PianoType::Stage, Era::LateI, false, true, GrandModel::Cp80),
        (
            PianoType::Stage,
            Era::Sparkletop,
            true,
            false,
            GrandModel::Cp80,
        ),
        (PianoType::Grand, Era::LateI, false, false, GrandModel::Cp80),
        (
            PianoType::Grand,
            Era::LateI,
            false,
            false,
            GrandModel::Cp70B,
        ),
    ] {
        let mut v = view(model, era);
        v.era_open = menu;
        let params = Arc::get_mut(&mut v.params).unwrap();
        params.stage.peterson = BoolParam::new("Peterson Suitcase", peterson);
        params.grand_model = EnumParam::new("Grand Model", grand);
        load_font_once(gl.canvas_mut(), &v.font, FONT_JETBRAINS_MONO);
        let labels = load_font_once(
            gl.canvas_mut(),
            &v.labels,
            nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
        );
        let revisions: &[bool] = if grand == GrandModel::Cp70B {
            &[false, true]
        } else {
            &[false]
        };
        for &original in revisions {
            Arc::get_mut(&mut v.params).unwrap().grand_70_original =
                BoolParam::new("Original CP-70", original);
            for light in [false, true] {
                let path = std::env::temp_dir().join(format!(
                    "electric-pianos-{model:?}-{era:?}{}{}-{light}.ppm",
                    if peterson { "-peterson" } else { "" },
                    if original {
                        "-cp70"
                    } else if grand == GrandModel::Cp70B {
                        "-cp70b"
                    } else {
                        ""
                    }
                ));
                gl.render_ppm(WIDTH as u32, HEIGHT as u32, path, |c| {
                    let mut d =
                        Draw::new_fitted(c, light, (0., 0., WIDTH, HEIGHT), SIZE, v.font.get())
                            .unwrap();
                    d.with_readable_style(labels, |d| v.paint(d));
                });
            }
        }
    }
}

#[test]
fn tab_slide_can_reverse_or_retarget_without_jump() {
    use std::time::Duration;
    let mut v = view(PianoType::Wurli, Era::LateI);
    let start = Instant::now();
    let half = Duration::from_secs_f32(pleasant_ui::page_slide::DURATION / 2.);
    let layers = |slide: &Slide| {
        slide
            .panels()
            .filter_map(|(page, x)| {
                if let Instrument::Piano(p) = page {
                    Some((p, x))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    assert!(v.slide.select(PianoType::Grand, start));
    v.slide.tick(start + half);
    assert_eq!(
        layers(&v.slide),
        [(PianoType::Wurli, 0.), (PianoType::Grand, 0.5)]
    );
    assert_eq!(v.hit(45., 440.), Some(Target::Dial(Knob::Volume)));
    assert_eq!(v.hit(300., 500.), Some(Target::Keyboard));
    assert_eq!(v.hit(tab(2).0 + 20., 24.), Some(Target::Piano(2)));
    assert_eq!(v.hit(350., 340.), None);
    assert!(v.slide.select(PianoType::Stage, start + half));
    v.slide.tick(start + half);
    assert_eq!(
        layers(&v.slide),
        [
            (PianoType::Wurli, 0.),
            (PianoType::Grand, 0.5),
            (PianoType::Stage, -1.)
        ]
    );
    v.slide.tick(start + 2 * half);
    let partial = layers(&v.slide);
    assert!(v.slide.select(PianoType::Wurli, start + 2 * half));
    assert_eq!(partial, layers(&v.slide));
    v.slide.tick(start + 4 * half);
    assert_eq!(layers(&v.slide), [(PianoType::Wurli, 0.)]);
    assert!(!v.slide.moving());
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "render actual tab animation frames to the temporary directory"]
fn render_tab_animation() {
    use std::time::Duration;
    let mut gl = pleasant_ui::preview::OffscreenGl::new();
    let mut v = view(PianoType::Wurli, Era::LateI);
    load_font_once(gl.canvas_mut(), &v.font, FONT_JETBRAINS_MONO);
    let labels = load_font_once(
        gl.canvas_mut(),
        &v.labels,
        nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
    );
    let dir = std::env::temp_dir().join("electric-pianos-tab-animation");
    std::fs::create_dir_all(&dir).unwrap();
    let start = Instant::now();
    for (leg, model) in [PianoType::Stage, PianoType::Grand, PianoType::Wurli]
        .into_iter()
        .enumerate()
    {
        Arc::get_mut(&mut v.params).unwrap().piano = EnumParam::new("Piano", model);
        let t = start + Duration::from_secs(leg as u64);
        v.slide.select(model, t);
        for frame in 0..=15 {
            v.slide
                .tick(t + Duration::from_secs_f64(frame as f64 / 60.));
            gl.render_ppm(
                WIDTH as u32,
                HEIGHT as u32,
                dir.join(format!("{leg}-{frame:02}.ppm")),
                |c| {
                    let mut d =
                        Draw::new_fitted(c, false, (0., 0., WIDTH, HEIGHT), SIZE, v.font.get())
                            .unwrap();
                    d.with_readable_style(labels, |d| v.paint(d));
                },
            );
        }
    }
}

#[test]
fn pickup_and_extended_controls_follow_each_keyboard() {
    for model in PianoType::ALL
        .into_iter()
        .map(Instrument::Piano)
        .chain(Instrument::extra_models())
    {
        let mut view = view(PianoType::Wurli, Era::LateI);
        let p = Arc::get_mut(&mut view.params).unwrap();
        p.select_model(model);
        if let Instrument::Piano(piano) = model {
            p.piano = EnumParam::new("Piano", piano);
        }
        p.extended_notes = BoolParam::new("Extended Notes", true);
        assert_eq!(p.key_range(), 0..=127);
        view.slide = Slide::new(model);
        assert_eq!(
            view.hit(EXTENDED.0 + 20., EXTENDED.1 + 12.),
            Some(Target::Extended)
        );
        assert_eq!(
            view.hit(regulation::BUTTON.0 + 30., regulation::BUTTON.1 + 16.),
            Some(Target::PickupMenu)
        );
        use crate::regulation::Adjustment::{Gap, Bridge, ReedGap};
        match model {
            Instrument::Vibanet => assert_eq!(crate::regulation::adjustments(model), &[Gap, Bridge]),
            Instrument::Road => assert_eq!(crate::regulation::adjustments(model), &[Gap, Bridge, ReedGap]),
            _ => {}
        }
        if model == Instrument::Piano(PianoType::Grand) {
            assert!(view.help(Some(Target::PickupMenu)).contains("bonded"));
            assert!(crate::regulation::adjustments(model).is_empty());
        } else {
            view.regulation_open = true;
            for &kind in crate::regulation::adjustments(model) {
                let (x, y, w, h) = regulation::row(model, kind);
                let target = Target::PickupSetting(model, kind);
                assert_eq!(view.hit(x + w / 2., y + h / 2.), Some(target));
                assert_eq!(target.knob().unwrap().rect(), (x, y, w, h));
                let field = target.knob().unwrap().param(&view.params);
                if kind == crate::regulation::Adjustment::PadReturn {
                    assert_eq!(field.value(), 1.);
                    assert!(field
                        .normalized_value_to_string(field.unmodulated_normalized_value(), true)
                        .contains('%'));
                } else {
                    assert_eq!(field.value(), 0.);
                }
            }
        }
    }
}

#[test]
fn hohner_selectors_and_action_targets_are_reachable() {
    let mut v = view(PianoType::Grand, Era::LateI);
    for (i, page) in Instrument::extra_models().enumerate() {
        v.hohner_open = true;
        let (x, y, w, h) = hohner::menu_row(i);
        assert_eq!(
            v.hit(x + w / 2., y + h / 2.),
            Some(Target::HohnerChoice(page))
        );
        v.hohner_open = false;
        Arc::get_mut(&mut v.params).unwrap().select_model(page);
        v.slide = Slide::new(page);
        let key = if page == Instrument::Vv64 {
            use vv64::Part;
            for part in [Part::Volume, Part::Bass, Part::Treble, Part::Sustain] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Vv64(part)));
            }
            (180., 155.)
        } else if page == Instrument::StereoVibe {
            use stereovibe::Part;
            for part in [
                Part::Volume,
                Part::Bass,
                Part::Treble,
                Part::Rate,
                Part::Depth,
                Part::Tremolo,
                Part::Half,
                Part::Input,
                Part::Sustain,
            ] {
                let (x, y, w, h) = part.rect();
                assert_eq!(
                    v.hit(x + w / 2., y + h / 2.),
                    Some(Target::StereoVibe(part))
                );
            }
            (180., 155.)
        } else if page == Instrument::Road {
            use road::Part;
            for part in [Part::Jack, Part::Board, Part::Pedal] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Road(part)));
            }
            (180., 155.)
        } else if page == Instrument::Baldwin {
            use baldwin::Part;
            for part in [Part::Volume, Part::Soft, Part::Sustain] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Baldwin(part)));
            }
            (180., 155.)
        } else if page == Instrument::Mk8 {
            use mk8::Part;
            for part in [
                Part::Volume,
                Part::Drive,
                Part::Envelope,
                Part::Low,
                Part::Mid,
                Part::High,
                Part::Frequency,
                Part::Resonance,
                Part::Rate,
                Part::Depth,
                Part::Enable,
                Part::Wave,
                Part::Sustain,
            ] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Mk8(part)));
            }
            (180., 155.)
        } else if page == Instrument::Vibanet {
            use vibanet::Part;
            for part in [
                Part::Volume,
                Part::Sensitivity,
                Part::Mute,
                Part::Wah,
                Part::Eq(1, 0),
                Part::Pickup(2),
            ] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Vibanet(part)));
            }
            (180., 155.)
        } else if page == Instrument::Valente {
            use valente::Part;
            for part in [Part::Volume, Part::Brilliance, Part::Sustain] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Valente(part)));
            }
            (180., 155.)
        } else if page == Instrument::Vivi {
            use vivi::Part;
            for part in [Part::Volume, Part::Expression, Part::Bass, Part::Sustain] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Vivi(part)));
            }
            (180., 155.)
        } else if page == Instrument::Elepian {
            use elepian::Part;
            for part in [Part::Volume, Part::Tone, Part::Alignment, Part::Sustain] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Elepian(part)));
            }
            (180., 155.)
        } else if page == Instrument::Neo {
            (180., 150.)
        } else if let Instrument::Claviset(which) = page {
            use claviset::Part;
            let parts: Vec<_> = match which {
                pianet_n::claviset::Instrument::Claviset200 => (0..9)
                    .map(Part::Register)
                    .chain((0..4).map(Part::Modulation))
                    .chain([Part::FootSwell, Part::Power, Part::Sustain])
                    .collect(),
                pianet_n::claviset::Instrument::Pianotron => {
                    vec![Part::KneeSwell, Part::Tone, Part::Depth, Part::Rate]
                }
            };
            for part in parts {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Claviset(part)));
            }
            (192., 140.)
        } else if page == Instrument::Electra {
            assert_eq!(v.hit(187., 128.), Some(Target::ElectraFork));
            assert_eq!(v.hit(510., 160.), Some(Target::ElectraPickup));
            assert_eq!(v.hit(120., 320.), Some(Target::ElectraSoft));
            assert_eq!(v.hit(120., 364.), Some(Target::ElectraSustain));
            (309., 200.)
        } else if matches!(page, Instrument::Clavinet(_)) {
            (522., 125.)
        } else {
            (420., 140.)
        };
        let target = if matches!(page, Instrument::Reed(m) if m.is_cembalet()) {
            assert_eq!(v.hit(435., 199.), Some(Target::CembaletPlectrum));
            assert_eq!(v.hit(490., 172.), Some(Target::CembaletElectrode));
            Target::CembaletDamper
        } else {
            Target::HohnerKey
        };
        assert_eq!(v.hit(key.0, key.1), Some(target));
        if page == Instrument::Storytone {
            for part in [
                storytone::Part::Optimum,
                storytone::Part::Manual,
                storytone::Part::Swell,
                storytone::Part::Soft,
                storytone::Part::Sustain,
            ] {
                let (x, y, w, h) = part.rect();
                assert_eq!(v.hit(x + w / 2., y + h / 2.), Some(Target::Storytone(part)));
                if let Some(knob) = part.level() {
                    assert_eq!(knob.rect(), part.rect());
                    assert!((part.slider_value(x + w / 2.) - 0.5).abs() < 1e-6);
                }
            }
        }
        if page == Instrument::Duo {
            assert_eq!(v.hit(64., 326.), Some(Target::Duo(duo::Part::Two)));
            assert_eq!(v.hit(255., 326.), Some(Target::Duo(duo::Part::Foot)));
            assert_eq!(v.hit(569., 371.), Some(Target::Duo(duo::Part::Recording)));
        }
        if let Instrument::Reed(m @ (pianet_n::Model::C | pianet_n::Model::L)) = page {
            assert_eq!(v.hit(100., 357.), Some(Target::EarlyLocalOut));
            assert_eq!(
                v.hit(478., 287.),
                if m == pianet_n::Model::C {
                    Some(Target::PianetVibrato)
                } else {
                    None
                }
            );
        }
        if page == Instrument::Clavinet(ClavinetModel::E7) {
            assert_eq!(v.hit(640., 289.), Some(Target::E7Power));
            assert_eq!(v.hit(640., 350.), Some(Target::E7Volume));
        }
        assert_eq!(
            v.hit(EXTENDED.0 + 30., EXTENDED.1 + 14.),
            Some(Target::Extended)
        );
        if matches!(page, Instrument::Clavinet(_)) {
            for (i, mode) in Pickup::ALL.into_iter().enumerate() {
                assert_eq!(
                    v.hit(50. + 82. * i as f32, 310.),
                    Some(Target::Pickup(mode))
                );
                assert_eq!(v.hit(50. + 82. * i as f32, 364.), Some(Target::ToneTab(i)));
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "offscreen production editor previews"]
fn render_hohner_previews() {
    let mut gl = pleasant_ui::preview::OffscreenGl::new();
    for page in Instrument::extra_models() {
        let mut v = view(PianoType::Wurli, Era::LateI);
        Arc::get_mut(&mut v.params).unwrap().select_model(page);
        v.slide = Slide::new(page);
        load_font_once(gl.canvas_mut(), &v.font, FONT_JETBRAINS_MONO);
        let labels = load_font_once(
            gl.canvas_mut(),
            &v.labels,
            nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
        );
        let previews: &[(bool, bool, &str)] = if page == Instrument::Duo {
            &[
                (false, false, ""),
                (false, false, "-pickups"),
                (false, false, "-sustain"),
            ]
        } else if page == Instrument::Mk8 {
            &[
                (false, false, ""),
                (false, false, "-fx"),
                (false, false, "-alt"),
            ]
        } else if page == Instrument::Storytone {
            &[
                (false, false, ""),
                (false, true, "-light"),
                (true, false, "-menu"),
            ]
        } else if v.params.sustain_mod.for_model(page).is_some() {
            &[(false, false, ""), (false, false, "-sustain")]
        } else {
            &[(false, false, "")]
        };
        for &(menu, light, suffix) in previews {
            v.hohner_open = menu;
            v.regulation_open = suffix == "-pickups";
            if matches!(suffix, "-fx" | "-alt") {
                let fx = &mut Arc::get_mut(&mut v.params).unwrap().mk8.fx;
                fx.fitted = BoolParam::new("Factory FX Board", true);
                fx.set_page(if suffix == "-fx" { 1 } else { 2 });
            }
            if suffix == "-sustain" {
                let c = &mut Arc::get_mut(&mut v.params).unwrap().sustain_mod;
                for p in [&mut c.n, &mut c.c, &mut c.l, &mut c.t, &mut c.duo] {
                    p.enabled = BoolParam::new("Sustain Mod", true);
                    p.pedal = BoolParam::new("Sustain Pedal", true);
                }
                // Key is up while the pedal holds the pad above middle C.
                c.publish_catches(1u128 << 60);
            }
            let path = std::env::temp_dir().join(format!("electric-pianos-{page:?}{suffix}.ppm"));
            gl.render_ppm(WIDTH as u32, HEIGHT as u32, path, |c| {
                let mut d = Draw::new_fitted(c, light, (0., 0., WIDTH, HEIGHT), SIZE, v.font.get())
                    .unwrap();
                d.with_readable_style(labels, |d| v.paint(d));
            });
        }
    }
}

#[test]
fn sustain_mod_buttons_follow_only_the_four_pads_and_duo() {
    let mut v = view(PianoType::Wurli, Era::LateI);
    for model in Instrument::extra_models() {
        use pianet_n::Model::{C, L, N, T};
        let enabled = matches!(model, Instrument::Duo | Instrument::Reed(C | L | N | T));
        let p = Arc::get_mut(&mut v.params).unwrap();
        p.select_model(model);
        let c = &mut p.sustain_mod;
        for which in [&mut c.c, &mut c.l, &mut c.n, &mut c.t, &mut c.duo] {
            which.enabled = BoolParam::new("Sustain Mod", false);
        }
        v.slide = Slide::new(model);
        let (s, pedal) = sustain_rects(model);
        let center = |(x, y, w, h): Rect| (x + w / 2., y + h / 2.);
        let (x, y) = center(s);
        assert_eq!(v.hit(x, y) == Some(Target::SustainMod), enabled);
        let (x, y) = center(pedal);
        assert_ne!(v.hit(x, y), Some(Target::ModPedal));
        if enabled {
            let c = &mut Arc::get_mut(&mut v.params).unwrap().sustain_mod;
            for which in [&mut c.c, &mut c.l, &mut c.n, &mut c.t, &mut c.duo] {
                which.enabled = BoolParam::new("Sustain Mod", true);
            }
            assert_eq!(v.hit(x, y), Some(Target::ModPedal));
        }
    }
}
