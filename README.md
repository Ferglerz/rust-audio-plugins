# Damian Channel Strip

High-performance, zero-latency VST3 channel strip plugin written in Rust using [NIH-plug](https://github.com/robbert-vdh/nih-plug) and [Vizia](https://github.com/robbert-vdh/vizia) with FemtoVG hardware-accelerated rendering.

Designed with a sleek, flat charcoal UI, interactive dynamic EQ, classic vocal compression, and a signature gold cursive accent.

---

## Features

### 1. Parametric Dynamic EQ (FabFilter Pro-Q style)
- **Unlimited Bands**: Add as many dynamic or static EQ bands as needed.
- **Intelligent Gesture-Based Band Creation**:
  - Double-click anywhere to create a band.
  - Frequency / position-based automatic shape inference:
    - **Low Cut** (< 50 Hz edge clicks)
    - **High Cut** (> 13 kHz edge clicks)
    - **Low Shelf** (pulling the curve at low frequencies)
    - **High Shelf** (pulling the curve at high frequencies)
    - **Notch** (clicks near bottom < -19 dB)
    - **Bell** (default anywhere on graph)
- **Dynamic Multiband Compression**:
  - Per-band threshold (-60 dB to 0 dB), ratio (1:1 to 20:1), attack (0.1 ms to 200 ms), release (10 ms to 2000 ms), and dynamic range (0 to 24 dB).
  - Real-time per-band gain reduction metering.
- **Interactive EQ Controls**:
  - Drag nodes horizontally for frequency (20 Hz - 20 kHz log scale), vertically for gain (-24 dB to +24 dB).
  - Mouse scroll anywhere over a node to adjust Q / filter bandwidth (0.15 to 18.0).
  - Alt-click or double-click an existing node to toggle bypass.
  - Right-click or press Delete/Backspace to remove selected band.
  - Keyboard shortcuts: `Cmd+Z` / `Ctrl+Z` (Undo), `Shift+Cmd+Z` / `Cmd+Y` (Redo).
  - Live 128-band FFT spectrum analyzer overlay with smooth peak decay.

### 2. Vocal Compressor (RVox style)
- **Single-Knob Compression Drive**:
  - Smooth 0 to 48 dB compression drive with automatic makeup gain compensation.
  - Soft knee dynamics with fast vocal-tailored attack and release curves.
- **Sidechain High-Pass Filter (HPF)**:
  - Adjustable 20 Hz to 500 Hz high-pass filter in the detector sidechain to eliminate plosives and low-end rumble from triggering over-compression.
- **Built-in Expander / Gate**:
  - Downward expander with threshold down to -80 dB (off) for transparent noise floor gating.
- **Parallel Mix & Master Output**:
  - 0% to 100% dry/wet mix slider for parallel compression.
  - Master output trim (-24 dB to +24 dB) with peak overload indicators.
  - Large gain reduction meter and reduction bar.

### 3. Visual Styling & Branding
- Modern flat charcoal dark theme.
- Dynamic color-coded EQ nodes and filled response curves.
- Flashy gold cursive **Damian Birdsey** signature at the bottom right rendered with custom calligraphy typography and metallic gradient.

---

## Build & Installation

### Requirements
- macOS 12+ (Apple Silicon or Intel)
- Rust toolchain (managed via Cargo)

### Quick Start

```bash
# Build and install to ~/Library/Audio/Plug-Ins/VST3/
./scripts/install.sh

# Run all test suites and lint checks
./scripts/test.sh

# Bundle release VST3 only
./scripts/bundle.sh
```

---

## Architecture

- **`src/dsp.rs`**: High-precision biquad filter implementations, Coeff generators, sidechain detector, dynamic EQ envelope follower, and soft-knee vocal compressor with auto makeup gain.
- **`src/engine.rs`**: Audio engine handling real-time sample processing, lock-free bank crossfading between band changes, FFT spectrum analysis, and metering.
- **`src/band.rs`**: Band data structures, shape inference logic, and parameter sanitization.
- **`src/params.rs`**: NIH-plug parameter definitions, host DAW automation, and full preset/session state serialization.
- **`src/ui/mod.rs`**: Vizia GUI view implementing mouse interactions, drag handling, curve drawing, spectrum rendering, and gold cursive signature.
- **`xtask/`**: NIH-plug bundle task to generate signed `.vst3` macOS bundles.

---

## License

GPL-3.0-or-later
