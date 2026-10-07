# Release acceptance

## Automated/source verification

Run focused crate check, unit tests, Clippy and the install script. `scripts/verify-clap.py` additionally loads the real CLAP bundle as a tiny headless host and verifies passthrough, event timing, parameter automation, harmony, CC1 and MPE. It does not access a DAW, MIDI hardware or audio device.

For the focused Hold chord regression, run `python3 chordboard/scripts/verify-clap.py ~/Library/Audio/Plug-Ins/CLAP/Chordboard.clap --held-chord` from the workspace root. It checks generated sus2 MIDI after simultaneous or adjacent-sample releases in either order, alteration-only release, and root re-triggering back to the base chord.

The native `ui-preview` executable uses the real plugin, parameter updates and processing loop with a dummy backend by default. It supports interactive layout and control inspection without opening audio/MIDI devices. MIDI Learn requires an explicitly selected MIDI-capable backend. Preview and source checks do not establish DAW routing or hardware behavior; no DAW or hardware verification is claimed by the interface redesign.

## Interface acceptance checklist

- Inspect the balanced 1120 × 704 layout at normal and scaled sizes in dark and light appearances. Memory keys form the fourth staggered row inside the chord module. Quality, the single piano control for Close/Open/Wide spread, inversion and transpose belong beneath it, with clear margins between modules.
- Check Learn MIDI keyboard and Keys active/enabled/off state labels. Existing chord and Shift-memory shortcuts retain their focus and enable rules.
- Save with an empty + card, or arm Save chord and choose an empty or filled card. Cancel, focus loss and opening a menu or panel must cancel armed saving. Saving is unavailable without a valid chord. Confirm acknowledgement follows the persisted slot update; identical chords report already stored. Check recall, deletion, drag move/swap and readable labels with full hover detail.
- Check direct Up/Down/Alternate direction buttons, Auto/MPE/Regular MIDI selection and effective protocol display in Auto. Menus and popovers must close explicitly.
- Check displayed percentages, milliseconds and semitones; enter values using their displayed units. Verify host/manual tempo and sweep-sync retain their existing semantics.
- Check X Strum and Y Destination summaries and local controller editors. Preserve learned source/channel and range behavior, field calibration, strum rendering, trail and processing timing.
- Check EXPAND / COLLAPSE on Auto and Manual Strum. The field should cover the body below the title header and return without changing strum calibration.
- Check Manual Strum trackpad Latch: hover plays the field without a click, including when expanded; click-drag still captures, and MIN / MAX grips still calibrate.
- Click and slightly move each Modulation source: no route or parameter should change. Drag to Strings or Spread: eligible controls highlight, the cable names the proposed link, and only a valid drop assigns it. Drop elsewhere, press Escape during the drag, and lose focus to cancel. Repeating a link must open its existing slot; replacing its source must preserve minimum, maximum and curve. Check linked-source hover highlights and both appearance modes.

These checks cover the interface. The user-operated checks below remain necessary for host and hardware acceptance.

## Manual host + Osmose acceptance (user-operated)

