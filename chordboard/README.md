# Chordboard

A MIDI-only harmony and strumming effect with Pleasant UI. VST3 and CLAP; no internal sound engine. Place Chordboard before an instrument and enable MIDI output/routing in the host. Stereo audio passes through unchanged.

## Play

Click the chord keyboard to focus the editor. QWERTY captures playing keys only while focused and enabled. The number row (`1 2 3 4 5 6 7 8 9 0 - +`) selects major chords, `Q W E R T Y U I O P [ ]` selects minor chords, and `A S D F G H J K L ; ' Enter` selects dominant seventh chords. The `+` position uses the physical `=/+` key without requiring Shift; Enter is labelled `ENT`. Each staggered row includes all twelve roots; Chromatic/Fifths changes their order. The bottom row (`Z X C V B N M ,`) recalls the eight memories; Shift plus a memory key captures the current chord. Memory shortcuts follow the same QWERTY focus and enable rules. Transpose adjusts the playing register; Key sets the first chord button and keyboard key’s root, along with scale highlighting.

On a MIDI keyboard, click **Set control octave**, then press C in the desired octave (or another key to transpose the control layout). The twelve keys select chords by interval from that learned tonic: C major, D♭ b9, D sus2, E♭ minor, E major, F sus4, F♯ diminished, G power, A♭ augmented, A sixth, B♭ dominant seventh, B major seventh. The learned range is labelled above the chord keys and highlighted on the full MIDI keyboard below; hover a control key to see its assignment. Control keys select harmony silently: Note On, Note Off, zero-velocity Note On and per-note pressure are consumed, without starting a chord or strum. Everything outside the zone is a performance note. Learn can be cancelled with a second click.

The first performance note anchors the chord. A second note adds or replaces a degree: C+D gives Csus2, C+F gives Csus4, C+F# gives C–E–F#. Extra notes wait while two inputs own the chord. With **Hold chord off**, releasing the second input removes its alteration. When only one key remains held, it becomes the root of a fresh chord, including a key pressed early while the previous chord was still held. Computer-keyboard chords retain that key’s original quality; MIDI notes retain their velocity and take expression from their own channel.

With **Hold chord on**, releasing either or both notes retains the complete altered chord, regardless of release order or timing. Releasing just D in C+D keeps Csus2. Re-press C to return to the base C chord; while C remains down, pressing another alteration replaces the retained one. Releases do not restart the strum or arpeggiator. Turning Hold chord off returns to the physically held inputs.

Harmony currently combines one of twelve base qualities with **one alteration**. A dominant seventh plus a semitone adds ♭9, but a tritone replaces the fifth with ♭5. Combined extensions such as 7♭9♯11, an additive ♯11 retaining the fifth, and ordinary ninth chords retaining the third are not available yet; a second note a tone above the root produces sus2 instead of add9.

Up/Down arrows cycle inversions within the original register, wrapping to root position. They do not accumulate octave changes. Input key repeats are ignored. Focus loss releases computer-keyboard notes without releasing unrelated MIDI inputs.

## Split keyboard and sustained layers

**Key split** separates left-hand chord selection from right-hand melody. Notes below the split choose the chord; the split note and higher notes pass through unchanged on their incoming MIDI channels. The default boundary is C4 (MIDI 60). Drag the split marker, click a key on the bottom piano when split is enabled, or click the split readout and type a note such as C4 or 60. The learned control octave is always consumed before the split is applied. Left-hand selection remains available after its keys are released.

**Always play bass** holds the selected root. **Always play full chord** holds the resolved chord tones. They start when a right-hand key is held and stop when the last right-hand key is released. With **Hold chord** on, the accompaniment stays after release and changes to the next selected chord. Melody notes always follow their own releases. Both buttons can be enabled without doubling shared tones; ordinary strum tails retain their note lengths. With Key split off, held performance keys gate these layers instead. All three buttons start off in new instances.

The full MIDI 0–127 piano shows control keys, the left/right split, held input keys, sounding output notes, and selected voicing. Voice-leading paths animate above the keys, with exact note movements available on hover. Changing the split releases current notes to prevent stranded note ownership.

## Performance

The full MIDI keyboard animates each voice-leading transition: hollow starting dots connect to moving destination dots. Upward, downward and stationary voices use distinct colors. The latest paths remain visible; hover for exact note names and semitone changes.

