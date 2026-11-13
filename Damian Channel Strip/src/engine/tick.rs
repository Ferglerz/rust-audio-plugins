use super::*;

impl Engine {
    pub fn sync(&mut self) {
        if !self.shared.retired_aux.is_full() {
            if self.pending_aux.is_none() {
                self.pending_aux = self.shared.pending_aux.pop();
            }
            if let Some(mut aux) = self.pending_aux.take() {
                let grow_meters = aux.sr == self.sr
                    && aux.dyn_gr_scratch.capacity() > self.dyn_gr_scratch.capacity();
                if grow_meters {
                    if let (Ok(mut gr), Ok(mut input)) = (
                        self.shared.dyn_gr_uncapped.try_lock(),
                        self.shared.dyn_band_input.try_lock(),
                    ) {
                        aux.band_input.inherit(&self.band_input);
                        std::mem::swap(&mut self.dyn_gr_scratch, &mut aux.dyn_gr_scratch);
                        std::mem::swap(&mut self.dyn_input_scratch, &mut aux.dyn_input_scratch);
                        std::mem::swap(&mut *gr, &mut aux.dyn_gr_slot);
                        std::mem::swap(&mut *input, &mut aux.dyn_input_slot);
                        std::mem::swap(&mut self.band_input, &mut aux.band_input);
                    } else {
                        self.pending_aux = Some(aux);
                        return;
                    }
                }
                // Only the worker can grow these buffers. Keep replaced storage
                // alive until it can also be destroyed on the worker.
                if aux.sr == self.sr
                    && (aux.lift_bands.len() > self.lift_scratch.capacity()
                        || aux.sc_eq_bands.len() > self.sc_eq_scratch.capacity())
                {
                    for runtime in &mut aux.sc_eq_bands {
                        if let Some(old) = self
                            .sc_eq_bands
                            .iter()
                            .find(|old| old.band.id == runtime.band.id)
                        {
                            runtime.inherit(old);
                        }
                    }
                    std::mem::swap(&mut self.lift_bands, &mut aux.lift_bands);
                    std::mem::swap(&mut self.lift_scratch, &mut aux.lift_scratch);
                    std::mem::swap(&mut self.sc_eq_bands, &mut aux.sc_eq_bands);
                    std::mem::swap(&mut self.sc_eq_scratch, &mut aux.sc_eq_scratch);
                    self.shared.latency.store(self.latency(), Ordering::Relaxed);
                }
                let result = self.shared.retired_aux.push(aux);
                debug_assert!(result.is_ok());
            }
        }
        if let Ok(guard) = self.shared.lift_bands.try_lock() {
            if *guard != self.lift_bands && guard.len() <= self.lift_scratch.capacity() {
                self.lift_scratch.clear();
                self.lift_scratch.extend_from_slice(&guard);
                std::mem::swap(&mut self.lift_bands, &mut self.lift_scratch);
                self.shared.latency.store(self.latency(), Ordering::Relaxed);
            }
        }
        if let Ok(guard) = self.shared.sc_eq_bands.try_lock() {
            if guard.len() <= self.sc_eq_scratch.capacity()
                && (self.sc_eq_bands.len() != guard.len()
                    || self
                        .sc_eq_bands
                        .iter()
                        .zip(guard.iter())
                        .any(|(r, b)| r.band != *b))
            {
                self.sc_eq_scratch.clear();
                for b in guard.iter() {
                    let mut runtime = BandRuntime::new(b.clone(), self.sr);
                    if let Some(old) = self.sc_eq_bands.iter().find(|old| old.band.id == b.id) {
                        runtime.inherit(old);
                    }
                    self.sc_eq_scratch.push(runtime);
                }
                std::mem::swap(&mut self.sc_eq_bands, &mut self.sc_eq_scratch);
            }
        }
        if self.transition == 0 && !self.shared.retired.is_full() {
            if let Some(old) = self.previous.take() {
                let result = self.shared.retired.push(old);
                debug_assert!(result.is_ok());
            }
        }
        if self.previous.is_none() && !self.shared.retired.is_full() {
            if let Some(mut next) = self.shared.pending.pop() {
                if next.sr == self.sr
                    && next.config.encode() == self.shared.requested_config.load(Ordering::Relaxed)
                {
                    let mut j = 0;
                    for b in &mut next.bands {
                        while j < self.bank.bands.len() && self.bank.bands[j].band.id < b.band.id {
                            j += 1;
                        }
                        if let Some(old) = self.bank.bands.get(j) {
                            if old.band.id == b.band.id && next.config == self.bank.config {
                                b.inherit(old);
                            }
                        }
                    }
                    let mut j2 = 0;
                    for b in &mut next.eq2_bands {
                        while j2 < self.bank.eq2_bands.len()
                            && self.bank.eq2_bands[j2].band.id < b.band.id
                        {
                            j2 += 1;
                        }
                        if let Some(old) = self.bank.eq2_bands.get(j2) {
                            if old.band.id == b.band.id && next.config == self.bank.config {
                                b.inherit(old);
                            }
                        }
                    }
                    std::mem::swap(&mut self.bank, &mut next);
                    self.previous = Some(next);
                    // Warm the new FIR / resampler before crossfading an EQ edit.
                    self.transition =
                        (self.sr * 0.010) as usize + self.bank.config.latency(self.sr) * 2;
                    self.shared
                        .active_config
                        .store(self.bank.config.encode(), Ordering::Relaxed);
                    self.shared.latency.store(self.latency(), Ordering::Relaxed);
                } else {
                    let result = self.shared.retired.push(next);
                    debug_assert!(result.is_ok());
                }
            }
        }
    }

