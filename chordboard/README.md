# Chordboard

A MIDI-only harmony and strumming effect with Pleasant UI. VST3 and CLAP; no internal sound engine. Place Chordboard before an instrument and enable MIDI output/routing in the host. Stereo audio passes through unchanged.

## Play

Click the chord keyboard to focus the editor. QWERTY captures playing keys only while focused and enabled. The number row (`1 2 3 4 5 6 7 8 9 0 - +`) selects major chords, `Q W E R T Y U I O P [ ]` selects minor chords, and `A S D F G H J K L ; ' Enter` selects dominant seventh chords. The `+` position uses the physical `=/+` key without requiring Shift; Enter is labelled `ENT`. Each staggered row includes all twelve roots; Chromatic/Fifths changes their order. Keyboard base C and transpose are independent controls.

On a MIDI keyboard, click **Learn Low Octave**, then press its lowest physical key. That key and the following eleven semitones become latched quality selectors: major, minor, dominant seventh, major seventh, minor seventh, diminished, augmented, major sixth, minor sixth, diminished seventh, half-diminished seventh, power chord. The control diagram shows assignments and MIDI numbers. Everything outside the zone is a performance note. Learn can be cancelled with a second click.

The first performance note anchors the chord. A second note adds or replaces a degree: C+D gives Csus2, C+F gives Csus4, C+F# gives C–E–F#. A third note is ignored until released and pressed again when space is available. Releasing the second input removes its alteration. Releasing the root first keeps its original identity until the second key also releases. The second note never becomes the root.

Up/Down arrows cycle inversions within the original register, wrapping to root position. They do not accumulate octave changes. Input key repeats are ignored. Escape releases generated notes while the editor has keyboard focus. Focus loss releases computer-keyboard notes without releasing unrelated MIDI inputs.

## Performance

- **Chord** plays the voiced notes together and preserves common notes on changes.
- **Auto Strum** sweeps on chord changes, with up/down/alternating direction, duration and velocity contour.
- **Manual Strum** uses the XY pad or **CC1 mod wheel**, automatically mapped to X. Increasing CC1 sweeps upward; decreasing it sweeps downward. Stationary controls do not retrigger. Switching modes is explicit.
- **Arpeggiator** offers up, down, up/down, played order and seeded random. Rate is in quarter-note beats (0.25 = sixteenth notes). Host tempo, gate, swing, octave range and bounded humanization are supported. It also runs while playing live with stopped transport; stop/seek transitions clear owned notes.

X crosses 3–12 virtual strings, eight by default. Y controls strike velocity by default, or gate/pressure/timbre/bend/custom CC. Touching a string directly plucks it. Learned controllers seed their initial position before sweeping, avoiding a jump from an unknown start point. Gate controls can arm/disarm a fader; both travel directions play. Latch and CC64 sustain retain the last harmony without retaining physical input slots.

Choose MAP X, MAP Y or MAP GATE, click MIDI LEARN, then move a controller. The mapping row also cycles input type (off / 7-bit CC / paired 14-bit CC / pitch bend), channel and CC number. For 14-bit CC use the MSB number 0–31; the LSB is number+32. Use Expression controls for range calibration and reversal. Channel-filtered pitch-bend faders must be outside the incoming MPE member zone. MPE member bend and CC74 cannot be learned away from expression. The default lower MPE input zone reserves channels 2–16, so a pitch-bend fader can use master channel 1; CC faders remain usable. No hardware-specific motor feedback is emitted.

## Osmose / MPE

Use **Osmose Play / Port 1**, External MIDI mode **MPE**. Enable MPE Output and select Manual Strum. Select an MPE-capable destination instrument and enable MPE in that instrument/host. Configure matching bend ranges: default member ±48 semitones, master ±2.

The first accepted note's pressure, CC74 and bend fan out to every generated voice. The second input affects harmony only. Each output voice receives its own member channel, with expression initialized before Note On, including later strummed notes. Master expression remains separate. Incoming RPN bend sensitivity and zone configuration are recognized. Released source channels cannot take expression ownership when reused.

MPE output defaults to lower zone: master channel 1, members 2–16. Upper-zone and reduced member counts are configurable. Standard single-channel MIDI is also available. In MPE mode, bass/upper channel splitting is intentionally ignored to preserve one member channel per voice. An engaged Y assignment overrides only its chosen expression dimension.

Actual Osmose feel, downstream patch response, and host-specific MIDI routing require the hardware acceptance checks in `docs/ACCEPTANCE.md`.

## Discover and recall

Select a key and scale from the menus below the chord keyboard to highlight compatible chords and display Roman numerals. Outside chords remain playable. Highlighting considers all chord tones, not only the root. Major, natural/harmonic minor, Dorian, Mixolydian, Lydian and major/minor pentatonic scales are available.

**Shift-click one of eight memory slots** to capture the current harmonic recipe: root, alteration, quality, inversion, transpose and spread. Click to recall it into the current performance mode. Recall remains available for strumming until a new performance input replaces it or Escape clears it. Empty slots have a dashed outline. Hover a filled slot to reveal its × delete button. Drag a filled slot onto an empty slot to move it, or onto another filled slot to swap the two. Release outside the slots or press Escape to cancel a drag. Slots persist with project state and host presets.

Discovery's **Note Filter** outputs all tones, bass, top, bass+top, odd tones or even tones after voicing. Standard MIDI can route bass and upper voices to separate channels. MPE retains its member-channel allocator.

The eight slots are a manual progression sketchpad, not a step sequencer. MIDI drag-out, constrained chord search and a separate previous/current comparison view are follow-ons.

## Interface

The interface uses shared colors, appearance preferences and controls. Key depression, pressure glow, string pulses, gesture trails and expression meters follow engine state; visuals never control MIDI timing. Reduced Motion removes decorative motion while preserving state feedback. Continuous controls can be dragged, scrolled or double-clicked for text entry; on/off settings are buttons.

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

Optional read-only native visual preview (no audio devices or DAW):

```sh
cargo run -p chordboard --features ui-preview --bin chordboard-preview
```

The preview verifies rendering, not host parameter editing or MIDI routing. On macOS it requires a graphical session and OpenGL access. No REAPER automation is used.

## Architecture and reference

`harmony.rs` resolves fixed-capacity voicings. `engine/` owns input admission, lifecycle, timing, controllers and MPE. `lib.rs` adapts sample-timed NIH-plug events. `bridge.rs` provides bounded lock-free UI queues; UI overflow requests Panic rather than losing releases. `params.rs` defines stable host IDs and versioned saved fields. `ui/` renders and handles focused input.

All audio-thread storage is bounded: two inputs, fifteen generated voices, sixty-four pending strikes, a 512-event outgoing batch and bounded GUI queues. An overflowing batch is discarded atomically and previously sounding notes are released; a new performance input resumes playback. Note Off ownership is independent of the pending-strike queue. Channel exhaustion ends the oldest voice before reuse.

Original software inspired by the interaction model of Benjamin Poilvé's [minichord](https://github.com/BenjaminPoilve/minichord); firmware was consulted as a reference, not copied or linked. Discovery features were inspired by KawaChord2. Chordboard does not include their branding, firmware, presets or artwork. GPL-3.0-or-later, consistent with the shared project libraries.
