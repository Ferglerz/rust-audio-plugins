# SCD Rust (SoundChef Drums)

Rust-based drum sampler plugin built with `nih-plug`, `nih_plug_vizia`, and `pleasant-ui`.
Multi-mic sample playback, hi-hat CC pedal engine, kick sub-synthesizer, and mixer with Sends-On-Fader.

On macOS, run `./scripts/install.sh` from this directory to rebuild and install the CLAP and VST3 bundles through the repository's shared installer.

Packs store signed 16-bit PCM (`SCDPACK2`) and deduplicate WAV paths. The packer
requires 16-bit integer WAV sources and refuses implicit precision reduction.
Mono sources stay mono. Stereo sources stay stereo unless every frame has identical
left and right values, in which case only one channel is stored. Close, Mono, and Room sources with differing stereo channels are rejected.
Playback reads PCM directly from the mapped file and converts only active samples to float for DSP.
There is one supported pack format; old float packs must be rebuilt.