    fn sc_detector(&mut self, tap: [f64; 2]) -> [f64; 2] {
        if self.sc_eq_mix <= 0.0001 {
            return tap;
        }
        let mut sc_filtered = tap;
        for b in &mut self.sc_eq_bands {
            sc_filtered = b.tick(sc_filtered, self.sr);
        }
        [
            tap[0] + self.sc_eq_mix * (sc_filtered[0] - tap[0]),
            tap[1] + self.sc_eq_mix * (sc_filtered[1] - tap[1]),
        ]
    }

    fn publish_dynamics(
        &self,
        gr: f64,
        sc_level: f64,
        comp_on: bool,
        bypass: bool,
        wall_on: bool,
    ) {
        self.shared.sc_level.store(
            if comp_on && !bypass {
                (sc_level as f32).max(-90.0)
            } else {
                -90.0
            },
            Ordering::Relaxed,
        );
        self.shared.gr.store(
            (gr * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
            Ordering::Relaxed,
        );
        self.shared.gr_uncapped.store(
            (self.comp.uncapped_gr() * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
            Ordering::Relaxed,
        );
        self.shared.wall_level.store(
            if wall_on && !bypass {
                (gain_db(self.wall_peak) as f32).max(-90.0)
            } else {
                -90.0
            },
            Ordering::Relaxed,
        );
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Audio tick receives independent per-section bypass and routing flags"
    )]
    pub fn tick(
        &mut self,
        input: [f64; 2],
        settings: CompSettings,
        eq_on: bool,
        eq2_on: bool,
        sc_eq_on: bool,
        lift_on: bool,
        comp_on: bool,
        comp_pre: bool,
        bypass: bool,
    ) -> [f64; 2] {
        let k = self.k_10ms;
        smooth_eq_bypass(&mut self.eq1_mix, eq_on, k);
        smooth_eq_bypass(&mut self.eq2_mix, eq2_on, k);
        self.sc_eq_mix += k * (f64::from(sc_eq_on) - self.sc_eq_mix);
        self.lift_mix += k * (f64::from(lift_on) - self.lift_mix);
        self.wall_mix += k * (f64::from(settings.wall.on) - self.wall_mix);
        self.wall_lookahead = settings.wall.lookahead;
        self.comp_mix += k * (f64::from(comp_on) - self.comp_mix);
        self.comp_pre_mix += k * (f64::from(comp_pre) - self.comp_pre_mix);
        self.bypass_mix += k * (f64::from(bypass) - self.bypass_mix);

        let mono = ((input[0] + input[1]) * 0.5) as f32;
        self.vad.push_sample(mono);

        let effective_comp_mix = self.comp_mix * (1.0 - self.solo_mix);
        let is_pre = self.comp_pre_mix > 0.5;

        // PSE always sits first on the audible path. PRE/POST only moves the
        // compressor (and its SC EQ detector tap).
        let (staged, pse_gr) = self.comp.tick_pse(input, settings, self.sr);

        let (bank_in, pre_gr, pre_sc) = if is_pre {
            let sc_detector = self.sc_detector(staged);
            let (compressed, gr, sc) = self.comp.tick_comp(staged, sc_detector, settings, self.sr);
            let mix = if settings.pse.listen {
                1.0
            } else {
                effective_comp_mix
            };
            let comp_in = [
                staged[0] + mix * (compressed[0] - staged[0]),
                staged[1] + mix * (compressed[1] - staged[1]),
            ];
            (comp_in, gr, sc)
        } else {
            (staged, 0.0, -90.0)
        };

        self.band_input.tick(bank_in);
        let (mut x, mut dry) = self.bank.tick(bank_in, self.eq1_mix, self.eq2_mix);
        let selected = self.shared.selected_id.load(Ordering::Relaxed);
        let band_gr = if self.bank.config.mode == ProcessingMode::LinearPhase {
            0.0
        } else {
            self.bank
                .bands
                .iter()
                .chain(self.bank.eq2_bands.iter())
                .find(|b| b.band.id == selected)
                .map_or(0.0, |b| b.reduction)
        };
        if self.transition > 0 {
            if let Some(previous) = &mut self.previous {
                let (old, old_dry) = previous.tick(bank_in, self.eq1_mix, self.eq2_mix);
                if previous.config == self.bank.config {
                    dry = old_dry;
                }
                let wet =
                    (1.0 - self.transition as f64 / (self.sr * 0.010).floor()).clamp(0.0, 1.0);
                // Different latencies cannot be crossfaded coherently. Use the
                // new aligned dry path during a mode change's warm-up instead.
                let source = if previous.config == self.bank.config {
                    old
                } else {
                    dry
                };
                for i in 0..2 {
                    x[i] = source[i] + wet * (x[i] - source[i]);
                }
            }
            self.transition -= 1;
        }
        let lift_active = self.lift_bands.iter().any(|b| b.enabled);
        let (lift_out, eq_delayed, dry_delayed) = self.lift.tick(dry, x, &self.lift_bands, self.sr);
        if self.lift.take_hop_updated() {
            for i in 0..8 {
                self.shared.lift_band_gr[i].store(self.lift.band_gr[i], Ordering::Relaxed);
            }
            for i in 0..256 {
                self.shared.lift_bin_gr[i].store(self.lift.bin_gr[i], Ordering::Relaxed);
                self.shared.lift_bin_gr_uncapped[i]
                    .store(self.lift.bin_gr_uncapped[i], Ordering::Relaxed);
            }
        }
        if lift_active {
            for i in 0..2 {
                x[i] = eq_delayed[i] + self.lift_mix * lift_out[i];
            }
            dry = dry_delayed;
        }
        let solo_id = self.shared.solo_id.load(Ordering::Relaxed);
        let is_solo = solo_id != 0;
        self.solo_mix += self.k_8ms * (f64::from(is_solo) - self.solo_mix);

        let is_lift_solo = is_solo && solo_id >= LIFT_ID_BASE;
        let solo_sample = if is_lift_solo {
            Some([lift_out[0] * 1.5, lift_out[1] * 1.5])
        } else if is_solo {
            self.bank
                .bands
                .iter()
                .chain(self.bank.eq2_bands.iter())
                .chain(self.sc_eq_bands.iter())
                .find(|b| b.band.id == solo_id)
                .map(|b| {
                    let mut audition = b.band.clone();
                    if !audition.shape.is_cut() {
                        audition.shape = crate::band::Shape::BandPass;
                        audition.q = audition.q.max(0.35);
                    }
                    let topology = (audition.id, audition.shape, audition.order);
                    if self.solo_topology != Some(topology) {
                        self.solo_filters = [crate::dsp::Cascade::default(); 2];
                        self.solo_topology = Some(topology);
                    }
                    match &self.solo_coeffs {
                        Some((cached, coeffs)) if *cached == audition => *coeffs,
                        _ => {
                            let coeffs = crate::dsp::BandCoeffs::make(&audition, self.sr);
                            self.solo_coeffs = Some((audition, coeffs));
                            coeffs
                        }
                    }
                })
                .map(|c| {
                    [
                        self.solo_filters[0].tick(dry[0], c) * 1.5,
                        self.solo_filters[1].tick(dry[1], c) * 1.5,
                    ]
                })
        } else {
            None
        };

        let (gr, sc_level) = if !is_pre {
            let sc_detector = self.sc_detector(x);
            let (compressed, gr, sc) = self.comp.tick_comp(x, sc_detector, settings, self.sr);
            let mix = if settings.pse.listen {
                1.0
            } else {
                effective_comp_mix
            };
            for i in 0..2 {
                x[i] += mix * (compressed[i] - x[i]);
            }
            (gr, sc)
        } else {
            (pre_gr, pre_sc)
        };

        self.wall_peak = self.wall_peak.max(x[0].abs()).max(x[1].abs());
        let clipped = self.wall.process(x, settings.wall, self.sr);
        let base_delayed = self.wall_base_delay.process(x);
        let base = if settings.wall.lookahead {
            base_delayed
        } else {
            x
        };
        for i in 0..2 {
            x[i] = base[i] + self.wall_mix * (clipped[i] - base[i]);
        }

        // Dry and solo enter the final mixes after the WALL stage, so both
        // take the same lookahead delay as the limited path.
        let dry_delayed = self.dry_delay.process(dry);
        let solo_delayed = self.solo_delay.process(solo_sample.unwrap_or([0.0; 2]));
        let (dry, solo_sample) = if settings.wall.lookahead {
            (dry_delayed, solo_sample.map(|_| solo_delayed))
        } else {
            (dry, solo_sample)
        };

        if settings.output_gain.to_bits() != self.last_output_gain_db.to_bits() {
            self.last_output_gain_db = settings.output_gain;
            self.last_output_gain = 10.0_f64.powf(settings.output_gain / 20.0);
        }
        let out_gain = self.last_output_gain;
        for i in 0..2 {
            x[i] *= out_gain;
        }
        for i in 0..2 {
            x[i] += self.bypass_mix * (dry[i] - x[i]);
            if let Some(s) = solo_sample {
                x[i] = (1.0 - self.solo_mix) * x[i] + self.solo_mix * s[i];
            }
            if !x[i].is_finite() {
                x[i] = 0.0;
            }
        }
        if !is_solo && self.solo_mix < 0.0001 {
            self.solo_filters = [crate::dsp::Cascade::default(); 2];
            self.solo_topology = None;
            self.solo_coeffs = None;
        }
        self.in_peak = self.in_peak.max(input[0].abs()).max(input[1].abs());
        self.out_peak = self.out_peak.max(x[0].abs()).max(x[1].abs());
        self.push_spectrum_sample(input);
        const _: () = assert!(2048 % 256 == 0);
        self.position += 1;
        if self.position.is_multiple_of(256) {
            self.band_input.sync(
                self.bank
                    .bands
                    .iter()
                    .chain(self.bank.eq2_bands.iter())
                    .map(|b| &b.band),
            );
            self.band_input.write_levels_db(&mut self.dyn_input_scratch);
            if let Ok(mut slot) = self.shared.dyn_band_input.try_lock() {
                std::mem::swap(&mut *slot, &mut self.dyn_input_scratch);
            }
            self.publish_dynamics(gr, sc_level, comp_on, bypass, settings.wall.on);
            self.shared
                .pse_gr
                .store((pse_gr * (1.0 - self.bypass_mix)) as f32, Ordering::Relaxed);
        }
        self.flush_spectrum();
        if self.position == 2048 {
            self.position = 0;
            self.shared
                .input
                .store(gain_db(self.in_peak) as f32, Ordering::Relaxed);
            self.shared
                .output
                .store(gain_db(self.out_peak) as f32, Ordering::Relaxed);
            self.shared.band_gr.store(band_gr as f32, Ordering::Relaxed);
            self.dyn_gr_scratch.clear();
            if self.bank.config.mode != ProcessingMode::LinearPhase {
                for b in self.bank.bands.iter().chain(self.bank.eq2_bands.iter()) {
                    if b.band.dynamic
                        && b.band.shape.has_gain()
                        && self.dyn_gr_scratch.len() < self.dyn_gr_scratch.capacity()
                    {
                        self.dyn_gr_scratch
                            .push((b.band.id, b.reduction_uncapped as f32));
                    }
                }
            }
            if let Ok(mut slot) = self.shared.dyn_gr_uncapped.try_lock() {
                std::mem::swap(&mut *slot, &mut self.dyn_gr_scratch);
            }
            self.in_peak = 0.0;
            self.out_peak = 0.0;
            self.wall_peak = 0.0;
        }
        x
    }
}

fn smooth_eq_bypass(mix: &mut f64, enabled: bool, coefficient: f64) {
    let target = f64::from(enabled);
    *mix += coefficient * (target - *mix);
    if (*mix - target).abs() < 1.0e-10 {
        *mix = target;
    }
}
