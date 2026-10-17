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
- Check all seven arpeggiator orderings and the MPE on/off switch, including its input-detection warning. Menus and popovers must close explicitly.
- Check displayed percentages, milliseconds and semitones; enter values using their displayed units. Verify host/manual tempo and sweep-sync retain their existing semantics.
- Check X/Y modulator tiles open the routing window; no local controller editor or stale mapping tooltip remains. Preserve learned source/channel and range behavior, field calibration, strum rendering, trail and processing timing.
- Check the graph zoom icon in Play. The field should cover the body below the title header and return without changing strum calibration.
- Check Play Hover strum: hover plays the field without a click, including when expanded; click-drag still captures, and MIN / MAX grips still calibrate.
- Click and slightly move each Modulation source: no route or parameter should change. Drag to Strings or Spread: eligible controls highlight, the cable names the proposed link, and only a valid drop assigns it. Drop elsewhere, press Escape during the drag, and lose focus to cancel. Repeating a link must open its existing slot; replacing its source must preserve minimum, maximum and curve. Check linked-source hover highlights and both appearance modes.

- Check Escape cancels valid or invalid value drafts without changing the parameter or closing its panel. Shift fine adjustments must not toggle chord latch.
- After focus loss, an arpeggiator string click must restore keyboard focus; focus loss during a drag must end the gesture, including when host bounds are temporarily zero.
- Navigate grouped route menus with arrow keys across group boundaries and unavailable choices. Escape, focus loss, or changing source cancels pending deletion.
- Inspect all new icons, long names, and narrow Y ranges (0–1%, 99–100%, and close middle endpoints) in both appearances and field sizes. Labels must not overlap handles or each other.

These checks cover the interface. The user-operated checks below remain necessary for host and hardware acceptance.

## Manual host + Osmose acceptance (user-operated)

1. Connect only Osmose's external Play / Port 1 to the test track; choose its MPE configuration. Place Chordboard before an MPE destination instrument and confirm generated MIDI reaches it.
2. Enable MPE on and open Play. Match the destination member bend ±48 and master ±2 ranges. Learn the physical bottom octave. Confirm selector keys are silent and latch the displayed quality.
3. With Latch off, with Chord on Select play C alone, then D, F and F# as the second input. Expect sus2, sus4 and C–E–F#. Press third and later keys: check octave-independent ♭9/9/♯9/11/♯11/♭13/13 additions and remaining chord tones, including multiple variants together. Check release orders: with Latch off, released additions disappear and the sole remaining key starts its own chord.
4. With Latch off, release the second input first: base chord returns. Repeat, releasing the root first: the sole remaining key starts its own chord and takes expression ownership. Turn Latch on, play C+D and release both almost simultaneously in either order: Csus2 must remain. Release only D: Csus2 must also remain. Re-press C to reset; while C is held, press F to replace the alteration. Repeat with the arpeggiator bypassed, Once, Loop and Sync Loop; manual tails must retain their deadlines and Sync Loop must preserve phase.
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

## Arpeggiator and MIDI Delay acceptance

Run focused verification from the workspace root:

```sh
cargo check -p chordboard --all-targets
cargo test -p chordboard engine --quiet
cargo test -p chordboard ui --quiet
cargo test -p chordboard params --quiet
cargo clippy -p chordboard
chordboard/scripts/install.sh
python3 chordboard/scripts/verify-clap.py ~/Library/Audio/Plug-Ins/CLAP/Chordboard.clap --arpeggiator --midi-delay --play-contract
```

The compile check includes test targets because the editor and engine tests changed. The install script rebuilds from the current checkout and verifies both the bundled executable and installed plugin. The CLAP harness loads that installed binary, discovers serialized page parameter IDs, and uses the fresh `arpeggiator-v1` state. Old presets and sequencer migration are outside this release's compatibility contract. Archived sequencer restoration instructions are in the sequencer archive notes.

The installed tests check C → E → G → E as four independent occurrences, exact six-thousand-sample 1/16 intervals at 120 BPM / 48 kHz, Once completion, Mute versus Skip timing, all-skipped silence and eight-Page settings roundtrip. Delay checks cover exact ten-millisecond repeats, independent tap intervals, feedback decay below MIDI velocity 1, the 64-generation cap, captured durations, per-tap pitch/pan/fine pitch/expression isolation, ping pong, Arp source exclusion and panic. The power check covers independent arp/Strumfield power and Chord on Select ownership. Engine tests additionally cover host beat alignment/seeks, modulation position, page boundaries, source capture, routing exhaustion and callback-size independence.

