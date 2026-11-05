//! Source-aware, bounded MIDI FX graph. Every emitted note retains its release owner.
use super::{delay, Expression, Out};
use crate::harmony::{self, Notes};
use crate::midi_fx::{self, HarmonizerMode, MidiFxId, MidiFxState};
use std::cell::Cell;

const MAX_ROOTS: usize = 2304;
const MAX_NODES: usize = 8192;
const END: usize = 5;
const EPS: f64 = 1e-9;

#[derive(Clone, Copy, Debug)]
pub(super) struct Input {
    pub token: u64,
    pub source: u8,
    pub channel: u8,
    pub note: u8,
    pub velocity: f32,
    pub expression: Expression,
    pub off: Option<u64>,
    pub retrigger: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Context {
    pub now: u64,
    pub beat: f64,
    pub sample_rate: f64,
    pub tempo: f64,
    pub key: u8,
    pub scale: u8,
    pub chord: Notes,
    pub mpe: bool,
    pub upper: bool,
    pub members: u8,
    pub bend_range: f32,
    pub master_range: f32,
    pub output_channel: u8,
    pub bass_channel: u8,
    pub melody_channel: u8,
    pub aligned: bool,
    pub delay_map: [u8; 128],
}
impl Context {
    fn samples_per_beat(self) -> f64 {
        self.sample_rate.max(1.0) * 60.0 / self.tempo.max(1.0)
    }
    fn deadline(self, beat: f64) -> u64 {
        self.now.saturating_add(
            ((beat - self.beat).max(0.0) * self.samples_per_beat() - EPS)
                .max(0.0)
                .ceil() as u64,
        )
    }
    fn routing(self) -> Routing {
        Routing {
            mpe: self.mpe,
            upper: self.upper,
            members: self.members,
            bend: self.bend_range,
            master: self.master_range,
            output: self.output_channel,
            bass: self.bass_channel,
            melody: self.melody_channel,
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
struct Routing {
    mpe: bool,
    upper: bool,
    members: u8,
    bend: f32,
    master: f32,
    output: u8,
    bass: u8,
    melody: u8,
}
#[derive(Clone, Copy)]
struct Root {
    input: Input,
    pending: bool,
}
#[derive(Clone, Copy)]
struct Output {
    channel: u8,
    expression: Expression,
}
#[derive(Clone, Copy)]
struct Node {
    id: u64,
    input: Input,
    parent: Option<u64>,
    next: usize,
    live: bool,
    gate: bool,
    pulse: bool,
    gate_off: Option<f64>,
    gate_after: usize,
    destination: Option<usize>,
    split_seen: bool,
    frozen: bool,
    echo_token: Option<u64>,
    fine: f32,
    pan: Option<f32>,
    requested: u8,
    output: Option<Output>,
}
impl Node {
    fn root(input: Input) -> Self {
        Self {
            id: 0,
            input,
            parent: None,
            next: 0,
            live: true,
            gate: false,
            pulse: false,
            gate_off: None,
            gate_after: 0,
            destination: None,
            split_seen: false,
            frozen: false,
            echo_token: None,
            fine: 0.0,
            pan: None,
            requested: 0,
            output: None,
        }
    }
    fn child(self, next: usize) -> Self {
        Self {
            id: 0,
            parent: Some(self.id),
            next,
            live: true,
            gate: false,
            gate_off: None,
            gate_after: 0,
            output: None,
            echo_token: None,
            split_seen: false,
            ..self
        }
    }
}
#[derive(Clone, Copy)]
struct GateClock {
    next: f64,
    step: i64,
}

pub(super) struct Runtime {
    roots: Box<[Option<Root>]>,
    root_len: usize,
    nodes: Box<[Option<Node>]>,
    node_len: usize,
    output_serial: u64,
    controllers: Box<[[Cell<Option<f32>>; 128]]>,
    delay: delay::Runtime,
    active: bool,
    order: [MidiFxId; 4],
    harmonizer: midi_fx::HarmonizerSettings,
    gate: midi_fx::RhythmGateSettings,
    splitter: midi_fx::VoiceSplitterSettings,
    delay_settings: midi_fx::DelaySettings,
    routing: Option<Routing>,
    gate_clock: Option<GateClock>,
    serial: u64,
    dirty: bool,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            roots: vec![None; MAX_ROOTS].into_boxed_slice(),
            root_len: 0,
            nodes: vec![None; MAX_NODES].into_boxed_slice(),
            node_len: 0,
            output_serial: 0,
            controllers: (0..16)
                .map(|_| std::array::from_fn(|_| Cell::new(None)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            delay: delay::Runtime::default(),
            active: false,
            order: midi_fx::EFFECTS,
            harmonizer: Default::default(),
            gate: Default::default(),
            splitter: Default::default(),
            delay_settings: Default::default(),
            routing: None,
            gate_clock: None,
            serial: 0,
            dirty: false,
        }
    }
}
impl Runtime {
    pub(super) fn active(&self) -> bool {
        self.active
    }
    pub(super) fn output_serial(&self) -> u64 {
        self.output_serial
    }
    pub(super) fn delay_due(&self, ctx: Context) -> bool {
        self.active
            && self
                .delay
                .deadline(ctx.now, ctx.beat, ctx.samples_per_beat())
                .is_some_and(|at| at <= ctx.now)
    }
    pub(super) fn has_root(&self, token: u64) -> bool {
        self.roots[..self.root_len]
            .iter()
            .flatten()
            .any(|root| root.input.token == token)
    }
    pub(super) fn capture(&mut self, input: Input, ctx: Context) -> bool {
        if input.note > 127
            || input.channel > 15
            || !input.velocity.is_finite()
            || input.velocity <= 0.0
        {
            return false;
        }
        if !self.active {
            for root in &mut self.roots[..self.root_len] {
                if root.is_some_and(|r| r.input.off.is_some_and(|off| off <= ctx.now)) {
                    *root = None;
                }
            }
        }
        if self.has_root(input.token) {
            return false;
        }
        let index = self.roots[..self.root_len]
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.root_len);
        if index == MAX_ROOTS {
            return false;
        }
        self.root_len = self.root_len.max(index + 1);
        self.roots[index] = Some(Root {
            input,
            pending: self.active,
        });
        self.dirty |= self.active;
        true
    }
    pub(super) fn configure(
        &mut self,
        state: &MidiFxState,
        ctx: Context,
        out: &mut impl FnMut(Out),
    ) -> bool {
        self.expire_roots(ctx, out);
        let mut order = midi_fx::EFFECTS;
        for (to, from) in order.iter_mut().zip(&state.order) {
            *to = *from;
        }
        let requested =
            state.harmonizer.enabled || state.rhythm_gate.enabled || state.voice_splitter.enabled;
        // Keep the allocator responsible for notes it remapped until they drain.
        let active = requested
            || self.active
                && (self.roots[..self.root_len].iter().any(Option::is_some)
                    || self.nodes[..self.node_len].iter().any(Option::is_some));
        let changed = active != self.active
            || order != self.order
            || state.harmonizer.enabled != self.harmonizer.enabled
            || state.rhythm_gate.enabled != self.gate.enabled
            || state.voice_splitter.enabled != self.splitter.enabled
            || state.voice_splitter.pools != self.splitter.pools
            || state.voice_splitter.channels != self.splitter.channels
            || self.routing.is_some_and(|routing| routing != ctx.routing());
        if changed && self.active {
            self.clear_outputs(out);
        }
        if !changed && self.active && self.delay_settings.enabled && !state.delay.enabled {
            for index in 0..self.node_len {
                if self.nodes[index].is_some_and(|n| n.live && n.frozen && n.parent.is_none()) {
                    self.release_node(index, ctx, out);
                }
            }
            self.delay.clear();
            self.collect();
        }
        if changed && !self.active && active {
            let mut released = [[false; 128]; 16];
            for root in self.roots[..self.root_len].iter().flatten() {
                let i = root.input;
                if !released[i.channel as usize][i.note as usize] {
                    released[i.channel as usize][i.note as usize] = true;
                    out(Out::Off(i.channel, i.note, 0.0));
                }
            }
        }
        self.order = order;
        self.harmonizer = state.harmonizer;
        self.gate = state.rhythm_gate;
        self.splitter = state.voice_splitter;
        self.delay_settings = state.delay;
        self.routing = Some(ctx.routing());
        let was_active = self.active;
        self.active = active;
        if changed {
            self.nodes[..self.node_len].fill(None);
            self.node_len = 0;
            self.delay.clear();
            self.gate_clock = None;
            for root in self.roots[..self.root_len].iter_mut().flatten() {
                root.pending = active;
            }
            self.dirty = active;
            if was_active && !active {
                let mut restored = [[false; 128]; 16];
                for root in self.roots[..self.root_len].iter().flatten() {
                    let i = root.input;
                    if !restored[i.channel as usize][i.note as usize] {
                        restored[i.channel as usize][i.note as usize] = true;
                        expression_out(i.channel, i.expression, out);
                        out(Out::On(i.channel, i.note, i.velocity));
                    }
                }
            }
        }
        changed
    }
    pub(super) fn expression(&mut self, token: u64, expression: Expression) {
        let mut changed = false;
        if let Some(root) = self.roots[..self.root_len]
            .iter_mut()
            .flatten()
            .find(|root| root.input.token == token)
        {
            if root.input.expression != expression {
                root.input.expression = expression;
                changed = true;
            }
        }
        if changed && self.active {
            for node in self.nodes[..self.node_len].iter_mut().flatten() {
                if node.input.token == token && !node.frozen {
                    node.input.expression = expression;
                }
            }
            self.dirty = true;
        }
    }
    pub(super) fn set_off(&mut self, token: u64, off: Option<u64>) {
        let Some(root) = self.roots[..self.root_len]
            .iter_mut()
            .flatten()
            .find(|r| r.input.token == token)
        else {
            return;
        };
        if root.input.off == off {
            return;
        }
        root.input.off = off;
        for node in self.nodes[..self.node_len]
            .iter_mut()
            .flatten()
            .filter(|n| n.input.token == token && !n.frozen && !n.pulse)
        {
            node.input.off = off;
        }
    }
    pub(super) fn controller(&self, token: u64, cc: u8, value: f32, out: &mut impl FnMut(Out)) {
        if cc > 127 {
            return;
        }
        let mut sent = [false; 16];
        for node in self.nodes[..self.node_len]
            .iter()
            .flatten()
            .filter(|n| n.live && !n.frozen && n.input.token == token)
        {
            if let Some(output) = node.output {
                if !sent[output.channel as usize] {
                    sent[output.channel as usize] = true;
                    let cache = &self.controllers[output.channel as usize][cc as usize];
                    if cache.get() != Some(value) {
                        cache.set(Some(value));
                        out(Out::Cc(output.channel, cc, value));
                    }
                }
            }
        }
    }
    pub(super) fn release(&mut self, token: u64, ctx: Context, out: &mut impl FnMut(Out)) {
        for root in &mut self.roots[..self.root_len] {
            if root.is_some_and(|root| root.input.token == token) {
                *root = None;
            }
        }
        for i in 0..self.node_len {
            if self.nodes[i].is_some_and(|node| {
                node.live && node.parent.is_none() && !node.frozen && node.input.token == token
            }) {
                self.release_node(i, ctx, out);
            }
        }
        self.collect();
    }
    fn expire_roots(&mut self, ctx: Context, out: &mut impl FnMut(Out)) {
        for i in 0..self.root_len {
            if let Some(root) =
                self.roots[i].filter(|r| r.input.off.is_some_and(|off| off <= ctx.now))
            {
                self.release(root.input.token, ctx, out);
            }
        }
    }
    fn add(&mut self, mut node: Node) -> Option<usize> {
        let index = self.nodes[..self.node_len]
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.node_len);
        if index == MAX_NODES {
            return None;
        }
        self.node_len = self.node_len.max(index + 1);
        self.serial = self.serial.wrapping_add(1).max(1);
        node.id = self.serial;
        self.nodes[index] = Some(node);
        Some(index)
    }
    fn release_node(&mut self, index: usize, ctx: Context, out: &mut impl FnMut(Out)) {
        let Some(mut node) = self.nodes[index].filter(|n| n.live) else {
            return;
        };
        node.live = false;
        let output = node.output.take();
        self.nodes[index] = Some(node);
        self.delay.release(node.id, ctx.now, ctx.beat);
        for child in 0..self.node_len {
            if self.nodes[child].is_some_and(|n| n.live && n.parent == Some(node.id)) {
                self.release_node(child, ctx, out);
            }
        }
        if let Some(output) = output {
            if !self.nodes[..self.node_len].iter().flatten().any(|n| {
                n.live
                    && n.input.note == node.input.note
                    && n.output.is_some_and(|o| o.channel == output.channel)
            }) {
                out(Out::Off(output.channel, node.input.note, 0.0));
            }
            if !self.nodes[..self.node_len]
                .iter()
                .flatten()
                .any(|n| n.live && n.output.is_some_and(|o| o.channel == output.channel))
            {
                reset_expression(output.channel, out);
                self.invalidate_controllers(output.channel);
            }
        }
    }
    fn collect(&mut self) {
        for node in &mut self.nodes[..self.node_len] {
            if node.is_some_and(|n| !n.live && !self.delay.references(n.id)) {
                *node = None;
            }
        }
        if !self.nodes[..self.node_len]
            .iter()
            .flatten()
            .any(|n| n.live && n.gate)
        {
            self.gate_clock = None;
        }
        while self.node_len > 0 && self.nodes[self.node_len - 1].is_none() {
            self.node_len -= 1;
        }
        while self.root_len > 0 && self.roots[self.root_len - 1].is_none() {
            self.root_len -= 1;
        }
    }
    fn invalidate_controllers(&self, channel: u8) {
        for value in &self.controllers[channel as usize] {
            value.set(None);
        }
    }
    fn clear_outputs(&mut self, out: &mut impl FnMut(Out)) {
        let notes = self.owned_notes();
        for (channel, notes) in notes.iter().enumerate() {
            for (note, &owned) in notes.iter().enumerate() {
                if owned {
                    out(Out::Off(channel as u8, note as u8, 0.0));
                }
            }
            if notes.iter().any(|&owned| owned) {
                reset_expression(channel as u8, out);
                self.invalidate_controllers(channel as u8);
            }
        }
    }
    pub(super) fn clear(&mut self, _ctx: Context, out: &mut impl FnMut(Out)) {
        self.clear_outputs(out);
        self.roots[..self.root_len].fill(None);
        self.root_len = 0;
        self.nodes[..self.node_len].fill(None);
        self.node_len = 0;
        self.delay.clear();
        self.gate_clock = None;
        self.dirty = false;
    }
    pub(super) fn owned_notes(&self) -> [[bool; 128]; 16] {
        let mut notes = [[false; 128]; 16];
        for node in self.nodes[..self.node_len]
            .iter()
            .flatten()
            .filter(|n| n.live)
        {
            if let Some(output) = node.output {
                notes[output.channel as usize][node.input.note as usize] = true;
            }
        }
        notes
    }
    pub(super) fn echo_notes(&self, notes: &mut [u8]) -> usize {
        let mut count = 0;
        for node in self.nodes[..self.node_len]
            .iter()
            .flatten()
            .filter(|n| n.live && n.frozen && n.output.is_some())
        {
            if count == notes.len() {
                break;
            }
            notes[count] = node.input.note;
            count += 1;
        }
        count
    }
    pub(super) fn deadline(&self, ctx: Context) -> Option<u64> {
        if !self.active {
            return None;
        }
        if self.dirty {
            return Some(ctx.now);
        }
        self.roots[..self.root_len]
            .iter()
            .flatten()
            .filter_map(|r| r.input.off)
            .chain(
                self.nodes[..self.node_len]
                    .iter()
                    .flatten()
                    .filter(|n| n.live)
                    .filter_map(|n| n.gate_off.map(|at| ctx.deadline(at))),
            )
            .chain(self.gate_clock.map(|clock| ctx.deadline(clock.next)))
            .chain(
                self.delay
                    .deadline(ctx.now, ctx.beat, ctx.samples_per_beat()),
            )
            .min()
    }
    pub(super) fn flush(&mut self, ctx: Context, out: &mut impl FnMut(Out)) {
        if self.active && self.deadline(ctx).is_none_or(|at| at > ctx.now) {
            return;
        }
        self.expire_roots(ctx, out);
        if !self.active {
            return;
        }
        self.dirty = false;
        for i in 0..self.node_len {
            if self.nodes[i]
                .is_some_and(|n| n.live && n.gate_off.is_some_and(|at| at <= ctx.beat + EPS))
            {
                self.release_node(i, ctx, out);
            }
        }
        for i in 0..self.root_len {
            if let Some(mut root) = self.roots[i].filter(|root| root.pending) {
                root.pending = false;
                self.roots[i] = Some(root);
                self.add(Node::root(root.input));
            }
        }
        self.process_pending(ctx);
        while let Some(action) = self.delay.pop_due(ctx.now, ctx.beat) {
            match action {
                delay::Action::Off { token } => {
                    if let Some(index) = self.nodes[..self.node_len]
                        .iter()
                        .position(|n| n.is_some_and(|n| n.live && n.echo_token == Some(token)))
                    {
                        self.release_node(index, ctx, out);
                    }
                }
                delay::Action::On {
                    token,
                    source_token,
                    note,
                    velocity,
                    expression,
                    fine,
                    pan,
                    channel,
                } => {
                    if let Some(parent) = self.nodes[..self.node_len]
                        .iter()
                        .flatten()
                        .find(|n| n.id == source_token)
                        .copied()
                    {
                        let mut echo = parent.child(self.delay_stage() + 1);
                        echo.parent = None;
                        echo.frozen = true;
                        echo.echo_token = Some(token);
                        echo.input.note = ctx.delay_map[note as usize];
                        echo.input.velocity = velocity;
                        echo.input.expression = expression;
                        echo.input.off = None;
                        echo.input.retrigger = false;
                        echo.fine = fine;
                        echo.pan = Some(pan);
                        echo.requested = channel;
                        self.add(echo);
                    }
                }
            }
        }
        self.process_pending(ctx);
        self.tick_gate(ctx);
        self.process_pending(ctx);
        self.emit_ready(ctx, out);
        self.collect();
    }
    fn delay_stage(&self) -> usize {
        self.order
            .iter()
            .position(|id| *id == MidiFxId::Delay)
            .unwrap_or(3)
    }
    fn process_pending(&mut self, ctx: Context) {
        for stage in 0..4 {
            let id = self.order[stage];
            // Split rank is computed for the whole stage before any node advances.
            if id == MidiFxId::VoiceSplitter
                && self.splitter.enabled
                && (!ctx.mpe || self.splitter.pool_ranges(ctx.members, ctx.upper).is_some())
            {
                for index in 0..self.node_len {
                    let Some(mut node) = self.nodes[index].filter(|n| {
                        n.live && n.next == stage && n.input.source & self.splitter.sources != 0
                    }) else {
                        continue;
                    };
                    let mut low = node.input.note;
                    let mut high = node.input.note;
                    for other in self.nodes[..self.node_len].iter().flatten().filter(|n| {
                        n.live
                            && n.input.source & node.input.source & self.splitter.sources != 0
                            && (n.next == stage || n.split_seen)
                    }) {
                        low = low.min(other.input.note);
                        high = high.max(other.input.note);
                    }
                    node.destination = Some(if node.input.note == high {
                        2
                    } else if node.input.note == low {
                        0
                    } else {
                        1
                    });
                    // Marking here does not remove this node from the rank population.
                    node.split_seen = true;
                    self.nodes[index] = Some(node);
                }
            }
            for index in 0..self.node_len {
                let Some(mut node) = self.nodes[index].filter(|n| n.live && n.next == stage) else {
                    continue;
                };
                node.next = stage + 1;
                self.nodes[index] = Some(node);
                match id {
                    MidiFxId::Harmonizer
                        if self.harmonizer.enabled
                            && self.harmonizer.sources & node.input.source != 0 =>
                    {
                        let settings = self.harmonizer;
                        let mut pitches = [u8::MAX; 4];
                        let mut count = 0;
                        if settings.original {
                            pitches[0] = node.input.note;
                            count = 1;
                        } else {
                            self.nodes[index].as_mut().unwrap().next = END;
                        }
                        for voice in settings
                            .voices
                            .iter()
                            .filter(|v| v.enabled && v.velocity > 0.0)
                        {
                            let Some(pitch) =
                                harmony_pitch(node.input.note, voice.interval, settings.mode, ctx)
                            else {
                                continue;
                            };
                            if pitches[..count].contains(&pitch) {
                                continue;
                            }
                            pitches[count] = pitch;
                            count += 1;
                            let mut child = node.child(stage + 1);
                            child.input.note = pitch;
                            child.input.velocity =
                                (child.input.velocity * voice.velocity).clamp(0.0, 1.0);
                            self.add(child);
                        }
                    }
                    MidiFxId::RhythmGate
                        if self.gate.enabled && self.gate.sources & node.input.source != 0 =>
                    {
                        let n = self.nodes[index].as_mut().unwrap();
                        n.next = END;
                        n.gate = true;
                        n.gate_after = stage + 1;
                        if self.gate_clock.is_none() {
                            let rate = f64::from(self.gate.rate);
                            let step = if ctx.aligned {
                                (ctx.beat / rate - EPS).ceil() as i64
                            } else {
                                0
                            };
                            self.gate_clock = Some(GateClock {
                                next: if ctx.aligned {
                                    step as f64 * rate
                                } else {
                                    ctx.beat
                                },
                                step,
                            });
                        }
                    }
                    MidiFxId::Delay => {
                        self.delay.capture(
                            self.delay_settings,
                            node.input.source,
                            node.id,
                            node.input.note,
                            node.input.velocity,
                            node.input.expression,
                            node.input.off,
                            ctx.now,
                            ctx.beat,
                            ctx.sample_rate,
                            ctx.tempo,
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    fn tick_gate(&mut self, ctx: Context) {
        let Some(mut clock) = self.gate_clock else {
            return;
        };
        if clock.next > ctx.beat + EPS {
            return;
        }
        let step = clock.step.rem_euclid(i64::from(self.gate.steps.max(1))) as usize;
        let velocity = self.gate.pattern[step];
        let off = clock.next + f64::from(self.gate.rate * self.gate.gate);
        if velocity > 0.0 && off > ctx.beat + EPS {
            for index in 0..self.node_len {
                if let Some(parent) = self.nodes[index].filter(|n| n.live && n.gate) {
                    let mut pulse = parent.child(parent.gate_after);
                    pulse.input.velocity = (pulse.input.velocity * velocity).clamp(0.0, 1.0);
                    pulse.input.off = Some(ctx.deadline(off));
                    pulse.input.retrigger = false;
                    pulse.pulse = true;
                    pulse.gate_off = Some(off);
                    self.add(pulse);
                }
            }
        }
        clock.step += 1;
        clock.next += f64::from(self.gate.rate);
        // A seek is normally cleared by Engine; this bound also handles a late caller.
        if clock.next <= ctx.beat + EPS {
            let missed = ((ctx.beat - clock.next) / f64::from(self.gate.rate)).floor() as i64 + 1;
            clock.step += missed;
            clock.next += missed as f64 * f64::from(self.gate.rate);
        }
        self.gate_clock = Some(clock);
    }
    fn emit_ready(&mut self, ctx: Context, out: &mut impl FnMut(Out)) {
        for index in 0..self.node_len {
            let Some(mut node) = self.nodes[index].filter(|n| n.live && n.next == 4) else {
                continue;
            };
            let range = if ctx.mpe {
                ctx.bend_range
            } else {
                ctx.master_range
            }
            .max(1.0);
            let mut expression = node.input.expression;
            expression.bend = (expression.bend + node.fine / (200.0 * range)).clamp(0.0, 1.0);
            if let Some(mut output) = node.output {
                if expression != output.expression {
                    expression_out(output.channel, expression, out);
                    output.expression = expression;
                    node.output = Some(output);
                    self.nodes[index] = Some(node);
                }
                continue;
            }
            if !ctx.mpe && !node.frozen {
                let channel = node
                    .destination
                    .map_or(node.input.channel, |d| self.splitter.channels[d] - 1);
                for old in 0..self.node_len {
                    if self.nodes[old].is_some_and(|n| {
                        n.live && n.frozen && n.output.is_some_and(|o| o.channel == channel)
                    }) {
                        self.release_node(old, ctx, out);
                    }
                }
            }
            let mut channel = self.allocate(node, expression, ctx);
            if channel.is_none() && ctx.mpe && !node.frozen {
                let mask = self.pool_mask(node, ctx);
                let victim = self.nodes[..self.node_len]
                    .iter()
                    .enumerate()
                    .filter_map(|(index, n)| {
                        let n = n.filter(|n| n.live && n.frozen)?;
                        let output = n.output?;
                        (mask & (1 << output.channel) != 0).then_some((index, n.id))
                    })
                    .min_by_key(|(_, id)| *id)
                    .map(|(index, _)| index);
                if let Some(victim) = victim {
                    self.release_node(victim, ctx, out);
                    channel = self.allocate(node, expression, ctx);
                }
            }
            let Some(channel) = channel else {
                node.next = END;
                self.nodes[index] = Some(node);
                continue;
            };
            let owned = self.nodes[..self.node_len].iter().flatten().any(|n| {
                n.live
                    && n.input.note == node.input.note
                    && n.output.is_some_and(|o| o.channel == channel)
            });
            if ctx.mpe || node.frozen {
                super::midi::emit_bend_range(channel, range, out);
            }
            expression_out(channel, expression, out);
            out(Out::Cc(channel, 10, (node.pan.unwrap_or(0.0) + 1.0) * 0.5));
            if owned && node.input.retrigger {
                out(Out::Off(channel, node.input.note, 0.0));
            }
            if !owned || node.input.retrigger {
                out(Out::On(
                    channel,
                    node.input.note,
                    node.input.velocity.clamp(0.01, 1.0),
                ));
            }
            node.output = Some(Output {
                channel,
                expression,
            });
            self.output_serial = self.output_serial.wrapping_add(1);
            self.invalidate_controllers(channel);
            self.nodes[index] = Some(node);
        }
    }
    fn allocate(&self, node: Node, expression: Expression, ctx: Context) -> Option<u8> {
        let occupied = |channel| {
            self.nodes[..self.node_len]
                .iter()
                .flatten()
                .any(|n| n.live && n.output.is_some_and(|o| o.channel == channel))
        };
        if ctx.mpe {
            let mask = self.pool_mask(node, ctx);
            if node.input.retrigger && !node.frozen {
                if let Some(channel) = self.nodes[..self.node_len]
                    .iter()
                    .flatten()
                    .filter(|n| {
                        n.live
                            && !n.frozen
                            && n.input.note == node.input.note
                            && n.destination == node.destination
                            && n.input.channel == node.input.channel
                    })
                    .filter_map(|n| n.output.map(|o| o.channel))
                    .find(|&channel| mask & (1 << channel) != 0)
                {
                    return Some(channel);
                }
            }
            if node.requested > 0 {
                let channel = node.requested - 1;
                return (channel < 16 && mask & (1 << channel) != 0 && !occupied(channel))
                    .then_some(channel);
            }
            return (0..16).find(|&channel| mask & (1 << channel) != 0 && !occupied(channel));
        }
        let destination = node.destination.map(|d| self.splitter.channels[d] - 1);
        if let Some(channel) = destination {
            if node.requested != 0 && node.requested - 1 != channel {
                return None;
            }
            if node.frozen
                && self.nodes[..self.node_len].iter().flatten().any(|n| {
                    n.live
                        && n.output.is_some_and(|o| {
                            o.channel == channel
                                && (o.expression != expression || n.pan != node.pan)
                        })
                })
            {
                return None;
            }
            return Some(channel);
        }
        if node.frozen {
            if node.requested != 0 {
                let channel = node.requested - 1;
                return (channel < 16 && !occupied(channel)).then_some(channel);
            }
            return (0..16).find(|&channel| {
                channel != ctx.output_channel
                    && channel != ctx.bass_channel
                    && channel != ctx.melody_channel
                    && !occupied(channel)
            });
        }
        Some(node.input.channel)
    }
    fn pool_mask(&self, node: Node, ctx: Context) -> u16 {
        if let Some(pools) = self
            .splitter
            .enabled
            .then(|| self.splitter.pool_ranges(ctx.members, ctx.upper))
            .flatten()
        {
            pools[node.destination.unwrap_or(3)]
        } else {
            (0..ctx.members.min(15)).fold(0u16, |mask, i| {
                mask | (1 << if ctx.upper { 14 - i } else { 1 + i })
            })
        }
    }
}
fn expression_out(channel: u8, expression: Expression, out: &mut impl FnMut(Out)) {
    out(Out::Bend(channel, expression.bend));
    out(Out::Pressure(channel, expression.pressure));
    out(Out::Cc(channel, 74, expression.timbre));
}
fn reset_expression(channel: u8, out: &mut impl FnMut(Out)) {
    expression_out(channel, Expression::default(), out);
    out(Out::Cc(channel, 10, 0.5));
}
fn harmony_pitch(note: u8, interval: i8, mode: HarmonizerMode, ctx: Context) -> Option<u8> {
    if mode == HarmonizerMode::Fixed {
        return u8::try_from(i16::from(note) + i16::from(interval))
            .ok()
            .filter(|&n| n <= 127);
    }
    let mut mask = [false; 12];
    match mode {
        HarmonizerMode::Scale => {
            for &tone in harmony::scale_tones(ctx.scale) {
                mask[((ctx.key % 12 + tone) % 12) as usize] = true;
            }
        }
        HarmonizerMode::Chord => {
            for &tone in ctx.chord.as_slice() {
                mask[(tone % 12) as usize] = true;
            }
        }
        HarmonizerMode::Fixed => unreachable!(),
    }
    let mut pitches = [0u8; 128];
    let mut count = 0;
    for pitch in 0..128 {
        if mask[pitch % 12] {
            pitches[count] = pitch as u8;
            count += 1;
        }
    }
    let nearest = (0..count).min_by_key(|&i| (pitches[i].abs_diff(note), pitches[i]))? as i16;
    let target = nearest + i16::from(interval);
    (target >= 0 && target < count as i16).then(|| pitches[target as usize])
}
#[cfg(test)]
#[path = "fx_tests.rs"]
mod tests;
