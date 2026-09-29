# Chordboard

A MIDI-only harmony and strumming effect with Pleasant UI. VST3 and CLAP; no internal sound engine. Place Chordboard before an instrument and enable MIDI output/routing in the host. Stereo audio passes through unchanged.

## Play

Click the chord keyboard to focus the editor. QWERTY captures playing keys only while focused and enabled. The number row (`1 2 3 4 5 6 7 8 9 0 - +`) selects major chords, `Q W E R T Y U I O P [ ]` selects minor chords, and `A S D F G H J K L ; ' Enter` selects dominant seventh chords. The `+` position uses the physical `=/+` key without requiring Shift; Enter is labelled `ENT`. Each staggered row includes all twelve roots; Chromatic/Fifths changes their order. The bottom row (`Z X C V B N M ,`) recalls the eight memories; Shift plus a memory key captures the current chord. Memory shortcuts follow the same QWERTY focus and enable rules. Transpose adjusts the playing register; Key sets the first chord button and keyboard key’s root, along with scale highlighting.

On a MIDI keyboard, click **Learn Low Octave**, then press its lowest physical key. That key and the following eleven semitones become latched quality selectors: major, minor, dominant seventh, major seventh, minor seventh, diminished, augmented, major sixth, minor sixth, diminished seventh, half-diminished seventh, power chord. The Control Octave menu beside the keyboard shows assignments and MIDI numbers. Everything outside the zone is a performance note. Learn can be cancelled with a second click.

The first performance note anchors the chord. A second note adds or replaces a degree: C+D gives Csus2, C+F gives Csus4, C+F# gives C–E–F#. A third note is ignored until released and pressed again when space is available. Releasing the second input removes its alteration. Releasing the root first keeps its original identity until the second key also releases. The second note never becomes the root.

Up/Down arrows cycle inversions within the original register, wrapping to root position. They do not accumulate octave changes. Input key repeats are ignored. Focus loss releases computer-keyboard notes without releasing unrelated MIDI inputs.

## Performance

- **Auto Strum** sweeps on chord changes, with up/down/alternating direction, duration and velocity contour.
- **Manual Strum** uses the XY pad or **CC1 mod wheel**, automatically mapped to X. Increasing CC1 sweeps upward; decreasing it sweeps downward. Stationary controls do not retrigger. Switching modes is explicit.
- **Arpeggiator** offers up, down, up/down, played order and seeded random. Rate is in quarter-note beats (0.25 = sixteenth notes). Host tempo, gate, swing, octave range and bounded humanization are supported. It also runs while playing live with stopped transport; stop/seek transitions clear owned notes.

X crosses 3–12 virtual strings, eight by default. Y controls strike velocity by default, or gate/pressure/timbre/bend/custom CC. Touching a string directly plucks it. Learned controllers seed their initial position before sweeping, avoiding a jump from an unknown start point. Both travel directions play. Latch and CC64 sustain retain the last harmony without retaining physical input slots.

Click X or Y beneath the performance surface to edit that controller directly. Click MIDI LEARN, then move a controller. The mapping row also cycles input type (off / 7-bit CC / paired 14-bit CC / pitch bend), CC number, with learned CC assignments showing their captured channel. For 14-bit CC use the MSB number 0–31; the LSB is number+32. Calibrate X by dragging MIN / MAX grips on the Manual Strum pad; strings stretch with the range and stop at a 25% minimum span. Calibrate Y with the field’s horizontal MIN / MAX grips; reverse X or Y inside the corresponding controller editor. Channel-filtered pitch-bend faders must be outside the incoming MPE member zone. MPE member bend and CC74 cannot be learned away from expression. The default lower MPE input zone reserves channels 2–16, so a pitch-bend fader can use master channel 1; CC faders remain usable. No hardware-specific motor feedback is emitted.

## Osmose / MPE

Use **Osmose Play / Port 1**, External MIDI mode **MPE**. Enable MPE Output and select Manual Strum. Select an MPE-capable destination instrument and enable MPE in that instrument/host. Configure matching bend ranges: default member ±48 semitones, master ±2.

The first accepted note's pressure, CC74 and bend fan out to every generated voice. The second input affects harmony only. Each output voice receives its own member channel, with expression initialized before Note On, including later strummed notes. Master expression remains separate. Incoming RPN bend sensitivity and zone configuration are recognized. Released source channels cannot take expression ownership when reused.

The output protocol defaults to AUTO; click to cycle AUTO → MPE → REG. AUTO detects announced MPE zones, host per-note tuning/brightness, or member-channel expression with single notes on multiple member channels. Notes alone do not enable MPE. Existing saved MPE/regular choices are preserved. MPE output uses the lower zone by default: master channel 1, members 2–16. Upper-zone and reduced member counts are configurable. Standard single-channel MIDI is also available. In MPE mode, bass/upper channel splitting is intentionally ignored to preserve one member channel per voice. An engaged Y assignment overrides only its chosen expression dimension.

