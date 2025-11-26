# USAGE: python3 plot-coremark.py <input.csv> <output.png>
#
# - Reads the <input.csv> file which has two columns and a row per Wasm runtime.
#   The first column represents the Wasm runtime's name, the second column its associated Coremark score.
# - Outputs a bar diagram in <output.png> for all the Coremark scores.
#
# Example <input.csv> file:
# 
# ```
# runtime,score
# Wasmi v0.31,880
# Wasmi v0.32,1277
# Wasmi 1.0,1763
# ```

import sys
import csv
import matplotlib.pyplot as plt

def plot_coremark(csv_path: str, out_path: str):
    runtimes = []
    scores = []

    with open(csv_path, newline="") as f:
        reader = csv.DictReader(f)
        if "runtime" not in reader.fieldnames or "score" not in reader.fieldnames:
            raise ValueError("CSV must contain 'runtime' and 'score' columns")

        for row in reader:
            runtimes.append(row["runtime"])
            scores.append(float(row["score"]))

    plt.figure(figsize=(10, 5))

    # colors = ["#b58900", "#ddb600", "#ffd900"]  # dark yellow → medium gold → bright yellow
    # plt.bar(runtimes, scores, color=colors[:len(runtimes)])
    plt.bar(runtimes, scores)

    plt.title("Coremark - Apple M2 Pro - rustc 1.91.1 (ed61e7d7e 2025-11-07)")
    plt.xlabel("Wasmi Version")
    plt.ylabel("Score (higher is better)")
    plt.ylim(bottom=0)

    for idx, val in enumerate(scores):
        plt.text(idx, val, str(val), ha="center", va="bottom")

    plt.tight_layout()
    plt.savefig(out_path)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 plot-coremark.py <input.csv> <output.png>")
        sys.exit(1)

    input_csv = sys.argv[1]
    output_img = sys.argv[2]

    plot_coremark(input_csv, output_img)
