#!/usr/bin/env python3
"""Derive a one-hop, magnitude-mask-only monitoring graph from the shipped model.

Weights are unchanged. This is a latency/quality tradeoff, NOT an equivalent
reordering of the full deep filter. Requires onnx==1.19.1 and numpy.
"""
import hashlib
from pathlib import Path
import onnx
from onnx import helper, numpy_helper
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
SOURCE_HASH = "7f0575a5cec0ba4ffd8f8bd657e06d007e4ccdd955d76faab922b9d3291dc14b"
PREFIX_STATE = 14628  # normalization, both encoders, and magnitude decoder
BINS = 481
STATE = PREFIX_STATE + BINS * 2  # one unnormalized input spectrum


def export():
    source = ROOT / "assets/dpdfnet2_48khz_hr.onnx"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == SOURCE_HASH, "Source model changed; re-audit state offsets"
    model = onnx.load(source)
    producers = {out: node for node in model.graph.node for out in node.output}
    state_parts = list(producers["state_out"].input[:11])
    assert state_parts[-1] == "/model/erb_dec/emb_gru/gru.1/Reshape_1_output_0"
    constants = {v.name: numpy_helper.to_array(v) for v in model.graph.initializer}
    first_df = next(n for n in model.graph.node if n.name == "/model/df_dec/df_gru/gru.0/Slice")
    assert int(constants[first_df.input[1]][0]) == PREFIX_STATE
    mask = "/model/erb_dec/Pad_output_0"
    assert mask in producers
    extra = [
        helper.make_node("Slice", ["state_in", "live_start", "live_end"], ["live_previous_flat"]),
        helper.make_node("Reshape", ["live_previous_flat", "live_spec_shape"], ["live_previous"]),
        helper.make_node("Reshape", [mask, "live_gain_shape"], ["live_gain"]),
        helper.make_node("Mul", ["live_previous", "live_gain"], ["spec_e"]),
        helper.make_node("Reshape", ["spec", "live_flat_shape"], ["live_current_flat"]),
        helper.make_node("Concat", state_parts + ["live_current_flat"], ["state_out"], axis=0),
    ]
    for name, value in [("live_start", [PREFIX_STATE]), ("live_end", [STATE]),
                        ("live_spec_shape", [1, 1, BINS, 2]), ("live_gain_shape", [1, 1, BINS, 1]), ("live_flat_shape", [BINS * 2])]:
        model.graph.initializer.append(numpy_helper.from_array(np.array(value, dtype=np.int64), name))
    # Backward reachability removes the entire deep-filter decoder and delay path.
    nodes = [n for n in model.graph.node if not any(x in ("spec_e", "state_out") for x in n.output)] + extra
    required = {"spec_e", "state_out"}
    retained = []
    for node in reversed(nodes):
        if required.intersection(node.output):
            retained.append(node)
            required.update(node.input)
    del model.graph.node[:]
    model.graph.node.extend(reversed(retained))
    initializers = [v for v in model.graph.initializer if v.name in required]
    del model.graph.initializer[:]
    model.graph.initializer.extend(initializers)
    del model.graph.value_info[:]
    model.graph.input[1].type.tensor_type.shape.dim[0].dim_value = STATE
    model.graph.output[1].type.tensor_type.shape.dim[0].dim_value = STATE
    metadata = {p.key: p.value for p in model.metadata_props}
    metadata.update(profile="dereverb_live_mask_1hop", state_size=str(STATE),
                    model_delay_frames="1", source_sha256=SOURCE_HASH,
                    mode="Magnitude mask only; one frame alignment; original weights; no retraining")
    helper.set_model_props(model, metadata)
    onnx.checker.check_model(model, full_check=True)
    destination = ROOT / "assets/dpdfnet2_48khz_live.onnx"
    onnx.save(model, destination)
    print(f"{destination.name}: {len(model.graph.node)} nodes, state={STATE}, sha256={hashlib.sha256(destination.read_bytes()).hexdigest()}")


if __name__ == "__main__":
    export()
