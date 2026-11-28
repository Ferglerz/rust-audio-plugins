"""Choose a common hammer and shim regulation on four manually played notes.

Compose changes to the isolated bearing and rail spectra, then check the selected
setup in a complete jack render. No per-harmonic gain correction is fitted.
"""
import argparse
import csv
from pathlib import Path

import numpy as np


TRAIN = (38, 54, 69, 74)
RESERVED = (43, 53, 67, 76)
KEYS = TRAIN + RESERVED


def candidate_spectra(path, first_partial):
    spectra = {}
    with path.open() as stream:
        rows = csv.reader(stream)
        next(rows)
        for row in rows:
            note = int(row[0])
            setting = tuple(map(float, row[1:first_partial]))
            amplitude = np.maximum(np.asarray(row[first_partial:], float), 1e-12)
            spectra[note, setting] = 20 * np.log10(amplitude[1:] / amplitude[0])
    return spectra


def spectrum_changes(candidates, initial):
    for setting in sorted({setting for _, setting in candidates}):
        difference = [candidates[note, setting] - candidates[note, initial] for note in KEYS]
        yield setting, np.array(difference)


def main(folder, baseline):
    hammers = candidate_spectra(folder / "hammer-candidates.csv", 4)
    shims = candidate_spectra(folder / "shim-candidates.csv", 3)
    hardware = {}
    unvoiced = {}
    with baseline.open() as stream:
        for row in csv.DictReader(stream):
            if row["use"] == "chord":
                continue
            key = int(row["midi"]), int(row["partial"])
            hardware[key] = float(row["owner_dBc"])
            unvoiced[key] = float(row["model_dBc"])

    def matrix(levels):
        return np.array([[levels[n, h] for h in range(2, 7)] for n in KEYS])

    reference = np.maximum(matrix(hardware), -40)
    initial = matrix(unvoiced)
    errors = abs(np.maximum(initial, -40) - reference)
    print("Baseline median error by note, fit then reserved", KEYS, np.round(np.median(errors, axis=1), 1))

    ranked = []
    shim_changes = list(spectrum_changes(shims, (3.2e6, 10.)))
    for hammer, bearing_change in spectrum_changes(hammers, (1., 1., 1.)):
        for shim, shim_change in shim_changes:
            predicted = initial + bearing_change + shim_change
            errors = abs(np.maximum(predicted, -40) - reference)
            # A smooth absolute loss limits the weight of a single large residual.
            loss = np.mean(np.sqrt(errors[:len(TRAIN)] ** 2 + 4) - 2)
            ranked.append((loss, hammer, shim, np.median(errors, axis=1)))
    for loss, hammer, shim, medians in sorted(ranked, key=lambda result: result[0])[:7]:
        print("Training loss", round(loss, 2), "felt, mass, strike", hammer,
              "shim stiffness/damping", shim, "median error by note", np.round(medians, 1))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path, help="candidate spectrum directory")
    parser.add_argument("baseline", type=Path,
                        help="hardware-harmonics.csv generated from the --unvoiced probe")
    args = parser.parse_args()
    main(args.folder, args.baseline)
