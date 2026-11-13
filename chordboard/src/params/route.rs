use super::catalog::{control, toggle};
use super::*;

#[derive(Params)]
pub struct ModulatorParams {
    #[id = "modulator_bypass"]
    pub bypass: BoolParam,
}

impl Default for ModulatorParams {
    fn default() -> Self {
        Self {
            bypass: BoolParam::new("Bypass modulator", false),
        }
    }
}

#[derive(Params)]
pub struct RouteParams {
    #[id = "route_enabled"]
    pub enabled: BoolParam,
    #[id = "route_source"]
    pub source: IntParam,
    #[id = "route_target"]
    pub target: IntParam,
    #[id = "route_min"]
    pub min: FloatParam,
    #[id = "route_max"]
    pub max: FloatParam,
    #[id = "route_curve"]
    pub curve: FloatParam,
    #[id = "route_smoothing_ms"]
    pub smoothing_ms: FloatParam,
}

impl Default for RouteParams {
    fn default() -> Self {
        Self::from_route(Route::default())
    }
}

impl RouteParams {
    pub(super) fn from_route(route: Route) -> Self {
        Self {
            smoothing_ms: FloatParam::new(
                "Smoothing",
                route.smoothing_ms,
                FloatRange::Linear {
                    min: 0.0,
                    max: 2000.0,
                },
            )
            .with_unit(" ms")
            .with_value_to_string(Arc::new(|v| format!("{v:.0}")))
            .with_string_to_value(Arc::new(|s| {
                s.trim().trim_end_matches("ms").trim().parse().ok()
            })),
            enabled: BoolParam::new("Enabled", route.enabled),
            source: IntParam::new(
                "Source",
                route.source as i32,
                IntRange::Linear {
                    min: 0,
                    max: (SOURCES.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| SOURCES[v as usize].into())),
            target: IntParam::new(
                "Destination",
                route.target as i32,
                IntRange::Linear {
                    min: 0,
                    max: (TARGETS.len() - 1) as i32,
                },
            )
            .with_value_to_string(Arc::new(|v| TARGETS[v as usize].name.into())),
            min: FloatParam::new(
                "Minimum",
                route.min,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0)))
            .with_string_to_value(Arc::new(|s| {
                s.trim()
                    .trim_end_matches('%')
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .map(|v| v / 100.0)
            })),
            max: FloatParam::new(
                "Maximum",
                route.max,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0)))
            .with_string_to_value(Arc::new(|s| {
                s.trim()
                    .trim_end_matches('%')
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .map(|v| v / 100.0)
            })),
            curve: FloatParam::new(
                "Curve",
                route.curve,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| {
                if v.abs() < 0.005 {
                    "Linear".into()
                } else {
                    format!("{v:+.2}")
                }
            }))
            .with_string_to_value(Arc::new(|s| {
                if s.trim().eq_ignore_ascii_case("linear") {
                    Some(0.0)
                } else {
                    s.trim().parse().ok()
                }
            })),
        }
    }

    pub fn route(&self) -> Route {
        Route {
            enabled: self.enabled.value(),
            source: self.source.value() as u8,
            target: self.target.value() as u8,
            min: self.min.value(),
            max: self.max.value(),
            curve: self.curve.value(),
            smoothing_ms: self.smoothing_ms.value(),
        }
    }
}

nested_control_catalog! {
    RouteParams, 7,
    toggle("route_enabled", enabled, 6),
    control("route_source", source, 6),
    control("route_target", target, 6),
    control("route_min", min, 6),
    control("route_max", max, 6),
    control("route_curve", curve, 6),
    control("route_smoothing_ms", smoothing_ms, 6),
}

