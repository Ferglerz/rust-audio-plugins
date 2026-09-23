# SCD: component-based sound shaping alongside Punch

Status: revised proposal. Supersedes the envelope-only revision and the earlier four-feature LoRA plan. No DSP implementation has been made.

## Objective

Keep existing Punch, pitch, velocity response, round robins, mixer, and sub-kick behavior. Add meaningful control over components of the recorded sound: impact, tonal body, ring, and noisy texture. Allow selective decay changes, such as tightening shell ring while retaining snare wires and room character.

This is a sound-shaping layer alongside existing controls. The implementation should be selected by audible usefulness, not by a requirement to use LoRA or any particular separation algorithm.

## Proposed player controls

Prototype these controls before choosing final names and ranges:

- **Impact:** more or less of the short attack component. Intended to change stick/beater presence without simply turning up the whole hit.
- **Tone:** more or less of the stable resonant component. On kick/toms this may feel like body; on cymbals it can affect metallic ringing. Do not label it Fundamental or imply harmonic-only content.
- **Texture:** more or less of the diffuse/residual component. On a snare this may affect wire buzz, but also room and separation leakage. Do not promise a pure Snare Wires stem.
- **Ring Decay:** shorten the extracted resonant sustain while retaining the original impact and the unmodified residual as far as separation permits.
- **Texture Decay:** optional second decay control only if it produces a useful independent change, such as drying buzz without killing shell resonance.

All controls default to neutral. Existing Punch remains an independent parameter with its current behavior and automation. Start with modest component gain changes, then establish usable ranges by listening. Body versus Ring should become two separate controls only if the prototype can separate them usefully; they are not automatically different sources.

## Primary experiment: offline decomposition, realtime component mixing

Prepare component WAVs for each recorded strike outside the audio callback. At playback, read aligned component samples using the same playhead and interpolate them with the same resampler as the original recording. Apply smoothed gains and per-voice decay envelopes. Retain the original waveform as the neutral reference.

For each mic channel, let x be the original, p the extracted impact, h the tonal component, and r the residual, constructed so x = p + h + r within numerical tolerance. An efficient neutral-preserving form is:

    y = x + (g_impact - 1) * p + (g_tone(t) - 1) * h + (g_texture(t) - 1) * r

These gains are linear. Derive r as x - p - h when appropriate so unassigned content is retained. At all gains = 1, bypass component reads and return the original path exactly. A good neutral sum does not establish good separation: independent gain changes can expose leakage and processing artifacts.

Use soft spectral masks with a defined sum of one, original complex phase, and correct inverse-STFT overlap/add normalization. Do not replace the extracted components with oscillators or synthetic noise. Pad and trim analysis boundaries explicitly so components retain exact sample alignment and frame counts. Check for pre-onset energy when components are remixed; perfect reconstruction at neutral can hide cancellation of such artifacts.

## Methods to compare

### A. HPSS: useful baseline, not a semantic drum separator

Harmonic-percussive separation identifies comparatively horizontal and vertical spectrogram structures. A stable inharmonic drum resonance can appear in the harmonic component; the percussive component is not necessarily only the first attack. Broadband snare rattle, cymbal wash, and room energy can be split in unintuitive ways.

Implement or prototype a conventional soft-mask HPSS baseline. Compare different analysis resolutions expressed in physical time/frequency units. A single FFT size and median-filter setting is unlikely to suit both kick fundamentals and sharp snare attacks.

Expected value: separating sustained resonance from sharper events on selected kicks and toms. Main risk: misleading controls on snares/cymbals and loss of impact under remixing.

### B. Harmonic-percussive-residual separation: preferred first candidate

Extend the baseline with a residual for ambiguous content instead of forcing everything into Tone or Impact. This provides a plausible third Texture control, but residual means uncertain/mixed content, not pure noise.

Compare it directly with HPSS using the same source strikes and gain sweeps. Select it only if its independent controls sound more useful. Research basis: Driedger, Müller, Disch, Extending Harmonic-Percussive Separation of Audio Signals (2014).

### C. Sines/transients/noise soft-mask decomposition: stronger alternative

