# Pleasant Voice

Shared off-audio-thread Silero VAD used by Damian Channel Strip and Automixer.
Extracted from Damian's existing detector, with the same 16 kHz / 512-sample
inference path, filtering, resampling and confidence smoothing.

`SpeechVad::start_worker()` preserves the PSE's always-ready behavior.
`start_worker_enabled(false)` prepares a sleeping worker for Automixer; enabling
assistance creates its ONNX session on that worker. The audio thread uses a
fixed lock-free queue and never performs inference. Failed initialization or
inference leaves confidence at zero, allowing normal level detection.

The embedded model is the original official Silero v6.2.1 asset:
SHA-256 `1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3`.
See `licenses/` for Silero and ONNX Runtime notices. License sources:
[Silero](https://github.com/snakers4/silero-vad/blob/master/LICENSE) and
[ONNX Runtime](https://github.com/microsoft/onnxruntime/blob/main/LICENSE).
