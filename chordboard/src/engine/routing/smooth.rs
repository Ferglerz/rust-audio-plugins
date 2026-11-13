use super::{bipolar_source, Config, Engine, Out, Route, SOURCE_COUNT, TARGETS, TARGET_COUNT};

#[derive(Clone, Copy, Default)]
pub(in crate::engine) struct RouteSmoother {
    route: Option<Route>,
    value: f32,
    target: f32,
    increment: f32,
    remaining: u64,
}

impl Engine {
    fn route_output(&self, slot: usize, input: f32) -> f32 {
        let route = self.host_config.routes[slot];
        let smoother = self.route_smoothers[slot];
        if route.smoothing_ms > 0.0
            && smoother
                .route
                .is_some_and(|old| old.source == route.source && old.target == route.target)
        {
            smoother.value
        } else {
            route.output(input)
        }
    }

    pub(in crate::engine) fn prepare_route_smoothing(&mut self) {
        let sources = self.effective_sources();
        for (slot, route) in self.host_config.routes.iter().copied().enumerate() {
            let smoother = &mut self.route_smoothers[slot];
            if !route.active() {
                *smoother = RouteSmoother::default();
                continue;
            }
            let target = route.output(sources[route.source as usize - 1]);
            let fresh = smoother
                .route
                .is_none_or(|old| old.source != route.source || old.target != route.target);
            let time_changed = smoother
                .route
                .is_some_and(|old| old.smoothing_ms != route.smoothing_ms);
            smoother.route = Some(route);
            if fresh || route.smoothing_ms <= 0.0 {
                smoother.value = target;
                smoother.target = target;
                smoother.remaining = 0;
            } else if smoother.target != target || time_changed {
                smoother.target = target;
                smoother.remaining = (route.smoothing_ms.clamp(0.0, 2000.0) * self.sample_rate
                    / 1000.0)
                    .round()
                    .max(1.0) as u64;
                smoother.increment = (target - smoother.value) / smoother.remaining as f32;
            }
        }
    }

    pub(in crate::engine) fn smoothing_active(&self) -> bool {
        self.route_smoothers.iter().any(|s| s.remaining > 0)
    }

    pub(in crate::engine) fn tick_route_smoothing(&mut self, out: &mut impl FnMut(Out)) {
        if !self.smoothing_active() {
            return;
        }
        for smoother in &mut self.route_smoothers {
            if smoother.remaining > 0 {
                smoother.remaining -= 1;
                smoother.value = if smoother.remaining == 0 {
                    smoother.target
                } else {
                    smoother.value + smoother.increment
                };
            }
        }
        // Advance ramps per sample, but refresh MIDI/parameter destinations at
        // most once per millisecond so smoothing cannot flood the host queue.
        let interval = (self.sample_rate / 1000.0).round().max(1.0) as u64;
        if self.now.is_multiple_of(interval) || !self.smoothing_active() {
            self.apply_smoothed_targets(out);
        }
    }

    /// Push smoothed route values into the live config. Structural changes still
    /// take the full configure path so voice and harmony resets stay intact.
    pub(in crate::engine) fn apply_smoothed_targets(&mut self, out: &mut impl FnMut(Out)) {
        let old = self.config;
        let previous_sources = self.source_overrides;
        let mut config = self.host_config;
        for page in 0..8 {
            if let Some(settings) = self.arp.paste_settings[page] {
                if config.arp_settings[page] != settings {
                    config.arp_settings[page] = settings;
                }
            }
        }
        let initialized = self.overlay_arp_settings(&mut config);
        self.apply_routes(&mut config);
        config.arp_octaves = config.arp_octaves.clamp(1, initialized);
        if smoothed_needs_configure(&old, &config) {
            self.configure(self.host_config, out);
            return;
        }
        self.config = config;
        self.source_overrides = self.resolved_source_overrides();
        self.publish_source_overrides(previous_sources, out);
    }

    pub(in crate::engine) fn resolved_source_overrides(&self) -> [Option<f32>; SOURCE_COUNT] {
        let effective = self.effective_sources();
        let mut values = [None; SOURCE_COUNT];
        for route in self.host_config.routes {
            if route.active() {
                if let Some(index) = TARGETS[route.target as usize].source_index() {
                    values[index] = Some(effective[index]);
                }
            }
        }
        values
    }

    pub(in crate::engine) fn routed_expression(&self) -> crate::engine::Expression {
        crate::engine::Expression {
            bend: self.source_overrides[0].unwrap_or(self.expression.bend),
            pressure: self.source_overrides[4].unwrap_or(self.expression.pressure),
            timbre: self.source_overrides[5].unwrap_or(self.expression.timbre),
        }
    }

