# Release acceptance

## Automated/source verification

Run focused crate check, unit tests, Clippy and the install script. `scripts/verify-clap.py` additionally loads the real CLAP bundle as a tiny headless host and verifies passthrough, event timing, parameter automation, harmony, CC1 and MPE. It does not access a DAW, MIDI hardware or audio device.

For the focused chord latch regression, run `python3 chordboard/scripts/verify-clap.py ~/Library/Audio/Plug-Ins/CLAP/Chordboard.clap --held-chord` from the workspace root. It checks generated sus2 MIDI after simultaneous or adjacent-sample releases in either order, alteration-only release, and root re-triggering back to the base chord.

For combined extensions, run `python3 chordboard/scripts/verify-clap.py ~/Library/Audio/Plug-Ins/CLAP/Chordboard.clap --extensions`. It checks combined altered extensions, octave independence, duplicate-note ownership, and latch/release behavior in the installed binary.

The native `ui-preview` executable uses the real plugin, parameter updates and processing loop with a dummy backend by default. It supports interactive layout and control inspection without opening audio/MIDI devices. MIDI Learn requires an explicitly selected MIDI-capable backend. Preview and source checks do not establish DAW routing or hardware behavior; no DAW or hardware verification is claimed by the interface redesign.

## Interface acceptance checklist

- Inspect the balanced 1621 × 840 layout at normal and scaled sizes in dark and light appearances. Memory keys form the fourth staggered row inside the chord module. Ten memories and Latch occupy the fourth row; piano voicing controls offer Close/Open/Wide/Drop 2/Drop 3, with inversion above the piano and transpose in Chords, with clear margins between modules.
- Check Learn octave / Control octave and Bass/Melody Learn Split labels, including Cancel while learning. Chord and Shift-memory shortcuts work while the editor is focused and release on focus loss or text entry.
- Save with an empty + card, Shift-click a card, or Shift plus its memory shortcut to replace it. Saving is unavailable without a valid chord. Confirm acknowledgement follows the persisted slot update; identical chords report already stored. Check recall, deletion, drag move/swap and readable labels with full hover detail.
- Check all seven sequencer direction choices and the MPE on/off switch, including its input-detection warning. Menus and popovers must close explicitly.
- Check displayed percentages, milliseconds and semitones; enter values using their displayed units. Verify host/manual tempo and sweep-sync retain their existing semantics.
- Check X Strum and Y Destination summaries and local controller editors. Preserve learned source/channel and range behavior, field calibration, strum rendering, trail and processing timing.
- Check the graph zoom icon in Play. The field should cover the body below the title header and return without changing strum calibration.
- Check Play Hover strum: hover plays the field without a click, including when expanded; click-drag still captures, and MIN / MAX grips still calibrate.
- Click and slightly move each Modulation source: no route or parameter should change. Drag to Strings or Spread: eligible controls highlight, the cable names the proposed link, and only a valid drop assigns it. Drop elsewhere, press Escape during the drag, and lose focus to cancel. Repeating a link must open its existing slot; replacing its source must preserve minimum, maximum and curve. Check linked-source hover highlights and both appearance modes.

- Check Escape cancels valid or invalid value drafts without changing the parameter or closing its panel. Shift fine adjustments must not toggle chord latch.
- After focus loss, a sequencer click must restore keyboard focus; focus loss during a drag must end the gesture, including when host bounds are temporarily zero.
- Navigate grouped route menus with arrow keys across group boundaries and unavailable choices. Escape, focus loss, or changing source cancels pending deletion.
- Inspect all new icons, long names, and narrow Y ranges (0–1%, 99–100%, and close middle endpoints) in both appearances and field sizes. Labels must not overlap handles or each other.

These checks cover the interface. The user-operated checks below remain necessary for host and hardware acceptance.

## Manual host + Osmose acceptance (user-operated)