#[cfg(test)]
mod routing_state_tests {
    use super::*;
    #[test]
    fn modulator_bypass_preserves_routes_and_resumes_every_source() {
        let mut p = ChordboardParams::default();
        for source in 0..SOURCE_COUNT {
            let route = Route {
                source: source as u8 + 1,
                target: 4,
                min: 0.2,
                max: 0.7,
                curve: -0.3,
                smoothing_ms: 50.0,
                ..Route::default()
            };
            p.routes[0] = RouteParams::from_route(route);
            p.routes[1] = RouteParams::from_route(Route { target: 5, ..route });
            p.routes[2] = RouteParams::from_route(Route {
                enabled: false,
                ..route
            });
            p.routes[3] = RouteParams::from_route(Route {
                source: (source as u8 + 1) % SOURCE_COUNT as u8 + 1,
                ..route
            });
            assert_eq!(p.config().routes[0], route);
            p.modulators[source].bypass = BoolParam::new("Bypass", true);
            let config = p.config();
            assert!(!config.routes[0].active());
            assert!(!config.routes[1].active());
            assert!(config.routes[3].active());
            assert_eq!(p.routes[0].route(), route);
            assert_eq!(
                config.routes[0],
                Route {
                    enabled: false,
                    ..route
                }
            );
            p.modulators[source].bypass = BoolParam::new("Bypass", false);
            assert_eq!(p.config().routes[0], route);
            assert!(p.config().routes[1].active());
            assert!(!p.config().routes[2].active());
        }
        let ids: Vec<_> = p.param_map().into_iter().map(|v| v.0).collect();
        for index in 1..=SOURCE_COUNT {
            assert_eq!(
                ids.iter()
                    .filter(|id| **id == format!("modulator_bypass_{index}"))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn modulator_bypass_restores_bases_and_breaks_outgoing_chains() {
        let mut p = ChordboardParams::default();
        let target = |id| TARGETS.iter().position(|t| t.id == id).unwrap() as u8;
        p.routes[0] = RouteParams::from_route(Route {
            source: 2,
            target: target("source_9"),
            ..Route::default()
        });
        p.routes[1] = RouteParams::from_route(Route {
            source: 2,
            target: target("velocity"),
            smoothing_ms: 100.0,
            ..Route::default()
        });
        p.routes[2] = RouteParams::from_route(Route {
            source: 10,
            target: target("transpose"),
            ..Route::default()
        });
        let mut engine = crate::engine::Engine::default();
        engine.sources[1] = 1.0;
        engine.configure(p.config(), &mut |_| {});
        assert_eq!(engine.config.transpose, 24);
        assert_eq!(engine.config.velocity, 1.0);
        p.modulators[1].bypass = BoolParam::new("Bypass", true);
        engine.configure(p.config(), &mut |_| {});
        assert_eq!(engine.config.transpose, p.config().transpose);
        assert_eq!(engine.config.velocity, p.config().velocity);
        assert_eq!(engine.effective_sources()[1], 1.0);
        engine.sources[1] = 0.5;
        p.modulators[1].bypass = BoolParam::new("Bypass", false);
        engine.configure(p.config(), &mut |_| {});
        assert_eq!(
            engine.config.velocity,
            TARGETS[target("velocity") as usize].plain(0.5)
        );
    }

    #[test]
    fn route_constructor_preserves_every_nondefault_field() {
        let route = Route {
            enabled: false,
            source: 5,
            target: 3,
            min: 0.23,
            max: 0.81,
            curve: -0.4,
            smoothing_ms: 327.0,
        };
        assert_eq!(RouteParams::from_route(route).route(), route);
        assert_eq!(RouteParams::default().route(), Route::default());
    }

    #[test]
    fn arp_pages_are_independent_of_manual_play() {
        let p = ChordboardParams::default();
        assert_eq!(p.config().mode, crate::engine::MANUAL);
        assert!(!p.config().arp_enabled);
        assert!(p
            .config()
            .arp_settings
            .iter()
            .all(|p| p.mode == crate::arpeggiator::ArpMode::SyncLoop));
    }

    #[test]
    fn all_route_fields_have_stable_unique_host_ids_and_default_strumfield_link() {
        let p = ChordboardParams::default();
        let ids: Vec<_> = p.param_map().into_iter().map(|v| v.0).collect();
        for field in [
            "route_source",
            "route_target",
            "route_min",
            "route_max",
            "route_curve",
            "route_enabled",
            "route_smoothing_ms",
        ] {
            assert_eq!(
                ids.iter().filter(|id| id.starts_with(field)).count(),
                ROUTE_COUNT,
                "{field}"
            );
        }
        assert_eq!(
            ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
            ids.len()
        );
        assert!(!p.config().root_on_select);
        assert_eq!(p.config().routes, crate::engine::routing::default_routes());
        assert!(p.routes[0].route().active());
        assert!(p.config().routes[1..].iter().all(|r| !r.active()));
        let range = &p.routes[0];
        assert_eq!(range.min.normalized_value_to_string(0.5, true), "50%");
    }
}
