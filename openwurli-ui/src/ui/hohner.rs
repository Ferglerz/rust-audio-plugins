use super::{ClavinetModel, Instrument, PianoView, Pickup, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{rgb, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

pub(super) const MENU: Rect = (574., 10., 133., 32.);
const GROUP_HEIGHT: f32 = 21.;
pub(super) const ROW_HEIGHT: f32 = 26.;
pub(super) fn menu_heading(group: usize) -> Rect {
    // Preserve a complete maker group in each column and leave the keyboard exposed.
    let (mut column, mut height, mut here) = (0usize, 0f32, (0usize, 0f32));
    for (i, (_, models)) in Instrument::GROUPS.iter().enumerate() {
        let block = GROUP_HEIGHT + ROW_HEIGHT * models.len() as f32;
        if i == 1 || (height > 0. && height + block > 350.) {
            column += 1;
            height = 0.;
        }
        if i == group {
            here = (column, height);
        }
        height += block;
    }
    let width = MENU.2 + 37.;
    let right = MENU.0 + MENU.2 - width;
    (
        right - (column - here.0) as f32 * (width + 3.),
        MENU.1 + MENU.3 + 3. + here.1,
        width,
        GROUP_HEIGHT,
    )
}
pub(super) fn menu_row(mut index: usize) -> Rect {
    for (group, (_, items)) in Instrument::GROUPS.iter().enumerate() {
        if index < items.len() {
            let (x, y, w, h) = menu_heading(group);
            return (x, y + h + index as f32 * ROW_HEIGHT, w, ROW_HEIGHT);
        }
        index -= items.len();
    }
    unreachable!("instrument menu row outside available models")
}
const TANGENT: Rect = (486., 99., 96., 69.);
const PAD: Rect = (386., 101., 88., 59.);
const MUTE: Rect = (421., 275., 152., 27.);
const PHASE: Rect = (376., 330., 195., 30.);
const SOFT_PEDAL: Rect = (35., 302., 179., 29.);
const SUSTAIN_PEDAL: Rect = (35., 350., 179., 29.);
const EARLY_LOCAL: Rect = (33., 344., 273., 30.);
const POWER: Rect = (590., 275., 96., 27.);
const SLIDER: Rect = (589., 324., 96., 45.);
const PLECTRUM: Rect = (383., 177., 83., 61.);
const DAMPER: Rect = (374., 103., 77., 73.);
const ELECTRODE: Rect = (471., 148., 118., 58.);

pub(super) fn slider_value(x: f32) -> f32 {
    ((x - SLIDER.0 - 5.) / (SLIDER.2 - 10.)).clamp(0., 1.)
}

fn selector(i: usize) -> Rect {
    (32. + i as f32 * 82., 297., 78., 26.)
}
fn tab(i: usize) -> Rect {
    (32. + i as f32 * 82., 354., 78., 26.)
}

impl PianoView {
    pub(super) fn hohner_hit(&self, model: Instrument, x: f32, y: f32) -> Option<Target> {
        if model == Instrument::Duo {
            return self.duo_hit(x, y);
        }
        let on = |r| point_in_rect(x, y, r);
        if model == Instrument::Electra {
            if on((258., 188., 115., 55.)) {
                return Some(Target::HohnerKey);
            }
            if on((166., 104., 318., 81.)) {
                return Some(Target::ElectraFork);
            }
            if on((488., 143., 92., 68.)) {
                return Some(Target::ElectraPickup);
            }
            if on(SOFT_PEDAL) {
                return Some(Target::ElectraSoft);
            }
            if on(SUSTAIN_PEDAL) {
                return Some(Target::ElectraSustain);
            }
            for knob in [super::Knob::ElectraVolume, super::Knob::ElectraSoftDepth] {
                if on(knob.rect()) {
                    return Some(Target::Dial(knob));
                }
            }
        } else if let Instrument::Clavinet(board) = model {
            if on(TANGENT) {
                return Some(Target::HohnerKey);
            }
            if on(MUTE) {
                return Some(Target::D6Mute);
            }
            if on(PHASE) {
                return Some(Target::D6Phase);
            }
            if board == ClavinetModel::E7 {
                if on(POWER) {
                    return Some(Target::E7Power);
                }
                if on(SLIDER) {
                    return Some(Target::E7Volume);
                }
            } else if on(super::Knob::D6Volume.rect()) {
                return Some(Target::Dial(super::Knob::D6Volume));
            }
            for (i, p) in Pickup::ALL.into_iter().enumerate() {
                if on(selector(i)) {
                    return Some(Target::Pickup(p));
                }
                if on(tab(i)) {
                    return Some(Target::ToneTab(i));
                }
            }
            let (bridge, centre) = self.pickup_locations(model);
            if on((bridge - 14., 125., 28., 27.)) {
                return Some(Target::Pickup(Pickup::Bridge));
            }
            if on((centre - 14., 178., 28., 27.)) {
                return Some(Target::Pickup(Pickup::Centre));
            }
        } else if let Instrument::Reed(m) = model {
            if m.is_cembalet() {
                if on(DAMPER) {
                    return Some(Target::CembaletDamper);
                }
                if on(PLECTRUM) {
                    return Some(Target::CembaletPlectrum);
                }
                if on(ELECTRODE) {
                    return Some(Target::CembaletElectrode);
                }
            } else if on(PAD) {
                return Some(Target::HohnerKey);
            }
            if matches!(m, pianet_n::Model::C | pianet_n::Model::L) && on(EARLY_LOCAL) {
                return Some(Target::EarlyLocalOut);
            }
            if m != pianet_n::Model::T {
                if m != pianet_n::Model::L && on(MUTE) {
                    return Some(Target::PianetVibrato);
                }
                if on(super::Knob::PianetSwell.rect()) {
                    return Some(Target::Dial(super::Knob::PianetSwell));
                }
            }
        }
        None
    }

    fn pickup_locations(&self, model: Instrument) -> (f32, f32) {
        let n = self.strikes[model.index()]
            .note()
            .clamp(clavinet_d6::live::LOW, clavinet_d6::live::HIGH);
        let g = clavinet_d6::live::geometry(n).expect("preview note inside D6 range");
        (
            72. + 452. * (g.bridge_m / g.speaking_m) as f32,
            72. + 452. * (g.centre_m / g.speaking_m) as f32,
        )
    }

    pub(super) fn paint_hohner(&self, d: &mut Draw, model: Instrument) {
        if model == Instrument::Duo {
            self.paint_duo(d);
            return;
        }
        let p = &self.params;
        let strike = &self.strikes[model.index()];
        let n = strike.note();
        let held = self.keyboard.is_down(n);
        let t = strike.elapsed();
        if model == Instrument::Electra {
            let pedals = p.electra_pedal_state();
            let lifted = held || pedals & 1 != 0;
            action::panel(d, WIDTH, "ELECTRA-PIANO · FORK ACTION", n);
            action::hardware(d, |m| {
                let ring = if lifted {
                    action::vibration(t, 0.65)
                } else {
                    0.
                };
                let light_y = 167. - 7. * ring;
                let heavy_y = 126. + 2. * ring;
                action::solid(
                    m,
                    [(145., 113.), (172., 113.), (172., 179.), (145., 179.)],
                    (6., -4.),
                    CASTING,
                );
                m.poly(
                    &[
                        (169., 165.),
                        (223., 165.),
                        (353., 165. + (light_y - 165.) * 0.3),
                        (479., light_y),
                    ],
                    STEEL,
                    3.,
                );
                m.poly(
                    &[(169., 128.), (219., 128.), (329., 128.), (418., heavy_y)],
                    STEEL,
                    8.,
                );
                let pad_y = if lifted { 143. } else { 164. };
                m.line(375., 105., 394., pad_y - 3., WOOD, 4.);
                m.line(383., pad_y, 406., pad_y, RUBBER, 5.);
                action::solid(
                    m,
                    [(490., 153.), (525., 153.), (525., 177.), (490., 177.)],
                    (6., -4.),
                    rgb(73, 122, 109),
                );
                m.line(525., 165., 552., 180., STEEL, 2.);
                let tip = (309., 195.);
                action::hammer(m, (228., 223.), tip, action::hammer_lift(t), RUBBER);
            });
            d.text(182., 109., "RETURN ARM", 10., MUTED);
            d.text(191., 186., "SENSED ARM", 10., MUTED);
            d.text(338., 101., "DAMPER", 10., MUTED);
            d.text(283., 237., "HAMMER", 11., TEXT);
            d.text(494., 202., "PICKUP", 11., MUTED);
            d.panel((18., 267., 692., 131.), PANEL, LINE);
            d.toggle_switch(SOFT_PEDAL, "SOFT PEDAL", pedals & 2 != 0, TEAL);
            d.toggle_switch(SUSTAIN_PEDAL, "SUSTAIN", pedals & 1 != 0, TEAL);
            self.draw_dial(d, super::Knob::ElectraSoftDepth);
            self.draw_dial(d, super::Knob::ElectraVolume);
        } else if let Instrument::Clavinet(board) = model {
            let (bridge, centre) = self.pickup_locations(model);
            action::panel(
                d,
                WIDTH,
                if board == ClavinetModel::E7 {
                    "CLAVINET E7 · STRING ACTION"
                } else {
                    "CLAVINET D6 · STRING ACTION"
                },
                n,
            );
            let (y0, junction) = (162., 524.);
            let dip = if held { (t / 0.07).min(1.) } else { 0. };
            let y = y0 + 13. * dip;
            action::hardware(d, |d| {
                let pickup = p.d6.pickup.value();
                for (x, yp, active) in [
                    (bridge, 129., pickup != Pickup::Centre),
                    (centre, 184., pickup != Pickup::Bridge),
                ] {
                    action::solid(
                        d,
                        [
                            (x - 12., yp),
                            (x + 12., yp),
                            (x + 12., yp + 12.),
                            (x - 12., yp + 12.),
                        ],
                        (7., -4.),
                        if active { rgb(59, 132, 116) } else { CASTING },
                    );
                }
                let tangent = y0 - 13. + dip * 26.;
                action::solid(
                    d,
                    [
                        (501., 112. + 8. * dip),
                        (571., 112. + 8. * dip),
                        (571., 122. + 8. * dip),
                        (501., 122. + 8. * dip),
                    ],
                    (7., -4.),
                    WOOD,
                );
                d.line(
                    junction,
                    121. + 8. * dip,
                    junction,
                    tangent - 10.,
                    CASTING,
                    5.,
                );
                action::solid(
                    d,
                    [
                        (junction - 10., tangent - 12.),
                        (junction + 10., tangent - 12.),
                        (junction + 10., tangent),
                        (junction - 10., tangent),
                    ],
                    (4., -2.),
                    RUBBER,
                );
                action::solid(
                    d,
                    [(510., 176.), (538., 176.), (538., 187.), (510., 187.)],
                    (4., -2.),
                    STEEL,
                );
                let string: [_; 49] = std::array::from_fn(|i| {
                    let u = i as f32 / 48.;
                    (
                        72. + 452. * u,
                        y0 + (y - y0) * u
                            + 6. * action::vibration(t, 0.34) * (std::f32::consts::PI * u).sin(),
                    )
                });
                let wire = if n <= 51 { rgb(192, 144, 87) } else { STEEL };
                d.poly(&string, wire, if n <= 51 { 3. } else { 1.8 });
                d.line(junction, y, 627., y0, wire, 2.);
                for i in 1..20 {
                    let u = i as f32 / 20.;
                    let x = junction + 103. * u;
                    let v = y + (y0 - y) * u;
                    d.line(x - 2., v - 4., x + 1., v + 4., WOOD, 2.7);
                }
                action::screw(d, 72., y0, 4.);
                action::screw(d, 627., y0, 5.);
            });
            d.text(bridge - 18., 114., "BRIDGE", 10., MUTED);
            d.text(centre - 18., 219., "CENTRE", 10., MUTED);
            d.text(491., 103., "TANGENT", 10., TEXT);
            d.text(506., 212., "ANVIL", 10., MUTED);
            d.text(592., 196., "YARN", 10., MUTED);
            d.panel((18., 267., 692., 131.), PANEL, LINE);
            d.text(32., 288., "PICKUPS", 11., TEXT);
            d.text(32., 345., "TONE TABS", 11., TEXT);
            let labels = if p.d6.invert_da.value() {
                ["CA · Centre", "CB · Bridge", "DA · Diff", "DB · Sum"]
            } else {
                ["CA · Centre", "CB · Bridge", "DA · Sum", "DB · Diff"]
            };
            for (i, mode) in Pickup::ALL.into_iter().enumerate() {
                d.tab_button(selector(i), labels[i], p.d6.pickup.value() == mode, GOLD);
                d.button(
                    tab(i),
                    ["Soft", "Medium", "Treble", "Brilliant"][i],
                    p.d6.tabs()[i].value(),
                    TEAL,
                );
            }
            d.toggle_switch(MUTE, "MUTE BAR", p.d6.mute.value(), TEAL);
            d.toggle_switch(PHASE, "REVERSE DA / DB", p.d6.invert_da.value(), TEAL);
            if board == ClavinetModel::E7 {
                d.toggle_switch(POWER, "POWER", p.clavinet_e7_power.value(), TEAL);
                let value = p.clavinet_volume().value();
                let x = SLIDER.0 + 5. + value * (SLIDER.2 - 10.);
                let y = SLIDER.1 + 25.;
                d.text(SLIDER.0, SLIDER.1 + 6., "VOLUME", 11., TEXT);
                d.line(SLIDER.0 + 5., y, SLIDER.0 + SLIDER.2 - 5., y, LINE, 4.);
                d.rounded_rect(x - 4., y - 8., 8., 16., 2., GOLD);
                d.text(
                    SLIDER.0,
                    SLIDER.1 + 45.,
                    &format!("{:.0} %", 100. * value),
                    11.,
                    TEXT,
                );
            } else {
                self.draw_dial(d, super::Knob::D6Volume);
            }
        } else if let Instrument::Reed(reed) = model {
            let is_t = reed == pianet_n::Model::T;
            action::panel(
                d,
                WIDTH,
                if reed == pianet_n::Model::Cembalet {
                    "CEMBALET · PLECTRUM ACTION"
                } else if reed == pianet_n::Model::CembaletN {
                    "CEMBALET N · PLECTRUM ACTION"
                } else if is_t {
                    "PIANET T · SUCTION PAD"
                } else {
                    match reed {
                        pianet_n::Model::C => "PIANET C · ADHESIVE PAD",
                        pianet_n::Model::L => "PIANET L · ADHESIVE PAD",
                        _ => "PIANET N · ADHESIVE PAD",
                    }
                },
                n,
            );
            let caught = !reed.is_cembalet() && p.sustain_mod.pad_is_caught(n);
            let lift = if caught {
                1.
            } else if held {
                (t / 0.09).min(1.)
            } else {
                0.
            };
            let reed_y = 177.
                + if held || caught {
                    action::vibration(t, 0.4) * 8.
                } else {
                    0.
                };
            action::hardware(d, |d| {
                action::solid(
                    d,
                    [(136., 163.), (184., 163.), (184., 197.), (136., 197.)],
                    (6., -4.),
                    CASTING,
                );
                d.poly(
                    &[
                        (166., 177.),
                        (262., 177.),
                        (348., 177. + (reed_y - 177.) * 0.38),
                        (446., reed_y),
                    ],
                    STEEL,
                    3.,
                );
                if reed.is_cembalet() {
                    // Independent upper damper and lower plectrum on the moving key arms.
                    let damper = 175. - 25. * lift;
                    d.line(389., 114., 410., damper - 6., WOOD, 5.);
                    d.line(397., damper, 425., damper, RUBBER, 6.);
                    let pluck = 181. - 19. * lift;
                    let tip = 436. + 17. * lift;
                    d.poly(
                        &[(469., 216.), (461., pluck + 15.), (tip, pluck)],
                        rgb(211, 167, 86),
                        4.,
                    );
                } else {
                    let pad_y = 177. - 30. * lift;
                    d.line(420., 108. - 8. * lift, 420., pad_y - 12., STEEL, 6.);
                    action::solid(
                        d,
                        [
                            (401., pad_y - 14.),
                            (439., pad_y - 14.),
                            (441., pad_y - 2.),
                            (398., pad_y - 2.),
                        ],
                        (5., -3.),
                        if is_t { rgb(159, 165, 164) } else { WOOD },
                    );
                    d.line(398., pad_y - 1., 441., pad_y - 1., RUBBER, 3.);
                }
                action::solid(
                    d,
                    [(453., 165.), (474., 165.), (474., 190.), (453., 190.)],
                    (6., -4.),
                    if is_t {
                        rgb(72, 125, 109)
                    } else {
                        rgb(182, 149, 97)
                    },
                );
            });
            d.text(140., 222., "REED CLAMP", 11., MUTED);
            d.text(275., 208., "STEEL REED", 11., MUTED);
            if reed.is_cembalet() {
                d.text(381., 232., "PLECTRUM", 11., TEXT);
            }
            d.text(
                366.,
                105.,
                if reed.is_cembalet() {
                    "DAMPER"
                } else if is_t {
                    "SUCTION PAD"
                } else {
                    "ADHESIVE PAD"
                },
                11.,
                TEXT,
            );
            d.text(
                475.,
                217.,
                if is_t { "MAGNETIC PICKUP" } else { "ELECTRODE" },
                11.,
                MUTED,
            );
            d.panel((18., 267., 692., 131.), PANEL, LINE);
            if !is_t {
                if reed != pianet_n::Model::L {
                    d.toggle_switch(MUTE, "VIBRATO", p.reed_controls().1.value(), TEAL);
                }
                self.draw_dial(d, super::Knob::PianetSwell);
            }
            d.text(
                32.,
                291.,
                if reed.is_cembalet() {
                    "PICKUP CIRCUIT"
                } else {
                    "OUTPUT"
                },
                11.,
                MUTED,
            );
            let local_output = p.early_local_output.value();
            if matches!(reed, pianet_n::Model::C | pianet_n::Model::L) {
                let label = if reed == pianet_n::Model::C {
                    "HEADPHONE DIN"
                } else {
                    "SPEAKER TERMINALS"
                };
                d.toggle_switch(EARLY_LOCAL, label, local_output, TEAL);
            }
            d.text(
                32.,
                334.,
                match reed {
                    pianet_n::Model::T => "Passive jack",
                    pianet_n::Model::N => "600 mV jack",
                    pianet_n::Model::C if local_output => "Headphone DIN",
                    pianet_n::Model::L if local_output => "Speaker terminals",
                    pianet_n::Model::C | pianet_n::Model::L => "Amplifier DIN",
                    pianet_n::Model::Cembalet => "RF detector",
                    pianet_n::Model::CembaletN => "Transistor preamp",
                },
                18.,
                TEXT,
            );
        }
    }
}
