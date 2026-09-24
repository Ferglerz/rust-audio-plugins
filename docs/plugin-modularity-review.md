# Plugin modularity review — 21 September baseline and rollout

Reviewed 21 September 2026. Scope: Damian Channel Strip, Flattery, Tape Stop, SoundChef Drums (SCD), pleasant-ui, and SCD's core/packer crates. Source inspection and targeted SCD verification; no audio benchmarking or live DAW validation was performed. Counts below describe the initial checkout, before the SCD UI split, and include comments, blank lines, and inline tests. Vendored code, assets, build output, and external test directories are excluded from the counts.

This document records the original extraction plan and its rollout. The file counts and dependency findings below describe the pre-rollout checkout; they are not a current inventory. Composure was outside this review's scope. The current repository review is recorded separately in `repository-review-2026-09-23.md`.

## Recommendation

Build a reusable library layer beneath the plugin wrappers, with host-independent DSP and curve math, reusable interactive graphics above it, and explicit host adapters at the outside. Start as independently consumable Rust crates in this repository. A new effect should be able to use an EQ processor without pulling in Vizia, and use its editor without inheriting Damian's parameter IDs, routing, worker ownership, or presets.

Do not create a repository per effect yet. Crate boundaries provide reuse and enforce dependency direction; repository boundaries mainly add release and coordination costs. Once these APIs have a second real consumer, a `pleasant-audio` repository containing several versioned crates would be a reasonable distribution home. EQ and dynamics can become separate crates inside it without needing separate repositories.

The highest-value sequence is: unify the host implementation; split the largest UI responsibilities; remove DSP's dependency on pleasant-ui; extract filters and curve math; separate processor configuration from plugin parameters; then extract larger effect units.

## Delivery status

The in-repository rollout is now implemented. `pleasant-dsp`, `pleasant-curves`, `pleasant-eq`, and `pleasant-dynamics` provide host-independent foundations. pleasant-ui retains compatibility re-exports and adds shared graph coordinates. Damian, Flattery, Tape Stop, SCD, and pleasant-ui now split their largest responsibilities into private modules.

Damian consumes shared EQ path and dynamics processors while retaining routing, IDs, presets, Lift, and host latency ownership. Flattery now passes plain process settings into its engine, publishes immutable node snapshots, prebuilds supported FFT analyzers, and preallocates maximum analysis vectors. Tape Stop uses shared S-curve and interpolation primitives and no longer allocates release-matching buffers in sample processing.

Headless consumers and lifecycle documentation live under `examples/` and [pleasant-libraries.md](/Users/ferg/GitHub/audio-plugins/docs/pleasant-libraries.md). `scripts/verify-modularity.sh` checks both workspaces without selecting vendored crates. External repository publication remains deferred.

## Initial shape before the rollout

- **Damian:** 11,930 lines across 10 Rust source files. The reusable DSP exists, but is embedded in a plugin-specific engine, parameter schema, and large editor. Its UI entry file is 6,776 lines. `engine.rs` is 1,439 lines and `dsp.rs` is 1,296; about half of each latter file is its inline test module, so size alone overstates the implementation problem.
- **Flattery:** 5,182 lines across 15 files. DSP is already divided into analyzer, filter bank, biquad, leveling, delay, telemetry, and tilt. Its UI controller is 2,051 lines and `graph.rs` another 1,283. The most important boundary problem is `Engine::tick` accepting the complete host parameter object, plus DSP importing pleasant-ui.
- **Tape Stop:** 2,634 lines across 8 files. A relatively good engine/wrapper separation already exists. Its 1,413-line editor mixes curve display, controls, MIDI learning, events, and rendering. The 639-line engine combines trajectory mathematics with playback state and correlation-based return matching.
- **SCD plugin:** 6,688 lines across 13 files. Core/packer separation is already useful, and voice, voice-pool, hi-hat, and sub-kick processing are distinct. The 3,688-line editor is the concentration point. Velocity mapping mixes generic curve math, drum articulation routing, persistence, and lookup publication.
- **pleasant-ui:** 1,718 lines across 9 files. It successfully shares visual primitives, fonts, themes, value editing, and pointer helpers. `draw.rs` holds 1,095 lines. It also owns DSP-consumed math and spectrum helpers that belong below the UI layer.
- **scd-core / scd-packer:** 771 / 439 lines, four source files each. Preserve this separation. Packaging/import and sample playback have different dependencies and execution constraints.