Actual Osmose feel, downstream patch response, and host-specific MIDI routing require the hardware acceptance checks in `docs/ACCEPTANCE.md`.

## Discover and recall

Compatible chords are always highlighted and Roman numerals are always displayed. Select a key and scale from the menus below the chord keyboard to update them. Outside chords remain playable. Highlighting considers all chord tones, not only the root. Major, natural/harmonic minor, Dorian, Mixolydian, Lydian and major/minor pentatonic scales are available.

**Shift-click one of eight memory slots** to capture the current harmonic recipe: root, alteration, quality, inversion, transpose and spread. Click to recall it into the current performance mode. Recall remains available for strumming until a new performance input replaces it. Empty slots have a dashed outline. Hover a filled slot to reveal its × delete button. Drag a filled slot onto an empty slot to move it, or onto another filled slot to swap the two. Release outside the slots to cancel a drag. Slots persist with project state and host presets.

The **Note Filter** in Output outputs all tones, bass, top, bass+top, odd tones or even tones after voicing. Standard MIDI can route bass and upper voices to separate channels. MPE retains its member-channel allocator.

The eight slots are a manual progression sketchpad, not a step sequencer. MIDI drag-out, constrained chord search and a separate previous/current comparison view are follow-ons.

## Interface

The interface keeps Quality and Spread beneath playback, beside the expression meters, in strum and arpeggiator modes. Spread affects the playback voicing. Auto Strum defaults to zero sweep for immediate chords. Quality remains the shared chord choice. The X and Y MIN / MAX grips on the strum pad set their ranges directly. Manual CC assignments accept any input channel; MIDI Learn locks the assignment to the learned channel. Y defaults to CC11 (Expression). Spread is a clickable piano keyboard that cycles Close, Open and Wide and shows the full voicing. Arpeggiator controls appear only in Arpeggiator mode; Auto Strum keeps its sweep controls beside the strings. AUTO / MPE / REG cycles directly beside the voice count, with channel, bend-range and note-filter options in the adjacent Output menu. X and Y open local controller editors, and Control Octave stays beside the keyboard. There are no settings tabs or pages. Key labels and playing shortcuts are unchanged. Hover memories, meters or controls for contextual detail. The interface uses shared colors, appearance preferences and controls. Key depression, pressure glow, string pulses, gesture trails and expression meters follow engine state; visuals never control MIDI timing. Continuous controls can be dragged from anywhere in their bounds or scrolled; click a value without dragging for text entry (Shift-drag makes fine adjustments); on/off settings are buttons.

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

Click the mode and MPE buttons to change modes. This uses NIH-plug's standalone wrapper, including parameter updates and the real processing loop. The default dummy backend opens no audio/MIDI devices, so MIDI Learn requires an explicitly selected MIDI-capable backend (`-- --help` lists options). On macOS it requires a graphical session and OpenGL access. No REAPER automation is used.

## Architecture and reference

`harmony.rs` resolves fixed-capacity voicings. `engine/` owns input admission, lifecycle, timing, controllers and MPE. `lib.rs` adapts sample-timed NIH-plug events. `bridge.rs` provides bounded lock-free UI queues; UI overflow requests Panic rather than losing releases. `params.rs` defines stable host IDs and versioned saved fields. `ui/` renders and handles focused input.

All audio-thread storage is bounded: two inputs, fifteen generated voices, sixty-four pending strikes, a 512-event outgoing batch and bounded GUI queues. An overflowing batch is discarded atomically and previously sounding notes are released; a new performance input resumes playback. Note Off ownership is independent of the pending-strike queue. Channel exhaustion ends the oldest voice before reuse.

Original software inspired by the interaction model of Benjamin Poilvé's [minichord](https://github.com/BenjaminPoilve/minichord); firmware was consulted as a reference, not copied or linked. Discovery features were inspired by KawaChord2. Chordboard does not include their branding, firmware, presets or artwork. GPL-3.0-or-later, consistent with the shared project libraries.

Y destination, custom CC and Reverse Y share the Y controller editor; Reverse X is in the X controller editor. Compact vertical pressure, timbre and bend meters sit at bottom right. Control-octave Learn is on the main interface. Transpose buttons beside inversion step by −12, −1, +1 or +12 semitones, with Reset returning to zero. Escape has no plugin action.

Playback pages use the shared quarter-second slide animation. QWERTY labels hide when keyboard playing is disabled. Latched chords retain their Root/Color readouts after release. Swing supports −75% to +75%, reversing the long/short step pattern below zero.

HOST SYNC follows DAW tempo (120 BPM fallback if unavailable); disable it to set manual BPM. Auto Strum’s SWEEP SYNC selects beat divisions instead of milliseconds, using the same tempo source.
