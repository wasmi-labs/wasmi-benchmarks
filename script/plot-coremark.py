# USAGE: python3 plot-coremark.py <input.csv> [-o <output.png>] [--title <title>]
#
# - Reads the <input.csv> file which has two columns and a row per Wasm runtime.
#   The first column represents the Wasm runtime's name, the second column its associated Coremark score.
# - Outputs a bar diagram in <output.png> for all the Coremark scores.
#   Defaults to <input-stem>.png in the current working directory.
#
# Example <input.csv> file:
#
# ```
# runtime,score
# Wasmi v0.31,880
# Wasmi v0.32,1277
# Wasmi 1.0,1763
# ```

import argparse
import csv
from pathlib import Path
import matplotlib.pyplot as plt

DEFAULT_TITLE = "Coremark"

def plot_coremark(csv_path: str, out_path: str, title: str):
    runtimes = []
    scores = []

    with open(csv_path, newline="") as f:
        reader = csv.DictReader(f)
        if "runtime" not in reader.fieldnames or "score" not in reader.fieldnames:
            raise ValueError("CSV must contain 'runtime' and 'score' columns")

        for row in reader:
            runtimes.append(row["runtime"])
            scores.append(int(row["score"]))

    plt.figure(figsize=(10, 5))

    # colors = ["#b58900", "#ddb600", "#ffd900"]  # dark yellow → medium gold → bright yellow
    # plt.bar(runtimes, scores, color=colors[:len(runtimes)])
    plt.bar(runtimes, scores)

    plt.title(title)
    plt.xlabel("Wasm Runtimes")
    plt.ylabel("Score (higher is better)")
    # plt.ylim(bottom=0)
    plt.ylim(0, max(scores) * 1.1)

    for idx, val in enumerate(scores):
        plt.text(idx, val, str(val), ha="center", va="bottom")

    plt.xticks(rotation=45, ha="right")
    plt.tight_layout()
    plt.savefig(out_path)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Plot Coremark scores from a CSV file as a bar diagram."
    )
    parser.add_argument("input", help="input CSV file with 'runtime' and 'score' columns")
    parser.add_argument(
        "-o",
        "--output",
        help="output PNG file (default: <input-stem>.png in the current working directory)",
    )
    parser.add_argument(
        "--title",
        default=DEFAULT_TITLE,
        help=f"plot title (default: {DEFAULT_TITLE!r})",
    )
    args = parser.parse_args()

    output = args.output
    if output is None:
        output = str(Path.cwd() / f"{Path(args.input).stem}.png")

    plot_coremark(args.input, output, args.title)
