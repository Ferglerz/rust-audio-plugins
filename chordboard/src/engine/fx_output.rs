//! Bridge musical source ownership to the ordered rack. Existing dry allocation
//! remains the source of note lifetimes; the rack owns transformed output voices.
use super::*;

impl Engine {
    pub(super) fn fx_context(&self) -> fx::Context {
        let chord = if self.config.tracker_enabled {
            self.arp_cycle_chord()
        } else if self.notes.len > 0 {
            self.full_notes
        } else {
            Notes::default()
        };
        fx::Context {
            now: self.now,
            beat: self.beat,
            sample_rate: self.sample_rate as f64,
            tempo: self.tempo,
            key: self.config.key,
            scale: self.config.scale,
            chord,
            mpe: self.config.mpe,
            upper: self.config.upper,
            members: self.config.members,
            bend_range: self.config.bend_range,
            master_range: self.config.master_range,
            output_channel: self.config.output_channel,
            bass_channel: self.config.bass_channel,
            melody_channel: self.config.upper_channel,
            aligned: self.config.daw_alignment
                && self.config.host_sync
                && self.playing
                && self.host_beat_available
                && self.host_tempo_available,
            delay_map: std::array::from_fn(|n| n as u8),
        }
    }

    pub(super) fn configure_fx(&mut self, out: &mut impl FnMut(Out)) {
        self.refresh_fx_lifetimes();
        let enabled = self.arp.data.midi_fx.harmonizer.enabled
            || self.arp.data.midi_fx.rhythm_gate.enabled
            || self.arp.data.midi_fx.voice_splitter.enabled;
        if enabled && !self.fx.active() {
            self.reset_delay(out);
        }
        let ctx = self.fx_context();
        self.fx.configure(&self.arp.data.midi_fx, ctx, out);
    }

    pub(super) fn refresh_fx_lifetimes(&mut self) {
        for (index, voice) in self.voices.iter().enumerate() {
            let lifetime = voice
                .filter(|v| v.owner != delay_output::DELAY_OWNER)
                .map(|v| (v.token, (!v.layer && v.off != u64::MAX).then_some(v.off)));
            if lifetime != self.fx_lifetimes[index] {
                self.fx_lifetimes[index] = lifetime;
                if let Some((token, off)) = lifetime {
                    self.fx.set_off(token, off);
                }
            }
        }
    }

    pub(super) fn refresh_fx_expression(&mut self) {
        if !self.fx.active() {
            return;
        }
        let expression = self.attack_expression();
        let bends = std::array::from_fn(|channel| self.melody_bend(channel as u8));
        let stamp = (self.channels, bends, expression);
        if self.fx_expression_stamp == Some(stamp) {
            return;
        }
        self.fx_expression_stamp = Some(stamp);
        for voice in self.voices.iter().flatten() {
            if voice.owner != delay_output::DELAY_OWNER {
                self.fx.expression(voice.token, expression);
            }
        }
        for channel in 0..16u8 {
            let mut expression = self.channels[channel as usize];
            expression.bend = self.melody_bend(channel);
            for note in 0..128u8 {
                if self.melody[channel as usize][note as usize] {
                    self.fx
                        .expression(Self::melody_delay_token(channel, note), expression);
                }
                if self.bass[channel as usize][note as usize].is_some() {
                    self.fx.expression(
                        Self::bass_delay_token(channel as usize, note as usize),
                        self.channels[channel as usize],
                    );
                }
            }
        }
    }

    pub(crate) fn flush_fx(&mut self, out: &mut impl FnMut(Out)) {
        if !self.fx.active() {
            return;
        }
        self.refresh_fx_lifetimes();
        self.refresh_fx_expression();
        let mut ctx = self.fx_context();
        if self.fx.deadline(ctx).is_none_or(|at| at > self.now) {
            return;
        }
        if self.fx.delay_due(ctx)
            && (self.arp.data.midi_fx.delay.follow_chords
                || self.arp.data.midi_fx.delay.voice_leading)
        {
            ctx.delay_map = std::array::from_fn(|n| self.delay_note(n as u8));
        }
        self.fx.flush(ctx, out);
        let serial = self.fx.output_serial();
        if serial == self.fx_output_serial {
            return;
        }
        self.fx_output_serial = serial;
        // The rack may have just allocated voices. Seed routed controller values
        // on their real channels, including Y's custom CC destination.
        for voice in self.voices.iter().flatten() {
            if voice.owner != delay_output::DELAY_OWNER {
                self.emit_fx_controllers(voice.token, out);
            }
        }
    }

    pub(super) fn emit_fx_controllers(&self, token: u64, out: &mut impl FnMut(Out)) {
        self.emit_routed_controllers(0, &mut |event| {
            if let Out::Cc(_, cc, value) = event {
                self.fx.controller(token, cc, value, out);
            }
        });
        if self.y_active && self.config.y_target == 5 {
            self.fx.controller(token, self.config.y_cc, self.y, out);
        }
    }

    pub(super) fn fx_master_pedal(&mut self, channel: u8, held: bool, out: &mut impl FnMut(Out)) {
        let master = if self.input_upper { 15 } else { 0 };
        if !self.fx.active() || !self.config.mpe || channel != master {
            return;
        }
        for i in 0..self.input_members.min(15) {
            let member = if self.input_upper { 14 - i } else { 1 + i };
            self.bass_pedal(member, held, out);
            self.melody_sustain[member as usize] = held;
            if !held {
                self.release_fx_melody_pedal(member, out);
            }
        }
    }

    pub(super) fn release_fx_melody_pedal(&mut self, channel: u8, out: &mut impl FnMut(Out)) {
        if !self.fx.active() {
            return;
        }
        for note in 0..128u8 {
            if self.melody[channel as usize][note as usize]
                && !self.down[channel as usize * 128 + note as usize]
            {
                self.melody_note(false, channel, note, 0.0, out);
            }
        }
    }
}