Changing chords preserves ringing notes. Strum and arpeggiator notes finish their scheduled lengths, including across key releases; outgoing sustained Chord-mode tones receive the current Note length. Retriggering the same pitch or exhausting voice capacity can still end an older voice. Panic and playback/routing changes stop voices.

- **Auto Strum** sweeps all selected **Strings** on chord changes, matching the displayed pitches, with up/down/alternating direction, duration and velocity contour. **Strings played** limits each sweep to 1–12 of those strings (capped by Strings); upward sweeps start at the low end, downward sweeps at the high end. It is also a controller-routing destination.
- **Manual Strum** uses the XY pad or **CC1 mod wheel**, automatically mapped to X. Increasing CC1 sweeps upward; decreasing it sweeps downward. Drag a modulator chip onto the highlighted pad to link **Strum X / sweep**; explicit routes override the direct X mapping and support range/curve adjustment. Stationary controls do not retrigger. Switching modes is explicit.
- **Arpeggiator** offers up, down, up/down, played order and seeded random. Rate is in quarter-note beats (0.25 = sixteenth notes). Host tempo, gate, swing, octave range and bounded humanization are supported. It also runs while playing live with stopped transport; stop/seek transitions clear owned notes.

X crosses 3–12 virtual strings, eight by default. Y controls strike velocity by default, or gate/pressure/timbre/bend/custom CC. Touching a string directly plucks it. Learned controllers seed their initial position before sweeping, avoiding a jump from an unknown start point. Both travel directions play. **Hold chord** and CC64 sustain retain the last harmony without retaining physical input slots.

Click X or Y beneath the performance surface to edit that controller directly. Click MIDI LEARN, then move a controller. The mapping row also cycles input type (off / 7-bit CC / paired 14-bit CC / pitch bend), CC number, with learned CC assignments showing their captured channel. For 14-bit CC use the MSB number 0–31; the LSB is number+32. Calibrate X by dragging MIN / MAX grips on the Manual Strum pad; strings stretch with the range and stop at a 25% minimum span. Calibrate Y with the field’s horizontal MIN / MAX grips; reverse X or Y inside the corresponding controller editor. Channel-filtered pitch-bend faders must be outside the incoming MPE member zone. MPE member bend and CC74 cannot be learned away from expression. The default lower MPE input zone reserves channels 2–16, so a pitch-bend fader can use master channel 1; CC faders remain usable. No hardware-specific motor feedback is emitted.

## Osmose / MPE

Use **Osmose Play / Port 1**, External MIDI mode **MPE**. Choose MPE in the protocol dropdown and select Manual Strum. Select an MPE-capable destination instrument and enable MPE in that instrument/host. Configure matching bend ranges: default member ±48 semitones, master ±2.

The first accepted note's pressure, CC74 and bend fan out to every generated voice. The second input affects harmony only. Each output voice receives its own member channel, with expression initialized before Note On, including later strummed notes. Master expression remains separate. Incoming RPN bend sensitivity and zone configuration are recognized. Released source channels cannot take expression ownership when reused.

The output protocol defaults to Auto; its dropdown offers Auto, MPE and Regular MIDI. Auto shows the effective output protocol beside its automatic selection. Auto detects announced MPE zones, host per-note tuning/brightness, or member-channel expression with single notes on multiple member channels. Notes alone do not enable MPE. Existing saved MPE/regular choices are preserved. MPE output uses the lower zone by default: master channel 1, members 2–16. Upper-zone and reduced member counts are configurable. Standard single-channel MIDI is also available. In MPE mode, bass/upper channel splitting is intentionally ignored to preserve one member channel per voice. An engaged Y assignment overrides only its chosen expression dimension.

Actual Osmose feel, downstream patch response, and host-specific MIDI routing require the hardware acceptance checks in `docs/ACCEPTANCE.md`.

## Discover and recall

Compatible chords are always highlighted and Roman numerals are always displayed. Select a key and scale from the menus below the chord keyboard to update them. Outside chords remain playable. Highlighting considers all chord tones, not only the root. Major, natural/harmonic minor, Dorian, Mixolydian, Lydian and major/minor pentatonic scales are available.

Eight memory keys form the fourth staggered row inside the chord keyboard module and store the current harmonic recipe: root, alteration, quality, inversion, transpose and spread. Click an empty **+** card to save, or click **Save chord** and then choose a card to save or replace it; **Cancel** leaves the cards unchanged. Saving requires a valid current chord. Shift-click and Shift plus a memory key remain capture shortcuts. The interface confirms a save only after the stored slot updates; an identical chord reports that it is already stored, and an unconfirmed request asks you to try again. Click a filled card to recall it into the current performance mode. Recall remains available for strumming until a new performance input replaces it. Empty cards have a dashed outline. Hover a filled card to reveal its × delete button. Drag a filled card onto an empty card to move it, or onto another filled card to swap the two. Release outside the cards to cancel a drag. Slots persist with project state and host presets.

