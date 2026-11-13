use super::*;

// Nested catalogs keep UI IDs, ordering, lookup, and polling values together.
macro_rules! nested_control_catalog {
    ($params:ty, $count:literal, $($builder:ident($id:literal, $field:ident, $group:literal)),* $(,)?) => {
        impl $params {
            pub(super) fn values(&self) -> [f32; $count] {
                [$(self.$field.unmodulated_normalized_value()),*]
            }

            pub fn controls(&self) -> Vec<Control> {
                vec![$($builder($id, $group, &self.$field)),*]
            }

            pub fn control(&self, id: &str) -> Option<Control> {
                match id {
                    $($id => Some($builder($id, $group, &self.$field)),)*
                    _ => None,
                }
            }
        }
    };
}

// One catalog supports both full UI snapshots and allocation-light single lookups.
macro_rules! control_catalog {
    ($($builder:ident($field:ident, $group:literal)),* $(,)?) => {
        impl ChordboardParams {
            /// Update the polling signature without allocating formatted controls.
            pub fn update_control_signature(&self, signature: &mut Vec<f32>) -> bool {
                let mut index = 0;
                let mut changed = false;
                let mut visit = |value| {
                    if let Some(previous) = signature.get_mut(index) {
                        if *previous != value {
                            *previous = value;
                            changed = true;
                        }
                    } else {
                        signature.push(value);
                        changed = true;
                    }
                    index += 1;
                };
                $(visit(self.$field.unmodulated_normalized_value());)*
                for modulator in &self.modulators { visit(modulator.bypass.unmodulated_normalized_value()); }
                for route in &self.routes {
                    for value in route.values() { visit(value); }
                }
                // The last two legacy adapter values are now global host parameters above.
                for value in self.arp_pages[self.arp_page.value() as usize].values().into_iter().take(6) { visit(value); }
                changed |= signature.len() != index;
                signature.truncate(index);
                changed
            }

            pub fn controls(&self) -> Vec<Control> {
                let mut controls = vec![$($builder(stringify!($field), $group, &self.$field)),*];
                for modulator in &self.modulators { controls.push(toggle("modulator_bypass", 6, &modulator.bypass)); }
                for route in &self.routes { controls.extend(route.controls()); }
                controls.extend(self.arp_pages[self.arp_page.value() as usize].controls().into_iter().filter(|c| !matches!(c.id, "swing" | "humanize")));
                controls
            }

            pub fn control(&self, id: &str) -> Option<Control> {
                match id {
                    $(stringify!($field) => Some($builder(stringify!($field), $group, &self.$field)),)*
                    _ => self.arp_pages[self.arp_page.value() as usize].control(id),
                }
            }
        }
    };
}

control_catalog! {
    toggle(always_bass, 0),
    toggle(always_chord, 0),
    toggle(key_split, 0),
    toggle(bass_enabled, 0),
    control(split_note, 0),
    control(bass_split, 0),
    toggle(arp_enabled, 0),
    control(quality, 0),
    control(inversion, 0),
    control(base_inversion, 0),
    control(complexity, 0),
    control(voicing_override, 0),
    control(gate_scale, 0),
    control(velocity_scale, 0),
    control(swing, 0),
    control(humanize, 0),
    control(voice_leading, 0),
    control(transpose, 0),
    control(spread, 0),
    toggle(latch, 0),
    toggle(strum_enabled, 0),
    toggle(root_on_select, 0),
    toggle(strum_latch, 1),
    toggle(strum_chord_latch, 1),
    control(velocity, 0),
    control(length_ms, 0),
    control(strings, 1),
    control(strings_played, 1),
    control(gliss_ms, 0),
    control(gliss_direction, 0),
    control(strum_ms, 1),
    toggle(strum_sync, 1),
    control(strum_beats, 1),
    toggle(strum_hold, 1),
    toggle(tempo_sync, 1),
    toggle(daw_alignment, 1),
    control(tempo, 1),
    control(direction, 1),
    control(contour, 1),
    control(contour_curve, 1),
    control(octaves, 1),
    control(output_mode, 2),
    toggle(upper, 2),
    control(members, 2),
    control(bend_range, 2),
    control(master_range, 2),
    control(output_channel, 2),
    control(y_target, 3),
    control(y_cc, 3),
    toggle(x_reverse, 3),
    toggle(y_reverse, 3),
    control(x_min, 3),
    control(x_max, 3),
    control(y_min, 3),
    control(y_max, 3),
    control(x, 4),
    control(y, 4),
    toggle(fifths, 4),
    control(scale_layout, 4),
    control(key_spelling, 5),
    control(bank, 4),
    control(keyboard_octave, 4),
    control(key, 5),
    control(scale, 5),
    control(filter, 5),
    control(bass_channel, 5),
    control(upper_channel, 5),
}

