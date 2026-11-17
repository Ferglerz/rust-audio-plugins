# Automixer

Mono/stereo linked automixer with gain sharing and optional simultaneous spectral ducking, built as VST3 and CLAP. Every instance contains the same 64-microphone mixer and shared controls. The gain-sharing reference is [leafac's REAPER automixer](https://github.com/leafac/reaper/blob/main/Effects/leafac_Automixer.jsfx); this is a separate Rust implementation for linked track instances.

## Setup

1. Insert one Automixer on each microphone track, before track faders. Use the same plugin format on every track.
2. Leave the host’s **This microphone (0 = Auto)** parameter at **Auto** to assign the hardware input number when available and free. Stereo and multichannel inputs use their first channel. If the input number is unavailable, in use, or above 64, the instance takes a free number. Enter a number to pin a cue target manually. Conflicting manual numbers pass audio at unity and are excluded from sharing until corrected.
3. Open any instance to see all connected microphones, eight strips per page. **IN**, **OUT** and **GR** show input RMS, output RMS, and applied reduction. Click the type button to cycle **VOICE**, **OTHER**, and **DRUMS**. The local strip carries **THIS MIC**. Each strip has a combined MIDI channel/weight CC dropdown. Double-click a strip name to set an alias, or clear the alias to follow the host.
4. Drag a strip's **GAIN WEIGHT** to change that microphone's priority from −12 to +12 dB. This changes its share of the available gain; it does not boost its output above unity. Click values to type, Shift-drag for finer adjustments, or double-click controls to reset them.
5. **GAIN AMOUNT** sets how much of the computed reduction to apply. 0% disables gain-sharing reduction. 100% applies the full broadband reduction; optional spectral ducking and EQ remain independent. **VOICE MAX GR** and **OTHER MAX GR** are the hard reduction limits for the two groups. Those three stay on the main row, next to the mix-state chip. Open the upper **cog** for chorus count, threshold, hold, fade, and voice assist. Automate shared controls and weights from one instance to avoid conflicting automation lanes.

Track names come from CLAP track info or VST3 channel context when the host supplies them. In REAPER, both formats can also use the REAPER extension API to read the track name and record input. Track sends, MIDI, and virtual loopback inputs have no hardware number for this purpose.

Automatic assignments are resolved at audio activation. Saved assignments are reused when still available and the hardware input is unchanged or unavailable. A different hardware input is considered on the next activation. Track renames update labels without renumbering. REAPER input labels refresh on activation, editor opening, and host track notifications. Use a manually pinned number if you need a cue target to stay fixed across hardware repatching.

## Attenuation and chorus behavior

Sharing uses each input's smoothed RMS multiplied by its priority, divided by the sum of weighted levels **in the same group**. Voice microphones share only with other Voice microphones. Other and Drums share one non-voice pool. A single microphone in its group receives unity gain; two equal microphones in that group each approach −6.02 dB. Stereo input channels are linked and count as one microphone. Gain Sharing has zero audio latency; detector and gain envelopes have response times.

**VOICE MAX GR** and **OTHER MAX GR** are hard attenuation limits (default 18 dB). Voice GR limits Voice microphones, including during fades and chorus. Other GR limits Other and Drums. Reducing a cap takes effect immediately. **GAIN AMOUNT** interpolates reduction in dB for every microphone: 0% disables broadband reduction, including quiet ones; 100% applies the full computed reduction. **FADE** sets the full-scale Automix and chorus transition time, with proportionally shorter partial moves.

Activity uses unweighted input RMS, an adjustable **THRESHOLD** (default −40 dBFS), 20 ms onset confirmation, 3 dB hysteresis, and **HOLD** below the closing threshold. MIDI priority does not change the active count. Count reflects microphones, not distinct human speakers: bleed can make multiple microphones active.

Chorus counts **Voice** microphones only. When that count is **strictly greater than N**, the active Voice microphones fade to unity while the quiet Voice microphones fade toward **VOICE MAX GR**. Other microphones keep ordinary gain sharing inside their own group and are never opened by chorus. Quiet Voice microphones are never opened just because chorus mode engages. When the Voice count returns to N or fewer, the chorus hold expires and Voice gain sharing fades back in. Set **CHORUS ABOVE** to 0 to disable this feature. Activity hold applies before chorus return hold; these delays intentionally accumulate.

**VOICE ASSIST** applies only to the Voice group. It uses the same off-thread Silero detector as Damian Channel Strip's PSE. Assistance × speech confidence lowers a Voice microphone's activity threshold by up to **6 dB**. It helps quiet speech cross the threshold; it is not a hard speech gate or a singing classifier. Zero disables inference. Other microphones always use the plain RMS threshold. If the detector is starting or unavailable, ordinary RMS activity detection remains in use. Speech confidence is shown under the meters on a Voice strip while Voice Assist is on. `SPEECH WAIT` means no valid result yet. The type button shows Voice, Other, or Drums. No audio passes through the neural model.

**DRUMS** uses a 1 ms attack / 3 ms release power detector and 1 ms control updates. Its extra influence on Other tracks uses a separate 2 ms attack / 15 ms release gain envelope, preserving the ordinary envelope for sustained non-drum sharing. Drum tracks themselves open in 2 ms and close in 15 ms. Drum activity has no speech-onset debounce and a 20 ms hold; drums never enter the voice chorus count or speech detector. The existing Other GR limit and Amount control still apply.

## Spectral processing alongside gain sharing

**SPECTRAL ON / OFF** enables the spectral stage for all linked instances. Gain sharing, chorus, voice assist and their controls stay available in both states. Spectral processing defaults off. Switch while stopped: enabling it adds **2048 samples** of latency (42.7 ms at 48 kHz); disabling it restores zero latency. The switch saves with the project and is not automatable.

Each taller strip has three independent controls:

- **GAIN WEIGHT**: −12 to +12 dB priority within its gain-sharing group. MIDI cues affect this weight only.
- **SPECTRAL WEIGHT**: −12 to +12 dB spectral priority. Only strictly lower Spectral Weights yield where spectra overlap; equal weights create no new cuts. Existing cuts recover over Release. This priority is independent of Gain Weight and MIDI.
- **EQ BLEND**: that track’s tonal adjustment, −100% to +100%. Positive values move its tone toward the linked tracks’ shared balance; negative values emphasize differences. Zero is neutral. It compares level-normalized input spectra, ignores both weights, and works with either Amount or Max Cut at zero. EQ is limited to ±6 dB and can boost output level. It returns to neutral without eligible peers.

Spectral Weight and EQ Blend are editable from any instance and are saved and automatable per microphone. They take effect only with Spectral On. No legacy preset conversion is provided during beta.

The top row controls gain sharing. The second row’s **DUCK AMOUNT** scales spectral cuts and **MAX CUT** caps spectral reduction (default 9 dB). The upper cog opens gain-sharing chorus/detection settings; the second-row cog opens **LOW FREQ / HIGH FREQ**, **ATTACK / RELEASE**, and **SMOOTHING**. The selected frequency range and smoothing affect both spectral ducking and EQ. Low above High leaves no affected frequencies and shows **LOW > HIGH · CHECK RANGE**, while gain sharing continues.

Gain sharing and the spectral detector both analyze original input. Spectral processing uses a 2048-point FFT with 75% overlap and 32 logarithmic bands, including both stereo channels so antiphase stereo does not cancel detection. The gain-sharing envelope is delayed to match its audio sample at the FFT output. Neither stage’s reduction feeds back into linked input detection. No external sidechain routing is needed.

Voice, Other, and Drums participate together in spectral processing while retaining their separate gain-sharing pools. Chorus opens the broadband gain-sharing stage only; it does not cancel spectral cuts or EQ. Voice/Other Max GR caps broadband reduction only; total reduction can exceed those caps when spectral cuts are added. Spectral noise-floor detection is independent of the activity threshold. The strongest eligible influence in each band wins rather than summing cuts with track count. Silent, bypassed, conflicted, deleted and stale sources cannot initiate cuts.

Drum-related spectral cuts above approximately **3 kHz** use at most **5 ms attack / 40 ms release**. Low bands retain normal timing. Drums also use 5/40 ms high-band EQ response; other EQ changes follow Release with a 100 ms minimum. These timings are in addition to FFT analysis and linked-track observation delays.

- Each strip’s gold graph shows **spectral reduction only**, from 40 Hz to 20 kHz horizontally and 0 to 24 dB downward. There is no input-spectrum trace. Missing or stale data shows **NO CUT DATA**.
- **GR** shows gain-sharing reduction plus the largest spectral cut, excluding EQ, on a fixed 0–24 dB visual scale with an uncapped numeric readout. It is not a measurement of overall loudness loss. **OUT** measures the final signal after both stages and EQ.
- **IN MIX** counts fresh, non-bypassed tracks without ID conflicts, including silent tracks. **VOICE ACTIVE** is the activity count used for chorus. **BYPASSED** identifies strip bypass; **NO METER DATA** identifies missing or stale updates.

Gain Amount zero, Duck Amount zero, Max Cut zero, and bypass all retain the FFT delay while Spectral On is enabled. Bypass withdraws the track’s influence, fades broadband gain to unity, and releases spectral cuts and EQ; it does not switch to undelayed audio. Frequency selection is limited by FFT resolution (23.4 Hz per bin at 48 kHz).

Linked observations are asynchronous. Stale spectral publications expire after approximately 0.5 seconds plus two maximum buffers of receiver processing. Use the same format and host process, and realtime rendering for linked-track work.

## MIDI cues

Route Cue Manager's MIDI output to **any one** of the linked instances. That instance can address all microphone weights, including microphones on other tracks. MIDI is also passed through unchanged.

- Defaults: MIDI channel 1, Mic 1 = CC 20, Mic 2 = CC 21, through Mic 64 = CC 83.
- Each mixer strip has one **CH / CC** dropdown. Open it to choose both the MIDI channel and weight CC in two columns; scroll for more CCs and click outside to close. Mappings are editable from any editor. Channel Off disables that mapping. CC numbers 0–119 are supported; channel-mode messages 120–127 are ignored.
- CC values map to absolute priority: 0 → −12 dB; 64 → exactly 0 dB; 127 → +12 dB, with linear interpolation on each side of 64.
- Example: MIDI bytes `B0 15 7F` set Mic 2 (CC 21) to +12 dB. `B0 15 40` restores its neutral priority.
- The latest cue overrides that strip's base host parameter. Editing its Gain Weight control or changing its host weight parameter replaces the MIDI override. The strip shows the effective weight. Generic host parameter views display the base parameter.
- Names, mappings, current MIDI weights, microphone identities and base weights are included in saved state. MIDI does not create host automation points; replay cues for timeline recall. Save the project after setup or cue changes.

Cue Manager sending is a separate integration; this plugin implements the receiving protocol.

## Linking and host processing

Up to 64 instances link within one loaded plugin module in one host process. VST3 and CLAP are separate networks. Sandboxed/bridged processes and separate DAW applications do not share a network. There is one network per format/process, so simultaneously active projects in the same process also share it.

This design targets live microphone mixing. In REAPER, use live/record-monitored tracks and disable anticipative FX processing for automixed tracks. Shared meter snapshots are asynchronous: control updates occur every 5 ms (1 ms for Drums) and cross-track observation may trail by a host block. There is no blocking barrier between audio threads. Faster-than-realtime renders, independently anticipative tracks, and neural worker scheduling are not guaranteed to reproduce live results sample-for-sample; use a realtime render for this workflow.

Use **BYPASS** on any strip to fade that microphone to unity and remove it from its group's gain sharing and, for a Voice microphone, from the chorus count. The strip setting saves with the track's plugin instance and can be set before audio starts. **HOST BYPASS** means the DAW's bypass parameter is on. The track stays bypassed until both controls are off. Deactivated/deleted instances withdraw immediately; a host that stops calling an instance without deactivating it is detected after roughly 0.5 seconds plus two maximum buffers. An idle instance remains visible for identification, with stale meters cleared. A full registry fails open and displays a capacity warning.

## Build and install

From the workspace root:

```sh
cargo check -p automixer
cargo test -p automixer --quiet
./automixer/scripts/install.sh
```

The installer rebuilds, bundles, signs and verifies `Automixer.vst3` and `Automixer.clap` under the user's Audio/Plug-Ins folders. The Silero model is embedded, and ONNX Runtime is linked into the release binary. No runtime model download is required.

Verification covers DSP, MIDI routing, plugin processing, lifecycle, shared parameter ownership, state fields, and an offscreen render of the real editor. It does not replace an in-host listening check.
