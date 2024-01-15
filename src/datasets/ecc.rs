use std::{
    ffi::CString,
    path::{Path, PathBuf},
};

use super::CircuitDataset;

/// A circuit dataset obtained from an ECC file.
pub struct ECCDataset {
    circuit_folder: PathBuf,
}

impl ECCDataset {
    /// Creates a new ECC dataset from an ECC file.
    pub fn from_ecc(ecc_file: &Path, qasm_folder: PathBuf) -> Self {
        save_qasm(ecc_file, &qasm_folder);
        Self {
            circuit_folder: qasm_folder,
        }
    }

    pub fn new(circuit_folder: PathBuf) -> Self {
        Self { circuit_folder }
    }
}

impl CircuitDataset for ECCDataset {
    fn from_circuit_folder(folder: &Path) -> Self {
        Self {
            circuit_folder: folder.to_path_buf(),
        }
    }

    fn circuit_folder(&self) -> &Path {
        &self.circuit_folder
    }
}

// This file is generated from the C header file found in the same directory.
include!("../../quartz_bindings/bindings.rs");

/// Converts an ECC file to QASM files.
fn save_qasm(ecc_file: &Path, qasm_folder: &Path) {
    let ecc_file = CString::new(ecc_file.to_str().unwrap()).unwrap();
    let qasm_folder = CString::new(qasm_folder.to_str().unwrap()).unwrap();
    unsafe {
        ecc_to_qasm(ecc_file.as_ptr(), qasm_folder.as_ptr());
    }
}
