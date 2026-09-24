# Repository review follow-up — 23 September 2026

This records the implementation prompted by the repository-wide review of
modularity, duplication, realtime work, dead code, clarity, and folder layout.
The earlier [plugin modularity review](plugin-modularity-review.md) contains
historical baseline counts; those counts do not describe this checkout.

## Safety and realtime processing

- SCD pack loading validates the archive and the complete mapped-audio layout
  before playback. Sample slices borrow from `ScdPack`; `VoicePool` owns the pack
  and voices keep checked sample offsets. The on-disk pack format is unchanged.
- Composure prepares graph snapshots and lookup tables for each supported range
  outside the callback. The audio callback loads the range captured from host
  parameters, even during a concurrent preset replacement; old graph
  generations are retained until audio readers have released them. Lookahead
  delay buffers are allocated when the chain is constructed.
- Flattery reuses prepared FFT scratch for both channels and handles empty
  high/low cutoff ranges correctly. It caches output-gain conversion while
  still observing gain changes on the next processed sample.
- Damian reuses dynamic-band meter publication buffers and precomputes
  spectrum bin ranges when the engine is built. The normal Silero VAD worker,
  sample feed, and envelope update path remain active; only the unused manual
  injection override was removed. Meter storage and both publication buffers
  reserve 256 bands, covering the existing 200-band engine case; a larger
  imported band set can still grow these vectors on the callback.
- `pleasant-eq` stops redesigning settled static coefficients every 32 samples.
  State inherited at a new sample rate still triggers coefficient preparation.

## Module ownership and maintenance

- Damian editor drawing and interaction are grouped under `ui/eq`,
  `ui/dynamics`, and `ui/lift`, with the top-level render and event files acting
  as dispatchers. Its single-path EQ tick helpers are compiled only for tests.
- Composure processing takes plain block settings supplied by a plugin adapter.
  Graph hit testing belongs to the UI, and the appearance example shares the
  production view construction. Related graph and control modules are grouped
  under `ui/graph` and `ui/controls`.
- `scripts/verify-modularity.sh` now includes Composure and SCD core in its
  two-workspace check list.

## Verification and limits

Crate-specific compile checks, focused DSP and pack tests, and Clippy checks
were used for changed processing crates. Damian's 80 crate tests, including all
27 UI tests, pass. Composure's 102 unit and 8 integration tests pass; SCD core
and plugin tests, Flattery's targeted DSP tests, and `pleasant-eq`'s four tests
also pass. The repository's full workspace test suite and a live DAW playback
session have not been run for this follow-up. The workspace-wide Rustfmt check
still reports formatting differences across untouched files, so it was not
applied to the whole repository. Existing public APIs with no in-repository
callers were retained because external consumers have not been audited.

Damian's release VST3 was built and installed to
`~/Library/Audio/Plug-Ins/VST3/Damian Channel Strip.vst3` using its required
`scripts/install.sh`; the installed binary matched the built bundle checksum.

The host forks remain under Damian's subtree while standalone subtree export
is in use. `SUBTREES.md` describes that ownership constraint. The unrelated,
pre-existing deletion of `nih-plug-disconnected-engine-sliders.md` was not part
of this work.