#[derive(Clone)]
pub struct Control {
    pub id: &'static str,
    pub group: usize,
    pub name: String,
    pub ptr: ParamPtr,
    pub norm: f32,
    pub prev: f32,
    pub next: f32,
    pub value: String,
    pub toggle: bool,
}

pub(super) fn control(id: &'static str, group: usize, p: &impl Param) -> Control {
    let norm = p.unmodulated_normalized_value();
    Control {
        id,
        group,
        name: p.name().to_string(),
        ptr: p.as_ptr(),
        norm,
        prev: p.previous_normalized_step(norm, false),
        next: p.next_normalized_step(norm, false),
        value: p.normalized_value_to_string(norm, true),
        toggle: false,
    }
}

pub(super) fn toggle(id: &'static str, group: usize, p: &BoolParam) -> Control {
    let mut c = control(id, group, p);
    c.toggle = true;
    c
}

#[cfg(test)]
mod signature_tests {
    use super::*;

    fn assert_control_matches(actual: Control, expected: &Control) {
        assert_eq!(
            (
                actual.id,
                actual.group,
                &actual.name,
                actual.ptr,
                actual.norm,
                actual.prev,
                actual.next,
                &actual.value,
                actual.toggle,
            ),
            (
                expected.id,
                expected.group,
                &expected.name,
                expected.ptr,
                expected.norm,
                expected.prev,
                expected.next,
                &expected.value,
                expected.toggle,
            ),
            "{}",
            expected.id,
        );
    }

    #[test]
    fn nested_control_lookups_match_catalog_values_and_parameter_pointers() {
        let mut params = ChordboardParams::default();
        params.arp_page = IntParam::new("Arp page", 5, IntRange::Linear { min: 0, max: 7 });
        let page = &params.arp_pages[5];
        let route = &params.routes[0];
        let route_controls = route.controls();
        assert_eq!(
            route_controls.iter().map(|c| c.id).collect::<Vec<_>>(),
            [
                "route_enabled",
                "route_source",
                "route_target",
                "route_min",
                "route_max",
                "route_curve",
                "route_smoothing_ms",
            ],
        );
        for expected in &route_controls {
            assert_control_matches(route.control(expected.id).unwrap(), expected);
        }
        assert_eq!(
            route.values().as_slice(),
            route_controls.iter().map(|c| c.norm).collect::<Vec<_>>(),
        );
        assert!(route.control("missing").is_none());

        let page_controls = page.controls();
        assert_eq!(
            page_controls.iter().map(|c| c.id).collect::<Vec<_>>(),
            [
                "arp_mode",
                "arp_pattern",
                "arp_octaves",
                "arp_sync",
                "arp_free_ms",
                "rate",
                "swing",
                "humanize",
            ],
        );
        for expected in &page_controls {
            assert_control_matches(page.control(expected.id).unwrap(), expected);
            if !matches!(expected.id, "swing" | "humanize") {
                assert_control_matches(params.control(expected.id).unwrap(), expected);
            }
        }
        assert_eq!(
            page.values().as_slice(),
            page_controls.iter().map(|c| c.norm).collect::<Vec<_>>(),
        );
        assert_eq!(params.control("swing").unwrap().ptr, params.swing.as_ptr());
        assert_eq!(params.control("humanize").unwrap().ptr, params.humanize.as_ptr());
        let host = params.param_map();
        for id in ["swing", "humanize"] {
            assert_eq!(host.iter().filter(|entry| entry.0 == id).count(), 1);
            assert_eq!(params.controls().iter().filter(|c| c.id == id).count(), 1);
        }
        assert!(page.control("missing").is_none());
        assert!(params.control("missing").is_none());
    }

    #[test]
    fn polling_matches_catalog_and_reuses_storage_when_values_change() {
        let mut params = ChordboardParams::default();
        let mut signature = Vec::new();
        assert!(params.update_control_signature(&mut signature));
        let storage = signature.as_ptr();
        assert!(!params.update_control_signature(&mut signature));
        params.latch = BoolParam::new("Latch", !params.latch.value());
        params.routes[0].min =
            FloatParam::new("Minimum", 0.37, FloatRange::Linear { min: 0.0, max: 1.0 });
        assert!(params.update_control_signature(&mut signature));
        assert_eq!(
            signature,
            params
                .controls()
                .iter()
                .map(|control| control.norm)
                .collect::<Vec<_>>()
        );
        assert_eq!(storage, signature.as_ptr());
        assert!(!params.update_control_signature(&mut signature));
    }
}
