use clap::{Parser, Subcommand};
use itertools::izip;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{
    fs::{self, File},
    io::{self, Write},
    mem,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    time::{self, Duration, Instant},
};
use tket2::{
    json::load_tk1_json_str,
    portmatching::{CircuitPattern, PatternMatcher},
};
use uuid::Uuid;
use walkdir::WalkDir;

use datasets::{ecc::ECCDataset, Dataset, NoGenFolderDataset, QasmAndJson};

use crate::datasets::random::RandomDataset;

mod datasets;
mod utils;

/// Command line arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    action: Actions,
}

#[derive(Subcommand)]
enum Actions {
    /// Generate the datasets. If empty, all default datasets are generated.
    Generate {
        /// Number of qubits in each of the random datasets.
        #[arg(short, long)]
        qubits: Vec<usize>,
        /// Number of gates in each of the random datasets.
        #[arg(short, long)]
        gates: Vec<usize>,
        /// Number of circuits in each of the random datasets.
        #[arg(short, long)]
        n_circuits: Vec<usize>,

        /// The ECC datasets to generate circuits from.
        #[arg(short, long)]
        ecc_datasets: Vec<PathBuf>,

        /// Whether to save the generated circuits to separate files.
        #[arg(short, long)]
        save_files: bool,

        /// Randomness seed.
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Run benchmarks on datasets. If empty, all default datasets are run.
    ///
    /// Run either `--quartz` or `--portmatching` or both. If none is provided,
    /// both are run.
    Run {
        /// Whether to benchmark the quartz pattern matching.
        #[arg(short, long)]
        quartz: bool,
        /// Whether to benchmark the portmatching pattern matching.
        #[arg(short, long)]
        portmatching: bool,
        /// The datasets to use for benchmarking.
        #[arg(short, long)]
        datasets: Vec<PathBuf>,
        /// The target circuit to run pattern matching on.
        ///
        /// Either JSON or QASM file
        target_file: PathBuf,
        /// Folder to save results in (default: results).
        #[arg(short, long)]
        output_folder: Option<String>,
    },
    /// Plot the results from the benchmarks.
    Plot {
        /// The folder containing the results (default: results/).
        #[arg(short, long)]
        results_folder: Option<PathBuf>,
        /// The file name for the saves plots (default: results/bench-plot.pdf).
        #[arg(short, long)]
        output_file: Option<PathBuf>,
    },
}

fn main() {
    match Cli::parse().action {
        Actions::Generate {
            mut qubits,
            mut gates,
            mut n_circuits,
            mut ecc_datasets,
            save_files,
            seed,
        } => {
            default_gen_params(&mut qubits, &mut gates, &mut n_circuits, &mut ecc_datasets);
            generate_ecc_datasets(ecc_datasets, save_files);
            let rng = SmallRng::seed_from_u64(seed.unwrap_or((1u64 << 32) - 1));
            generate_random_datasets(&qubits, &gates, &n_circuits, save_files, rng);
        }
        Actions::Run {
            mut quartz,
            mut portmatching,
            mut datasets,
            target_file,
            output_folder,
        } => {
            let output_folder = output_folder.unwrap_or("results".to_string());
            let target_circ = load_circ_file(&target_file);
            default_run_params(&mut datasets);
            let datasets: Vec<_> = datasets
                .into_iter()
                .map(|path| {
                    let path = path.with_extension("");
                    NoGenFolderDataset::new(path)
                })
                .collect();
            if !quartz && !portmatching {
                quartz = true;
                portmatching = true;
            }
            let bench_sizes = (200..=10000).step_by(200);
            for dataset in datasets {
                if quartz {
                    let bench_result = run_quartz(&dataset, &target_circ, bench_sizes.clone());
                    save_csv(
                        &output_folder,
                        "quartz",
                        &dataset.name(),
                        bench_sizes.clone(),
                        bench_result,
                    );
                }
                if portmatching {
                    let bench_result =
                        run_portmatching(&dataset, &target_circ, bench_sizes.clone());
                    save_csv(
                        &output_folder,
                        "portmatching",
                        &dataset.name(),
                        bench_sizes.clone(),
                        bench_result,
                    );
                }
            }
        }
        Actions::Plot {
            results_folder,
            output_file,
        } => {
            let results_folder = results_folder.unwrap_or(PathBuf::from("results"));
            let output_file = output_file.unwrap_or(PathBuf::from("results/bench-plot.pdf"));
            plot(&results_folder, &output_file);
        }
    };
}

fn default_gen_params(
    qubits: &mut Vec<usize>,
    gates: &mut Vec<usize>,
    n_circuits: &mut Vec<usize>,
    ecc_datasets: &mut Vec<PathBuf>,
) {
    assert!(qubits.len() == gates.len() && gates.len() == n_circuits.len());

    // If no params are provided, generate the default datasets
    if qubits.is_empty() && ecc_datasets.is_empty() {
        ecc_datasets.extend(
            DEFAULT_ECC_DATASETS
                .iter()
                .map(|path| PathBuf::from_str(path).unwrap()),
        );
        qubits.extend(DEFAULT_RANDOM_QB);
        gates.extend(DEFAULT_RANDOM_GATES);
        n_circuits.extend(DEFAULT_RANDOM_N_CIRC);
    }
}

const DEFAULT_ECC_DATASETS: &[&str] = &[
    "datasets/eccs/2_6-eccs.json",
    "datasets/eccs/3_6-eccs.json",
    "datasets/eccs/4_6-eccs.json",
];

const DEFAULT_RANDOM_QB: &[usize] = &[2, 3, 4, 6, 8, 10];
const DEFAULT_RANDOM_GATES: &[usize] = &[15, 15, 15, 15, 15, 15];
const DEFAULT_RANDOM_N_CIRC: &[usize] = &[10000, 10000, 10000, 10000, 10000, 10000];

fn generate_ecc_datasets(ecc_datasets: Vec<PathBuf>, save_files: bool) {
    let ecc_datasets = ecc_datasets.into_iter().map(|path| {
        let new_folder = path.with_extension("");
        ECCDataset::new(path, new_folder)
    });
    generate_datasets(ecc_datasets, save_files)
}

fn generate_random_datasets(
    qubits: &[usize],
    gates: &[usize],
    n_circuits: &[usize],
    save_files: bool,
    mut rng: impl Rng + Clone,
) {
    let random_datasets = izip!(n_circuits, qubits, gates).map(|(&n, &qb, &g)| {
        let folder = format!("datasets/random/{}_{}-random", qb, g,);
        let new_rng = SmallRng::from_rng(&mut rng).unwrap();
        RandomDataset::new(new_rng, n, qb, g, folder.into())
    });
    generate_datasets(random_datasets, save_files)
}

fn generate_datasets(datasets: impl IntoIterator<Item = impl Dataset>, save_files: bool) {
    let n_circs = datasets
        .into_iter()
        .map(|dataset| {
            println!("Generating dataset {}...", dataset.name());
            dataset.generate(save_files)
        })
        .sum::<usize>();
    println!("Generated {} circuits", n_circs);
}

include!("../quartz_bindings/bindings.rs");

fn run_portmatching(
    dataset: &impl Dataset,
    target: &QasmAndJson,
    bench_sizes: impl IntoIterator<Item = usize>,
) -> Vec<Duration> {
    // Load patterns
    println!("[portmatching] Loading patterns from {}...", dataset.name());
    let target_json = target.json().unwrap();
    let all_hugrs = dataset
        .iter_json()
        .map(|json| load_tk1_json_str(&json))
        .collect::<Result<Vec<_>, _>>()
        .expect("invalid JSON file");
    let all_patterns = all_hugrs
        .iter()
        .map(CircuitPattern::try_from_circuit)
        .collect::<Result<Vec<_>, _>>()
        .expect("invalid pattern");
    println!("\tLoaded {} patterns", all_patterns.len());

    // Load circuit
    let target_hugr = load_tk1_json_str(target_json).unwrap();

    let bench_sizes = bench_sizes.into_iter();
    let mut bench_results = Vec::with_capacity(bench_sizes.size_hint().0);
    println!("[portmatching] Pattern matching...");
    for n in bench_sizes.filter(|&n| n <= all_patterns.len()) {
        println!("\tn = {}", n);
        // TODO: store matcher as binary
        let matcher = PatternMatcher::from_patterns(all_patterns[..n].to_vec());
        let start_time = Instant::now();
        matcher.find_matches(&target_hugr);
        bench_results.push(start_time.elapsed());
    }
    bench_results
}

fn run_quartz(
    dataset: &impl Dataset,
    target: &QasmAndJson,
    bench_sizes: impl IntoIterator<Item = usize>,
) -> Vec<Duration> {
    use std::ffi::CString;

    let target_qasm = CString::new(target.qasm().unwrap()).unwrap();
    let graph = unsafe { load_graph(target_qasm.as_ptr()) };
    let mut n_ops = 0;
    let ops = unsafe { get_ops(graph, &mut n_ops) };

    println!("[quartz] Loading patterns from {}...", dataset.name());
    let patterns_qasm: Vec<_> = dataset
        .iter_qasm()
        .map(|qasm| CString::new(qasm).unwrap())
        .collect();
    let patterns_qasm_ptrs: Vec<_> = patterns_qasm.iter().map(|qasm| qasm.as_ptr()).collect();
    let n_xfers = patterns_qasm.len() as u32;
    let xfers = unsafe { load_xfers(patterns_qasm_ptrs.as_ptr(), n_xfers) };
    println!("\tLoaded {} patterns", n_xfers);

    let bench_sizes = bench_sizes.into_iter();
    let mut bench_results = Vec::with_capacity(bench_sizes.size_hint().0);
    println!("[quartz] Pattern matching...");
    for n in bench_sizes.filter(|&n| n <= n_xfers as usize) {
        println!("\tn = {}", n);
        let start_time = Instant::now();
        unsafe { pattern_match(graph, ops, n_ops, xfers, n as u32) };
        bench_results.push(start_time.elapsed());
    }

    // Free memory!
    unsafe {
        free_xfers(xfers, n_xfers);
        free_ops(ops);
        free_graph(graph);
    };

    bench_results
}

fn plot(results_folder: &PathBuf, output_file: &PathBuf) {
    let out = Command::new("python")
        .arg("py-scripts/plot.py")
        .arg("-r")
        .arg(results_folder)
        .arg("-o")
        .arg(output_file)
        .output()
        .unwrap();
    println!("{}", String::from_utf8(out.stdout).unwrap());
}

fn save_csv(
    output_folder: &str,
    bench_type: &str,
    dataset: &str,
    bench_sizes: impl IntoIterator<Item = usize>,
    bench_results: Vec<Duration>,
) {
    let file_path = Path::new(output_folder).join(format!("{bench_type}/{dataset}.csv"));
    if let Some(parent_path) = file_path.parent() {
        fs::create_dir_all(parent_path).unwrap();
    }
    let mut f = File::create(&file_path).expect("Unable to create file");

    writeln!(f, "size,duration").expect("Unable to write to file");

    for (size, duration) in bench_sizes.into_iter().zip(bench_results) {
        writeln!(f, "{},{}", size, duration.as_secs_f64()).expect("Unable to write to file");
    }
}

fn load_circ_file(target_file: &Path) -> QasmAndJson {
    let ext = target_file.extension().unwrap().to_str().unwrap();
    assert!(ext == "json" || ext == "qasm");
    let mut target = QasmAndJson::from_path(target_file);
    target.load().unwrap();
    target
}

/// If no datasets are provided, run all datasets for which `.bin` files exist.
fn default_run_params(datasets: &mut Vec<PathBuf>) {
    let mut folders = mem::replace(datasets, Vec::new());
    if folders.is_empty() {
        folders.push("datasets".into());
    }
    for folder in folders {
        let folder = folder.with_extension("");
        for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().unwrap() == "bin" {
                datasets.push(path.parent().unwrap().to_path_buf());
            }
        }
    }
    datasets.sort_unstable();
    datasets.dedup();
}
