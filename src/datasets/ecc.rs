use std::{
    ffi::CString,
    path::{Path, PathBuf}, fs,
};

use super::FolderDataset;

/// A circuit dataset obtained from an ECC file.
pub struct ECCDataset {
    circuit_folder: PathBuf,
    ecc_file: PathBuf,
}

impl ECCDataset {
    /// Creates a new ECC dataset from an ECC file.
    pub fn new(ecc_file: PathBuf, circuit_folder: PathBuf) -> Self {
        Self {
            ecc_file,
            circuit_folder,
        }
    }
}

impl FolderDataset for ECCDataset {
    fn unpack(&self) {
        fs::create_dir_all(&self.circuit_folder).unwrap();
        save_qasm(&self.ecc_file, &self.circuit_folder);
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
