use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use itertools::Itertools;
use rand::Rng;
use union_find::{QuickUnionUf, UnionBySize, UnionFind};
use uuid::Uuid;

use super::FolderDataset;

/// Dataset of random circuits.
pub struct RandomDataset<R> {
    circuit_folder: PathBuf,
    n_circuits: usize,
    n_qubits: usize,
    n_gates: usize,
    rng: RefCell<R>,
}

impl<R: Rng> FolderDataset for RandomDataset<R> {
    fn unpack(&self) {
        fs::create_dir_all(&self.circuit_folder).unwrap();

        let mut seen_circs = HashSet::new();
        let mut n_circs = 0;
        let mut n_iter = 0;
        loop {
            // Generate random circuit, save if not seen before
            let Some(qasm) =
                random_circuit(self.n_qubits, self.n_gates, &mut *self.rng.borrow_mut())
            else {
                continue;
            };
            if seen_circs.insert(qasm.clone()) {
                let path = self.circuit_folder.join(format!("{}.qasm", Uuid::new_v4()));
                fs::write(path, qasm).unwrap();
                n_circs += 1;
                if n_circs == self.n_circuits {
                    break;
                }
            }
            // Make sure we are not in an infinite loop (if params are too small)
            n_iter += 1;
            if n_iter > 10 * self.n_circuits {
                panic!("Could not generate {} circuits", self.n_circuits);
            }
        }
    }

    fn circuit_folder(&self) -> &Path {
        &self.circuit_folder
    }
}

impl<R> RandomDataset<R> {
    /// Creates a new random dataset.
    pub fn new(
        rng: R,
        n_circuits: usize,
        n_qubits: usize,
        n_gates: usize,
        circuit_folder: PathBuf,
    ) -> Self {
        Self {
            rng: RefCell::new(rng),
            circuit_folder,
            n_circuits,
            n_qubits,
            n_gates,
        }
    }
}

fn random_circuit(n_qubits: usize, n_gates: usize, rng: &mut impl Rng) -> Option<String> {
    assert!(n_qubits <= n_gates + 1);
    let gates = (0..n_gates).map(|_| Gate::random(n_qubits, rng));
    let mut uf = QuickUnionUf::<UnionBySize>::new(n_qubits);

    let mut qasm = format!(
        r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[{}];"#,
        n_qubits
    );

    for g in gates {
        match g {
            Gate::Cx(a, b) => {
                uf.union(a, b);
                qasm.push_str(&format!("cx q[{}],q[{}];\n", a, b));
            }
            Gate::H(a) => qasm.push_str(&format!("h q[{}];\n", a)),
            Gate::T(a) => qasm.push_str(&format!("t q[{}];\n", a)),
            // Gate::Tdg(a) => qasm.push_str(&format!("tdg q[{}];\n", a)),
            Gate::Invalid => unreachable!(),
        }
    }
    // check that all qubits are connected
    (0..n_qubits)
        .map(|i| uf.find(i))
        .all_equal()
        .then_some(qasm)
}

enum Gate {
    Cx(usize, usize),
    H(usize),
    T(usize),
    // Tdg(usize),
    Invalid,
}

impl Gate {
    fn random(n_qubits: usize, rng: &mut impl Rng) -> Self {
        assert!(n_qubits > 0);
        let mut g = Self::Invalid;
        while !g.is_valid() {
            g = match rng.gen_range(0..3) {
                0 => Gate::Cx(rng.gen_range(0..n_qubits), rng.gen_range(0..n_qubits)),
                1 => Gate::H(rng.gen_range(0..n_qubits)),
                _ => Gate::T(rng.gen_range(0..n_qubits)),
                // _ => Gate::Tdg(rng.gen_range(0..n_qubits)),
            };
        }
        g
    }

    fn is_valid(&self) -> bool {
        match self {
            Gate::Invalid => false,
            Gate::Cx(a, b) => a != b,
            _ => true,
        }
    }
}