The **Note Filter** in Output outputs all tones, bass, top, bass+top, odd tones or even tones after voicing. Standard MIDI can route bass and upper voices to separate channels. MPE retains its member-channel allocator.

The eight slots are a manual progression sketchpad, not a step sequencer. MIDI drag-out, constrained chord search and a separate previous/current comparison view are follow-ons.

## Root on select and modulation

**Root on select** sounds the transposed root immediately when choosing a chord, including in Manual Strum. It follows Note length and normal release/sustain behavior; Auto Strum and the first arp step do not double it. It is off by default in existing projects.

The lower-right **Modulation** strip meters pitch wheel, mod wheel (CC1), expression (CC11), breath (CC2), pressure, timbre (CC74), accepted note velocity, and raw strum X/Y. MIDI source meters use the latest incoming value across channels. **Drag a source meter onto a control** to link it. Source cards highlight on hover; dragging shows eligible destinations, a cable and the proposed link. Clicking a meter makes no assignment. Drop elsewhere or press Escape during the drag to cancel. Hovering a linked source highlights its visible destinations.

A successful drop opens that link’s range and curve editor. Dropping onto an already linked destination reuses the effective route and preserves its range and curve; the preview says when the drop will replace its source. Repeating the same link opens it without adding duplicates. Open **Output** or a controller editor first to drop onto its local controls. **Routes** opens all 16 slots and also offers destination menus for controls hidden in other playback modes.

Each slot has an enable switch, source, destination, and a compact interactive graph. Drag the left/right endpoints vertically to set minimum/maximum; drag the center node vertically to bend the curve. Destination readouts use musical units (for example, Strings 3–12 or Spread Close–Wide). Minimum greater than maximum reverses the mapping. **Linear ↺** resets the curve. The small live dot and input/output readout show the current modulation.

Sources can drive multiple destinations across performance, voicing, expression and MIDI-output controls. Routes replace the destination within their range; when several routes target the same parameter, the highest-numbered enabled route wins. Routed controls show teal live values while edits still change the saved base setting. Disabling a route restores that base setting (or recalled voicing). Routes and their settings save with the host project and can be automated. Existing X/Y MIDI assignments remain available alongside these routes. Touch-range X/Y minimum and maximum are not modulation destinations; their grips remain available for direct calibration. Saved routes to those four bounds are inactive. Strum X / sweep remains routable without feeding its result back into modulation sources.

## Interface

The 1120 × 856 interface balances the chord keyboard and voicing controls with the performance surface. Manual Strum includes **EXPAND**, which grows the strum field over the plugin body below the title header. The module edge uses the shared quarter-second quintic slide: performance width and position interpolate while the chord surfaces shrink, matching Composure’s envelope expansion. **COLLAPSE** returns the field. Auto Strum and Arpeggiator close an expanded field. Quality, Spread, voice-leading mode, inversion and transpose sit beneath the chord buttons. Click **Spread** to cycle **Close / Open / Wide**; the full keyboard below shows the resulting voicing. Spread affects playback voicing; Quality remains the shared chord choice. **Set control octave** sets the control octave. The computer-keyboard control shows **Keys active**, **Keys enabled** or **Keys off** to distinguish focused playing, enabled playing without focus, and disabled shortcuts.

Auto Strum defaults to zero sweep for immediate chords. Direct **Up / Down / Alternate** buttons select sweep direction. Arpeggiator controls appear only in Arpeggiator mode; Auto Strum keeps its sweep controls beside the strings. The protocol dropdown beside the voice count offers Auto, MPE and Regular MIDI, with channel, bend-range and note-filter options in the adjacent Output menu. Menus and controller popovers have explicit close controls. **X Strum** and **Y Destination** summaries open local controller editors. Manual CC assignments accept any input channel; MIDI Learn locks the assignment to the learned channel. Y defaults to CC11 (Expression). The X and Y MIN / MAX grips retain direct range calibration on the strum surface. In Manual Strum, the touch-icon **Latch** control plays the strum field on pointer hover so a mouse or trackpad does not need to click and hold.

