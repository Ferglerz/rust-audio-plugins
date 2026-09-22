# Damian Channel Strip

High-performance VST3 channel strip plugin written in Rust using [NIH-plug](https://github.com/robbert-vdh/nih-plug) and [Vizia](https://github.com/robbert-vdh/vizia) with FemtoVG hardware-accelerated rendering.

Designed with a sleek, flat charcoal UI, interactive dynamic EQ, classic vocal compression, and a signature gold cursive accent.

---

## Features

### 1. Parametric Dynamic EQ (FabFilter Pro-Q style)
- **Unlimited Bands**: Add as many dynamic or static EQ bands as needed.
- **Selectable EQ Shapes**: Click the shape name in a selected band's floating card to choose Bell, Low Shelf, High Shelf, Low Cut (high-pass), High Cut (low-pass), Notch, or Band Pass.
- **Variable Pass-Filter Order**: For Low Cut and High Cut, click the slope in the band card to choose orders 1–8: 6, 12, 18, 24, 30, 36, 42, or 48 dB/octave. The graph and solo audition follow the selected order. Existing sessions default to their original second-order (12 dB/octave) response.
  - Cuts use a Butterworth response at Q ≈ 0.707; Q adjusts resonance for orders 2–8. First-order cuts have no Q control.
  - Dynamics apply to bell and shelf shapes; notch, band-pass, and pass filters are static.
- **Intelligent Gesture-Based Band Creation**:
  - Double-click anywhere to create a band.
  - Frequency / position-based automatic shape inference:
    - **Low Cut** (< 50 Hz edge clicks below 0 dB)
    - **High Cut** (> 13 kHz edge clicks below 0 dB)
    - **Low Shelf** (low-frequency edge clicks at or above 0 dB, or pulling the curve at low frequencies)
    - **High Shelf** (high-frequency edge clicks at or above 0 dB, or pulling the curve at high frequencies)
    - **Notch** (clicks near bottom < -19 dB)
    - **Bell** (default anywhere on graph)
- **Dynamic Multiband Compression**:
  - Per-band threshold (-60 dB to 0 dB), ratio (1:1 to 20:1), attack (0.1 ms to 200 ms), release (10 ms to 2000 ms), and dynamic range (0 to 24 dB).
  - Real-time per-band gain reduction metering.
- **Interactive EQ Controls**:
  - Drag nodes horizontally for frequency (20 Hz - 20 kHz log scale), vertically for gain (-24 dB to +24 dB).
  - Mouse scroll anywhere over a node to adjust Q / filter bandwidth (0.15 to 18.0).
  - Click any parameter value to type an exact setting: frequency, gain, Q, band dynamics, or compressor/mix controls. Enter applies, Escape cancels, and Tab / Shift-Tab moves between fields. Unit suffixes such as Hz, kHz, dB, ms, s, %, and :1 are accepted where applicable.
  - Choose ±12 to ±72 dB in 12 dB steps using the scale menu below the left graph axis, or scroll over that axis. Band gain remains limited to ±24 dB.
  - The transparent selected-band card moves above or below the node to leave drag clearance. The headphone icon toggles solo audition.
  - Alt-click or double-click an existing node to toggle bypass.
  - Right-click or press Delete/Backspace to remove selected band.
  - Live 128-band FFT spectrum analyzer overlay with smooth peak decay.

### EQ Processing Modes
Select the processing mode in the bottom bar. Mode and resolution are saved as host parameters; existing sessions default to Zero Latency.

- **Zero Latency**: Original minimum-phase IIR EQ, with dynamic bands and no added delay.
- **Natural Phase (4×)**: Runs the IIR EQ and band dynamics at four times the host sample rate with windowed-sinc interpolation and anti-alias filtering. This reduces high-frequency bilinear warping and adds 64 samples of latency. It is our oversampled alternative, **not FabFilter's proprietary Natural Phase algorithm or an exact analog-response match**. The resampling filters gently roll off near Nyquist.
- **Linear Phase**: Symmetric FIR EQ using partitioned convolution. Choose Low, Medium, High, Very High, or Maximum resolution. At 44.1 kHz these add 3072, 5120, 9216, 17408, and 66560 samples respectively. FIR length scales with sample rate, rounded up to 1024-sample partitions. Higher resolutions improve low-frequency / high-Q accuracy but increase CPU, memory, latency, and possible pre-ringing. The graph displays the target curve; finite FIR resolution can soften narrow features.
  - **Static EQ only in this version**: dynamic bands retain their static gain, with envelope processing paused and shown as `DYN PAUSED`. Their dynamic settings remain saved and resume in the other modes. The voice compressor remains active in every mode.
  - Kernel design and allocation run on the background worker. Band edits warm the new filter and crossfade over 10 ms after its history fills, so long resolutions respond more slowly to edits.
- Latency is reported to the host. EQ bypass, global bypass, and solo audition use an aligned dry signal. Switching modes/resolutions changes latency and refills filter history; switch while stopped to avoid an audible interruption. This is not a seamless or sample-accurate processing-mode automation control.
- These are independent implementations inspired by the processing choices described in [FabFilter's documentation](https://www.fabfilter.com/help/pro-q/using/processingmode); they do not claim sonic equivalence to Pro-Q.

### 2. Vocal Compressor (RVox style)
- **Threshold, Ratio, Attack & Release Controls**:
  - Threshold from 0.0 down to -48.0 dB with optional automatic makeup gain compensation.
  - Variable compression Ratio from 1:1 to 20:1.
  - Attack control (0.1 ms to 100 ms) and Release control (10 ms to 2000 ms).
  - Toggle buttons for Soft Knee, Auto Makeup, and Linked Stereo processing.
- **Primary Source Enhancer (PSE)**:
  - Defaults to ZingZap’s RMS detector and C time constant (200 ms), with 10 dB depth, 3 dB hysteresis, and a 6 dB Hermite soft knee.
  - Right-click the PSE knob to replace the compressor strip with its detailed controls: Depth, Hysteresis, Knee, RMS/Peak detection, time constants A–F, and Listen SC. Use **Back** to return. These audio settings are saved with presets; appearance remains independent.
  - Single threshold knob: -80 dB (OFF) to -20 dB. The original gate parameter ID/range is retained for saved automation.
  - Shares SC HPF with the compressor; stereo linking uses the stronger filtered channel to avoid phase cancellation. PSE applies to the wet path and follows compressor section bypass.
- **Sidechain High-Pass Filter (HPF)**:
  - Adjustable 20 Hz to 500 Hz high-pass filter in the detector sidechain to eliminate plosives and low-end rumble from triggering over-compression.
- **Parallel Dry/Wet**:
  - Independent Dry and Wet level controls (0% to 100%) for parallel compression.
  - Large gain reduction meter and reduction bar.

### 3. Visual Styling & Branding
- Switchable charcoal dark and warm light themes with gently rounded panels and controls. The header theme toggle saves a user preference outside presets, so loading presets does not change the appearance.
- EQ and compressor power icons control section bypass. Global bypass is handled by the DAW; the plugin header has no bypass or undo/redo buttons.
- Dynamic color-coded EQ nodes and filled response curves.
- Flashy gold cursive **Damian Birdsey** signature at the top right rendered with custom calligraphy typography and metallic gradient.

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
