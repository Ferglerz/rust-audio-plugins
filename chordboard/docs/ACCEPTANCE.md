# Release acceptance

## Automated/source verification

Run focused crate check, unit tests, Clippy and the install script. `scripts/verify-clap.py` additionally loads the real CLAP bundle as a tiny headless host and verifies passthrough, event timing, parameter automation, harmony, CC1 and MPE. It does not access a DAW, MIDI hardware or audio device.

The native `ui-preview` executable renders the real view for dark/light layout inspection. It is deliberately not an interactive host simulator. Never treat visual-preview checks as proof of hardware behavior.

## Manual host + Osmose acceptance (user-operated)

1. Connect only Osmose's external Play / Port 1 to the test track; choose its MPE configuration. Place Chordboard before an MPE destination instrument and confirm generated MIDI reaches it.
2. Enable MPE Output and select Manual Strum. Match the destination member bend ±48 and master ±2 ranges. Learn the physical bottom octave. Confirm selector keys are silent and latch the displayed quality.
3. In Chord mode play C alone, then D, F and F# as the second input. Expect sus2, sus4 and C–E–F#. Press a third key: it must not enter until released/repressed with a free slot.
4. Release the second input first: base chord returns. Repeat, releasing the root first: the original root remains until both accepted notes release. Confirm the second key never takes expression ownership.
5. Apply initial pressure, deeper aftertouch and sideways motion to the first input. Every generated voice must follow pressure, CC74 and bend. Applying those gestures to the second input must not take over the chord.
6. Hold the second input while releasing the first, then play another note that reuses its MPE channel. Confirm expression does not jump to that ignored note.
7. Switch to Manual Strum. Sweep the mod wheel both directions, slowly and quickly. Every crossed string should trigger in order; holding still and small jitter must not retrigger. The wheel must not change modes.
8. Hold expressive pressure/bend while slowly strumming: later notes should begin with the existing expression, without a neutral-pitch attack. Test touch gate and a learned fader if available.
9. Press Up/Down: inversion wraps after the number of chord tones, with no accumulating octave shift. Test changing three tones to four while inverted, and verify releases for replaced notes.
10. Test latch, CC64, repeated notes, mode changes, seek/loop, transport stop, Escape, editor close, preset load and project reload. There must be no stuck notes or restored held notes.
11. Choose a key/scale and play both highlighted and outside chords. Shift-click memories, change harmony, recall them while strumming. Verify alteration, inversion and spread. Test all note filters; verify optional bass/upper channels in standard MIDI and per-voice channels in MPE.
12. With QWERTY enabled and the editor focused, test every mapped key and both banks. Hold keys through OS key repeat, then focus a value editor or another app. Confirm no unintended text-entry notes or retained keyboard notes. Compare UI at normal and scaled sizes, both appearances and Reduced Motion.
13. Save a host preset and the host project. Reload each and verify chord slots, learned mappings, control zone, inversion, filter and MPE settings. Repeat in both CLAP and VST3 where the host exposes MIDI output.

Record host/version, plugin format, destination patch and MIDI configuration with any failure. Hardware acceptance is not represented as complete until performed with the actual Osmose.
