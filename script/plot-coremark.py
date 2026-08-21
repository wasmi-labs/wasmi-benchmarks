# USAGE: python3 plot-coremark.py <input.csv> [-o <output.svg>] [--title <title>]
#
# - Reads the <input.csv> file which has two columns and a row per Wasm runtime.
#   The first column represents the Wasm runtime's name, the second column its associated Coremark score.
#   Scores may be fractional and are rounded to the nearest integer.
# - Outputs a horizontal bar diagram in <output.svg> for all the Coremark scores,
#   with the highest score at the top and the lowest score at the bottom.
#   Defaults to <input-stem>.svg in the current working directory.
#
# Example <input.csv> file:
#
# ```
# runtime,score
# Wasmi v0.31,880.4
# Wasmi v0.32,1277
# Wasmi 1.0,1763.62
# ```

import argparse
import csv
from pathlib import Path
import matplotlib.pyplot as plt

DEFAULT_TITLE = "Coremark"

# The runtime that is highlighted in the plot, and the colors used for it and all others.
HIGHLIGHT_RUNTIME = "wasmi v2"
HIGHLIGHT_COLOR = "tab:orange"
DEFAULT_COLOR = "tab:blue"

# Layout: the figure height scales with the number of runtimes so that the bar
# thickness stays constant whether the CSV has 6 rows or 22.
FIG_WIDTH = 9.0
ROW_HEIGHT = 0.34   # inches of figure height per runtime
FIG_PADDING = 1.3   # inches for the title, x-axis label and ticks
BAR_HEIGHT = 0.68   # fraction of a row slot occupied by the bar

def normalize_runtime(runtime: str) -> str:
    """Normalizes a runtime name so that e.g. `wasmi-v2` and `Wasmi v2` compare equal."""
    return runtime.strip().lower().replace("-", " ").replace("_", " ")

def plot_coremark(csv_path: str, out_path: str, title: str):
    rows = []

    with open(csv_path, newline="") as f:
        reader = csv.DictReader(f)
        if "runtime" not in reader.fieldnames or "score" not in reader.fieldnames:
            raise ValueError("CSV must contain 'runtime' and 'score' columns")

        for row in reader:
            rows.append((row["runtime"].strip(), round(float(row["score"]))))

    if not rows:
        raise ValueError("CSV contains no data rows")

    # Sorted ascending so that the highest score ends up at the top of the plot.
    rows.sort(key=lambda row: row[1])
    runtimes = [runtime for runtime, _ in rows]
    scores = [score for _, score in rows]

    fig, ax = plt.subplots(figsize=(FIG_WIDTH, FIG_PADDING + ROW_HEIGHT * len(rows)))

    colors = [
        HIGHLIGHT_COLOR if normalize_runtime(runtime) == HIGHLIGHT_RUNTIME else DEFAULT_COLOR
        for runtime in runtimes
    ]
    bars = ax.barh(runtimes, scores, height=BAR_HEIGHT, color=colors)

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
        description="Plot Coremark scores from a CSV file as a bar diagram."
    )
    parser.add_argument("input", help="input CSV file with 'runtime' and 'score' columns")
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
