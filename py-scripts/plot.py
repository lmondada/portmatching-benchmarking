import argparse
from pathlib import Path
import re

import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt

def create_eccs_plot(all_data):
    eccs_data = all_data[all_data['dataset type'] == 'eccs']
    col_order = sorted(eccs_data['n_qubits'].unique())

    sns.set_style('ticks', {
        'grid.linestyle': ':',
        'font.family': 'serif',
        'font.serif': ["Computer Modern Roman"],
    })
    sns.set_context("paper")

    order = ['Portmatching', 'Quartz']
    # Create a plot comparing portmatching with quartz
    f = sns.relplot(
        data=eccs_data,
        x="Number of patterns",
        y="Runtime (s)",
        style="Pattern matching algorithm",
        style_order=order,
        markers=["P", "^"],
        hue="Pattern matching algorithm",
        hue_order=order,
        col="n_qubits",
        col_order=col_order,
        palette="colorblind",
        # facet_kws={"sharex": False, "sharey": False},
    )
    for axs in f.axes:
        for ax in axs:
            ax.grid(axis='y')

    sns.move_legend(f, "upper left", bbox_to_anchor=(0.1, 0.9), frameon=True, alignment='left')
    plt.savefig("eccs-plot.pdf")

def create_random_plot(all_data):
    random_data = all_data[all_data['dataset type'] == 'random']
    random_data = random_data[random_data['Pattern matching algorithm'] == 'Portmatching']
    qubit_order = sorted(random_data['n_qubits'].unique())

    sns.set_style('ticks', {
        'grid.linestyle': ':',
        'font.family': 'serif',
        'font.serif': ["Computer Modern Roman"],
    })
    sns.set_context("paper")

    # Create a plot comparing portmatching across qubit counts
    f = sns.relplot(
        data=random_data,
        x="Number of patterns",
        y="Runtime (s)",
        hue="n_qubits",
        style="n_qubits",
        style_order=qubit_order,
        hue_order=qubit_order,
        # markers=["P", "^"],
        palette="crest_r",
    )

    for axs in f.axes:
        for ax in axs:
            ax.grid(axis='y')

    sns.move_legend(f, "upper left", bbox_to_anchor=(0.2, 0.95), frameon=True, alignment='left')
    plt.savefig("random-plot.pdf")

def get_results_folder():
    # Parse command line arguments
    parser = argparse.ArgumentParser(description="Generate plots for benchmarking data.")
    parser.add_argument(
        "-r", "--results_folder", default="results", help="Path to the results folder"
    )
    args = parser.parse_args()
    return Path(args.results_folder)

def load_data(results_folder):
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
    return all_data

def rename_data_columns(all_data):
    # Rename the columns for plotting
    renamed_data = all_data.rename(
        columns={
            "size": "Number of patterns",
            "duration": "Runtime (s)",
            "source": "Pattern matching algorithm",
        },
    )

    # Get the number of qubits from the dataset name
    def get_qb(s):
        # string format: "{n_qbs}_{n_gates}-{dataset_type}}"
        match = re.search(r'(\d+)_(\d+)-.+', s)
        if match:
            return match.group(1)
        else:
            print("Warning: could not extract number of qubits from dataset name")
            return s

    # Get the number of qubits from the dataset name
    def get_type(s):
        # string format: "{n_qbs}_{n_gates}-{dataset_type}}"
        match = re.search(r'(\d+)_(\d+)-(.+)', s)
        if match:
            return match.group(3)
        else:
            print("Warning: could not extract dataset type from dataset name")
            return s

    renamed_data["n_qubits"] = all_data["dataset"].apply(get_qb)
    renamed_data['dataset type'] = all_data['dataset'].apply(get_type)
    return renamed_data


# Path to the results folder
results_folder = get_results_folder()

# Load all CSV data
all_data = load_data(results_folder)

# Rename the columns for plotting
all_data = rename_data_columns(all_data)

# Create a plot comparing portmatching with quartz
create_eccs_plot(all_data)

# Create a plot comparing portmatching across qubit counts
create_random_plot(all_data)