Benchmark a three-way fuzzy-mask method aimed explicitly at stable components, transients, and noise. It matches the proposed control vocabulary more closely than two-way HPSS, while still requiring listening validation on drums and multi-mic recordings.

Use it to extract recorded waveform components; do not adopt oscillator/noise resynthesis. Fierro and Välimäki's enhanced fuzzy decomposition reports complementary soft masks and improved transient separation, with quality dependent on the material. Those research results do not establish SCD kit quality in advance.

### D. Resonance-focused spectral shaping: potentially better for Ring

If the desired change is primarily less ringing, detect persistent narrow spectral ridges after the onset and attenuate those regions with smooth time-frequency masks. This may provide a better Ring Decay control than broad harmonic gain, while leaving the rest of the sample intact.

Compare against restrained dynamic EQ as a simpler baseline. Avoid automatic deep notches in every detected peak: drum modes are part of the sound. Protect the onset, limit attenuation, and inspect cymbal tails for watery or metallic artifacts.

### E. LoRA: experimental rendered edits, not a live component knob

LoRA adapts a model; it does not itself isolate attack, shell, wires, or noise. Fine-tuning a general audio generator on an articulation does not establish independent, predictable control of those attributes.

A relevant experiment would use a suitable audio-editing model with audio conditioning and carefully labeled or paired examples of desired edits. LoRA could reduce adaptation cost if the chosen model supports it. Evaluate whether it preserves strike identity and timing while changing one requested aspect. A few slices/level variants of one hit are not evidence of such controllability.

Any neural edit would be rendered offline into a fixed, auditionable alternate sample. A strength slider would request a new render; it would not be advertised as an instantaneous independent DSP control. Multi-mic consistency remains unresolved for ordinary stereo generators. Initially restrict the experiment to a separate mono/stereo layer, not a replacement six-mic strike.

Pursue this only if decomposition cannot deliver a specifically desired transformation, such as changing the character of the shell rather than its balance. It is not a dependency of the first release. Stable Audio's documented LoRA and audio-to-audio support establishes API capability, not reliable drum-component editing.

### F. Learned masks: more directly relevant ML than generative LoRA

If classical masks systematically misclassify important drum components, a small model predicting complementary masks could preserve original audio while improving separation. It needs defensible target definitions, curated training examples, and held-out evaluation. Synthetic training mixtures may not represent the inseparable physical interactions in real drums.

Treat this as a later research option with a deterministic fallback. The first prototype should establish whether there is a separation problem worth training a model to solve.

## Multi-mic requirements

Each SCD strike can contain Close, XY, Mono, Wide, FrontMS, and Room streams. Preserve every recorded offset and stereo relationship. Do not analyze left and right independently with unrelated masks.

Benchmark stereo-linked masks and a common mask estimated from aggregated channel magnitudes across the strike. Preserve each channel's original complex phase when applying masks. Do not sum raw mic signals to create the analysis reference because phase cancellation can distort it. Also test a designated reference mic with a fallback for strikes without Close.

Shared masks are a conservative starting point, not a proof of transparency: mic arrivals differ, room tails differ, and time-varying spectral processing can change perceived cohesion. Audition close-only, room-heavy, all-mic, and mono combinations. If a method cannot preserve convincing multi-mic behavior, restrict its exposed scope to validated channels or defer it; do not hide the failure behind a solo demo.

Do not claim that generic HPSS separates room from drum source. Room, shell, wire, and transient are semantic/physical categories that need not match spectrogram geometry.

## Repo integration after the experiment passes

- `scd-packer`: offline analysis/preparation entry point and validation of aligned component WAVs. Existing generative.rs generates mappings, not neural audio.
- `scd-core`: add versioned optional component references alongside original mic slices, or a versioned sidecar index during prototyping. Old packs remain playable with original audio. Choose the final storage scheme after measuring disk and memory traffic.
- `scd-plugin/src/dsp/voice.rs`: component playback and per-voice gain/decay; retain original playhead, sample-rate/pitch behavior, Punch gain, pan, and choke ordering.
- `voice_pool.rs` / `lib.rs`: pass smoothed controls per kit piece, maintain independent states for overlapping strikes, and route shaped mic output through existing main/aux paths.
- `params.rs`, `presets.rs`, UI modules: new stable parameter IDs, neutral defaults, preset scope support, automation/reset gestures, and clear availability when a pack lacks component data.