There are no settings tabs or pages. Key labels and playing shortcuts are unchanged. Hover memories, meters or controls for contextual detail. The interface uses shared colors, appearance preferences and controls. Key depression, pressure glow, string pulses, gesture trails and expression meters follow engine state; visuals never control MIDI timing. The strum field, trails, timing and calibration behavior are preserved. Continuous controls can be dragged from anywhere in their bounds or scrolled; click a value without dragging for text entry (Shift-drag makes fine adjustments). Values display meaningful units such as percentages, milliseconds and semitones, and typed input accepts the displayed units. On/off settings are buttons.

Use the host's preset and project features to save custom setups. Host state persists mappings, chord memories and editor settings. Runtime held notes are never saved. Appearance follows the application's preference file.

## Build and install

From the parent `audio-plugins` workspace:

```sh
cargo check -p chordboard
cargo test -p chordboard --quiet
cargo clippy -p chordboard
chordboard/scripts/install.sh
```

The installer rebuilds, bundles, signs and verifies both Chordboard formats in the user's plugin directories. Running the bundler alone does not install them.

This repository is a full-history subtree at `chordboard/`. Standalone checkout builds need the sibling Pleasant libraries and the parent workspace's patched NIH-plug/baseview/Vizia sources under `Damian Channel Strip/vendor`, matching existing plugins. Prefer the integrated workspace; subtree export does not carry those dependencies or the parent installer.

Interactive native preview (uses the real plugin with a dummy backend by default):

```sh
cargo run -p chordboard --features ui-preview --bin chordboard-preview
```

Click the mode controls and protocol dropdown to change performance mode and output protocol. This uses NIH-plug's standalone wrapper, including parameter updates and the real processing loop. The default dummy backend opens no audio/MIDI devices, so MIDI Learn requires an explicitly selected MIDI-capable backend (`-- --help` lists options). On macOS it requires a graphical session and OpenGL access. No REAPER automation is used.

## Architecture and reference

`harmony.rs` resolves fixed-capacity voicings. `engine/` owns input admission, lifecycle, timing, controllers and MPE. `lib.rs` adapts sample-timed NIH-plug events. `bridge.rs` provides bounded lock-free UI queues; UI overflow requests Panic rather than losing releases. `params.rs` defines stable host IDs and versioned saved fields. `ui/` renders and handles focused input.

All audio-thread storage is bounded: two inputs, fifteen generated voices, sixty-four pending strikes, a 512-event outgoing batch and bounded GUI queues. An overflowing batch is discarded atomically and previously sounding notes are released; a new performance input resumes playback. Note Off ownership is independent of the pending-strike queue. Channel exhaustion ends the oldest voice before reuse.

Original software inspired by the interaction model of Benjamin Poilvé's [minichord](https://github.com/BenjaminPoilve/minichord); firmware was consulted as a reference, not copied or linked. Discovery features were inspired by KawaChord2. Chordboard does not include their branding, firmware, presets or artwork. GPL-3.0-or-later, consistent with the shared project libraries.

Y destination, custom CC and Reverse Y share the Y controller editor; Reverse X is in the X controller editor. Compact vertical pressure, timbre and bend meters sit at bottom right. Control-octave Learn is on the main interface. Transpose buttons beside inversion step by −12, −1, +1 or +12 semitones, with Reset returning to zero. Escape closes an open menu or popover and cancels armed memory saving.

Playback pages use the shared quarter-second slide animation. QWERTY labels hide when keyboard playing is disabled. Latched chords retain their Root/Color readouts after release. Swing supports −75% to +75%, reversing the long/short step pattern below zero.

HOST SYNC follows DAW tempo (120 BPM fallback if unavailable); disable it to set manual BPM. Auto Strum’s SWEEP SYNC selects beat divisions instead of milliseconds, using the same tempo source.

### Smart voice leading

The Voice leading button cycles **Nearest resolution → Furthest dominant resolution → Off** (default). Nearest chooses the inversion and octave placement with the least ordered voice movement from the previous chord. Furthest dominant favors the most movement for major/dominant chords resolving down a fifth to major or minor, and uses nearest movement elsewhere. Candidates stay within one octave of the requested voicing and retain its chord tones; note filters apply afterward.

The first chord uses the requested inversion/register. Repeated chords retain their resolved voicing across note releases. Panic/reset clears the history; changing inversion, transpose, spread, playback mode, or voice-leading mode starts a fresh voicing. The mode is saved and automatable as `voice_leading`; existing presets default to Off.
