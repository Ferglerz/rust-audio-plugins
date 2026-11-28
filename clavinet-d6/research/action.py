"""Build the portable action diagram from the restrike grid captures and jack WAVs."""
import base64
import json
from pathlib import Path

import numpy as np
import soundfile as sf


def write_action(probe):
    keys = {}
    for midi in (46, 64, 78):
        folder = probe / "restrike" / str(midi)
        motion = folder / "motion.csv"
        with motion.open() as source:
            next(source)
            geom = [float(v) for v in next(source).lstrip("# ").split(",")]
        data = np.loadtxt(motion, delimiter=",")
        # Every stored number after time is a distance. Quantize to quarter micrometres.
        values = np.rint(data[:, 1:] * 4e6)
        if not np.isfinite(values).all() or abs(values).max() > 32767:
            raise ValueError(f"Cannot encode the displacement range on key {midi}")
        keys[midi] = dict(geometry=geom, width=values.shape[1],
                          motion=base64.b64encode(values.astype("<i2").tobytes()).decode())
        audio = {mode: f"jack-{mode}.wav" for mode in ("CA", "CB", "DA", "DB")}
        audio.update(centre="centre.wav", bridge="bridge.wav")
        for mode, filename in audio.items():
            y, rate = sf.read(folder / filename)
            if rate != 44100 or abs(y).max() > 1:
                raise ValueError(f"Unexpected playback level or rate for key {midi} {mode}")
            keys[midi][mode] = base64.b64encode(np.rint(y * 32767).astype("<i2").tobytes()).decode()
    html = Path(__file__).with_name("action.html").read_text()
    comparisons = {}
    for take in ("attacks", "switches", "releases"):
        source = probe / f"jack-{take}-hardware-then-model.wav"
        comparisons[take] = base64.b64encode(source.read_bytes()).decode()
    html = html.replace("/*CAPTURES*/{}", json.dumps(keys))
    (probe / "action.html").write_text(html.replace("/*COMPARISONS*/{}", json.dumps(comparisons)))