The four main UI entry files alone contain 13,928 lines. This is a useful prioritization signal, not an estimate of removable duplication. The editors have substantially different behavior.

## Initial dependency findings before the rollout

**DSP currently depends on UI.** [Flattery's engine](/Users/ferg/GitHub/audio-plugins/Flattery/src/dsp/engine.rs:14) imports conversion and spectrum operations from pleasant-ui. Its [tilt module](/Users/ferg/GitHub/audio-plugins/Flattery/src/dsp/tilt.rs:1) imports the blended frequency mapping, and [Damian's engine](</Users/ferg/GitHub/audio-plugins/Damian Channel Strip/src/engine.rs:10>) imports spectrum fall behavior. pleasant-ui itself depends on NIH-plug and Vizia. Extract these numerical functions into a host-independent crate and keep temporary re-exports in pleasant-ui so UI callers can migrate separately.

**Host behavior had two owners.** The root workspace uses patched NIH-plug, baseview, and vizia_baseview under Damian's vendor directory. SCD used its own NIH-plug/Vizia copy and upstream backend sources. SCD now consumes the same runtime stack. Its existing vendored xtask remains a build-time dependency. The old runtime sources remain on disk but are no longer selected by scd-plugin. Moving vendor ownership to a neutral directory is a later mechanical migration, not a reason to copy more patches.

**Workspace policy is duplicated.** SCD remains an excluded, nested workspace with its own lockfile and patches. A future consolidation can use uniquely named xtask packages and shared workspace dependency versions. Until then, both lockfiles and both patch tables must be maintained; a root `cargo check` does not cover SCD.

