use clap::{Parser, Subcommand};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    time::{Duration, Instant},
};
use tket2::{
    json::load_tk1_json_str,
    portmatching::{CircuitPattern, PatternMatcher},
};

use datasets::{ecc::ECCDataset, Dataset, QasmAndJson};

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
        /// The ECC datasets to generate circuits from.
        #[arg(short, long)]
        ecc_datasets: Vec<PathBuf>,
        /// Whether to save the generated circuits to separate files.
        #[arg(short, long)]
        save_files: bool,
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
        /// The ECC datasets to use for benchmarking.
        #[arg(short, long)]
        ecc_datasets: Vec<PathBuf>,
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
            mut ecc_datasets,
            save_files,
        } => {
            add_default_datasets(&mut ecc_datasets);
            generate_datasets(ecc_datasets, save_files)
        }
        Actions::Run {
            mut quartz,
            mut portmatching,
            mut ecc_datasets,
            target_file,
            output_folder,
        } => {
            let output_folder = output_folder.unwrap_or("results".to_string());
            let target_circ = load_circ_file(&target_file);
            add_default_datasets(&mut ecc_datasets);
            let all_datasets: Vec<_> = ecc_datasets
                .into_iter()
                .map(|path| {
                    let path = path.with_extension("");
                    ECCDataset::new(path)
                })
                .collect();
            if !quartz && !portmatching {
                quartz = true;
                portmatching = true;
            }
            let bench_sizes = (200..=10000).step_by(200);
            for dataset in all_datasets {
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
            plot(&results_folder, &output_file).unwrap();
        }
    };
}

const DEFAULT_ECC_DATASETS: &[&str] = &[
    "datasets/voqc-eccs/Voqc_7_3_complete_ECC_set.json",
    "datasets/voqc-eccs/Voqc_7_2_complete_ECC_set.json",
    "datasets/voqc-eccs/Voqc_7_1_complete_ECC_set.json",
];

fn generate_datasets(ecc_datasets: Vec<PathBuf>, save_files: bool) {
    let ecc_datasets = ecc_datasets.into_iter().map(|path| {
        let new_folder = path.with_extension("");
        println!("Generating dataset in {}...", new_folder.to_str().unwrap());
        fs::create_dir_all(&new_folder).unwrap();
        ECCDataset::from_ecc(&path, new_folder)
    });
    let n_circs = ecc_datasets
        .map(|dataset| dataset.generate(save_files))
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

fn plot(results_folder: &PathBuf, output_file: &PathBuf) -> io::Result<String> {
    Command::new("python")
        .arg("py-scripts/plot.py")
        .arg("-r")
        .arg(results_folder)
        .arg("-o")
        .arg(output_file)
        .output()
        .and_then(|output| {
            String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        })
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

fn add_default_datasets(ecc_datasets: &mut Vec<PathBuf>) {
    if ecc_datasets.is_empty() {
        ecc_datasets.extend(
            DEFAULT_ECC_DATASETS
                .iter()
                .map(|path| PathBuf::from_str(path).unwrap()),
        );
    }
}