    pub(in crate::engine) fn emit_routed_controllers(&self, channel: u8, out: &mut impl FnMut(Out)) {
        for (index, cc) in [(1, 1), (2, 11), (3, 2)] {
            if let Some(value) = self.source_overrides[index] {
                out(Out::Cc(channel, cc, value));
            }
        }
    }

    /// Resolve each input once. A back edge reads the original input, so cycles
    /// stay deterministic and bounded instead of accumulating across samples.
    pub fn effective_sources(&self) -> [f32; SOURCE_COUNT] {
        fn resolve(
            engine: &Engine,
            index: usize,
            values: &mut [f32; SOURCE_COUNT],
            marks: &mut [u8; SOURCE_COUNT],
        ) -> f32 {
            if marks[index] == 2 {
                return values[index];
            }
            if marks[index] == 1 {
                return engine.sources[index];
            }
            marks[index] = 1;
            if let Some((slot, route)) =
                engine
                    .host_config
                    .routes
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, route)| {
                        route.active()
                            && !route.additive()
                            && TARGETS[route.target as usize].source_index() == Some(index)
                    })
            {
                let input = resolve(engine, route.source as usize - 1, values, marks);
                values[index] =
                    TARGETS[route.target as usize].plain(engine.route_output(slot, input));
            }
            let mut value = values[index];
            for (slot, route) in engine.host_config.routes.iter().enumerate() {
                if route.active()
                    && route.additive()
                    && TARGETS[route.target as usize].source_index() == Some(index)
                {
                    let input = resolve(engine, route.source as usize - 1, values, marks);
                    value += engine.route_output(slot, input);
                }
            }
            values[index] = value.clamp(if bipolar_source(index) { -1.0 } else { 0.0 }, 1.0);
            marks[index] = 2;
            values[index]
        }
        let mut values = self.sources;
        let mut marks = [0; SOURCE_COUNT];
        for index in 0..SOURCE_COUNT {
            resolve(self, index, &mut values, &mut marks);
        }
        values
    }

    pub(in crate::engine) fn apply_routes(&self, config: &mut Config) {
        let sources = self.effective_sources();
        let mut values = std::array::from_fn::<_, TARGET_COUNT, _>(|i| {
            if TARGETS[i].id == "x" && config.routed_x.is_none() {
                sources[7]
            } else {
                TARGETS[i].normalized(config)
            }
        });
        let mut touched = [false; TARGET_COUNT];
        for additive in [false, true] {
            for (slot, route) in self.host_config.routes.iter().enumerate() {
                if !route.active() || route.additive() != additive {
                    continue;
                }
                let index = route.target as usize;
                if TARGETS[index].source_index().is_some() {
                    continue;
                }
                let output = self.route_output(slot, sources[route.source as usize - 1]);
                if additive {
                    values[index] += output;
                } else {
                    values[index] = output;
                }
                touched[index] = true;
            }
        }
        for (index, target) in TARGETS.iter().enumerate() {
            if touched[index] {
                target.apply(config, values[index].clamp(0.0, 1.0));
            }
        }
    }

    pub(in crate::engine) fn source(&mut self, index: usize, value: f32, out: &mut impl FnMut(Out)) {
        if !value.is_finite() || index >= SOURCE_COUNT {
            return;
        }
        let value = value.clamp(if bipolar_source(index) { -1.0 } else { 0.0 }, 1.0);
        if self.sources[index] == value {
            return;
        }
        self.sources[index] = value;
        if self.host_config.routes.iter().any(|r| {
            r.active()
                && (r.source as usize == index + 1
                    || TARGETS[r.target as usize].source_index() == Some(index))
        }) {
            self.configure(self.host_config, out);
        }
    }

    pub fn routed_values(&self) -> [Option<f32>; TARGET_COUNT] {
        let sources = self.effective_sources();
        let mut values = [None; TARGET_COUNT];
        for route in self.host_config.routes {
            if route.active() {
                let index = route.target as usize;
                values[index] = Some(if let Some(source) = TARGETS[index].source_index() {
                    ((sources[source] - TARGETS[index].min)
                        / (TARGETS[index].max - TARGETS[index].min))
                        .clamp(0.0, 1.0)
                } else {
                    TARGETS[index].normalized(&self.config)
                });
            }
        }
        values
    }
}

fn smoothed_needs_configure(old: &Config, new: &Config) -> bool {
    fn structure(config: Config) -> Config {
        Config {
            velocity: 0.0,
            velocity_scale: 0.0,
            gate_scale: 0.0,
            length_ms: 0.0,
            gliss_ms: 0.0,
            strum_ms: 0.0,
            strum_beats: 0.0,
            contour: 0.0,
            contour_curve: 0.0,
            rate: 0.0,
            gate: 0.0,
            swing: 0.0,
            humanize: 0.0,
            arp_free_ms: 0.0,
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            ..config
        }
    }
    structure(*old) != structure(*new)
}