1. Connect only Osmose's external Play / Port 1 to the test track; choose its MPE configuration. Place Chordboard before an MPE destination instrument and confirm generated MIDI reaches it.
2. Choose MPE in the protocol dropdown and select Manual Strum. Match the destination member bend ±48 and master ±2 ranges. Learn the physical bottom octave. Confirm selector keys are silent and latch the displayed quality.
3. With Hold chord off, in Chord mode play C alone, then D, F and F# as the second input. Expect sus2, sus4 and C–E–F#. Press a third key: it waits while two inputs own the chord, then starts its own chord when it is the only held key. Check both release orders.
4. With Hold chord off, release the second input first: base chord returns. Repeat, releasing the root first: the sole remaining key starts its own chord and takes expression ownership. Turn Hold chord on, play C+D and release both almost simultaneously in either order: Csus2 must remain. Release only D: Csus2 must also remain. Re-press C to reset; while C is held, press F to replace the alteration. Repeat in Auto Strum, Manual Strum and Arpeggiator, checking that releases do not restart playback.
5. Apply initial pressure, deeper aftertouch and sideways motion to the first input. Every generated voice must follow pressure, CC74 and bend. Applying those gestures to the second input must not take over the chord.
6. Hold the second input while releasing the first, then play another note that reuses its MPE channel. Confirm expression stays with the promoted root rather than jumping to the reused channel.
7. Switch to Manual Strum. Sweep the mod wheel both directions, slowly and quickly. Every crossed string should trigger in order; holding still and small jitter must not retrigger. The wheel must not change modes.
8. Hold expressive pressure/bend while slowly strumming: later notes should begin with the existing expression, without a neutral-pitch attack. Test touch gate and a learned fader if available.
9. Press Up/Down: inversion wraps after the number of chord tones, with no accumulating octave shift. Test changing three tones to four while inverted, and verify releases for replaced notes.
10. Test latch, CC64, repeated notes, mode changes, seek/loop, transport stop, Escape, editor close, preset load and project reload. There must be no stuck notes or restored held notes.
11. Choose a key/scale and play both highlighted and outside chords. Save with empty cards and Save chord, cancel an armed save, then Shift-click memories, change harmony and recall them while strumming. Verify alteration, inversion and spread. Test all note filters; verify optional bass/upper channels in standard MIDI and per-voice channels in MPE.
12. With QWERTY enabled and the editor focused, test every mapped key and both banks. Hold keys through OS key repeat, then focus a value editor or another app. Confirm no unintended text-entry notes or retained keyboard notes. Compare UI at normal and scaled sizes, both appearances.
13. Save a host preset and the host project. Reload each and verify chord slots, learned mappings, control zone, inversion, filter and MPE settings. Repeat in both CLAP and VST3 where the host exposes MIDI output.

Record host/version, plugin format, destination patch and MIDI configuration with any failure. Hardware acceptance is not represented as complete until performed with the actual Osmose.

## Sequencer acceptance

Run `cargo check -p chordboard`, `cargo test -p chordboard --quiet`, and `cargo clippy -p chordboard` for the sequencer engine change. Build and install with `chordboard/scripts/install.sh`, then run `python3 chordboard/scripts/verify-clap.py ~/Library/Audio/Plug-Ins/CLAP/Chordboard.clap --sequencer`. The headless check verifies stored patterns through the actual CLAP state interface, relative harmony/bass output, exact first/second step timing, signed CC offsets, panic, persistence, and old-preset fallback.

- Inspect compact/expanded editors at the current 1621 × 840 design size, both appearances and scaled windows. Verify the bottom piano slides away, four lane rows remain aligned, selected-step text stays legible, and zoom/close return to the piano.
- Enable sequencing: expression step-edit buttons must appear above matching modulators, select the correct field/CC track, open the editor, and toggle back to Steps on a second click. Disable sequencing: those buttons must disappear. Check reassigned CC tracks and normal modulator routing.
- Click each numbered page button during playback: it must select the editing page and request playback for only that lane at its next step boundary, preserving phase. Request a page from the Pattern inspector, from a memory, and by dragging a modulator onto the whole 1–8 row (including gaps); check base/effective/pending indicators and next-step phase wrapping at differing lengths and rates.
- Edit and name inactive pages while another page plays: timing and ringing notes must continue. Check each generator, tone mute versus skip, ratchets, probability, rests, ties, gate, and microtiming. Focus loss/Escape discard uncommitted text; typing must not play QWERTY notes.
- Program relative Harmony steps, held empty steps, and Live resets; change the played chord without accumulating offsets. Strum alongside sequencing without enabling a separate Manual Strum toggle; verify the shared harmony and independent note releases.
- Check pressure, timbre, CC offsets, and semitone bends on an MPE destination and regular MIDI destination. Neutral steps must not emit an unknown CC11 baseline of zero. Live controller changes must retain step offsets without feeding them into the next baseline.
- Save/reload presets and projects. Old mode automation must retain its original mapping. State replacement, stop/seek, panic, and voice exhaustion must leave no stuck notes. DAW and hardware checks remain user-operated.
