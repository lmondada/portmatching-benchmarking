import argparse
from pathlib import Path
import re

import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt

# Parse command line arguments
parser = argparse.ArgumentParser(description="Generate plots for benchmarking data.")
parser.add_argument(
    "-r", "--results_folder", default="results", help="Path to the results folder"
)
parser.add_argument(
    "-o", "--output_file", default="results/bench-plot.pdf", help="Name of the output file"
)
args = parser.parse_args()

# Define the path to the results folder
results_folder = Path(args.results_folder)

all_data = pd.DataFrame()

# Get the list of CSV files in the quartz subfolder
quartz_folder = results_folder / "quartz"
for file in quartz_folder.iterdir():
    if file.suffix == ".csv":
        data = pd.read_csv(file)
        data["source"] = "Quartz"
        data["dataset"] = file.stem
        all_data = pd.concat([all_data, data], ignore_index=True)

# Get the list of CSV files in the portmatching subfolder
portmatching_folder = results_folder / "portmatching"
for file in portmatching_folder.iterdir():
    if file.suffix == ".csv":
        data = pd.read_csv(file)
        data["source"] = "Portmatching"
        data["dataset"] = file.stem
        all_data = pd.concat([all_data, data], ignore_index=True)

# Rename the columns for plotting
all_data.rename(
    columns={
        "size": "Number of patterns",
        "duration": "Duration (s)",
        "source": "Pattern matching algorithm",
    },
    inplace=True,
)
# Get the number of qubits from the dataset name (assuming Voqc datasets)
def get_qb(s):
    match = re.search(r'Voqc_(\d+)_(\d+)_complete_ECC_set', s)
    if match:
        return match.group(2)
    else:
        print("Warning: could not extract number of qubits from dataset name")
        return s
all_data["n_qubits"] = all_data["dataset"].apply(get_qb)

# Get a sorted list of unique n_qubits values

col_order = sorted(all_data['n_qubits'].unique())

# Create a plot comparing portmatching with quartz
sns.relplot(
    data=all_data,
    x="Number of patterns",
    y="Duration (s)",
    hue="Pattern matching algorithm",
    col="n_qubits",
    # facet_kws={"sharex": False, "sharey": False},
    col_order=col_order
)
# plt.title('Comparison between portmatching and quartz')
plt.legend()

# Save the plot to a file
plt.savefig(args.output_file)