- The module contains one string for each pattern occurrence, equally spaced at zero swing, including repeated notes and cycles longer than 32 steps. Verify every ordering over chord/octave changes; Random's visible cycle must match playback.
- Velocity is the initial lane. Lane and ordering buttons slide in large icon choices over the strings; verify hover, selection, keyboard navigation, and covered-input capture. Check Gate, Chance, Octave, Timing, Mod 1 and Mod 2. Fast drags fill crossed strings; Shift draws a straight origin ramp without changing Latch. Cmd/Ctrl-click and modifier-drag restore defaults. Gate is an absolute percentage of the step interval. Timing is one string dragged horizontally within ±49%, with a dotted swung reference; it never paints neighbors.
- Every lane keeps a visible Play → Mute → Skip button below every string. Mute consumes an interval; Skip removes that occurrence. Cmd/Ctrl-click restores Play. An entirely skipped cycle stays silent and responsive.
- Check repeat icons in both the selected mode and dropdown: skip-forward for Once, repeat arrows for Loop, repeat arrows with a clock for Sync Loop, and repeat arrows with beat-grid ticks for Aligned Loop. Once completes one cycle per new chord/memory. Loop restarts on chord selection. Sync Loop, the default, preserves rhythm and wraps the retained counter into changed cycle lengths. Aligned Loop follows host beat zero with swing across starts and seeks; swing must shift the visible strings and display whole −100%…+100% values; unavailable host timing or a free rate visibly falls back to Sync Loop.
- Eight Pages recall their own order, rate, octaves, swing, humanize, playback mode, adjustment lanes and Delay settings. Page changes animate with the shared slide. Requested Pages switch playback at the next rhythmic boundary, with active/pending indicators. Continuous modes preserve phase; a Page request after completed Once starts playback. Arp power and routing remain global. Arp octaves never alter manual strum octaves.
- Mod 1 and Mod 2 remain bipolar additive sources and update on mute/chance-failed steps. Arp Position is a separate unipolar 0–100% source over the effective cycle, counting mute and excluding skip. Check one-step/inactive zero, chained sources, bounded circular links, and baseline restoration when links are removed.
- Bypassing arp releases its owned notes while manual strum, held Chord on Select, Bass, Melody, voice leading and Delay tails retain their ownership. Check overlapping pitches, both output protocols, preset replacement, panic, transport stop/seek, editor focus loss and project reload.
- The Delay button is at the arp module's upper right. Its animated slide-over stays inside the module, captures covered controls and route targets, and closes without stopping Delay. Check both appearances, scaled sizes and keyboard focus. Delay power operates while arp power is off.
- Basic Delay controls edit power, sync/free time, tap velocity and feedback. Advanced adds up to eight independently positioned taps, source switches for Arp / Manual Strum / Chords / Bass / Melody, ping pong and routing. Default: off, Arp only, one synced eighth-note tap, 75% velocity, 35% feedback and neutral pitch/pan with automatic channels.
- Tap numbered handles move time laterally on every lane; strings edit Velocity, Feedback, Pitch, Fine Pitch, Pan or Output Channel. Check fast painting, Shift ramps and Cmd/Ctrl reset. Feedback is limited to 95%. Each tap repeats at its own interval with its own feedback; pitch stays fixed across generations and echoes never re-enter Delay.
- Check source note duration and attack pressure/timbre/bend on delayed notes. Static pan uses CC10; ping pong alternates left/right. Fine pitch combines with captured bend on isolated compatible channels. Explicit channel conflicts and channel exhaustion drop echoes without moving dry expression or stealing dry voices. Compare automatic and explicit routing against an instrument configured for the chosen bend range.
- Chord and Page changes preserve existing echo tails. Delay bypass, panic, transport stop/seek and preset replacement release the appropriate owned voices and cancel future echoes. Check long held notes and overlapping taps as well as short gates.

DAW appearance, destination sound and Osmose acceptance remain user-operated. Do not automate REAPER.