Prepare controls once per kit piece per output sample, not by advancing the same smoother once per voice. A new strike must not reshape earlier ringing hits. Hi-hat choke applies to the complete shaped voice and takes precedence. The separately synthesized sub-kick retains its own controls.

Avoid live pack replacement in v1. Current voices retain pointers into mapped sample storage; load prepared data before playback and keep its backing storage alive. A future background preparation workflow needs explicit storage lifetime management and off-thread reclamation.

Original plus two stored components can derive a third residual, but still increases storage/read bandwidth substantially. Benchmark that against storing three components and retaining an exact dry reference through another mechanism. Preserve the direct original path until neutral behavior is proven. Skip extra reads for neutral controls where possible.

## Experiment and delivery sequence

1. Build a rights-cleared listening set covering kick, tom, snare, closed/open hat, and crash, with quiet/loud layers and multiple round robins. Include all available mic perspectives.
2. Render HPSS, HPR, and one sines/transients/noise candidate offline. Compare against simple timed envelopes and dynamic EQ. Sweep individual component gains and decay settings; audition components solo for diagnosis but judge full remixes in grooves.
3. Select the smallest method that enables useful independent controls. Record failures by articulation. Tune stable profiles across round robins rather than hand-picking one flattering sample.
4. Integrate prepared components into the pack/playback path. Add Impact, Tone, Texture, and Ring Decay only where the experiment supports those meanings.
5. Complete automation, presets, backwards compatibility, and CPU/storage validation. Stop when moderate settings are musically useful. Neural experiments remain separate until they outperform this baseline on an identified task.

## Acceptance gates

- Neutral output matches existing playback, including nonzero Punch and existing velocity/pitch settings.
- Decomposed components sum to the original within measured numerical tolerance; output alignment/frame count is unchanged.
- Moderate Impact changes alter attack relative to sustain; Tone/Texture changes provide distinct audible effects; Ring Decay shortens resonance without broadly gating the whole hit.
- No distracting pre-echo, watery tails, stereo wandering, room incoherence, or lost low-frequency impact at useful settings.
- Test ghost notes, rolls, flams, open-hat choking, overlapping cymbals, and all-mic/mono mixes. Include level-matched A/B so louder is not mistaken for better.
- Deterministic recall and offline bounces; no analysis, allocation, locking, file decoding, or model inference in the audio callback.
- Benchmark the existing 48-voice workload with all mic streams at small buffers and 44.1/48/96 kHz. Measure CPU, mapped-data traffic, and pack size against the original sampler before choosing supported scope.
- New parameters restore safely; old sessions and packs retain their sound. Boosts retain honest peak metering and output trim, without an undisclosed limiter.

For implementation, follow the repo verification ladder: crate-scoped cargo check, targeted tests for changed DSP/params/pack behavior, and crate-scoped clippy when shipping logic changes. No workspace or vendored tests by default.

## Evidence and realistic expectations

- HPSS assumptions and implementation explanation: https://audiolabs-erlangen.de/resources/MIR/FMP/C8/C8S1_HPS.html
- HPR extension and residual interpretation: https://www.audiolabs-erlangen.de/resources/MIR/FMP/C8/C8S1_HRPS.html
- Enhanced fuzzy sines/transients/noise decomposition: https://arxiv.org/abs/2210.14041
- Stable Audio LoRA: https://github.com/Stability-AI/stable-audio-3/blob/main/docs/workflows/lora.md
- Stable Audio editing controls: https://github.com/Stability-AI/stable-audio-3/blob/main/docs/workflows/inference.md

Most promising expected results are controllable impact versus ring on kick/toms and useful shell-versus-texture balance on some snares. Cymbals and room-rich recordings are harder. These are hypotheses to test, not claimed listening results. Gains and selective shortening are more defensible than arbitrary stem soloing, large pitch changes, or natural sustain extension. Extreme edits may be interesting sound design even when they are no longer realistic drums.
