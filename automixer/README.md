# Automixer

Mono/stereo gain-sharing microphone automixer, built as VST3 and CLAP. Every instance contains the same 64-microphone mixer and shared controls. The gain-sharing reference is [leafac's REAPER automixer](https://github.com/leafac/reaper/blob/main/Effects/leafac_Automixer.jsfx); this is a separate Rust implementation for linked track instances.

## Setup

1. Insert one Automixer on each microphone track, before track faders. Use the same plugin format on every track.
2. Leave **THIS MIC** at **Auto** to assign the hardware input number when available and free. Stereo and multichannel inputs use their first channel. If the input number is unavailable, in use, or above 64, the instance takes a free number. Enter a number to pin a cue target manually. Conflicting manual numbers pass audio at unity and are excluded from sharing until corrected.
3. Open any instance to see all connected microphones, eight strips per page. **IN**, **OUT** and **GR** show input RMS, output RMS, and applied reduction. Select a strip to inspect its MIDI mapping. Double-click a strip name to set an alias, or clear the alias to follow the host.
4. Drag a strip's **WT** to change that microphone's priority from −12 to +12 dB. This changes its share of the available gain; it does not boost its output above unity. Click values to type, Shift-drag for finer adjustments, or double-click controls to reset them.
5. **ALL AMOUNT**, **MAX GR**, detection and chorus controls apply to every instance. Automate shared controls and weights from one instance to avoid conflicting automation lanes.

Track names come from CLAP track info or VST3 channel context when the host supplies them. In REAPER, both formats can also use the REAPER extension API to read the track name and record input. The selected strip shows the input name when REAPER supplies one. Track sends, MIDI, and virtual loopback inputs have no hardware number for this purpose.

Automatic assignments are resolved at audio activation. Saved assignments are reused when still available and the hardware input is unchanged or unavailable. A different hardware input is considered on the next activation. Track renames update labels without renumbering. REAPER input labels refresh on activation, editor opening, and host track notifications. Use a manually pinned number if you need a cue target to stay fixed across hardware repatching.

## Attenuation and chorus behavior

Sharing uses each input's smoothed RMS multiplied by its priority, divided by the sum of those weighted levels. A single microphone receives unity gain; two equal microphones each approach −6.02 dB. Stereo input channels are linked and count as one microphone. The plugin has zero audio latency; detector and gain envelopes have response times.

**MAX GR** is a hard attenuation limit (default 18 dB), including during fades and chorus operation. Reducing this cap takes effect immediately. **ALL AMOUNT** interpolates reduction in dB: 0% is unity for every microphone, including quiet ones; 100% applies the full computed reduction. **FADE** sets the full-scale Amount and chorus transition time, with proportionally shorter partial moves.

Activity uses unweighted input RMS, an adjustable **THRESHOLD** (default −40 dBFS), 20 ms onset confirmation, 3 dB hysteresis, and **HOLD** below the closing threshold. MIDI priority does not change the active count. Count reflects microphones, not distinct human speakers: bleed can make multiple microphones active.

When the active count is **strictly greater than N**, the active microphones fade to unity while the quiet microphones fade toward **MAX GR** attenuation. Quiet microphones are never opened just because chorus mode engages. When the count returns to N or fewer, the chorus hold expires and gain sharing fades back in. Set **CHORUS > N** to 0 to disable this feature. Activity hold applies before chorus return hold; these delays intentionally accumulate.

**VOICE ASSIST** optionally uses the same off-thread Silero detector as Damian Channel Strip's PSE. Assistance × speech confidence lowers the activity threshold by up to **6 dB**. It helps quiet speech cross the threshold; it is not a hard speech gate or a singing classifier. Zero disables inference. If the detector is starting or unavailable, ordinary RMS activity detection remains in use. The voice meter shows confidence; `VOICE WAIT` means no valid result yet. No audio passes through the neural model.

## MIDI cues

Route Cue Manager's MIDI output to **any one** of the linked instances. That instance can address all microphone weights, including microphones on other tracks. MIDI is also passed through unchanged.

- Defaults: MIDI channel 1, Mic 1 = CC 20, Mic 2 = CC 21, through Mic 64 = CC 83.
- A strip's **MIDI CHANNEL** and **WEIGHT CC** are editable from any editor. Channel 0 disables that mapping. CC numbers 0–119 are supported; channel-mode messages 120–127 are ignored.
- CC values map to absolute priority: 0 → −12 dB; 64 → exactly 0 dB; 127 → +12 dB, with linear interpolation on each side of 64.
- Example: MIDI bytes `B0 15 7F` set Mic 2 (CC 21) to +12 dB. `B0 15 40` restores its neutral priority.
- The latest cue overrides that strip's base host parameter. Editing its WT control or changing its host weight parameter replaces the MIDI override. The mixer always displays the effective weight and its control source. Generic host parameter views display the base parameter.
- Names, mappings, current MIDI weights, microphone identities and base weights are included in saved state. MIDI does not create host automation points; replay cues for timeline recall. Save the project after setup or cue changes.

Cue Manager sending is a separate integration; this plugin implements the receiving protocol.

## Linking and host processing

Up to 64 instances link within one loaded plugin module in one host process. VST3 and CLAP are separate networks. Sandboxed/bridged processes and separate DAW applications do not share a network. There is one network per format/process, so simultaneously active projects in the same process also share it.

This design targets live microphone mixing. In REAPER, use live/record-monitored tracks and disable anticipative FX processing for automixed tracks. Shared meter snapshots are asynchronous: control updates occur every 5 ms and cross-track observation may trail by a host block. There is no blocking barrier between audio threads. Faster-than-realtime renders, independently anticipative tracks, and neural worker scheduling are not guaranteed to reproduce live results sample-for-sample; use a realtime render for this workflow.

Use the plugin's own **BYPASS** to fade one microphone to unity and withdraw it from the network. Deactivated/deleted instances withdraw immediately; a host that stops calling an instance without deactivating it is detected after roughly 0.5 seconds plus two maximum buffers. An idle instance remains visible for identification, with stale meters cleared. A full registry fails open and displays a capacity warning.

## Build and install

From the workspace root:

```sh
cargo check -p automixer
cargo test -p automixer --quiet
./automixer/scripts/install.sh
```

The installer rebuilds, bundles, signs and verifies `Automixer.vst3` and `Automixer.clap` under the user's Audio/Plug-Ins folders. The Silero model is embedded, and ONNX Runtime is linked into the release binary. No runtime model download is required.

Verification covers DSP, MIDI routing, plugin processing, lifecycle, shared parameter ownership, state fields, and an offscreen render of the real editor. It does not replace an in-host listening check.