1. Connect only Osmose's external Play / Port 1 to the test track; choose its MPE configuration. Place Chordboard before an MPE destination instrument and confirm generated MIDI reaches it.
2. Enable MPE on and open Play. Match the destination member bend ±48 and master ±2 ranges. Learn the physical bottom octave. Confirm selector keys are silent and latch the displayed quality.
3. With Latch off, with Chord on Select play C alone, then D, F and F# as the second input. Expect sus2, sus4 and C–E–F#. Press third and later keys: check octave-independent ♭9/9/♯9/11/♯11/♭13/13 additions and remaining chord tones, including multiple variants together. Check release orders: with Latch off, released additions disappear and the sole remaining key starts its own chord.
4. With Latch off, release the second input first: base chord returns. Repeat, releasing the root first: the sole remaining key starts its own chord and takes expression ownership. Turn Latch on, play C+D and release both almost simultaneously in either order: Csus2 must remain. Release only D: Csus2 must also remain. Re-press C to reset; while C is held, press F to replace the alteration. Repeat with sequencing bypassed, Once, and Loop; manual tails must keep their deadlines and Loop must retain phase.
5. Apply initial pressure, deeper aftertouch and sideways motion to the first input. Every generated voice must follow pressure, CC74 and bend. Applying those gestures to the second input must not take over the chord.
6. Hold the second input while releasing the first, then play another note that reuses its MPE channel. Confirm expression stays with the promoted root rather than jumping to the reused channel.
7. Open Play. Sweep the mod wheel both directions, slowly and quickly. Every crossed string should trigger in order; holding still and small jitter must not retrigger. The wheel must not change modes.
8. Hold expressive pressure/bend while slowly strumming: later notes should begin with the existing expression, without a neutral-pitch attack. Test touch gate and a learned fader if available.
9. Press Up/Down: inversion wraps after the number of chord tones, with no accumulating octave shift. Test changing three tones to four while inverted, and verify releases for replaced notes.
10. Test latch, CC64, repeated notes, mode changes, seek/loop, transport stop, Escape, editor close, preset load and project reload. There must be no stuck notes or restored held notes.
11. Choose a key/scale and play both highlighted and outside chords. Save with empty cards and Shift-click memories, change harmony and recall them while strumming. Verify alteration, every added pitch class, inversion and spread. Test all note filters; verify optional bass/upper channels in standard MIDI and per-voice channels in MPE.
12. With the editor focused, test every mapped chord key and all ten memory shortcuts. Hold keys through OS key repeat, then focus a value editor or another app. Confirm no unintended text-entry notes or retained keyboard notes. Compare UI at normal and scaled sizes, both appearances.
13. Save a host preset and the host project. Reload each and verify chord slots, learned mappings, control zone, inversion, filter and MPE settings. Repeat in both CLAP and VST3 where the host exposes MIDI output.

Record host/version, plugin format, destination patch and MIDI configuration with any failure. Hardware acceptance is not represented as complete until performed with the actual Osmose.

## Play / Sequencer separation acceptance

Use `cargo check -p chordboard` (add `--all-targets` when tests changed), focused `cargo test -p chordboard engine --quiet`, `cargo test -p chordboard ui --quiet`, and `cargo test -p chordboard params --quiet`, plus `cargo clippy -p chordboard` for engine changes. Install with `chordboard/scripts/install.sh`, then run `python3 chordboard/scripts/verify-clap.py ~/Library/Audio/Plug-Ins/CLAP/Chordboard.clap --sequencer`. The headless CLAP check verifies the actual installed binary: note timing, expression offsets, disabled legacy lanes, state persistence, panic, and the Play/Sequencer power matrix.

- With sequencer power off, try every legacy mode value and all three selection behaviors. Strum Only is silent until a manual gesture; Root on Select plays one root; Chord on Select sustains the full voicing. Repeat with Strumfield power off. Neither power button changes the other control or Arp repeat choice.
- Enable sequencing while manually strumming; bypass sequencing while notes ring. Only sequencer notes are released. Repeat with overlapping pitches and Chord on Select; its sustain must remain. Voice-leading voicing/history must survive sequencer power changes.
- With voice leading Off, play several chord changes under each global/page Arp–Chord Accent choice. Voicing must stay as requested. Click blank diagram space: no parameter changes. Hover the voice-leading dots to reveal the mode; click them to cycle it. Use the global/page accent controls in the Arpeggiator header: only the intended setting changes. Neither accent control appears in the keyboard or sequencer step header.
- Once traverses the Arp page once per new root/memory selection; Loop repeats. Chords continues independently. Verify page changes, each lane's enable, independent Chords rate, and active/pending page indicators.
- In both appearances, check visible cell faces: 4/4 groups of four, 3/4 groups of three, 6/8 groups of six. First group is lighter; rests retain shading. Change rate, meter, pattern length, and editing page. Shading must remain visible beneath values and selection/playhead outlines.
- Edit Steps, Gate, Chance, Octave, Timing, Repeats, Speed, Fine, Velocity, Mod 1, and Mod 2. Check negative microtiming, live CC baseline changes, independent manual velocity, inactive-page edits, and state replacement. The two dedicated modulation selectors appear in the sequencer header. Verify saved legacy expression offsets still play.
- Verify split input routing, memory recall, latch/pedal, MPE member ownership, regular MIDI overlap, note filtering, panic, transport stop/seek, editor focus loss, and saved states. Old Bass/Harmony data must remain readable without playing hidden lanes.

DAW appearance, actual destination sound, and Osmose acceptance remain user-operated. Do not automate REAPER.

- Mod 1 and Mod 2 appear beside the sequencer modifiers; the MIDI modulator meters have no per-step circles. Drag/paint values, save/reload, and verify both sources follow steps including rests. Stop/reset returns them to zero. Old presets default these sources to zero.
- Route Mod 1 into Pressure or Pitch and verify emitted MIDI on generated notes; route Mod 1 into Mod 2 and then a parameter and verify chained values regardless of route slot order. Circular links remain bounded, and removing links restores live input baselines.
- The header displays the full chord including extensions and slash bass, without Held/Last chord subtitles. MIN/MAX labels remain centered beside handles at both edges, with no diagonal connector.
