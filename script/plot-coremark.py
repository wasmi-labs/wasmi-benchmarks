# USAGE: python3 plot-coremark.py <input.json> [-o <output.svg>] [--title <title>]
#
# - Reads the <input.json> file, a JSON object mapping a Wasm runtime's ID to its
#   associated Coremark score. This is the format printed by `cargo run --profile bench`.
#   Scores may be fractional and are rounded to the nearest integer.
# - Outputs a horizontal bar diagram in <output.svg> for all the Coremark scores,
#   with the highest score at the top and the lowest score at the bottom.
#   Defaults to <input-stem>.svg in the current working directory.
#
# Example <input.json> file:
#
# ```
# {
#   "wasmi-v0.31": 880.4,
#   "wasmi-v0.32": 1277.0,
#   "wasmi-v2.eager.checked": 1763.62
# }
# ```

import argparse
import json
from pathlib import Path
import matplotlib.pyplot as plt

DEFAULT_TITLE = "Coremark"

# Runtimes whose ID starts with this prefix are highlighted in the plot,
# and the colors used for them and all others.
HIGHLIGHT_ID_PREFIX = "wasmi-v2"
HIGHLIGHT_COLOR = "tab:orange"
DEFAULT_COLOR = "tab:blue"

# Maps a runtime ID, as emitted by the `coremark` binary, to its display label.
#
# The source of truth for both the IDs and the labels is `bins/plot.rs`
# (`VmAndConfig::from_str` and `VmAndConfig::label`); adding a runtime there
# requires adding it here as well.
RUNTIME_LABELS = {
    "dlr-wasm-interpreter": "DLR-wasm-interpreter",
    "fizzy": "Fizzy",
    "silverfir-nano.interpreter": "Silverfir-nano (interpreter)",
    "silverfir-nano.jit": "Silverfir-nano (JIT)",
    "spacewasm": "SpaceWasm",
    "stitch": "Stitch (lazy)",
    "submilli-wasm": "Submilli-wasm",
    "tinywasm": "Tinywasm",
    "toywasm": "Toywasm",
    "v8": "V8",
    "wamr": "WAMR fast-interpreter",
    "wasm3.eager": "Wasm3 (eager)",
    "wasm3.lazy": "Wasm3 (lazy)",
    "wasmedge": "WasmEdge (interpreter)",
    "wasmer.cranelift": "Wasmer (Cranelift)",
    "wasmer.singlepass": "Wasmer (Singlepass)",
    "wasmi-v0.31": "Wasmi 0.31",
    "wasmi-v0.32": "Wasmi 0.32",
    "wasmi-v1.eager.checked": "Wasmi 1.0 (eager)",
    "wasmi-v1.eager.unchecked": "Wasmi 1.0 (eager, unchecked)",
    "wasmi-v1.lazy.checked": "Wasmi 1.0 (lazy)",
    "wasmi-v1.lazy.unchecked": "Wasmi 1.0 (lazy, unchecked)",
    "wasmi-v1.lazy-translation.checked": "Wasmi 1.0 (lazy-translation)",
    "wasmi-v2.eager.checked": "Wasmi 2.0 (eager)",
    "wasmi-v2.eager.unchecked": "Wasmi 2.0 (eager, unchecked)",
    "wasmi-v2.lazy.checked": "Wasmi 2.0 (lazy)",
    "wasmi-v2.lazy.unchecked": "Wasmi 2.0 (lazy, unchecked)",
    "wasmi-v2.lazy-translation.checked": "Wasmi 2.0 (lazy-translation)",
    "wasmtime.cranelift": "Wasmtime (Cranelift)",
    "wasmtime.winch": "Wasmtime (Winch)",
    "wasmtime.pulley": "Wasmtime (Pulley)",
    "wasmz": "Wasmz",
}

# Layout: the figure height scales with the number of runtimes so that the bar
# thickness stays constant whether the input has 6 entries or 22.
FIG_WIDTH = 9.0
ROW_HEIGHT = 0.34   # inches of figure height per runtime
FIG_PADDING = 1.3   # inches for the title, x-axis label and ticks
BAR_HEIGHT = 0.68   # fraction of a row slot occupied by the bar

def plot_coremark(json_path: str, out_path: str, title: str):
    with open(json_path) as f:
        scores_by_id = json.load(f)

    if not isinstance(scores_by_id, dict):
        raise ValueError("JSON input must be an object mapping runtime ID to score")
    if not scores_by_id:
        raise ValueError("JSON input contains no runtimes")

    rows = []
    for runtime_id, score in scores_by_id.items():
        label = RUNTIME_LABELS.get(runtime_id)
        if label is None:
            raise ValueError(
                f"unknown runtime ID {runtime_id!r}; "
                f"add it to RUNTIME_LABELS (see `bins/plot.rs`)"
            )
        rows.append((runtime_id, label, round(float(score))))

    # Sorted ascending so that the highest score ends up at the top of the plot.
    rows.sort(key=lambda row: row[2])
    labels = [label for _, label, _ in rows]
    scores = [score for _, _, score in rows]

    fig, ax = plt.subplots(figsize=(FIG_WIDTH, FIG_PADDING + ROW_HEIGHT * len(rows)))

    colors = [
        HIGHLIGHT_COLOR if runtime_id.startswith(HIGHLIGHT_ID_PREFIX) else DEFAULT_COLOR
        for runtime_id, _, _ in rows
    ]
    bars = ax.barh(labels, scores, height=BAR_HEIGHT, color=colors)

    ax.set_title(title)
    ax.set_xlabel("Score (higher is better)")
    # Leaves room for the value labels rendered past the end of the longest bar.
    ax.set_xlim(0, max(scores) * 1.08)
    ax.bar_label(bars, padding=3)

    ax.grid(axis="x", color="0.9")
    ax.set_axisbelow(True)
    ax.spines[["top", "right", "left"]].set_visible(False)
    ax.tick_params(axis="y", length=0)

    fig.tight_layout()
    # The format is inferred from the `out_path` extension, so an explicit
    # `-o <file>.png` still renders a raster image.
    fig.savefig(out_path)
    plt.close(fig)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Plot Coremark scores from a JSON file as a bar diagram."
    )
    parser.add_argument("input", help="input JSON file mapping runtime ID to Coremark score")
    parser.add_argument(
        "-o",
        "--output",
        help="output SVG file (default: <input-stem>.svg in the current working directory)",
    )
    parser.add_argument(
        "--title",
        default=DEFAULT_TITLE,
        help=f"plot title (default: {DEFAULT_TITLE!r})",
    )
    args = parser.parse_args()

    output = args.output
    if output is None:
        output = str(Path.cwd() / f"{Path(args.input).stem}.svg")

    plot_coremark(args.input, output, args.title)