**Host schemas and runtime models are interleaved.** [Damian's processing modes](</Users/ferg/GitHub/audio-plugins/Damian Channel Strip/src/processing.rs:10>) derive host enums alongside DSP configuration. [SCD velocity mapping](/Users/ferg/GitHub/audio-plugins/scd-rust/crates/scd-plugin/src/vel_map.rs:1) combines serde, NIH persistence, drum IDs, geometry, and realtime lookup. Extraction should separate these responsibilities rather than exporting the complete current modules.

## Candidate libraries and their boundaries

### 1. pleasant-dsp: first shared numerical library

Initially use modules for `units`, `filters`, `envelope`, `spectrum`, `delay`, and `interpolation`. No Vizia, plugin parameter types, UI preferences, filesystems, or DAW callbacks. Default builds should not require an FFT dependency unless FFT-backed modules are enabled.

Immediate consumers are Damian and Flattery. Their duplicated dB conversion and bell influence operations are small, concrete starting points. Preserve floors, clamping, precision, and operation order: Damian's `gain_db`, pleasant-ui's `linear_to_db`, and SCD's f32 conversions are not automatically behavior-identical merely because they use the same formula.

Share filter coefficient design and frequency-response evaluation first. [Damian's `Coeff`, `BandCoeffs`, `Filter`, and `Cascade`](</Users/ferg/GitHub/audio-plugins/Damian Channel Strip/src/dsp.rs:12>) form the richer EQ foundation. [Flattery's `BiquadState` and `PeakingFilter`](/Users/ferg/GitHub/audio-plugins/Flattery/src/dsp/biquad.rs:1) implement a direct-form history and parallel difference output. Unify coefficient representation and compatible design functions; preserve the distinct state topology, unity-gain shortcut, summing behavior, and smoothing. A wholesale replacement of one filter engine with the other would change sound.

An initial API can expose `BiquadCoefficients`, `design_peak(...)`, `response(...)`, and separate state implementations. Make sample rate and frequency validity explicit. Coefficient creation belongs in preparation/control updates; sample processing should operate on prepared state.

Keep delay types specific: Flattery needs an integer analysis delay and chronological window reads; Damian needs compensation delays; Tape Stop needs a large stereo fractional read buffer. Share small indexing/interpolation primitives where useful, without making all three allocate the same buffer or use the same precision.

### 2. pleasant-curves: mathematical curves, independent of drawing

[SCD's `VelCurve`](/Users/ferg/GitHub/audio-plugins/scd-rust/crates/scd-plugin/src/vel_map.rs:133) provides the clearest general-purpose starting point: bounded nodes, handle constraints, cubic evaluation, inversion, sanitation, and rasterization. Move normalized curve geometry and solving into this library. Leave MIDI 1–127 conversion, KitPieceId/articulation grouping, persisted bank shape, and atomic lookup tables in SCD's adapter.

[Tape Stop's `s_curve` and inverse](</Users/ferg/GitHub/audio-plugins/Tape Stop/src/dsp/engine.rs:5>) can become a separate analytic curve type. It describes a time trajectory, not a freely editable Bézier envelope. Preserve that distinction.

[Flattery's strength nodes](/Users/ferg/GitHub/audio-plugins/Flattery/src/strength.rs:44) describe frequency weights and neighborhood radii. Damian's bands describe actual filter responses. Share frequency axes and influence primitives; keep each node's domain model separate. Do not force all four into one enum carrying every plugin's possible fields.

Useful contracts: normalized point/handle geometry, monotonic-x validation, evaluation with a documented inversion tolerance, and bounded rasterization into caller-owned storage. Unit tests should cover endpoints, coincident handles, monotonicity, finite output, and old serialized fixtures.

### 3. pleasant-ui: reusable interactive graphics

Keep drawing and interaction here. Add a `graph` family with viewport transforms, axes, curve paths, node/handle hit testing, selection/drag state, and optional overlays. Let callers supply data and consume semantic edit actions such as begin, move, insert, delete, and end.

Coordinate transforms must be shared by drawing and hit testing. Separate the domain axis (log frequency, Flattery's blended scale, normalized velocity, or time) from the window-to-artwork transform. Flattery's zoom-to-work-area buttons are graph viewport controls and should remain; they are unrelated to SCD's removed window percentage menu.

The reusable graph layer must not know about `StripParams`, `ScdParams`, NIH parameter pointers, articulation IDs, or shared DSP locks. A plugin adapter turns edit actions into automation gestures or persisted node edits. EQ response evaluation comes from DSP; pleasant-ui should not invent an approximate response when the processor can provide the real one.

Split `draw.rs` into primitives/text, controls, icons, handles, and value editing while retaining the `Draw` public facade. First consolidate genuinely repeated input handling: Damian still defines its own typed-character and value-entry handling while the other editors use shared helpers. Preserve units, reset behavior, command modifiers, and gesture completion during migration.

### 4. EQ as a reusable effect

After the numerical layer lands, extract `pleasant-eq` when a second plugin is ready to use it. Its public boundary should own EQ configuration, prepared coefficients/banks, filter state, response queries, reset, and latency. Damian's pre/post/sidechain routing, audition controls, band numbering ranges, preset schema, and automation IDs stay in Damian.

Start with static zero-latency EQ. Bring over dynamic-band detection and then oversampled/linear-phase paths as separately validated capabilities. [Damian's processing module](</Users/ferg/GitHub/audio-plugins/Damian Channel Strip/src/processing.rs:144>) already offers an `EqPath` seam, but its dependency on the complete `Band` model needs narrowing. Preparation and runtime processing should be different types so expensive filter-bank design cannot accidentally happen on the audio thread.

An EQ graph component in pleasant-ui can pair with this library, but neither should require the other. A headless processor, response-only test, and editor preview should all be possible consumers.

### 5. Dynamics, gating, and saturation

[Damian's dynamics code](</Users/ferg/GitHub/audio-plugins/Damian Channel Strip/src/dsp.rs:292>) contains time laws, PSE behavior, `VocalComp`, compressor settings, and the wall saturator. Split these internally into detector/envelope, compressor, PSE, and saturation modules first. Extract them as `pleasant-dynamics` once the configuration and detector-input contracts are clean.

Do not equate Flattery's [spectral leveling](/Users/ferg/GitHub/audio-plugins/Flattery/src/dsp/leveling.rs:1) with a broadband compressor. Share envelope/time-coefficient and gain-curve primitives where their definitions match; keep the spectral neighborhood algorithm separate. Expose measured reduction as numeric telemetry, not Vizia types.

[Speech VAD](</Users/ferg/GitHub/audio-plugins/Damian Channel Strip/src/vad.rs:147>) is another possible reusable detector, but should remain a local module until its preparation, frame-size, latency, and sample-rate assumptions are documented. It is not necessary to put every detector in one universal dynamics engine.

### 6. Spectrum and realtime transport

There are three different needs: Damian's display analyzer, Flattery's analysis driving processing, and Damian Lift's reconstruction path. Share window generation, FFT preparation, magnitude normalization, smoothing, and display decimation where equivalent. Keep hop sizes, fall units, scaling conventions, reconstruction windows, and latency explicit. A display analyzer and a processing STFT do not have the same correctness contract.

The existing [pleasant-ui spectrum helpers](/Users/ferg/GitHub/audio-plugins/pleasant-ui/src/spectrum.rs:1) are an easy first extraction. Their decay is expressed per reference sample count, not wall-clock time; do not silently change that when moving them.

Each plugin should retain its own typed telemetry payload. Share tested publication primitives for scalar meters or bounded snapshots, rather than one `Shared` object with every plugin's fields. SCD already separates editable curves from atomic velocity lookup; Tape Stop has a compact atomic telemetry adapter.

### 7. Sampler and playback reuse

Keep scd-core and the packer separate. Voice playback, choke envelopes, pan laws, and oscillator envelopes may eventually be reusable, but the current SCD voice depends on kit IDs, six mic channels, and ScdPack. It is a lower-priority extraction than curves and filters because there is only one sampler consumer.

A prerequisite for a general sampler library is making sample ownership explicit. [ScdPack::get_sample_slice](/Users/ferg/GitHub/audio-plugins/scd-rust/crates/scd-core/src/lib.rs:300) returns a `'static` slice into an owned mapping. Before exporting that API for arbitrary consumers, replace the implicit lifetime contract with borrowing or an owning sample handle that keeps the mapping alive. This is a boundary concern identified by inspection, not a reproduced crash.

SCD uses linear stereo interpolation, while Tape Stop uses four-point Hermite interpolation. They are alternatives with different output, not duplicate algorithms to merge into one implementation.

## Realtime constraints that should shape the extraction

A library is not safely reusable just because it compiles without the original plugin. Its API must distinguish preparation, updates, and processing.

- Flattery calls `snapshot_nodes` from its analysis path. That helper takes a blocking mutex and clones a vector. `Engine::tick` can also trigger FFT reconfiguration and buffer growth. Moving this into a shared crate unchanged would export those realtime behaviors. Prefer bounded immutable snapshots, configuration generations, and prepared FFT plans/buffers swapped at block boundaries.
- Damian's `Engine::sync` uses `try_lock`, but clones Lift bands and can allocate a new sidechain runtime vector. Its worker-prepared main EQ banks are a useful model, but do not establish that every update path is allocation-free. Reclamation of replaced banks also needs an explicit owner outside the callback.
- A failed nonblocking meter publication should drop that display update, not delay audio. Configuration updates need a different policy: retain the last valid prepared state and observe a generation/version so updates are not silently lost.
- Keep UI editing data, prepared processing state, and host serialization data distinct. The processor should receive plain settings or prepared objects, not a plugin's entire parameter tree.

These are structural findings, not measured glitch rates. Validate any resulting processing change with a targeted allocation/locking audit and numerical/audio equivalence tests.

## Large-file split plan

**Damian UI (highest priority):** split into `editor.rs` for construction/state, `layout.rs` for geometry, `input.rs` for gestures/value edits, `eq/{graph,hud,interaction}.rs`, and `dynamics/{compressor,pse,wall}.rs`. Move tests alongside the responsibility they verify. Keep the parameter adapter local. Merely moving a 1,650-line draw method into `render.rs` is an intermediate step; the stronger boundary is each effect module owning its own presentation and interaction.

**Damian DSP/engine:** split coefficient design/state, dynamic bands, compressor/PSE, and saturation. Split engine worker/bank preparation, block routing, and telemetry/analyzer. Preserve existing transition, latency, bypass, and solo tests. The inline tests are valuable and should remain in the repository.

**Flattery:** split UI event handling, value editing/parameter adapter, node interactions, footer/controls, and composition. Split graph layout/axis transforms from spectrum rendering and strength-node drawing. Keep graph zoom state in the editor. DSP already has useful file boundaries; decoupling settings and realtime updates matters more than further subdividing its 321-line engine.

**Tape Stop:** split graph layout/interaction, parameter controls, MIDI-learn interaction, and editor composition. Split analytic curve/return trajectory from playback state and return matching in the engine. Keep the actual tape state machine together; excessive per-function files would make it harder to understand.

**SCD (first split implemented):** `ui/mod.rs` now holds 1,395 lines; `events.rs` 753; `render.rs` 681; `drawing.rs` 637; `tests.rs` 111; existing `fader_law.rs` 77. These are private module boundaries and share the parent view state. They improve navigation but are not yet independent reusable widgets. Next split mapping editor, preset menu, mixer, and sub-kick dialog by ownership. Separate generic velocity curve geometry from drum bank persistence/publication.

**pleasant-ui:** split the Draw implementation into meaningful visual responsibilities without changing its external calls. Add viewport and graph interaction only after adapting two different editors to prove the boundary.

Aim for cohesive modules usually a few hundred lines long. Do not use an arbitrary maximum line count as the acceptance condition.

## SCD resizing delivered in this change

SCD now selects the same patched NIH-plug, Vizia, baseview, and vizia_baseview runtime paths as the other three plugins. The percentage selector, preset levels, nearest-level snapping, and forced scale reset were removed. Presets and Mapping move left to occupy the vacated header space.

Drawing and pointer coordinates now both derive scale from actual view width divided by artwork width, with the view origin included. The editor no longer resizes the shared canvas from a separately maintained percentage. It skips empty geometry, and cached modal backdrops are invalidated after size changes. New instances use the same `new_screen_sized` startup policy as the other plugins.

For compatibility, `editor-zoom` remains a read-only legacy base-size field. Existing saved percentages still determine the initial artwork size basis; continuous Vizia state is applied on top. Reopening must not copy that scale back into the percentage, which would compound it. Existing `editor-state` and all audio parameter IDs are preserved. The old range clamp applies only to that legacy field, not to a new zoom menu.

This adopts the current shared behavior, including width-based uniform scaling rather than responsive relayout. Host-driven aspect-ratio behavior, CLAP versus VST3 differences, and native host-window size persistence remain host/framework concerns. The shared VST3 fork accepts native resize requests; the shared Vizia state persists user-scale changes. Do not infer from that alone that every DAW saves native window dimensions the same way.

Verification: baseline `cargo check -p scd-plugin --offline` passed. After the change, `cargo check -p scd-plugin --all-targets --offline`, `cargo test -p scd-plugin --lib --quiet`, and `cargo clippy -p scd-plugin --offline` passed in SCD's workspace. All 39 library tests pass, including legacy-size and continuous-scale persistence checks. Clippy reports an existing derivable-default warning in `note_map.rs`; dependencies also emit existing warnings. No vendored crate was selected as a check/test target.

Remaining manual acceptance: open SCD and another plugin in the same DAW; drag larger and smaller to non-preset sizes; verify mixer, Mapping handles, text editing and modal alignment; change display DPI; save/reopen the DAW project; check a pre-change state with non-100% zoom. This change has not been installed or validated in a DAW.

## Implementation sequence and acceptance gates

1. **Host convergence and SCD scaling — implemented.** Shared dependency selection, bounds-based drawing/input, compatibility fields, and private UI split. Live DAW acceptance remains unverified.
2. **Internal module splits — implemented.** Damian, Flattery, Tape Stop, SCD, and pleasant-ui now separate major UI and processing responsibilities behind existing facades.
3. **Pure numerical foundation — implemented.** `pleasant-dsp` owns conversions, axes, spectrum helpers, compatible filter design/state, envelopes, and interpolation. pleasant-ui keeps temporary compatibility re-exports.
4. **Curve foundation and graphics adapters — implemented.** `pleasant-curves` owns SCD Bézier geometry and Tape Stop analytic curves. SCD and Flattery consume shared graph transforms while retaining domain adapters.
5. **Effect libraries — implemented.** `pleasant-eq` exposes prepared static, dynamic, oversampled, and linear-phase processing. `pleasant-dynamics` exposes compressor, PSE, and saturation processing. Damian retains plugin-specific routing and state.
6. **Distribution preparation — implemented.** Headless examples, lifecycle documentation, compatibility tests, and dual-workspace verification remain in this repository. External publication remains deferred.

Rollback of this change is limited to the SCD dependency/lockfile selection and UI/parameter edits. No vendor files, DSP algorithms, audio parameter IDs, external repositories, or installed plugin bundles were changed. The legacy field is intentionally retained; newer non-preset sizing is not guaranteed to survive running an older SCD binary that still snaps sizes.
